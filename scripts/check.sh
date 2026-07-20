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
    local step_str; step_str=$(printf "%02d" "${step_counter}")
    echo -n "${step_str}. checking ${name}... "
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

# Workspace-wide clippy (single compilation pass instead of N per-package invocations).
# Uses default features only — the `slow-tests` feature gates integration tests
# that don't need linting, and clippy doesn't benefit from compiling them.
step "clippy workspace" "cargo clippy --workspace --all-targets -- -D warnings" cargo clippy --workspace --all-targets -- -D warnings

# ── ADR #7 Phase 4: no raw store_id/user_id in command signatures ───────
step "no-raw-params (ADR #7 Phase 4)" "bash scripts/verify-no-raw-params.sh" bash scripts/verify-no-raw-params.sh

# Workspace-wide test via cargo-nextest — runs each test in its own process
# for 4.5× faster re-runs after compilation. Falls back to cargo test if
# nextest is not installed.
cpu_count=$(nproc --all 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
if command -v cargo-nextest &>/dev/null || cargo nextest --version &>/dev/null 2>&1; then
    step "test workspace (nextest)" "cargo nextest run --workspace --exclude oz-pos-app --exclude oz-pos-tablet" cargo nextest run --workspace --exclude oz-pos-app --exclude oz-pos-tablet
else
    echo -e "${YELLOW}⚠ nextest not found — falling back to cargo test (slower)${NC}"
    step "test workspace" "cargo test --workspace --all-features -- --test-threads $cpu_count" cargo test --workspace --all-features -- --test-threads "$cpu_count"
fi

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
    # i18n lint: runs AFTER ui test (which proves vitest works) but
    # BEFORE ui build (which is ~30s). Fail-fast on a ~1s lint check
    # so contributors don't pay the full build cost for a translation
    # gap. Detects translation gaps and Fluent key duplicates in
    # `ui/src/locales/*.id.ftl` before they reach CI.
    cd ..
    step "i18n lint" "bash scripts/lint-i18n.sh" bash scripts/lint-i18n.sh
    step "feature registry parity" "python3 scripts/verify-feature-registry.py" python3 scripts/verify-feature-registry.py
    # npm run build skipped — typecheck + vitest already cover correctness;
    # the production vite bundle is validated by CI independently.
else
    echo -e "${YELLOW}⚠ UI checks skipped (npm not found or ui/package-lock.json missing)${NC}"
fi

# ── Docker build smoke test (optional: --docker-dry-run) ──────────────────
if [ "${1:-}" = "--docker-dry-run" ]; then
    if command -v docker &>/dev/null; then
        step "docker build" "docker build -f Dockerfile.server -t oz-pos-cloud:local ." docker build -f Dockerfile.server -t oz-pos-cloud:local .

        SIZE=$(docker run --rm --entrypoint stat oz-pos-cloud:local --format=%s /app/oz-cloud-server 2>/dev/null || echo "0")
        if [ "$SIZE" -gt "0" ]; then
            MAX=$((50 * 1024 * 1024))
            if [ "$SIZE" -gt "$MAX" ]; then
                echo -e "${RED}Binary size $SIZE exceeds 50 MB limit${NC}"
                exit 1
            fi
            echo -e "${GREEN}Binary size: $((SIZE / 1024 / 1024)) MB (OK)${NC}"
        else
            echo -e "${YELLOW}⚠ Could not verify binary size (container may have exited)${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ Docker build skipped (docker not found)${NC}"
    fi
fi

# ── Done ──────────────────────────────────────────────────────────────────
total_end=$(date +%s)
echo -e "${GREEN}all checks passed ($((total_end - total_start))s)${NC}"

# ── Commit suggestion ─────────────────────────────────────────────────────
cat <<'COMMIT_GUIDE'

Now make a local commit:

  1. git add <files>     # stage only intended files
  2. git commit          # write a message following the guidelines below

Commit message guidelines:
  • Keep the summary line under 50 characters, imperative mood, no period
  • Leave a blank line after the summary
  • Use bullet points (- or *) for the body — focus on WHAT and WHY, not how
  • Reference related docs/decisions or issue numbers where relevant
  • Keep each bullet under 72 characters

Example:

    feat(sales): add deduction location override via PIN

    - Clicking the badge opens FastPINOverlay for PIN verification
    - Store method overrides deduction location with IMMEDIATE transaction
    - Badge shows "(Override)" indicator after successful override

    References ADR-19

COMMIT_GUIDE
