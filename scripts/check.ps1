# scripts/check.ps1 — Windows pre-push gate. Mirrors .github/workflows/ci.yml.
#
# Usage:  powershell -File scripts\check.ps1
#         powershell -File scripts\check.ps1 -Fast   (dev: unit tests + fmt + clippy only)
#         (run from the workspace root)

param(
    [switch]$Fast = $false
)

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

# Use explicit --test-threads to match CPU count. Cargo's default is already
# num_cpus, but making it explicit documents intent and ensures consistent
# parallelism across CI and local environments.
#
# In --Fast mode, only run lib tests (skip integration test compilation).
$cpuCount = $env:NUMBER_OF_PROCESSORS
if (-not $cpuCount) { $cpuCount = 4 }
if ($Fast) {
    $testArgs = @('--lib')
    $retryArgs = "--lib"
} else {
    $testArgs = @()
    $retryArgs = ""
}
foreach ($pkg in $Packages) {
    Step -Name "test $pkg" -RetryCommand "cargo test -p $pkg --all-features $retryArgs -- --test-threads $cpuCount" -ScriptBlock {
        cargo test -p $pkg --all-features @testArgs -- --test-threads $cpuCount
    }
}

# --- Migration (mirrors CI migration job) ---------------------------------
if (-not $Fast) {
    Step -Name "migration smoke test" -RetryCommand "cargo run -p oz-cli -- migrate" -ScriptBlock { cargo run -p oz-cli -- migrate }
    Step -Name "migration idempotency" -RetryCommand "cargo run -p oz-cli -- migrate" -ScriptBlock { cargo run -p oz-cli -- migrate }
    Remove-Item -LiteralPath "oz-pos.db", "oz-pos.db-wal", "oz-pos.db-shm" -ErrorAction Ignore
}

# --- Skill drift guard (extra local guard; CI doesn't run this) -----------
if (-not $Fast) {
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
}

# --- UI (mirrors CI ui job - auto-detected) -------------------------------
if (-not $Fast -and (Get-Command "npm" -ErrorAction SilentlyContinue) -and (Test-Path "ui/package-lock.json")) {
    Push-Location ui
    Step -Name "npm ci" -RetryCommand "cd ui; npm ci --no-audit --no-fund" -ScriptBlock { npm ci --no-audit --no-fund }
    Step -Name "ui lint" -RetryCommand "cd ui; npm run lint" -ScriptBlock { npm run lint }
    Step -Name "ui typecheck" -RetryCommand "cd ui; npm run typecheck" -ScriptBlock { npm run typecheck }
    Step -Name "ui test" -RetryCommand "cd ui; npm run test" -ScriptBlock { npm run test }
    Step -Name "ui build" -RetryCommand "cd ui; npm run build" -ScriptBlock { npm run build }
    Pop-Location
} elseif ($Fast) {
    Write-Host "SKIP UI checks (--Fast mode)"
} else {
    Write-Host "SKIP UI checks (npm not available or ui/package-lock.json missing)"
}

# --- Generate stats.json (for shields.io badges) --------------------------
if (-not $Fast) {
    Step -Name "generate code stats" -RetryCommand "powershell -File scripts\stats.ps1" -ScriptBlock {
        & powershell -File scripts\stats.ps1
    }
}

# --- Done -----------------------------------------------------------------
$totalElapsed = (Get-Date) - $totalStart
if ($Fast) {
    Write-Host ("fast checks passed (" + $totalElapsed.TotalSeconds.ToString('0.0') + "s)")
} else {
    Write-Host ("all checks passed (" + $totalElapsed.TotalSeconds.ToString('0.0') + "s)")
}