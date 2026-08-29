# scripts/poll-pr-checks.ps1 — Poll CI checks every 30s with fail-fast early exit
#
# Usage:
#   pwsh scripts/poll-pr-checks.ps1
#   pwsh scripts/poll-pr-checks.ps1 -Pr 57 -Interval 30

param (
    [string]$Pr = "",
    [int]$Interval = 30
)

if (-not $Pr) {
    $Pr = (gh pr view --json number -q .number 2>$null)
    if (-not $Pr) {
        Write-Error "Could not determine PR number for current branch. Specify -Pr <number>."
        exit 1
    }
}

Write-Host "Monitoring checks for PR #$Pr (polling every ${Interval}s, fail-fast on early failure)..."

while ($true) {
    $checks = gh pr checks $Pr 2>$null
    if ($LASTEXITCODE -ne 0 -and -not $checks) {
        Write-Warning "Could not fetch PR checks. Retrying in ${Interval}s..."
        Start-Sleep -Seconds $Interval
        continue
    }

    $failed = $checks | Where-Object { $_ -match "\bfail\b" }
    if ($failed) {
        Write-Host ""
        Write-Host "❌ Early CI failure detected! ($($failed.Count) failed check(s)):" -ForegroundColor Red
        $failed | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
        Write-Host ""
        Write-Host "Exiting watch early so you can repair immediately." -ForegroundColor Yellow
        exit 1
    }

    $pending = $checks | Where-Object { $_ -match "\bpending\b" }
    $passing = $checks | Where-Object { $_ -match "\bpass\b" }
    
    if (-not $pending -and $checks.Count -gt 0) {
        Write-Host ""
        Write-Host "✅ All $($passing.Count) checks passed!" -ForegroundColor Green
        exit 0
    }

    $time = Get-Date -Format "HH:mm:ss"
    Write-Host "[$time] In progress: $($passing.Count) passed, $($pending.Count) pending... re-checking in ${Interval}s"
    Start-Sleep -Seconds $Interval
}
