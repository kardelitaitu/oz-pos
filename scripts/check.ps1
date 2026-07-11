# scripts/check.ps1 — Windows pre-push gate. Mirrors .github/workflows/ci.yml.
#
# Usage:  powershell -File scripts\check.ps1
#         (run from the workspace root)

$ErrorActionPreference = "Stop"

Set-Location (Split-Path -Parent $PSCommandPath)
Set-Location ..

$totalStart = Get-Date
$script:stepCounter = 1

function Step {
    param(
        [string]$Name,
        [string]$RetryCommand,
        [scriptblock]$ScriptBlock
    )
    $stepStr = "{0:D2}" -f $script:stepCounter
    Write-Host "$stepStr. checking $Name... " -NoNewline
    $script:stepCounter++

    $start = Get-Date
    $failed = $false
    $oldEAP = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    $output = ""
    try {
        $global:LASTEXITCODE = 0
        $output = & $ScriptBlock 2>&1
        if ($LASTEXITCODE -ne 0) {
            $failed = $true
        }
    } catch {
        $failed = $true
        $output = $_
    } finally {
        $ErrorActionPreference = $oldEAP
    }

    if ($failed) {
        Write-Host "FAIL" -ForegroundColor Red
        Write-Host "Output/Error from failed command:"
        Write-Host $output
        Write-Host "run `"$RetryCommand`" for full detailed error messages"
        exit 1
    } else {
        $elapsed = (Get-Date) - $start
        Write-Host "PASS (" -NoNewline
        Write-Host ($elapsed.TotalSeconds.ToString('0.0') + "s)")
    }
}

# --- Rust (mirrors CI rust job) -------------------------------------------
Step -Name "cargo fmt (format)" -ScriptBlock { cargo fmt --all }
Step -Name "cargo fmt (check)" -RetryCommand "cargo fmt --all -- --check" -ScriptBlock { cargo fmt --all -- --check }

$Packages = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).packages.name

foreach ($pkg in $Packages) {
    Step -Name "clippy $pkg" -RetryCommand "cargo clippy -p $pkg --all-targets --all-features -- -D warnings" -ScriptBlock {
        cargo clippy -p $pkg --all-targets --all-features -- -D warnings
    }
}

foreach ($pkg in $Packages) {
    Step -Name "test $pkg" -RetryCommand "cargo test -p $pkg --all-features" -ScriptBlock {
        cargo test -p $pkg --all-features
    }
}

# --- Migration (mirrors CI migration job) ---------------------------------
Step -Name "migration smoke test" -RetryCommand "cargo run -p oz-cli -- migrate" -ScriptBlock { cargo run -p oz-cli -- migrate }
Step -Name "migration idempotency" -RetryCommand "cargo run -p oz-cli -- migrate" -ScriptBlock { cargo run -p oz-cli -- migrate }
Remove-Item -LiteralPath "oz-pos.db", "oz-pos.db-wal", "oz-pos.db-shm" -ErrorAction Ignore

# --- Skill drift guard (extra local guard; CI doesn't run this) -----------
$gitBash = if (Test-Path "C:\Program Files\Git\bin\bash.exe") {
    "C:\Program Files\Git\bin\bash.exe"
} elseif (Get-Command "bash" -ErrorAction SilentlyContinue) {
    (Get-Command "bash").Source
} else {
    $null
}
if ($gitBash) {
    Step -Name "skill-drift-guard" -RetryCommand "& '$gitBash' .agents/skills/skill-drift-guard/scripts/detect.sh --report" -ScriptBlock {
        & "C:\Program Files\Git\bin\bash.exe" .agents/skills/skill-drift-guard/scripts/detect.sh --report
    }
} else {
    Write-Host "SKIP skill-drift-guard (bash not available)"
}

# --- UI (mirrors CI ui job - auto-detected) -------------------------------
if ((Get-Command "npm" -ErrorAction SilentlyContinue) -and (Test-Path "ui/package-lock.json")) {
    Push-Location ui
    Step -Name "npm ci" -RetryCommand "cd ui; npm ci --no-audit --no-fund" -ScriptBlock { npm ci --no-audit --no-fund }
    Step -Name "ui lint" -RetryCommand "cd ui; npm run lint" -ScriptBlock { npm run lint }
    Step -Name "ui typecheck" -RetryCommand "cd ui; npm run typecheck" -ScriptBlock { npm run typecheck }
    Step -Name "ui test" -RetryCommand "cd ui; npm run test" -ScriptBlock { npm run test }
    Step -Name "ui build" -RetryCommand "cd ui; npm run build" -ScriptBlock { npm run build }
    Pop-Location
} else {
    Write-Host "SKIP UI checks (npm not available or ui/package-lock.json missing)"
}

# --- Done -----------------------------------------------------------------
$totalElapsed = (Get-Date) - $totalStart
Write-Host ("all checks passed (" + $totalElapsed.TotalSeconds.ToString('0.0') + "s)")