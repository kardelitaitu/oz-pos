# scripts/whitelabel.ps1 - Whitelabel brand injection engine.
#
# Thin wrapper around scripts/sync-branding.ps1, aliased to match
# the TODO.md plan name. Same usage:
#
#   powershell -File scripts\whitelabel.ps1                     # default brand
#   powershell -File scripts\whitelabel.ps1 -Brand "my-tenant"  # custom tenant
#   powershell -File scripts\whitelabel.ps1 -DryRun             # preview
#
# Note: uses direct parameter binding (not splatting) to ensure
# compatibility across PowerShell versions.

param(
    [string]$Brand = "default",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$syncScript = Join-Path $scriptDir "sync-branding.ps1"

if (-not (Test-Path $syncScript)) {
    Write-Host "Error: sync-branding.ps1 not found at $syncScript" -ForegroundColor Red
    exit 1
}

& $syncScript -Brand:$Brand -DryRun:$DryRun
