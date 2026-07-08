#!/usr/bin/env bats
#
# Test: shape-violation.bats
#
# Injects a footer that passes the digit-substring portion of $AUDIT_RE
# but fails the full regex (whitespace before a parenthetical in the
# by-clause violates `[^[:space:]]+$`). Asserts that detect.sh:
#   - exits non-zero
#   - reports the failure under the `doc-audit` FINDINGS key
#   - quotes the violation pattern "DD-MM-YY + by-clause"
#   - the value check is NOT taken (bypassed when shape fails)
#
# This pins the SHAPE-first invariant: if shape fails, we report shape
# and never even hand the date to Python. If this regresses (e.g. the
# shape check starts accepting `28-06-26 by repo (extra)` as OK, or the
# value check fires for a shape-failure case), this test catches it.

setup() {
  PROJECT_ROOT="$(cd "$(dirname "${BATS_TEST_FILENAME}")/../../.." && pwd)"
  cd "$PROJECT_ROOT"
  cp "$PROJECT_ROOT/CONTRIBUTING.md" "$BATS_TEST_TMPDIR/CONTRIBUTING.md.bak"
  printf '\n> last audited 28-06-26 by project-scaffold (extra)\n' \
    >> "$PROJECT_ROOT/CONTRIBUTING.md"
}

teardown() {
  cp "$BATS_TEST_TMPDIR/CONTRIBUTING.md.bak" "$PROJECT_ROOT/CONTRIBUTING.md"
  rm -f "$PROJECT_ROOT/skill-drift-report.md"
}

@test "shape-violation: bad by-clause fires Check 10 with shape message" {
  run bash "$PROJECT_ROOT/.agents/skills/skill-drift-guard/scripts/detect.sh" \
      --check=doc-audit
  [ "$status" -ne 0 ]
  [[ "$output" == *"doc-audit"* ]]
  [[ "$output" == *"DD-MM-YY + by-clause"* ]]
  [[ ! "$output" == *"shape OK but date"* ]]
}
