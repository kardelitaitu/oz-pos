# scripts/sync-branding.ps1 — Synchronize brand assets across Tauri and Web apps.
# Usage: powershell -File scripts\sync-branding.ps1 [-Brand "default"]

param(
    [string]$Brand = "default"
)

$ErrorActionPreference = "Stop"

Set-Location (Split-Path -Parent $PSCommandPath)
Set-Location ..

$BrandDir = "assets/branding/$Brand"

if (-not (Test-Path $BrandDir)) {
    Write-Host "Error: Brand directory '$BrandDir' not found." -ForegroundColor Red
    exit 1
}

Write-Host "Syncing brand '$Brand' across OZ-POS apps..." -ForegroundColor Cyan

# 1. Sync Desktop & Tablet Tauri Icons
$desktopIco = "$BrandDir/desktop/icon.ico"
if (Test-Path $desktopIco) {
    Copy-Item -Force $desktopIco "apps/desktop-client/icons/icon.ico"
    Copy-Item -Force $desktopIco "apps/tablet-client/icons/icon.ico"
    Write-Host "  [OK] Updated apps/desktop-client/icons/icon.ico" -ForegroundColor Green
    Write-Host "  [OK] Updated apps/tablet-client/icons/icon.ico" -ForegroundColor Green
} else {
    Write-Host "  [SKIP] No $desktopIco found" -ForegroundColor Yellow
}

# 2. Sync Web & PWA Icons to ui/public/
$webDir = "$BrandDir/web"
if (Test-Path $webDir) {
    Copy-Item -Force -Recurse "$webDir/*" "ui/public/"
    Write-Host "  [OK] Updated ui/public/ with web & PWA icons from $webDir" -ForegroundColor Green
} else {
    Write-Host "  [SKIP] No $webDir found" -ForegroundColor Yellow
}

# 3. Sync Vector Logos to ui/public/branding/
$vectorDir = "$BrandDir/vector"
if (Test-Path $vectorDir) {
    New-Item -ItemType Directory -Force -Path "ui/public/branding" | Out-Null
    Copy-Item -Force "$vectorDir/*.svg" "ui/public/branding/" -ErrorAction SilentlyContinue
    Write-Host "  [OK] Updated ui/public/branding/ with SVG vector logos from $vectorDir" -ForegroundColor Green
}

Write-Host "Branding sync complete!" -ForegroundColor Cyan
