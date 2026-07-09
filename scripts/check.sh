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

# Extract workspace member package names from cargo metadata.
# Uses mapfile to avoid IFS splitting issues with $(...) on Windows.
# Pipe through tr -d \r to strip carriage returns that mapfile -t doesn't remove.
mapfile -t packages < <(cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c "
import json, sys
try:
    d = json.load(sys.stdin)
    by_id = {p['id']: p['name'] for p in d['packages']}
    for mid in sorted(d.get('workspace_members', [])):
        raw = by_id.get(mid, mid.split()[0])
        # Keep only valid Cargo package name chars (alphanumeric, -, _)
        clean = ''.join(c for c in raw if c.isalnum() or c in '-_')
        if clean:
            print(clean)
except Exception:
    pass
" | tr -d '\r')

for pkg in "${packages[@]}"; do
    step "clippy $pkg" "cargo clippy -p $pkg --all-targets --all-features -- -D warnings" cargo clippy -p "$pkg" --all-targets --all-features -- -D warnings
done

for pkg in "${packages[@]}"; do
    step "test $pkg" "cargo test -p $pkg --all-features" cargo test -p "$pkg" --all-features
done

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
    step "ui build" "cd ui; npm run build" npm run build
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
