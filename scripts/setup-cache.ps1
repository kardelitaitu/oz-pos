# One-time setup for Rust compilation caching with sccache (Windows).
#
# Prerequisites: Chocolatey (https://chocolatey.org/install)
#
# Run once per machine (PowerShell as Administrator not required):
#   powershell -ExecutionPolicy Bypass -File scripts/setup-cache.ps1
#
# What it does:
# 1. Installs sccache via Chocolatey if missing
# 2. Sets a generous 20 GB local disk cache
# 3. Verifies sccache is active as the rustc wrapper

$ErrorActionPreference = "Stop"

Write-Host "==> Checking sccache..." -ForegroundColor Cyan
$sccache = Get-Command sccache -ErrorAction SilentlyContinue
if (-not $sccache) {
    if (-not (Get-Command choco -ErrorAction SilentlyContinue)) {
        Write-Host "    Chocolatey not installed. Install it first:" -ForegroundColor Red
        Write-Host "    https://chocolatey.org/install" -ForegroundColor Red
        exit 1
    }
    Write-Host "    sccache not found. Installing via Chocolatey..." -ForegroundColor Yellow
    choco install sccache -y
}

$version = sccache --version 2>&1
Write-Host "    $version"

Write-Host "==> Setting cache size to 20 GB..." -ForegroundColor Cyan
sccache --set-config cache.disk.size 20G

Write-Host "==> Zeroing stats (fresh start)..." -ForegroundColor Cyan
sccache --zero-stats

Write-Host "==> Verifying sccache is enabled (uncommented) in .cargo\config.toml ..." -ForegroundColor Cyan
$configPath = Join-Path $PSScriptRoot ".." ".cargo" "config.toml"
$line = Select-String -Path $configPath -Pattern '^rustc-wrapper.*sccache' -Quiet
if ($line) {
    Write-Host "    ✓ sccache enabled as rustc-wrapper (uncommented)" -ForegroundColor Green
} else {
    Write-Host "    ✗ sccache not wired or still commented in .cargo\config.toml" -ForegroundColor Red
    Write-Host "    The repo ships this file uncommented -- make sure you have the latest version."
    exit 1
}

Write-Host ""
Write-Host "Setup complete. Next:" -ForegroundColor Green
Write-Host "  1. Run a cold build:  cargo clean && cargo check --workspace --exclude oz-pos-app"
Write-Host "  2. Run a warm build:  cargo check --workspace --exclude oz-pos-app"
Write-Host "  3. Check stats:       sccache --show-stats"
