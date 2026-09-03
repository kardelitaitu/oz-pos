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

SCOPE EXTENSION (rev 2, Fluent page audit)
==========================================

The rev-1 surface (literal `<Localized id="...">` under
`ui/src/features/**`) was proven insufficient: an audit of all 41
registered pages + 8 gate screens found 14 keys that ship broken today
while `verify-bundle-parity.py` reported `0 missing key(s)`:

  * 11 `l10n.getString('key')` call sites whose key exists in NO bundle
    (`getString` returns null → React renders nothing → validation
    errors and ARIA names silently vanish), and
  * 3 `registerNavItem({ i18nKey: 'nav-…' })` values with no bundle
    entry (`AppLayout` falls back to the raw English `label`, so
    Indonesian users see English sidebar entries).

Three new opt-in surfaces close that gap. They are OFF by default so
the existing pre-commit contract is unchanged:

  --include-getstring   literal `getString('k')` and
                        `requiredLocalized(l10n, 'k')` sites
  --include-nav-keys    `registerNavItem({ i18nKey: 'k' })` plus every
                        string value in `SECTION_LABELS`
  --scan-dirs A,B,...   comma-separated dirs under `ui/src`
                        (default: `features`)
  --full-census         implies all of the above over
                        features, components, frontend, contexts,
                        hooks, platform

Template-literal `getString(\`prefix-${x}\`)` sites cannot be resolved
statically; like programmatic `<Localized id={expr}>` openings they are
counted and reported so coverage loss stays visible.

USAGE
=====

    python3 scripts/verify-bundle-parity.py                                    # strict: exit 1 if missing
    python3 scripts/verify-bundle-parity.py --verbose                          # list every <Localized id>, even OK ones
    python3 scripts/verify-bundle-parity.py --report-only                      # always exit 0 (ergonomic for human reports)
    python3 scripts/verify-bundle-parity.py --staged-only PATH …               # scan only the given files; intended for the pre-commit hook; exit 1 when a key is missing AND at least 1 eligible file was scanned, else exit 0
    python3 scripts/verify-bundle-parity.py --full-census --report-only        # whole-ui census: Localized + getString + nav keys

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
  * Only resolves LITERAL key references. Sites that pass id via
    template literal (`id={`prefix-${kind}`}`,
    `getString(`inv-log-type-${tx.type}`)`) or a JS variable
    (`id={SOME_KEY}`) cannot be statically checked; they are surfaced
    as "untracked" sites so the contributor knows about them.
  * The rev-2 surfaces (`getString`, `requiredLocalized`, `i18nKey`,
    `SECTION_LABELS`) are opt-in. Until the repo is clean under
    `--full-census`, the pre-commit hook and CI keep running the rev-1
    default so no existing contract changes.
  * Does not validate message `attrs={{...}}` attribute keys against
    `.attr = ...` definitions in the FTL. That is a smaller class of
    bug (placeholder / aria-label mismatches) and is out of scope here.
  * Fluent term definitions (`-brand-name = ...`) are not separately
    distinguished from regular message keys — both are reported under
    "missing key in .ftl/.id.ftl". Terms are rare in this repo.
  * The untracked count uses a permissive regex (`<Localized\b[^>]*>`)
    that ALSO matches JSX-shaped substrings inside string literals.
    Comments are blanked before matching (block comments and whole-line
    `//` comments), so quoted example syntax in prose no longer inflates
    the number. It remains an upper-bound estimate, not a precise
    metric.
"""

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UI_SRC_DIR = ROOT / "ui" / "src"
FEATURE_DIR = UI_SRC_DIR / "features"
LOCALE_DIR = UI_SRC_DIR / "locales"

# rev 2: which ui/src subdirectories are walked. `features` alone
# reproduces the rev-1 contract exactly.
DEFAULT_SCAN_DIRS: tuple[str, ...] = ("features",)
CENSUS_SCAN_DIRS: tuple[str, ...] = (
    "features",
    "components",
    "frontend",
    "contexts",
    "hooks",
    "platform",
)

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

# ── rev 2 surfaces ───────────────────────────────────────────────────
# `l10n.getString('key')` / `getString("key")` — the imperative lookup
# path. A miss returns null, which React renders as nothing: the copy
# simply disappears instead of warning.
GETSTRING_ID_PATTERN = re.compile(
    r"\.getString\(\s*(['\"])(?P<id>[^'\"]+)\1", flags=re.DOTALL
)
# `requiredLocalized(l10n, 'key')` — the project's fallback-to-id helper.
REQUIRED_LOCALIZED_ID_PATTERN = re.compile(
    r"\brequiredLocalized\(\s*[A-Za-z0-9_.]+\s*,\s*(['\"])(?P<id>[^'\"]+)\1",
    flags=re.DOTALL,
)
# `registerNavItem({ …, i18nKey: 'nav-…', … })` — sidebar labels. A miss
# makes AppLayout fall back to the raw English `label`.
NAV_I18NKEY_PATTERN = re.compile(
    r"\bi18nKey\s*:\s*(['\"])(?P<id>[^'\"]+)\1", flags=re.DOTALL
)
# The SECTION_LABELS record in platform/ui/menu-registry: every value is
# a Fluent key used as a sidebar group heading.
SECTION_LABELS_PATTERN = re.compile(
    r"\bSECTION_LABELS\s*:\s*Record<[^>]*>\s*=\s*\{(?P<body>.*?)\}",
    flags=re.DOTALL,
)
SECTION_LABEL_VALUE_PATTERN = re.compile(r":\s*(['\"])(?P<v>[^'\"]+)\1")
# Fluent ids stored in an object-literal field and looked up later:
#   { key: 'revenue', titleKey: 'analytics-card-revenue', … }   → getString(card.titleKey)
#   { key: 'dinein',  labelId: 'kds-settings-color-dinein', … } → <Localized id={labelId}>
# The lookup site passes a *variable*, so neither GETSTRING_ID_PATTERN nor
# the <Localized> walker can resolve it, and the literal itself sits in a
# plain object with no call syntax to anchor on. This is the third dynamic
# class, and by count the largest: AnalyticsScreen alone stores 36.
KEY_FIELD_ID_PATTERN = re.compile(
    r"\b(?P<field>titleKey|descKey|labelId|nameKey|ariaKey|placeholderKey)"
    r"\s*:\s*(['\"])(?P<id>[A-Za-z0-9][A-Za-z0-9._-]*)\2"
)
# Dynamic getString — unresolvable, but its volume must stay visible.
GETSTRING_TEMPLATE_PATTERN = re.compile(r"\.getString\(\s*`")

# ── comment suppression ──────────────────────────────────────────────
# Prose in comments routinely quotes real lookup syntax (`// wrote
# l10n.getString('key') || 'English'` in requiredLocalized.ts is a
# documented example, not a call site). Without suppression the census
# reports phantom missing keys. Only BLOCK comments and WHOLE-LINE `//`
# comments are blanked: a mid-line `//` is far more likely to be part of
# a URL string literal than a comment, and dropping it would hide real
# keys — a false negative, which is worse for a fail-closed gate.
BLOCK_COMMENT_PATTERN = re.compile(r"/\*.*?\*/", flags=re.DOTALL)
FULL_LINE_COMMENT_PATTERN = re.compile(r"^[ \t]*//[^\n]*", flags=re.MULTILINE)


def _blank(match: re.Match) -> str:
    """Replace a comment with same-length whitespace (newlines kept).

    Preserving byte offsets means every reported line number still
    points at the real source line.
    """
    return "".join(ch if ch == "\n" else " " for ch in match.group(0))


def strip_comments(text: str) -> str:
    text = BLOCK_COMMENT_PATTERN.sub(_blank, text)
    return FULL_LINE_COMMENT_PATTERN.sub(_blank, text)

# Match a top-level Fluent key OR term OR `#` comment at column 0.
# DEFINITION IS VERBATIM WITH `scripts/dedupe-ftl.py` so a key accepted
# as "exists" by dedupe is accepted identically here. Cross-reference,
# do not drift, in `KEY_PATTERN` updates.
KEY_PATTERN = re.compile(r"^([-a-zA-Z][a-zA-Z0-9_-]*)\s*=")

DESCRIPTION = (
    "Verify that every literal Fluent key reference in React components "
    "— <Localized id=\"...\"> by default, plus optionally getString(), "
    "requiredLocalized(), registerNavItem i18nKey and SECTION_LABELS — "
    "has a matching key in both the en .ftl and the id .id.ftl locale "
    "bundles. Catches missing-translation regressions before they ship. "
    "See the module docstring for the algorithm and rationale."
)


def extract_ids_from_source(
    path: Path,
) -> tuple[list[tuple[str, int]], int]:
    """Return ([(id, line_number), ...], untracked_count) for one file.

    Kept as the rev-1 `<Localized id="...">`-only entry point so the
    default scan path is provably unchanged; `extract_sites_from_source`
    is the generalised form used when rev-2 surfaces are enabled.

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
    text = strip_comments(path.read_text(encoding="utf-8"))
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


# Human-facing label for each reportable key surface.
KIND_LABELS = {
    "localized": "<Localized id>",
    "getstring": "getString()",
    "required": "requiredLocalized()",
    "navkey": "nav i18nKey",
    "section": "SECTION_LABELS",
    "keyfield": "key-field literal",
}


def extract_sites_from_source(
    path: Path,
    kinds: set[str],
) -> tuple[list[tuple[str, str, int]], int, int]:
    """Return ([(kind, id, line)], untracked_localized, dynamic_getstring).

    `kinds` selects which surfaces to read; see KIND_LABELS. Line
    numbers always point at the literal itself, never at the enclosing
    call, so `[file:line]` in the report is directly clickable.
    """
    text = strip_comments(path.read_text(encoding="utf-8"))
    sites: list[tuple[str, str, int]] = []

    untracked_localized = 0
    if "localized" in kinds:
        ids, untracked_localized = extract_ids_from_source(path)
        sites.extend(("localized", id_, line) for id_, line in ids)

    dynamic_getstring = 0
    if "getstring" in kinds:
        for pattern in (GETSTRING_ID_PATTERN, REQUIRED_LOCALIZED_ID_PATTERN):
            for match in pattern.finditer(text):
                line = text.count("\n", 0, match.start("id")) + 1
                kind = "getstring" if pattern is GETSTRING_ID_PATTERN else "required"
                sites.append((kind, match.group("id"), line))
        dynamic_getstring = sum(1 for _ in GETSTRING_TEMPLATE_PATTERN.finditer(text))

    if "navkey" in kinds:
        for match in NAV_I18NKEY_PATTERN.finditer(text):
            line = text.count("\n", 0, match.start("id")) + 1
            sites.append(("navkey", match.group("id"), line))
        for block in SECTION_LABELS_PATTERN.finditer(text):
            body = block.group("body")
            offset = block.start("body")
            for value in SECTION_LABEL_VALUE_PATTERN.finditer(body):
                line = text.count("\n", 0, offset + value.start("v")) + 1
                sites.append(("section", value.group("v"), line))

    if "keyfield" in kinds:
        for match in KEY_FIELD_ID_PATTERN.finditer(text):
            line = text.count("\n", 0, match.start("id")) + 1
            sites.append(("keyfield", match.group("id"), line))

    return sites, untracked_localized, dynamic_getstring


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


def _is_descendant(path: Path, directory: Path) -> bool:
    """True when `path` lives under `directory` (either may be relative)."""
    try:
        path.relative_to(directory)
    except ValueError:
        return False
    return True


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
        "--include-getstring",
        action="store_true",
        help="Also check literal getString('k') and requiredLocalized(l10n, 'k') "
             "key references (rev-2 surface; off by default).",
    )
    parser.add_argument(
        "--include-nav-keys",
        action="store_true",
        help="Also check registerNavItem({ i18nKey }) values and every "
             "SECTION_LABELS value (rev-2 surface; off by default).",
    )
    parser.add_argument(
        "--include-key-fields",
        action="store_true",
        help="Also check Fluent ids stored in object-literal fields "
             "(titleKey/descKey/labelId/…) and resolved through a variable "
             "(rev-3 surface; off by default).",
    )
    parser.add_argument(
        "--scan-dirs",
        default=None,
        help="Comma-separated directories under ui/src to walk "
             f"(default: {','.join(DEFAULT_SCAN_DIRS)}).",
    )
    parser.add_argument(
        "--full-census",
        action="store_true",
        help="Shorthand for --include-getstring --include-nav-keys "
             "--include-key-fields "
             f"--scan-dirs {','.join(CENSUS_SCAN_DIRS)}.",
    )
    parser.add_argument(
        "paths",
        nargs="*",
        help="Files to scan under --staged-only. Repo-relative paths "
             "(e.g. 'ui/src/features/foo.tsx'). Ignored without "
             "--staged-only.",
    )
    args = parser.parse_args()

    if args.full_census:
        args.include_getstring = True
        args.include_nav_keys = True
        args.include_key_fields = True
        args.scan_dirs = ",".join(CENSUS_SCAN_DIRS)

    kinds: set[str] = {"localized"}
    if args.include_getstring:
        kinds |= {"getstring", "required"}
    if args.include_nav_keys:
        kinds |= {"navkey", "section"}
    if args.include_key_fields:
        kinds |= {"keyfield"}

    dir_names = (
        DEFAULT_SCAN_DIRS
        if not args.scan_dirs
        else tuple(d.strip() for d in args.scan_dirs.split(",") if d.strip())
    )
    scan_dirs: list[Path] = []
    for name in dir_names:
        candidate = UI_SRC_DIR / name
        if not candidate.is_dir():
            print(f"error: scan dir not found: {candidate}", file=sys.stderr)
            return 2
        scan_dirs.append(candidate)

    if not UI_SRC_DIR.is_dir():
        print(f"error: ui src dir not found: {UI_SRC_DIR}", file=sys.stderr)
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

    # Walk components, collect every enabled key surface with
    # attribution as (kind, key, file, line). Also accumulate the
    # untracked (programmatic-id) and dynamic-template counts per file so
    # a future refactor that moves a static id to a runtime expression
    # visibly drops coverage rather than silently doing so.
    sites: list[tuple[str, str, str, int]] = []
    untracked_total = 0
    dynamic_getstring_total = 0

    # --staged-only: scan only the files passed positionally; intended
    # for the pre-commit hook. Each positional is treated as a repo-
    # relative path. Files outside the selected scan dirs are warned +
    # skipped (the script's job is component key checks; locale-side
    # files or non-JSX files aren't regression territory). Nonexistent
    # paths are warned + skipped (handles deletes + race conditions). If
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
            if not any(_is_descendant(path, d) for d in scan_dirs):
                print(
                    f"warning: --staged-only path outside "
                    f"{', '.join(str(d) for d in scan_dirs)}, skipping: {raw}",
                    file=sys.stderr,
                )
                continue
            staged.append(path)
        if not staged:
            print(
                f"verify-bundle-parity: --staged-only received "
                f"{len(args.paths)} path(s) but none are eligible "
                f"(in {', '.join(str(d) for d in scan_dirs)}); nothing to "
                f"verify. Returning 0 informational.",
                file=sys.stderr,
            )
            print("verify-bundle-parity: 0 missing key(s).")
            return 0
        source_files = sorted(staged)
    else:
        source_files = sorted(
            {
                p
                for d in scan_dirs
                for p in list(d.rglob("*.tsx")) + list(d.rglob("*.ts"))
            }
        )
    for path in source_files:
        extracted, untracked, dynamic_gs = extract_sites_from_source(path, kinds)
        untracked_total += untracked
        dynamic_getstring_total += dynamic_gs
        relpath = path.relative_to(ROOT).as_posix()
        for kind, id_, line in extracted:
            sites.append((kind, id_, relpath, line))

    # Categorize. A site is (surface-kind, key, file, line) so the report
    # can attribute a miss to the lookup path that produced it — a
    # `<Localized id>` miss and a `getString` miss fail very differently
    # at runtime (fallback children vs. rendered nothing).
    missing_in_en: list[tuple[str, str, str, int]] = []
    missing_in_id: list[tuple[str, str, str, int]] = []
    missing_in_both: list[tuple[str, str, str, int]] = []
    seen_ids = {id_ for _, id_, _, _ in sites}

    for kind, id_, relpath, line in sites:
        in_en = id_ in en_keys
        in_id = id_ in id_keys
        if not in_en and not in_id:
            missing_in_both.append((kind, id_, relpath, line))
        elif not in_en:
            missing_in_en.append((kind, id_, relpath, line))
        elif not in_id:
            missing_in_id.append((kind, id_, relpath, line))

    def _unique(bucket: list[tuple[str, str, str, int]]) -> list[tuple[str, str]]:
        return sorted({(id_, kind) for kind, id_, _, _ in bucket})

    unique_missing_en = _unique(missing_in_en)
    unique_missing_id = _unique(missing_in_id)
    unique_missing_both = _unique(missing_in_both)

    # ---- Report ----
    print(
        f"verify-bundle-parity: scanned {len(source_files)} file(s) in "
        f"[{', '.join(d.name for d in scan_dirs)}], "
        f"{len(sites)} key site(s) across {len(kinds)} surface(s), "
        f"{len(seen_ids)} unique key(s), "
        f"{untracked_total} untracked <Localized> opening(s) "
        f"(programmatic id={{...}}), "
        f"{dynamic_getstring_total} dynamic getString(`…`) site(s)."
    )
    print(
        "  surfaces checked: "
        + ", ".join(KIND_LABELS[k] for k in sorted(kinds))
    )
    print(f"  en bundle: {len(en_keys)} distinct key(s) across {len(en_files)} file(s)")
    print(f"  id bundle: {len(id_keys)} distinct key(s) across {len(id_files)} file(s)")
    print()

    if untracked_total > 0 or dynamic_getstring_total > 0:
        print(
            f"  note: {untracked_total} <Localized> opening(s) used a programmatic "
            f"id={{...}} expression and {dynamic_getstring_total} getString() call(s) "
            f"used a template literal; neither is statically checkable. Approximate "
            f"upper-bound: also matches string literals that contain JSX-shaped "
            f"openers (comments are blanked before matching)."
        )
        print()

    missing_ids = (
        {id_ for id_, _ in unique_missing_en}
        | {id_ for id_, _ in unique_missing_id}
        | {id_ for id_, _ in unique_missing_both}
    )

    if args.verbose:
        print("  ok (in both bundles):")
        for id_ in sorted(seen_ids - missing_ids):
            for kind, sid, relpath, line in sites:
                if sid == id_:
                    print(f"    [{relpath}:{line}] {id_}  ({KIND_LABELS[kind]})")
        print()

    def _report_bucket(
        bucket: list[tuple[str, str, str, int]],
        label: str,
        unique: list[tuple[str, str]],
    ) -> None:
        if not bucket:
            return
        print(f"  missing {label} ({len(unique)} unique):")
        for id_, kind in unique:
            for k, sid, relpath, line in bucket:
                if sid == id_ and k == kind:
                    print(f"    [{relpath}:{line}] {id_}  ({KIND_LABELS[kind]})")
        print()

    _report_bucket(
        missing_in_both,
        "in BOTH en .ftl AND id .id.ftl",
        unique_missing_both,
    )
    _report_bucket(missing_in_en, "in en .ftl only", unique_missing_en)
    _report_bucket(missing_in_id, "in id .id.ftl only", unique_missing_id)

    # Emit ONE unambiguous sentinel line as the LAST stdout line in
    # both clean and missing modes. Lint-i18n.sh and CI greps on the
    # exact pattern `^verify-bundle-parity: [1-9][0-9]* missing` so
    # the gate is robust to bucket rename / new bucket additions —
    # the sentinel is the ONLY thing the lint depends on; the body
    # of the report (bucket names, file:line entries) can grow
    # freely without breaking the gate.
    total_missing = len(missing_ids)
    print(f"verify-bundle-parity: {total_missing} missing key(s).")
    # Default + --dry-run both fail-closed so CI/pre-commit block
    # the regression. --report-only succeeds regardless so human
    # readers can audit at their leisure; a clean report (0 missing)
    # also returns 0 so it is never a gate failure.
    return 0 if (args.report_only or total_missing == 0) else 1


if __name__ == "__main__":
    sys.exit(main())
