#!/usr/bin/env bash
#
# Smoke-test runner for .agents/skills/skill-drift-guard/tests/*.bats.
#
# Usage:
#   bash scripts/run-tests.sh
#
# Exit codes: 0 = all tests passed, 1 = one or more tests failed,
# 2 = bats not installed (after printing install options).
#
# Install bats (any of):
#   Linux (apt):       sudo apt-get install -y bats
#   macOS (homebrew):  brew install bats-core
#   Windows (choco):   choco install bats
#   Windows (scoop):   scoop install bats
#   Cross-platform:    npm install -g bats
#
# CI integration: add a `skill-drift-tests` job that installs bats via
# the host's package manager and runs `bash scripts/run-tests.sh`.
#
# Local dev: install bats once, then run this script on every change.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="$(cd "$SCRIPT_DIR/../tests" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

if ! command -v bats >/dev/null 2>&1; then
  cat <<'EOF'
bats not installed. Pick one of these install paths:

  Linux  (apt):          sudo apt-get install -y bats
  macOS  (homebrew):     brew install bats-core
  Windows (choco):       choco install bats
  Windows (scoop):       scoop install bats
  Cross-platform (npm):  npm install -g bats

After install, re-run: bash .agents/skills/skill-drift-guard/scripts/run-tests.sh
EOF
  exit 2
fi

cd "$PROJECT_ROOT"
echo "Running bats suite from $TESTS_DIR"
bats "$TESTS_DIR"
