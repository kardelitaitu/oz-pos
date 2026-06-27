#!/usr/bin/env bash
# scripts/check.sh — local pre-push gate. Mirrors the CI matrix.
#
# Usage:  bash scripts/check.sh
#         (run from the workspace root)

set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "==> cargo clippy"
cargo clippy --workspace --all-targets --all-features --exclude oz-pos-app -- -D warnings

echo "==> cargo test"
cargo test --workspace --all-features --exclude oz-pos-app

echo "==> skill-drift-guard"
bash .agents/skills/skill-drift-guard/scripts/detect.sh --report >/dev/null

# UI checks are optional here (Node may not be installed). Uncomment
# once your environment has Node 18+.

# echo "==> ui lint"
# (cd ui && npm run lint)
# echo "==> ui typecheck"
# (cd ui && npm run typecheck)
# echo "==> ui test"
# (cd ui && npm run test)
# echo "==> ui build"
# (cd ui && npm run build)

echo "all checks passed"
