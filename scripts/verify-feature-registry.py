#!/usr/bin/env python3
r"""
scripts/verify-feature-registry.py — Catch feature key drift between
Rust backend, frontend FEATURES constant, and UI registrations.

WHY
====

The OZ-POS feature flag system requires three sources of truth to stay
in sync:
  1. The Rust `Feature` enum + `feature_key()` in
     `crates/oz-core/src/features.rs` — canonical backend definitions.
  2. The `FEATURES` constant in
     `ui/src/hooks/useFeatures.ts` — frontend feature key registry.
  3. `feature: '...'` attributes on `registerPage()` and `registerNavItem()`
     calls in `ui/src/App.tsx` and other registration sites.

When a developer adds a new Feature variant to the Rust enum but forgets
to:
  * add the kebab-case key to the `FEATURES` constant, or
  * update the `feature:` attribute on page/nav registrations,
the result is a silent UX bug — a page that never appears regardless of
toggle state, or a feature toggle that has no visible effect.

This script statically checks that every `feature:` string literal
referenced in registrations has a corresponding entry in both the Rust
`feature_key()` function and the frontend `FEATURES` constant. It also
reports any keys that exist in one source but not the other as
actionable gaps so they can be closed.

USAGE
=====

    python3 scripts/verify-feature-registry.py                    # strict: exit 1 if mismatch
    python3 scripts/verify-feature-registry.py --verbose          # list every feature, even OK ones
    python3 scripts/verify-feature-registry.py --report-only      # always exit 0

EXIT CODES
==========

  * 0  every feature key is consistent across all three sources.
  * 1  at least one mismatch was detected (unless --report-only).
  * 2  a runtime error occurred (Rust/TS source files not found).
"""

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

RUST_FEATURES_PATH = ROOT / "crates" / "oz-core" / "src" / "features.rs"
FRONTEND_FEATURES_PATH = ROOT / "ui" / "src" / "hooks" / "useFeatures.ts"
UI_SRC = ROOT / "ui" / "src"

# Matches `Feature::VariantName => "kebab-case-key",` in the Rust feature_key() function.
RUST_KEY_PATTERN = re.compile(
    r'Feature::\w+\s*=>\s*"([a-z][a-z0-9-]*)"',
)

# Matches `KEY_NAME: 'kebab-case-key',` in the FEATURES constant (TS).
TS_FEATURE_PATTERN = re.compile(
    r"\b[A-Z][A-Z0-9_]*:\s*'([a-z][a-z0-9-]*)'",
)

# Matches `feature: 'kebab-case-key'` in JSX/TSX registration calls.
FEATURE_ATTR_PATTERN = re.compile(
    r"""feature:\s*'([a-z][a-z0-9-]*)'""",
)

DESCRIPTION = (
    "Verify that every feature: string used in registerPage / registerNavItem "
    "calls has a matching key in both the Rust Feature enum and the frontend "
    "FEATURES constant. Prevents silent feature-drift bugs."
)


def extract_rust_keys() -> set[str]:
    """Parse all kebab-case keys from the Rust `feature_key()` function body."""
    text = RUST_FEATURES_PATH.read_text(encoding="utf-8")
    # Find the `feature_key` function body.
    fn_match = re.search(r"pub fn feature_key\(f: Feature\)", text)
    if not fn_match:
        print(
            "error: cannot find `feature_key` function in Rust file",
            file=sys.stderr,
        )
        return set()

    # Extract everything from the function body (starting after the match).
    # Find the opening brace and scan for key patterns until the closing brace.
    brace_start = text.index("{", fn_match.end())
    depth = 1
    i = brace_start + 1
    while depth > 0 and i < len(text):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
        i += 1
    body = text[brace_start : i - 1]

    keys = set()
    for m in RUST_KEY_PATTERN.finditer(body):
        keys.add(m.group(1))

    if not keys:
        print(
            "error: no feature keys found in Rust feature_key() body",
            file=sys.stderr,
        )
    return keys


def extract_frontend_keys() -> set[str]:
    """Parse all kebab-case keys from the FEATURES constant in useFeatures.ts."""
    text = FRONTEND_FEATURES_PATH.read_text(encoding="utf-8")

    # Find the FEATURES constant definition.
    fn_match = re.search(r"export\s+const\s+FEATURES\s*=\s*\{", text)
    if not fn_match:
        print(
            "error: cannot find `FEATURES` constant in frontend file",
            file=sys.stderr,
        )
        return set()

    brace_start = fn_match.end() - 1  # point to the opening {
    depth = 1
    i = brace_start + 1
    while depth > 0 and i < len(text):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
        i += 1
    body = text[brace_start : i - 1]

    keys = set()
    for m in TS_FEATURE_PATTERN.finditer(body):
        keys.add(m.group(1))

    if not keys:
        print(
            "error: no feature keys found in FEATURES constant body",
            file=sys.stderr,
        )
    return keys


def extract_registration_features() -> dict[str, list[tuple[str, int]]]:
    """Walk all .tsx/.ts files and collect feature: references with attribution."""
    sites: dict[str, list[tuple[str, int]]] = {}

    for path in sorted(UI_SRC.rglob("*.tsx")) + sorted(UI_SRC.rglob("*.ts")):
        relpath = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8")
        for m in FEATURE_ATTR_PATTERN.finditer(text):
            key = m.group(1)
            line = text.count("\n", 0, m.start()) + 1
            sites.setdefault(key, []).append((relpath, line))

    return sites


def main() -> int:
    parser = argparse.ArgumentParser(description=DESCRIPTION)
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print every feature key and its status.",
    )
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="Always exit 0; print report and return.",
    )
    args = parser.parse_args()

    if not RUST_FEATURES_PATH.is_file():
        print(f"error: Rust features file not found: {RUST_FEATURES_PATH}", file=sys.stderr)
        return 2
    if not FRONTEND_FEATURES_PATH.is_file():
        print(f"error: Frontend features file not found: {FRONTEND_FEATURES_PATH}", file=sys.stderr)
        return 2

    rust_keys = extract_rust_keys()
    frontend_keys = extract_frontend_keys()
    registration_sites = extract_registration_features()
    registration_keys = set(registration_sites.keys())

    total_keys = len(rust_keys)

    # ── Cross-reference ─────────────────────────────────────────────

    # Keys used in registrations but not in Rust.
    rust_missing = registration_keys - rust_keys
    # Keys used in registrations but not in FEATURES constant.
    frontend_missing = registration_keys - frontend_keys
    # Keys in Rust but not in FEATURES (extra Rust keys are informational).
    rust_extra = rust_keys - frontend_keys
    # Keys in FEATURES but not in Rust.
    frontend_extra = frontend_keys - rust_keys

    # ── Report ──────────────────────────────────────────────────────

    print(
        f"verify-feature-registry: {total_keys} Rust key(s), "
        f"{len(frontend_keys)} frontend key(s), "
        f"{len(registration_keys)} registration key(s) "
        f"across {sum(len(v) for v in registration_sites.values())} site(s)."
    )
    print()

    if args.verbose:
        for key in sorted(rust_keys & frontend_keys & registration_keys):
            sites = registration_sites[key]
            site_list = "; ".join(f"[{r}:{l}]" for r, l in sites)
            print(f"  ok: {key} ({site_list})")
        print()

    if rust_missing:
        print(
            f"  missing IN RUST feature_key() ({len(rust_missing)} unique) — "
            "used in registrations but has no matching Rust Feature variant:"
        )
        for key in sorted(rust_missing):
            sites = registration_sites[key]
            for relpath, line in sites:
                print(f"    [{relpath}:{line}] {key}")
        print()

    if frontend_missing:
        print(
            f"  missing IN FRONTEND FEATURES constant ({len(frontend_missing)} unique) — "
            "used in registrations but has no matching frontend constant:"
        )
        for key in sorted(frontend_missing):
            sites = registration_sites[key]
            for relpath, line in sites:
                print(f"    [{relpath}:{line}] {key}")
        print()

    if rust_extra - registration_keys:
        extras = rust_extra - registration_keys
        print(
            f"  defined in Rust but NOT in FEATURES constant "
            f"({len(extras)} unique) — may be unused or need frontend key:"
        )
        for key in sorted(extras):
            print(f"    {key}")
        print()

    if frontend_extra - registration_keys:
        extras = frontend_extra - registration_keys
        print(
            f"  defined in FEATURES constant but NOT in Rust feature_key() "
            f"({len(extras)} unique) — may be stale or missing Rust variant:"
        )
        for key in sorted(extras):
            print(f"    {key}")
        print()

    total_issues = len(rust_missing) + len(frontend_missing)
    print(f"verify-feature-registry: {total_issues} issue(s).")

    return 0 if (args.report_only or total_issues == 0) else 1


if __name__ == "__main__":
    sys.exit(main())
