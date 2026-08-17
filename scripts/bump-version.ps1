<#
.SYNOPSIS
    Automates version bumping across the entire OZ-POS codebase.

.DESCRIPTION
    This script finds all occurrences of the current codebase version (read dynamically from Cargo.toml)
    across Rust Cargo config files, Tauri app config files, UI packages, Fluent statusbar labels,
    the website package + i18n strings, health route tests, React status/footer views, the Docker
    cache-priming manifests, and the version-lock lines in the AGENTS.md mirrors, and updates them
    to the target version.
    It then automatically refreshes the package lockfiles (Cargo.lock and the ui/ + website/ package-lock.json files).
    It also inserts the "## [X.Y.Z] - date" heading into the canonical CHANGELOG.md so the
    AUDIT-28 release version gate (scripts/check-release-version.mjs) passes when the tag is
    created.

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
        # UTF-8 read/write: the files are BOM-less UTF-8, but Windows PowerShell 5.1
        # defaults to ANSI (cp1252), which mangles non-ASCII (em-dashes in i18n
        # strings and CHANGELOG headings) on read and writes mojibake on write.
        $utf8 = New-Object System.Text.UTF8Encoding($false)
        $content = [System.IO.File]::ReadAllText($Path, $utf8)
        if ($content.Contains($OldString)) {
            $updated = $content.Replace($OldString, $NewString)
            [System.IO.File]::WriteAllText($Path, $updated, $utf8)
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

# NOTE: the Markdown backticks around the version must be DOUBLED in the
# PowerShell string - a single backtick escapes the `$` and renders a
# literal "$currentVersion", so the pattern would never match and the file
# would be silently skipped (it happened for 0.0.26).
Update-File "AGENTS.md" "- **Version is locked at the current release (``$currentVersion``).** Never change the version number" "- **Version is locked at the current release (``$TargetVersion``).** Never change the version number"
Update-File ".agents/AGENTS.md" "- **Version is locked at ``$currentVersion``.** Never change the version number" "- **Version is locked at ``$TargetVersion``.** Never change the version number"
Update-File "Cargo.toml" "version = `"$currentVersion`"" "version = `"$TargetVersion`""
Update-File "Dockerfile.server" "version = `"$currentVersion`"" "version = `"$TargetVersion`""
Update-File "apps/desktop-client/tauri.conf.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","
Update-File "apps/tablet-client/tauri.conf.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","
Update-File "ui/package.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","
Update-File "ui/package-lock.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","

Update-File "apps/desktop-client/src/commands/data.rs" "app_version: `"$currentVersion`".into()" "app_version: `"$TargetVersion`".into()"

Update-File "apps/desktop-client/src/commands/health.rs" "version: `"$currentVersion`"," "version: `"$TargetVersion`","
Update-File "apps/desktop-client/src/commands/health.rs" "assert_eq!(v.version, `"$currentVersion`");" "assert_eq!(v.version, `"$TargetVersion`");"

Update-File "apps/tablet-client/src/commands/health.rs" "version: `"$currentVersion`"," "version: `"$TargetVersion`","
Update-File "apps/tablet-client/src/commands/health.rs" "assert_eq!(v.version, `"$currentVersion`");" "assert_eq!(v.version, `"$TargetVersion`");"

Update-File "ui/src/features/auth/LicenseActivationScreen.tsx" ("useState<string>('{0}')" -f $currentVersion) ("useState<string>('{0}')" -f $TargetVersion)
Update-File "ui/src/features/auth/StaffLoginScreen.tsx" "OZ-POS Enterprise v$currentVersion" "OZ-POS Enterprise v$TargetVersion"
Update-File "ui/src/features/auth/__tests__/LicenseActivationScreen.test.tsx" "Version $currentVersion" "Version $TargetVersion"
Update-File "ui/src/features/design/TooltipPreview.tsx" "OZ-POS v$currentVersion" "OZ-POS v$TargetVersion"

# The status-bar version label lives in Fluent, not TSX (StatusBar.tsx renders the
# `statusbar-version` key), so the FTL files are the real bump targets.
Update-File "ui/src/locales/shared.ftl" "statusbar-version = OZ-POS Enterprise v$currentVersion" "statusbar-version = OZ-POS Enterprise v$TargetVersion"
Update-File "ui/src/locales/shared.id.ftl" "statusbar-version = OZ-POS Enterprise v$currentVersion" "statusbar-version = OZ-POS Enterprise v$TargetVersion"

# Website (marketing site): package version + i18n version strings. Single-quoted
# format strings keep the em-dash out of the source; it is injected via [char]0x2014.
Update-File "website/package.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","
Update-File "website/package-lock.json" "`"version`": `"$currentVersion`"," "`"version`": `"$TargetVersion`","
Update-File "website/src/i18n/en.json" ('"versionValue": "{0}"' -f $currentVersion) ('"versionValue": "{0}"' -f $TargetVersion)
Update-File "website/src/i18n/en.json" ('"subtitle": "Version {0} {1} free 90-day trial, no signup required."' -f $currentVersion, [char]0x2014) ('"subtitle": "Version {0} {1} free 90-day trial, no signup required."' -f $TargetVersion, [char]0x2014)
Update-File "website/src/i18n/id.json" ('"versionValue": "{0}"' -f $currentVersion) ('"versionValue": "{0}"' -f $TargetVersion)
Update-File "website/src/i18n/id.json" ('"subtitle": "Versi {0} {1} uji coba gratis 90 hari, tanpa pendaftaran."' -f $currentVersion, [char]0x2014) ('"subtitle": "Versi {0} {1} uji coba gratis 90 hari, tanpa pendaftaran."' -f $TargetVersion, [char]0x2014)

# Dockerfile.unified carries the same cache-priming manifests as Dockerfile.server.
Update-File "Dockerfile.unified" "version = `"$currentVersion`"" "version = `"$TargetVersion`""

# .prime/AGENTS.md mirrors the root version-lock line (same wording as AGENTS.md).
Update-File ".prime/AGENTS.md" "- **Version is locked at the current release (``$currentVersion``).** Never change the version number" "- **Version is locked at the current release (``$TargetVersion``).** Never change the version number"

# README's "Latest release" claim (prose, updated per release).
Update-File "README.md" "Latest release: **v$currentVersion** (on branch ``$currentVersion``)." "Latest release: **v$TargetVersion** (on branch ``$TargetVersion``)."

# 2b. Sync canonical CHANGELOG.md heading (RELEASE-07)
Write-Host "`nSyncing CHANGELOG.md heading..." -ForegroundColor Cyan
$changelogPath = "CHANGELOG.md"
if (Test-Path $changelogPath) {
    $content = [System.IO.File]::ReadAllText($changelogPath, (New-Object System.Text.UTF8Encoding($false)))
    $date = Get-Date -Format "yyyy-MM-dd"
    # Build the em-dash via [char] so the script source stays pure-ASCII; the
    # heading is written as proper UTF-8 by the UTF-8 writer below (previously
    # PowerShell 5.1's ANSI Set-Content emitted a cp1252 0x97 byte instead).
    $heading = "## [$TargetVersion] $([char]0x2014) $date"
    $headingRe = "(?m)^## \[${TargetVersion}\]"
    if ($content -match $headingRe) {
        Write-Host "Skipped (heading already present): $changelogPath" -ForegroundColor Yellow
    } else {
        # Insert the new heading right after the intro block (before the first "## [").
        $insertAfter = [regex]::Match($content, "(?m)^## \[")
        if ($insertAfter.Success) {
            $block = "$heading`r`n`r`nRelease notes: see docs/releases/CHANGELOG-$TargetVersion.md (reviewed before tagging).`r`n`r`n---`r`n`r`n"
            $updated = $content.Substring(0, $insertAfter.Index) + $block + $content.Substring($insertAfter.Index)
            [System.IO.File]::WriteAllText($changelogPath, $updated, (New-Object System.Text.UTF8Encoding($false)))
            Write-Host "Updated: $changelogPath (inserted $heading)"
        } else {
            Write-Host "Skipped (no existing '## [' headings to anchor): $changelogPath" -ForegroundColor Yellow
        }
    }
} else {
    Write-Host "Warning: File not found: $changelogPath" -ForegroundColor Red
}

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

# website/package-lock.json
if (Test-Path "website") {
    Push-Location website
    Write-Host "Running npm install --package-lock-only to sync website/package-lock.json..."
    & npm install --package-lock-only
    Pop-Location
}

Write-Host "`nVersion successfully bumped from $currentVersion to $TargetVersion!" -ForegroundColor Green
