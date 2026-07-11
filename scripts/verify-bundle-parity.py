#!/usr/bin/env python3
r"""
scripts/verify-bundle-parity.py — Catch missing-translation regressions
between React <Localized> consumers and Fluent locale bundles.

WHY
====

The i18n quality gate (`scripts/lint-i18n.sh` mirroring
`.github/workflows/ci.yml`) catches two leak categories of bug:
  1. `[i18n]` — a .id.ftl file is byte-identical to its .ftl sibling.
  2. `Attempt to override an existing message` — same key defined in
     two .ftl files in the joined bundle.

But neither catches the third class of bug, which is the most
expensive to debug: a React `<Localized id="...">` consumer references
a key that is missing from BOTH the en .ftl AND the id .id.ftl. The
result is `@fluent/react` warning `[id] did not match any messages`
fired at every render of that component, but the component still
renders its fallback content (the between-tag children), so:
  * the test suite passes (the warning is non-fatal),
  * the bundled UI looks correct on screen,
  * the production bundle ships the warning as console spam.

The SettingsPage display-section gap (4 keys: section-display,
field-card-size, field-font-size, field-font-smoothing) was the
canonical example — surfacing it required a multi-turn SettingsPage
vitest run to see the warning, then 5+ iterations of placement +
translation + comment-width polish before the keys landed.

THE FIX
=======

This script walks every literal `<Localized id="...">` site in
`ui/src/features/**/*.tsx` (and `.ts`), extracts the literal id, and
asserts:
  * the id is defined in `ui/src/locales/*.ftl` (English source),
  * the id is ALSO defined in `ui/src/locales/*.id.ftl` (Indonesian
    translation, which the project ships side-by-side rather than as
    a separate locale dir; see `ui/src/i18n/index.ts`).

A key missing in either locale is reported with feature-file + line
number so the fix author can navigate directly. Missing in BOTH is
listed under "missing-in-both" (often a feature-shipped-without-FTL
case and the most damaging class because bilingual test coverage
fails in two languages at once).

A separate count of `<Localized>` openings that the literal-id
pattern did NOT match surfaces programmatic `id={expr}` sites — a
refactor that moves a static id to a variable would otherwise
silently drop coverage. The count is documented as approximate (it
uses a permissive regex to avoid false negatives) so reviewers know
not to take it as a precise surface-area metric.

USAGE
=====

    python3 scripts/verify-bundle-parity.py                                    # strict: exit 1 if missing
    python3 scripts/verify-bundle-parity.py --verbose                          # list every <Localized id>, even OK ones
    python3 scripts/verify-bundle-parity.py --report-only                      # always exit 0 (ergonomic for human reports)
    python3 scripts/verify-bundle-parity.py --staged-only PATH …               # scan only the given files; intended for the pre-commit hook; exit 1 when a key is missing AND at least 1 eligible file was scanned, else exit 0

EXIT CODES
==========

  * 0  every <Localized id> has a key in both .ftl and .id.ftl. Also
        returned when (a) --report-only was passed (informational),
        (b) --staged-only was invoked but no eligible feature files
        were found (nothing checked, no regression introduced), or
        (c) no <Localized id> sites exist anywhere in the scan.
  * 1  at least one id is missing in one or both locales AND at least
        one eligible file was scanned. CI/pre-commit gate on $? to
        fail-closed against missing-translation bugs. The same exit
        semantics hold for every gate (full repo vs. --staged-only)
        once the scan produced at least one extractable site.
  * 2  a runtime error occurred (locales/feature dirs missing).

LIMITATIONS
===========

  * `--staged-only PATH …` reads the FULL post-stage file content
    (not the diff vs. HEAD). Staged files that touch a feature
    containing a 78-baseline-missing key WILL fail-closed — the
    gate is a forcing-function toward incremental baseline
    repair by any contributor who edits that feature. To strictly
    detect *new* missing keys (those introduced by this commit
    alone), diff HEAD vs. staged content separately; out of scope
    here.
  * Only resolves LITERAL `<Localized id="...">` references. Sites that
    pass id via template literal (`id={`prefix-${kind}`}`) or a JS
    variable (`id={SOME_KEY}`) cannot be statically checked; they are
    surfaced as "untracked" sites so the contributor knows about them.
  * Does not validate message `attrs={{...}}` attribute keys against
    `.attr = ...` definitions in the FTL. That is a smaller class of
    bug (placeholder / aria-label mismatches) and is out of scope here.
  * Fluent term definitions (`-brand-name = ...`) are not separately
    distinguished from regular message keys — both are reported under
    "missing key in .ftl/.id.ftl". Terms are rare in this repo.
  * The untracked count uses a permissive regex (`<Localized\b[^>]*>`)
    that ALSO matches JSX-shaped substrings inside string literals,
    comments, and helper constants. The report explicitly calls this
    an upper-bound estimate, not a precise metric.
"""

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FEATURE_DIR = ROOT / "ui" / "src" / "features"
LOCALE_DIR = ROOT / "ui" / "src" / "locales"

# Matcher for each literal `<Localized id="...">` site. Multi-line via
# DOTALL so the `[^>]*?` between tag-name and `id` attribute crosses
# newlines (e.g. `<Localized\n  attrs={...}\n  id="...">`).
LOCALIZED_ID_PATTERN = re.compile(
    r"<Localized\b[^>]*?\bid\s*=\s*(['\"])(?P<id>[^'\"]+)\1",
    flags=re.DOTALL,
)

# Counts `<Localized>` JSX-shaped openings (requires `>` terminator or
# attribute run, NOT just a bare substring match) so the untracked
# count is at least close to the real surface area. Still
# approximate — bubble-up dependencies that render `<Localized id={}>`
# would also match.
LOCALIZED_OPEN_PATTERN = re.compile(r"<Localized\b[^>]*>", flags=re.DOTALL)

# Match a top-level Fluent key OR term OR `#` comment at column 0.
# DEFINITION IS VERBATIM WITH `scripts/dedupe-ftl.py` so a key accepted
# as "exists" by dedupe is accepted identically here. Cross-reference,
# do not drift, in `KEY_PATTERN` updates.
KEY_PATTERN = re.compile(r"^([-a-zA-Z][a-zA-Z0-9_-]*)\s*=")

DESCRIPTION = (
    "Verify that every <Localized id=\"...\"> reference in React "
    "components has a matching key in both the en .ftl and the id "
    ".id.ftl locale bundles. Catches missing-translation regressions "
    "before they ship. See the module docstring for the algorithm "
    "and rationale."
)


def extract_ids_from_source(
    path: Path,
) -> tuple[list[tuple[str, int]], int]:
    """Return ([(id, line_number), ...], untracked_count) for one file.

    `line_number` is the 1-based line of the `id="..."` attribute,
    NOT the `<Localized>` opening — for multi-line JSX sites that
    keeps the breakpoint-attribution at the line a developer would
    actually walk to.

    `untracked_count` is the number of `<Localized>` openings the ID
    pattern did NOT match — i.e. sites with `id={...}` programmatic
    expressions. Surfacing this in the report ensures a refactor
    from literal id to variable visibly drops coverage rather than
    doing so silently.
    """
    text = path.read_text(encoding="utf-8")
    results: list[tuple[str, int]] = []
    open_count = sum(1 for _ in LOCALIZED_OPEN_PATTERN.finditer(text))
    literal_count = 0
    for match in LOCALIZED_ID_PATTERN.finditer(text):
        # Walk newlines from byte 0 up to the byte where the literal
        # id string starts; that line matches where the developer
        # wrote `id="..."` (not where `<Localized>` opens).
        line_number = text.count("\n", 0, match.start("id")) + 1
        results.append((match.group("id"), line_number))
        literal_count += 1
    # If this ever fires, LOCALIZED_ID_PATTERN is matching things
    # LOCALIZED_OPEN_PATTERN doesn't see — a real bug. Bare `max(0)`
    # would silently hide it; `assert` would be stripped under
    # `python3 -O`; an explicit RuntimeError is unconditionally loud.
    if literal_count > open_count:
        raise RuntimeError(
            f"LOCALIZED_ID_PATTERN matched {literal_count} sites but "
            f"LOCALIZED_OPEN_PATTERN counted {open_count} openings in {path}. "
            "Patterns are inconsistent — fix one of the regexes."
        )
    return results, open_count - literal_count


def parse_ftl_keys(path: Path) -> set[str]:
    """Return the set of distinct keys defined in one .ftl file.

    Multi-line message blocks (key + indented continuation) are
    collapsed to just the key, so downstream checks need only test
    key presence rather than re-tokenizing the value.
    """
    keys: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        m = KEY_PATTERN.match(line)
        if m:
            keys.add(m.group(1))
    return keys


def main() -> int:
    parser = argparse.ArgumentParser(description=DESCRIPTION)
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="Always exit 0; print a categorized report and return. "
             "Useful for human-readable summaries without failing CI. "
             "(This is the only one of the human-facing flags that "
             "changes exit-code semantics; default is already strict.)",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print every <Localized id> site, not just the missing ones.",
    )
    parser.add_argument(
        "--staged-only",
        action="store_true",
        help="Scan only files passed positionally (intended for the "
             "pre-commit hook). Bypasses the full ui/src/features/ "
             "rglob. Files outside FEATURE_DIR or nonexistent are "
             "skipped with a warning; if NO eligible feature files "
             "are found, exit 0 (informational: nothing verified, no "
             "regression introduced). When at least one eligible file "
             "is found, fail-closed on any missing key the same as "
             "the default mode — so a contributor can't slip a new "
             "<Localized id> through pre-commit by referencing an "
             "undefined key.",
    )
    parser.add_argument(
        "paths",
        nargs="*",
        help="Files to scan under --staged-only. Repo-relative paths "
             "(e.g. 'ui/src/features/foo.tsx'). Ignored without "
             "--staged-only.",
    )
    args = parser.parse_args()

    if not FEATURE_DIR.is_dir():
        print(f"error: features dir not found: {FEATURE_DIR}", file=sys.stderr)
        return 2
    if not LOCALE_DIR.is_dir():
        print(f"error: locales dir not found: {LOCALE_DIR}", file=sys.stderr)
        return 2

    # Load per-locale key sets. The `\*.id.ftl` suffix carries the
    # Indonesian twin of each English feature locale; we union each.
    en_files = sorted(p for p in LOCALE_DIR.glob("*.ftl") if not p.name.endswith(".id.ftl"))
    id_files = sorted(LOCALE_DIR.glob("*.id.ftl"))
    en_keys: set[str] = set()
    for path in en_files:
        en_keys.update(parse_ftl_keys(path))
    id_keys: set[str] = set()
    for path in id_files:
        id_keys.update(parse_ftl_keys(path))

    # Walk components, collect all <Localized id> sites with attribution.
    # Also accumulate untracked (programmatic-id) site count per file
    # so a future refactor that moves a static id to a runtime
    # expression visibly drops coverage rather than silently doing so.
    sites: list[tuple[str, str, int]] = []
    untracked_total = 0

    # --staged-only: scan only the files passed positionally; intended
    # for the pre-commit hook. Each positional is treated as a repo-
    # relative path. Files outside FEATURE_DIR are warned + skipped
    # (the script's job is feature-component checks; locale-side files
    # or non-JSX files aren't regression territory). Nonexistent paths
    # are warned + skipped (handles deletes + race conditions). If
    # every path was filtered out, exit 0 loudly; otherwise proceed
    # with the eligible subset under the same strict-mode semantics
    # as the default scan.
    if args.staged_only:
        staged: list[Path] = []
        for raw in args.paths:
            path = ROOT / raw
            if not path.exists():
                print(
                    f"warning: --staged-only path not found, skipping: {raw}",
                    file=sys.stderr,
                )
                continue
            try:
                # The following line is intentional: we test path
                # membership in FEATURE_DIR via relative_to() which
                # raises ValueError for non-descendants. The result is
                # discarded — only the side-effect is needed.
                _ = path.relative_to(FEATURE_DIR)
            except ValueError:
                print(
                    f"warning: --staged-only path outside "
                    f"{FEATURE_DIR}, skipping: {raw}",
                    file=sys.stderr,
                )
                continue
            staged.append(path)
        if not staged:
            print(
                f"verify-bundle-parity: --staged-only received "
                f"{len(args.paths)} path(s) but none are eligible "
                f"(in {FEATURE_DIR}); nothing to verify. Returning 0 "
                f"informational.",
                file=sys.stderr,
            )
            print("verify-bundle-parity: 0 missing key(s).")
            return 0
        source_files = sorted(staged)
    else:
        source_files = sorted(
            list(FEATURE_DIR.rglob("*.tsx")) + list(FEATURE_DIR.rglob("*.ts"))
        )
    for path in source_files:
        extracted, untracked = extract_ids_from_source(path)
        untracked_total += untracked
        relpath = path.relative_to(ROOT).as_posix()
        for id_, line in extracted:
            sites.append((id_, relpath, line))

    # Categorize.
    missing_in_en: list[tuple[str, str, int]] = []
    missing_in_id: list[tuple[str, str, int]] = []
    missing_in_both: list[tuple[str, str, int]] = []
    seen_ids = {id_ for id_, _, _ in sites}

    for id_, relpath, line in sites:
        in_en = id_ in en_keys
        in_id = id_ in id_keys
        if not in_en and not in_id:
            missing_in_both.append((id_, relpath, line))
        elif not in_en:
            missing_in_en.append((id_, relpath, line))
        elif not in_id:
            missing_in_id.append((id_, relpath, line))

    unique_missing_en = sorted({id_ for id_, _, _ in missing_in_en})
    unique_missing_id = sorted({id_ for id_, _, _ in missing_in_id})
    unique_missing_both = sorted({id_ for id_, _, _ in missing_in_both})

    # ---- Report ----
    print(
        f"verify-bundle-parity: scanned {len(source_files)} feature file(s), "
        f"{len(sites)} <Localized id> site(s), {len(seen_ids)} unique id(s), "
        f"{untracked_total} untracked opening(s) (programmatic id={...})."
    )
    print(f"  en bundle: {len(en_keys)} distinct key(s) across {len(en_files)} file(s)")
    print(f"  id bundle: {len(id_keys)} distinct key(s) across {len(id_files)} file(s)")
    print()

    if untracked_total > 0:
        print(
            f"  note: {untracked_total} <Localized> opening(s) used a programmatic "
            f"id={{...}} expression rather than a string literal; not statically "
            f"checkable. Approximate upper-bound: also matches string literals "
            f"and comments that contain JSX-shaped openers."
        )
        print()

    if args.verbose:
        print("  ok (in both bundles):")
        for id_ in sorted(seen_ids - set(unique_missing_en) - set(unique_missing_id) - set(unique_missing_both)):
            occurrences = [
                (relpath, line) for sid, relpath, line in sites if sid == id_
            ]
            for relpath, line in occurrences:
                print(f"    [{relpath}:{line}] {id_}")
        print()

    if missing_in_both:
        print(
            f"  missing in BOTH en .ftl AND id .id.ftl "
            f"({len(unique_missing_both)} unique):"
        )
        for id_ in unique_missing_both:
            for relpath, line in [
                (r, l) for sid, r, l in missing_in_both if sid == id_
            ]:
                print(f"    [{relpath}:{line}] {id_}")
        print()

    if missing_in_en:
        print(f"  missing in en .ftl only ({len(unique_missing_en)} unique):")
        for id_ in unique_missing_en:
            for relpath, line in [
                (r, l) for sid, r, l in missing_in_en if sid == id_
            ]:
                print(f"    [{relpath}:{line}] {id_}")
        print()

    if missing_in_id:
        print(f"  missing in id .id.ftl only ({len(unique_missing_id)} unique):")
        for id_ in unique_missing_id:
            for relpath, line in [
                (r, l) for sid, r, l in missing_in_id if sid == id_
            ]:
                print(f"    [{relpath}:{line}] {id_}")
        print()

    # Emit ONE unambiguous sentinel line as the LAST stdout line in
    # both clean and missing modes. Lint-i18n.sh and CI greps on the
    # exact pattern `^verify-bundle-parity: [1-9][0-9]* missing` so
    # the gate is robust to bucket rename / new bucket additions —
    # the sentinel is the ONLY thing the lint depends on; the body
    # of the report (bucket names, file:line entries) can grow
    # freely without breaking the gate.
    total_missing = (
        len(unique_missing_en)
        + len(unique_missing_id)
        + len(unique_missing_both)
    )
    print(f"verify-bundle-parity: {total_missing} missing key(s).")
    # Default + --dry-run both fail-closed so CI/pre-commit block
    # the regression. --report-only succeeds regardless so human
    # readers can audit at their leisure; a clean report (0 missing)
    # also returns 0 so it is never a gate failure.
    return 0 if (args.report_only or total_missing == 0) else 1


if __name__ == "__main__":
    sys.exit(main())
