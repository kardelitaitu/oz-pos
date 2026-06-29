# scripts/check.ps1 — Windows pre-push gate. Mirrors .github/workflows/ci.yml.
#
# Usage:  powershell -File scripts\check.ps1
#         (run from the workspace root)

$ErrorActionPreference = "Stop"

Set-Location (Split-Path -Parent $PSCommandPath) | Set-Location ..

$totalStart = Get-Date

function Step {
    param([string]$Name, [scriptblock]$ScriptBlock)
    Write-Host "==> $Name"
    $start = Get-Date
    & $ScriptBlock
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $elapsed = (Get-Date) - $start
    Write-Host ("OK $Name (" + $elapsed.TotalSeconds.ToString('0.0') + "s)")
}

# --- Rust (mirrors CI rust job) -------------------------------------------
Step -Name "cargo fmt"     -ScriptBlock { cargo fmt --all -- --check }
Step -Name "cargo clippy"  -ScriptBlock { cargo clippy --workspace --all-targets --all-features -- -D warnings }
Step -Name "cargo test"    -ScriptBlock { cargo test --workspace --all-features }

# --- Migration (mirrors CI migration job) ---------------------------------
Step -Name "migration smoke test"     -ScriptBlock { cargo run -p oz-cli -- migrate }
Step -Name "migration idempotency"    -ScriptBlock { cargo run -p oz-cli -- migrate }
Remove-Item -LiteralPath "oz-pos.db", "oz-pos.db-wal", "oz-pos.db-shm" -ErrorAction Ignore

# --- Skill drift guard (extra local guard; CI doesn't run this) -----------
if (Get-Command "bash" -ErrorAction SilentlyContinue) {
    Step -Name "skill-drift-guard" -ScriptBlock {
        bash .agents/skills/skill-drift-guard/scripts/detect.sh --report
        if ($LASTEXITCODE -ne 0) {
            Write-Host "WARNING: skill-drift-guard detected drift, but continuing..."
            $global:LASTEXITCODE = 0
        }
    }
} else {
    Write-Host "SKIP skill-drift-guard (bash not available)"
}

# --- UI (mirrors CI ui job - auto-detected) -------------------------------
if ((Get-Command "npm" -ErrorAction SilentlyContinue) -and (Test-Path "ui/package-lock.json")) {
    Push-Location ui
    Step -Name "npm ci"          -ScriptBlock { npm ci --no-audit --no-fund }
    Step -Name "ui lint"         -ScriptBlock { npm run lint }
    Step -Name "ui typecheck"    -ScriptBlock { npm run typecheck }
    Step -Name "ui test"         -ScriptBlock { npm run test }
    Step -Name "ui build"        -ScriptBlock { npm run build }
    Pop-Location
} else {
    Write-Host "SKIP UI checks (npm not available or ui/package-lock.json missing)"
}

# --- Done -----------------------------------------------------------------
$totalElapsed = (Get-Date) - $totalStart
Write-Host ("all checks passed (" + $totalElapsed.TotalSeconds.ToString('0.0') + "s)")