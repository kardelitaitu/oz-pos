<#
.SYNOPSIS
    Automates version bumping across the entire OZ-POS codebase.

.DESCRIPTION
    This script finds all occurrences of the current codebase version (read dynamically from Cargo.toml)
    across Rust Cargo config files, Tauri app config files, UI packages, health route tests, and React
    status/footer views, and updates them to the target version.
    It then automatically refreshes the package lockfiles (Cargo.lock and package-lock.json).

.PARAMETER TargetVersion
    The new version number to bump the codebase to (e.g., "0.0.6").

.EXAMPLE
    powershell -File scripts\bump-version.ps1 "0.0.6"
    (Run this command from the project root workspace directory)
#>

param(
    [Parameter(Mandatory=$true)]
    [string]$TargetVersion
)

$ErrorActionPreference = "Stop"

# Ensure we are in workspace root
Set-Location (Split-Path -Parent $PSCommandPath)
Set-Location ..

# 1. Read current version from Cargo.toml
$cargoTomlPath = "Cargo.toml"
if (-not (Test-Path $cargoTomlPath)) {
    Write-Error "Could not find Cargo.toml in workspace root."
}
$cargoToml = Get-Content -Path $cargoTomlPath -Raw
$currentVersion = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value

if (-not $currentVersion) {
    Write-Error "Could not parse current version from Cargo.toml."
}

Write-Host "Current version detected: $currentVersion"
Write-Host "Target version: $TargetVersion"

if ($currentVersion -eq $TargetVersion) {
    Write-Host "Version is already at $TargetVersion. No changes needed."
    exit 0
}

# Helper function to do safe string replacement in a file
function Update-File {
    param(
        [string]$Path,
        [string]$OldString,
        [string]$NewString
    )
    if (Test-Path $Path) {
        $content = Get-Content -Path $Path -Raw
        if ($content.Contains($OldString)) {
            $updated = $content.Replace($OldString, $NewString)
            Set-Content -Path $Path -Value $updated -NoNewline
            Write-Host "Updated: $Path"
        } else {
            Write-Host "Skipped (target string not found): $Path" -ForegroundColor Yellow
        }
    } else {
        Write-Host "Warning: File not found: $Path" -ForegroundColor Red
    }
}

# 2. Update version strings in all codebase files
Write-Host "`nUpdating version strings..." -ForegroundColor Cyan

Update-File "AGENTS.md" "- **Version is locked at `$currentVersion`.**" "- **Version is locked at `$TargetVersion`.**"
Update-File "Cargo.toml" "version = `"$currentVersion`"" "version = `"$TargetVersion`""
Update-File "Dockerfile.server" "version = `"$currentVersion`"" "version = `"$TargetVersion`""
Update-File "apps/desktop-client/tauri.conf.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","
Update-File "apps/tablet-client/tauri.conf.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","
Update-File "ui/package.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","
Update-File "ui/package-lock.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","

Update-File "crates/oz-api/src/routes/health.rs" "version: `"$currentVersion`"," "version: `"$TargetVersion`","
Update-File "crates/oz-api/src/routes/health.rs" "`"version`":`"$currentVersion`"" "`"version`":`"$TargetVersion`""

Update-File "apps/desktop-client/src/commands/data.rs" "app_version: `"$currentVersion`".into()" "app_version: `"$TargetVersion`".into()"

Update-File "apps/desktop-client/src/commands/health.rs" "version: `"$currentVersion`"," "version: `"$TargetVersion`","
Update-File "apps/desktop-client/src/commands/health.rs" "assert_eq!(v.version, `"$currentVersion`");" "assert_eq!(v.version, `"$TargetVersion`");"

Update-File "apps/tablet-client/src/commands/health.rs" "version: `"$currentVersion`"," "version: `"$TargetVersion`","
Update-File "apps/tablet-client/src/commands/health.rs" "assert_eq!(v.version, `"$currentVersion`");" "assert_eq!(v.version, `"$TargetVersion`");"

Update-File "ui/src/__tests__/RetailOptionsScreen.test.tsx" ('expect(screen.getByDisplayValue("{0}"))' -f $currentVersion) ('expect(screen.getByDisplayValue("{0}"))' -f $TargetVersion)

Update-File "ui/src/features/auth/LicenseActivationScreen.tsx" ("useState<string>('{0}')" -f $currentVersion) ("useState<string>('{0}')" -f $TargetVersion)
Update-File "ui/src/features/auth/StaffLoginScreen.tsx" "OZ-POS Enterprise v$currentVersion" "OZ-POS Enterprise v$TargetVersion"
Update-File "ui/src/features/auth/__tests__/LicenseActivationScreen.test.tsx" "Version $currentVersion" "Version $TargetVersion"
Update-File "ui/src/features/design/TooltipPreview.tsx" "OZ-POS v$currentVersion" "OZ-POS v$TargetVersion"
Update-File "ui/src/features/retail/RetailOptionsScreen.tsx" "value=`"$currentVersion`"" "value=`"$TargetVersion`""
Update-File "ui/src/frontend/shell/StatusBar.tsx" "OZ-POS Enterprise v$currentVersion" "OZ-POS Enterprise v$TargetVersion"

# 3. Refresh Lockfiles
Write-Host "`nUpdating lockfiles..." -ForegroundColor Cyan

# Cargo.lock
Write-Host "Running cargo check to update Cargo.lock..."
& cargo check
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo check failed while updating Cargo.lock."
}

# ui/package-lock.json
if (Test-Path "ui") {
    Push-Location ui
    Write-Host "Running npm install --package-lock-only to sync package-lock.json..."
    & npm install --package-lock-only
    Pop-Location
}

Write-Host "`nVersion successfully bumped from $currentVersion to $TargetVersion!" -ForegroundColor Green
