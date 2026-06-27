# One-time setup for Rust compilation caching with sccache (Windows).
#
# Run once per machine (PowerShell as Administrator not required):
#   powershell -ExecutionPolicy Bypass -File scripts/setup-cache.ps1
#
# What it does:
# 1. Installs sccache via Chocolatey if missing
# 2. Sets a generous 20 GB local disk cache
# 3. Confirms sccache is wired as the rustc wrapper

$ErrorActionPreference = "Stop"

Write-Host "==> Checking sccache…" -ForegroundColor Cyan
$sccache = Get-Command sccache -ErrorAction SilentlyContinue
if (-not $sccache) {
    Write-Host "    sccache not found. Installing via Chocolatey…" -ForegroundColor Yellow
    choco install sccache -y
}

$version = sccache --version 2>&1
Write-Host "    $version"

Write-Host "==> Setting cache size to 20 GB…" -ForegroundColor Cyan
sccache --set-config cache.disk.size 20G

Write-Host "==> Zeroing stats (fresh start)…" -ForegroundColor Cyan
sccache --zero-stats

Write-Host "==> Verifying .cargo/config.toml …" -ForegroundColor Cyan
$configPath = Join-Path $PSScriptRoot ".." ".cargo" "config.toml"
if (Select-String -Path $configPath -Pattern 'rustc-wrapper.*sccache' -Quiet) {
    Write-Host "    ✓ sccache wired as rustc-wrapper" -ForegroundColor Green
} else {
    Write-Host "    ✗ .cargo/config.toml missing or not configured" -ForegroundColor Red
    Write-Host "    The repo ships this file — make sure you're on main."
    exit 1
}

Write-Host ""
Write-Host "Setup complete. Next:" -ForegroundColor Green
Write-Host "  1. Run a cold build:  cargo clean && cargo check --workspace --exclude oz-pos-app"
Write-Host "  2. Run a warm build:  cargo check --workspace --exclude oz-pos-app"
Write-Host "  3. Check stats:       sccache --show-stats"
