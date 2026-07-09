# build-docs.ps1 — Build and open the OZ-POS documentation portal
[CmdletBinding()]
param(
    [switch]$Open = $true,
    [switch]$SkipRust = $false,
    [switch]$SkipUI = $false
)

$ErrorActionPreference = "Continue"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$WorkspaceRoot = Split-Path -Parent $ScriptDir

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host " Building OZ-POS Documentation Portal..." -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

if (-not $SkipRust) {
    Write-Host "`n[1/3] Generating Rust Workspace API Docs (cargo doc)..." -ForegroundColor Yellow
    Push-Location $WorkspaceRoot
    try {
        cargo doc --workspace --no-deps --document-private-items
        Write-Host "[SUCCESS] Rust documentation generated in target/doc/" -ForegroundColor Green
    }
    finally {
        Pop-Location
    }
}

if (-not $SkipUI) {
    Write-Host "`n[2/3] Generating Frontend TypeScript Docs (TypeDoc)..." -ForegroundColor Yellow
    Push-Location "$WorkspaceRoot\ui"
    try {
        $npxCmd = Get-Command npx -ErrorAction SilentlyContinue
        if ($null -ne $npxCmd) {
            Write-Host "Running TypeDoc across frontend domain models..." -ForegroundColor Cyan
            npx -y typedoc --skipErrorChecking --entryPointStrategy expand ./src/api ./src/types ./src/hooks --out ../docs/html/ui-docs
            if (Test-Path "../docs/html/ui-docs/index.html") {
                Write-Host "[SUCCESS] Frontend TypeScript documentation generated in docs/html/ui-docs/" -ForegroundColor Green
            } else {
                Write-Host "[WARNING] TypeDoc generation skipped or optional." -ForegroundColor Yellow
            }
        } else {
            Write-Host "[WARNING] npx not found on PATH, skipping TypeDoc generation." -ForegroundColor Yellow
        }
    }
    finally {
        Pop-Location
    }
}

Write-Host "`n[3/3] Verifying Documentation Portal Hub..." -ForegroundColor Yellow
$PortalIndex = "$WorkspaceRoot\docs\html\index.html"
if (Test-Path $PortalIndex) {
    Write-Host "[SUCCESS] Master Documentation Hub ready at: $PortalIndex" -ForegroundColor Green
    if ($Open) {
        Write-Host "`nOpening Documentation Portal in default browser..." -ForegroundColor Cyan
        Start-Process $PortalIndex
    }
} else {
    Write-Error "Documentation portal index.html not found at $PortalIndex"
}

Write-Host "`n==========================================" -ForegroundColor Cyan
Write-Host " Documentation Build Complete!" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
