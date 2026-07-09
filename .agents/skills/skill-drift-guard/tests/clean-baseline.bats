#!/usr/bin/env bats
#
# Test: clean-baseline.bats
#
# Runs the drift-guard against the project root. Expects:
#   - exit code 0
#   - "No drift detected" marker in stdout
#   - exactly zero findings written to skill-drift-report.md
#   - the auto-patch path is NOT taken (no version/audit-date/refs lines)
#
# This is the load-bearing integration test for the "happy path". If this
# regresses, every contributor's PR cycle will see a non-zero drift-guard
# exit code, which is the loudest possible surface for the helper-family
# (audit_footer_check_in_file, batch_validate_audit_dates) and the
# AUDIT_RE/PYTHON_BIN plumbing breaking.

setup() {
  PROJECT_ROOT="$(cd "$(dirname "${BATS_TEST_FILENAME}")/../../../.." && pwd)"
  cd "$PROJECT_ROOT"
  rm -f "$PROJECT_ROOT/skill-drift-report.md"
}

teardown() {
  rm -f "$PROJECT_ROOT/skill-drift-report.md"
}

@test "clean baseline: detect.sh exits 0 with no-drift message" {
  run bash "$PROJECT_ROOT/.agents/skills/skill-drift-guard/scripts/detect.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"No drift detected"* ]]
  [[ ! "$output" == *"wrote skill-drift-report.md"* ]]
}

@test "clean baseline: detect.sh exits 0 with --check=audit-format" {
  run bash "$PROJECT_ROOT/.agents/skills/skill-drift-guard/scripts/detect.sh" \
      --check=audit-format
  [ "$status" -eq 0 ]
}

@test "clean baseline: detect.sh exits 0 with --check=doc-audit" {
  run bash "$PROJECT_ROOT/.agents/skills/skill-drift-guard/scripts/detect.sh" \
      --check=doc-audit
  [ "$status" -eq 0 ]
}
