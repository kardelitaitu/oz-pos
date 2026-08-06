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

# ── Architecture boundary checker (P1 pilot) ────────────────────────────
# Existing transitional debt is reported but only new, expired, or stale
# baseline entries fail. This is static-only and has no runtime impact.
step "architecture boundaries" "python3 scripts/verify-architecture-boundaries.py --strict" python3 scripts/verify-architecture-boundaries.py --strict

# ── Money formatting gate (IDR/JPY/KWD exp-2 regression guard) ───────────
# Fails when production .rs code hardcodes `/ 100` division or `{}.{:02}`
# format strings instead of foundation::format_minor(). Pure python — no
# toolchain deps, so it stays fast.
step "no-hardcoded-money-format" "python3 scripts/verify-no-hardcoded-money-format.py" python3 scripts/verify-no-hardcoded-money-format.py

# Workspace-wide test via cargo-nextest — runs each test in its own process
# for 4.5× faster re-runs after compilation. Also run doctests separately
# because nextest does not execute them. Falls back to cargo test if nextest
# is not installed.
cpu_count=$(nproc --all 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
if command -v cargo-nextest &>/dev/null || cargo nextest --version &>/dev/null 2>&1; then
    step "test workspace (nextest)" "cargo nextest run --workspace --all-features --exclude oz-pos-app --exclude oz-pos-tablet" cargo nextest run --workspace --all-features --exclude oz-pos-app --exclude oz-pos-tablet
    step "test doctests" "cargo test --doc --workspace" cargo test --doc --workspace
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

# ── Panic-inventory gate (RUST-07 / ADR #33) — fail-closed ────────────────
# Audits production unwrap()/expect() calls (excludes tests, benches, and
# cfg(test)-gated helpers). Panics are only acceptable for documented
# invariant-setup (// SAFETY: / // INVARIANT: on the same or preceding
# line); the recoverable set must stay at zero. Fails when any finding
# lacks a verifiable comment.
# Review the full inventory with: python3 scripts/scan-unwrap-panic.py
if command -v python3 &>/dev/null; then
    echo -n "panic-inventory scan... "
    # Capture combined output: on success the scanner prints one summary line;
    # on failure it prints the FAIL header + findings + fix hint, which we
    # replay below so the failure is self-explanatory.
    if panic_out=$(python3 scripts/scan-unwrap-panic.py --fail-on-recoverable 2>&1); then
        echo -e "${GREEN}PASS (${panic_out})${NC}"
    else
        echo -e "${RED}FAIL (recoverable unwrap/expect calls found — add // SAFETY: / // INVARIANT: or convert to Result)${NC}"
        echo "$panic_out"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ panic-inventory skipped (python3 not found)${NC}"
fi

# ── UI (mirrors CI `ui` job — auto-detected) ──────────────────────────────
# Windows can retain esbuild.exe briefly after a Vite/test process exits,
# causing npm ci's node_modules cleanup to fail with EPERM. Retry once after
# terminating only the known native helper; dependency-resolution failures
# still fail on the retry and remain visible to the caller.
npm_ci_with_windows_retry() {
    local npm_log; npm_log=$(mktemp)
    local first_status

    if npm ci --no-audit --no-fund --ignore-scripts >"$npm_log" 2>&1; then
        cat "$npm_log"
        rm -f "$npm_log"
        return 0
    else
        first_status=$?
    fi

    # Preserve ordinary npm failures verbatim. Only retry the known Windows
    # native-binary lock signatures; dependency and lockfile errors must fail
    # immediately instead of killing an unrelated process.
    if ! grep -q 'EPERM' "$npm_log" || \
       ! grep -qiE 'esbuild\.exe|rollup[^[:space:]]*\.node' "$npm_log"; then
        cat "$npm_log" >&2
        rm -f "$npm_log"
        return "$first_status"
    fi

    # esbuild is a standalone native helper and can be safely terminated.
    # Rollup's native module is loaded by Node itself, so never terminate all
    # node.exe processes; just allow its transient file handle to clear.
    if grep -qi 'esbuild\.exe' "$npm_log" && command -v taskkill.exe &>/dev/null; then
        MSYS_NO_PATHCONV=1 taskkill.exe /F /IM esbuild.exe >/dev/null 2>&1 || true
    fi
    sleep 2
    rm -f "$npm_log"
    npm ci --no-audit --no-fund --ignore-scripts
}

if command -v npm &>/dev/null && [ -f ui/package-lock.json ]; then
    cd ui
    step "npm ci" "cd ui; npm ci --no-audit --no-fund --ignore-scripts" npm_ci_with_windows_retry
    step "ui lint" "cd ui; npm run lint" npm run lint
    step "ui typecheck" "cd ui; npm run typecheck" npm run typecheck
    step "ui test" "cd ui; npm run test" npm run test
    # AUDIT-27 CI-06: A11y regression suite (advisory, mirrors CI's
    # continue-on-error since known product-level a11y bugs are tracked
    # but not yet fixed). Never fails the gate — reports status only.
    echo -n "ui a11y (advisory)... "
    if npm run test:a11y >/dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC}"
    else
        echo -e "${YELLOW}WARN (a11y regressions exist — non-blocking, see CI)${NC}"
    fi
    # i18n lint: runs AFTER ui test (which proves vitest works) but
    # BEFORE ui build (which is ~30s). Fail-fast on a ~1s lint check
    # so contributors don't pay the full build cost for a translation
    # gap. Detects translation gaps and Fluent key duplicates in
    # `ui/src/locales/*.id.ftl` before they reach CI.
    cd ..
    step "i18n lint" "bash scripts/lint-i18n.sh" bash scripts/lint-i18n.sh
    # AUDIT-27 CI-06: FTL dedupe — detect duplicate Fluent keys so local
    # validation matches check-ui.mjs and the pre-commit gate.
    step "ftl dedupe" "python3 scripts/dedupe-ftl.py" python3 scripts/dedupe-ftl.py
    step "feature registry parity" "python3 scripts/verify-feature-registry.py" python3 scripts/verify-feature-registry.py
    # npm run build skipped — typecheck + vitest already cover correctness;
    # the production vite bundle is validated by CI independently.
    # AUDIT-27 CI-07: E2E is NOT run here (Docker backend not provisioned).
    # Run `cd ui && npm run check:all` (uses npm run e2e with full
    # Docker+Vite provisioning) or `npm run e2e` directly for managed E2E.
else
    echo -e "${YELLOW}⚠ UI checks skipped (npm not found or ui/package-lock.json missing)${NC}"
fi

# ── Plugin guide / API parity (PLG-10 tail; Rust-side, always runs) ─────
step "plugin-guide parity" "python3 scripts/verify-plugin-guide-parity.py" python3 scripts/verify-plugin-guide-parity.py

# ── Windows config drift (AUDIT-28) — NSIS installMode + asInvoker ─────
# Static gate that runs on every local pre-CI run: tauri.conf.json must
# keep NSIS installMode at currentUser (perMachine reintroduces the UAC
# prompt) and every source app.manifest must carry asInvoker. The PE scan
# of actually-built Windows exes is enforced in release.yml's Windows job.
step "windows config drift" "python3 scripts/verify-windows-config.py" python3 scripts/verify-windows-config.py

# ── Release toolchain (AUDIT-28 RELEASE-04/05/06) — node self-tests ────
# Validates the release scripts on every local gate run, not only in CI:
# the tag↔version gate, the updater-manifest generator, and the signature
# verifier each carry a --self-test (mirroring release.yml's
# release-validate + release-publish self-test steps).
if command -v node &>/dev/null; then
    step "release version gate" "node scripts/check-release-version.mjs --self-test" node scripts/check-release-version.mjs --self-test
    step "updater manifest generator" "node scripts/generate-latest-json.mjs --self-test" node scripts/generate-latest-json.mjs --self-test
    step "updater signature verifier" "node scripts/verify-updater-signature.mjs --self-test" node scripts/verify-updater-signature.mjs --self-test
else
    echo -e "${YELLOW}⚠ release toolchain checks skipped (node not found)${NC}"
fi

# ── CI docs drift (AUDIT-27 CI-08) — docs/ci-pipeline.md must stay in
# sync with the workflows and the local runner gate vocabulary. The gate
# names + status derive from scripts/gates.json (the single source of
# truth shared with ci.yml, nightly.yml, and check:all). Mirrors the
# `ci-docs-drift` CI job; a named-but-missing job, a drifted
# check.sh/check:all gate, or a status that contradicts a workflow
# fails the gate.
step "ci docs drift" "python3 scripts/verify-ci-docs-drift.py" python3 scripts/verify-ci-docs-drift.py

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
