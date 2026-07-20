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

# --- Phase 1: auto-fix --------------------------------------------------
# Best-effort fixes so the strict pass below doesn't waste time on
# trivial issues. `--allow-dirty` handles both clean and dirty trees;
# `--allow warnings` means non-auto-fixable lints don't kill this pass —
# they'll be caught by the `-D warnings` check in phase 2.
Step -Name "clippy auto-fix" -RetryCommand "cargo clippy --fix --allow-dirty -- --allow warnings" -ScriptBlock {
    cargo clippy --fix --allow-dirty -- --allow warnings
}
Step -Name "cargo fmt" -RetryCommand "cargo fmt --all" -ScriptBlock { cargo fmt --all }

# --- Phase 2: strict verify (mirrors CI rust job) -----------------------
# Read-only checks that exit non-zero on any remaining issue.
Step -Name "cargo fmt (verify)" -RetryCommand "cargo fmt --all -- --check" -ScriptBlock {
    cargo fmt --all -- --check
}
Step -Name "clippy workspace" -RetryCommand "cargo clippy --workspace --all-targets -- -D warnings" -ScriptBlock {
    cargo clippy --workspace --all-targets -- -D warnings
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
# Workspace-wide test (single compilation pass instead of N per-package invocations).
Step -Name "test workspace" -RetryCommand "cargo test --workspace --all-features $retryArgs -- --test-threads $cpuCount" -ScriptBlock {
    cargo test --workspace --all-features @testArgs -- --test-threads $cpuCount
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

    # On Windows, Vite/Rollup native .node binaries (e.g. rollup-win32-x64-msvc.node)
    # can remain locked by Node worker-thread processes for a few seconds after
    # the previous step finishes, causing `npm ci` to fail with EPERM -4048.
    #
    # Strategy:
    #   1. Give workers up to 5 s to release the file naturally.
    #   2. If still locked, force-remove the specific .node file so npm ci
    #      can re-download it cleanly — this is safe because npm ci always
    #      performs a full, deterministic reinstall from package-lock.json.
    #   3. Retry npm ci up to 3 times with a 3 s back-off on any EPERM.
    Step -Name "npm ci" -RetryCommand "cd ui; npm ci --no-audit --no-fund" -ScriptBlock {
        # Step 1: short wait for worker-thread DLL release
        $lockedNode = "node_modules\@rollup\rollup-win32-x64-msvc\rollup.win32-x64-msvc.node"
        if (Test-Path $lockedNode) {
            $waited = 0
            while ($waited -lt 5) {
                try {
                    $fs = [System.IO.File]::Open($lockedNode,
                        [System.IO.FileMode]::Open,
                        [System.IO.FileAccess]::ReadWrite,
                        [System.IO.FileShare]::None)
                    $fs.Close()
                    break   # file is free
                } catch {
                    Start-Sleep -Seconds 1
                    $waited++
                }
            }

            # Step 2: if still locked after 5 s, force-remove it
            if ($waited -ge 5) {
                Remove-Item -LiteralPath $lockedNode -Force -ErrorAction SilentlyContinue
            }
        }

        # Step 3: npm ci with up to 3 retries on EPERM
        $maxAttempts = 3
        for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
            $global:LASTEXITCODE = 0
            $result = npm ci --no-audit --no-fund 2>&1
            if ($LASTEXITCODE -eq 0) { break }
            $isEperm = $result -match 'EPERM|operation not permitted'
            if ($isEperm -and $attempt -lt $maxAttempts) {
                Write-Host "" # newline after the "checking..." prompt
                Write-Host "  [npm ci] EPERM on attempt $attempt — waiting 3 s before retry..."
                Start-Sleep -Seconds 3
            } elseif ($attempt -eq $maxAttempts) {
                # Final failure — let the Step wrapper report it
                $result | Out-String | Write-Host
            }
        }
    }
    Step -Name "ui lint" -RetryCommand "cd ui; npm run lint" -ScriptBlock { npm run lint }
    Step -Name "ui typecheck" -RetryCommand "cd ui; npm run typecheck" -ScriptBlock { npm run typecheck }
    Step -Name "ui test" -RetryCommand "cd ui; npm run test" -ScriptBlock { npm run test }
    # Skip `npm run build` in local check: typecheck + vitest already cover
    # correctness; the Vite production bundle is validated by CI independently.
    Pop-Location
} else {
    Write-Host "SKIP UI checks (npm not available or ui/package-lock.json missing)"
}

# --- Generate stats.json (for shields.io badges) --------------------------
Step -Name "generate code stats" -RetryCommand "powershell -File scripts\stats.ps1" -ScriptBlock {
    & powershell -File scripts\stats.ps1
}

# --- Done -----------------------------------------------------------------
$totalElapsed = (Get-Date) - $totalStart
$label = if ($Fast) { "fast" } else { "all" }
Write-Host ("$label checks passed (" + $totalElapsed.TotalSeconds.ToString('0.0') + "s)")

# --- Commit suggestion ----------------------------------------------------
Write-Host ""
Write-Host "Now make a local commit:"
Write-Host ""
Write-Host "  1. git add <files>     # stage only intended files"
Write-Host "  2. git commit          # write a message following the guidelines below"
Write-Host ""
Write-Host "Commit message guidelines:"
Write-Host "  - Keep the summary line under 50 characters, imperative mood, no period"
Write-Host "  - Leave a blank line after the summary"
Write-Host "  - Use bullet points (- or *) for the body - focus on WHAT and WHY, not how"
Write-Host "  - Reference related docs/decisions or issue numbers where relevant"
Write-Host "  - Keep each bullet under 72 characters"
Write-Host ""
Write-Host "Example:"
Write-Host ""
Write-Host "    feat(sales): add deduction location override via PIN"
Write-Host ""
Write-Host "    - Clicking the badge opens FastPINOverlay for PIN verification"
Write-Host "    - Store method overrides deduction location with IMMEDIATE transaction"
Write-Host "    - Badge shows '(Override)' indicator after successful override"
Write-Host ""
Write-Host "    References ADR-19"
Write-Host ""