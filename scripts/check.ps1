# scripts/check.ps1 — Windows pre-push gate. Mirrors scripts/check.sh.
#
# Usage:  powershell -File scripts\check.ps1
#         (run from the workspace root)

$ErrorActionPreference = "Stop"

Set-Location (Split-Path -Parent $PSCommandPath) | Set-Location ..

Write-Host "==> cargo fmt --all -- --check"
cargo fmt --all -- --check

Write-Host "==> cargo clippy"
cargo clippy --workspace --all-targets --all-features --exclude oz-pos-app -- -D warnings

Write-Host "==> cargo test"
cargo test --workspace --all-features --exclude oz-pos-app

Write-Host "==> skill-drift-guard"
bash .agents/skills/skill-drift-guard/scripts/detect.sh --report | Out-Null

# UI checks optional until Node is installed.
# Push-Location ui
# npm run lint
# npm run typecheck
# npm run test
# npm run build
# Pop-Location

Write-Host "all checks passed"
