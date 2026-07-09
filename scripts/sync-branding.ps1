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
$desktopDir = "$BrandDir/desktop"
if (Test-Path "$desktopDir/icon.ico") {
    Copy-Item -Force "$desktopDir/icon.ico" "apps/desktop-client/icons/icon.ico"
    Copy-Item -Force "$desktopDir/icon.ico" "apps/tablet-client/icons/icon.ico"
    Write-Host "  [OK] Updated apps/*/icons/icon.ico" -ForegroundColor Green
} else {
    Write-Host "  [SKIP] No $desktopDir/icon.ico found" -ForegroundColor Yellow
}

if (Test-Path "$desktopDir/256x256.png") {
    Copy-Item -Force "$desktopDir/256x256.png" "apps/desktop-client/icons/256x256.png"
    Copy-Item -Force "$desktopDir/256x256.png" "apps/desktop-client/icons/128x128@2x.png"
    Copy-Item -Force "$desktopDir/256x256.png" "apps/tablet-client/icons/256x256.png"
    Copy-Item -Force "$desktopDir/256x256.png" "apps/tablet-client/icons/128x128@2x.png"
    Write-Host "  [OK] Updated apps/*/icons/256x256.png and 128x128@2x.png" -ForegroundColor Green
}

foreach ($size in @("128x128.png", "64x64.png", "32x32.png")) {
    if (Test-Path "$desktopDir/$size") {
        Copy-Item -Force "$desktopDir/$size" "apps/desktop-client/icons/$size"
        Copy-Item -Force "$desktopDir/$size" "apps/tablet-client/icons/$size"
        Write-Host "  [OK] Updated apps/*/icons/$size" -ForegroundColor Green
    }
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
