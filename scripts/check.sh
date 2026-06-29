#!/usr/bin/env bash
# scripts/check.sh — local pre-push gate. Mirrors .github/workflows/ci.yml.
#
# Usage:  bash scripts/check.sh
#         (run from the workspace root)

set -euo pipefail

cd "$(dirname "$0")/.."

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

step_counter=1

step() {
    local name=$1; shift
    local retry_cmd=$1; shift
    echo -n "${step_counter}. checking ${name}... "
    step_counter=$((step_counter + 1))

    local start; start=$(date +%s)
    if ! "$@" >/dev/null 2>&1; then
        echo -e "${RED}FAIL${NC}"
        echo "run \"$retry_cmd\" for full detailed error messages"
        exit 1
    else
        local end; end=$(date +%s)
        echo -e "${GREEN}PASS ($((end - start))s)${NC}"
    fi
}

total_start=$(date +%s)

# ── Rust (mirrors CI `rust` job) ──────────────────────────────────────────
step "cargo fmt" "cargo fmt --all -- --check" cargo fmt --all -- --check
step "cargo clippy" "cargo clippy --workspace --all-targets --all-features -- -D warnings" cargo clippy --workspace --all-targets --all-features -- -D warnings
step "cargo test" "cargo test --workspace --all-features" cargo test --workspace --all-features

# ── Migration (mirrors CI `migration` job) ────────────────────────────────
step "migration smoke test" "cargo run -p oz-cli -- migrate" cargo run -p oz-cli -- migrate
step "migration idempotency" "cargo run -p oz-cli -- migrate" cargo run -p oz-cli -- migrate
rm -f oz-pos.db oz-pos.db-wal oz-pos.db-shm

# ── Skill drift guard (extra local guard; CI doesn't run this) ────────────
if command -v bash &>/dev/null; then
    step "skill-drift-guard" "bash .agents/skills/skill-drift-guard/scripts/detect.sh --report" bash .agents/skills/skill-drift-guard/scripts/detect.sh --report
else
    echo -e "${YELLOW}⚠ skill-drift-guard skipped (bash not found)${NC}"
fi

# ── UI (mirrors CI `ui` job — auto-detected) ──────────────────────────────
if command -v npm &>/dev/null && [ -f ui/package-lock.json ]; then
    cd ui
    step "npm ci" "cd ui; npm ci --no-audit --no-fund" npm ci --no-audit --no-fund
    step "ui lint" "cd ui; npm run lint" npm run lint
    step "ui typecheck" "cd ui; npm run typecheck" npm run typecheck
    step "ui test" "cd ui; npm run test" npm run test
    step "ui build" "cd ui; npm run build" npm run build
    cd ..
else
    echo -e "${YELLOW}⚠ UI checks skipped (npm not found or ui/package-lock.json missing)${NC}"
fi

# ── Done ──────────────────────────────────────────────────────────────────
total_end=$(date +%s)
echo -e "${GREEN}all checks passed ($((total_end - total_start))s)${NC}"
