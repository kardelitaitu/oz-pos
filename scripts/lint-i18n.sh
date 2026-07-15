#!/usr/bin/env bash
# scripts/lint-i18n.sh — local i18n quality gate.
#
# Mirrors `.github/workflows/ci.yml` and `.github/workflows/release.yml`
# (the `i18n quality gate` step). Contributors run this locally to
# catch i18n issues before pushing to CI.
#
# Reports three categories of regressions. Only 1 and 2 fail-closed
# (drive the script's exit code); 3 is an informational pre-flight
# until the landing-baseline gaps in the parity check are resolved
# (see the inline comment near the parity gate):
#   1. `[i18n]` — translation completeness tests in
#      `ui/src/__tests__/i18nBundle.test.tsx` flag any .id.ftl file
#      that is byte-identical to its .ftl sibling (Indonesian users
#      see English text on those screens).
#   2. `Attempt to override an existing message` — FluentBundle
#      warns when two .ftl files in the same joined bundle define
#      the same key; the first loaded file wins silently, the
#      duplicate is dropped.
#   3. Pre-flight parity: every `<Localized id="…">` site in
#      `ui/src/features/**` must have a matching key in BOTH the
#      en .ftl AND the id .id.ftl (see
#      `scripts/verify-bundle-parity.py`).
#
# Usage:  bash scripts/lint-i18n.sh
#         (run from any directory)
#
# Exits 0 if no fail-closed regressions (1 and 2) are detected;
# 1 otherwise, with a categorized error report on stderr. The
# parity pre-flight (3) is reported to stderr when it finds issues
# but does NOT influence the exit code.

set -uo pipefail

cd "$(dirname "$0")/.."

# ── Pre-flight: bundle parity (silent on clean runs) ────────────────
#
# Sentinel-grep on the parity script's always-last stdout line
# `verify-bundle-parity: <N> missing key(s).` — robust to bucket
# renames / additions. Match triggers an informational `cat >&2`
# display (NOT a gate). `--report-only` is forced so the parity
# script's exit never propagates; this script fails-closed only
# via categories 1 and 2 downstream. Promote — once `--report-only`
# reports `0 missing` — by dropping `--report-only` AND adding
# `exit 1` inside the `if grep -q` block below.
OUT=$(mktemp)
PARITY_OUT=$(mktemp)
trap 'rm -f "$OUT" "$PARITY_OUT"' EXIT
python3 scripts/verify-bundle-parity.py --report-only > "$PARITY_OUT" 2>&1
if grep -qE '^verify-bundle-parity: [1-9][0-9]* missing' "$PARITY_OUT"; then
    cat "$PARITY_OUT" >&2
    echo "" >&2
fi

# Targeted: i18nBundle.test.tsx is the sole source of `[i18n]`
# warnings and triggers `getBundle('en')` + `getBundle('id')` which
# emit Fluent `Attempt to override` warnings once per duplicate key
# per locale. Running targeted keeps the lint under 2 seconds —
# important for pre-commit ergonomics.
VITEST_EXIT=0
(cd ui && npx vitest run src/__tests__/i18nBundle.test.tsx 2>&1) > "$OUT" || VITEST_EXIT=$?

untranslated=$(grep -E '\[i18n\]' "$OUT" || true)
duplicates=$(grep -E 'Attempt to override an existing message' "$OUT" || true)

if [ -z "$untranslated$duplicates" ]; then
    # If vitest itself failed (OOM, config error, etc.) but produced no
    # i18n warnings, still fail — the test infrastructure is broken.
    if [ "$VITEST_EXIT" -ne 0 ]; then
        echo "i18n lint: vitest infrastructure failure (exit $VITEST_EXIT) — no i18n issues detected but the test runner crashed." >&2
        cat "$OUT" >&2
        exit 1
    fi
    echo "i18n lint: no issues detected."
    exit 0
fi

echo "i18n lint: issues detected" >&2
if [ -n "$untranslated" ]; then
    echo "" >&2
    echo "  Untranslated .id.ftl files (Indonesian users see English text):" >&2
    echo "$untranslated" | sed -E 's/^\[i18n\] */    - /' >&2
fi
if [ -n "$duplicates" ]; then
    echo "" >&2
    echo "  Fluent key duplicates (consolidate into a single home .ftl file):" >&2
    echo "$duplicates" | sed -E 's/.*"([^"]+)".*/    - \1/' | sort -u >&2
fi
exit 1
