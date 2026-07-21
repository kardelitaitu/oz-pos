# DEPRECATED — Use scripts/profile.ps1 instead.
# This script is kept for backward compatibility but will be removed
# in a future release. scripts/profile.ps1 supports all options plus
# PID profiling, frequency control, timestamped output, and listing
# available benchmarks.
#
# Flamegraph Profiling Helper for Windows (PowerShell)
# Usage: powershell -File scripts/flamegraph.ps1 [-Bench <bench_name>] [-Binary <bin_name>]

param(
    [string]$Bench = "",
    [string]$Binary = ""
)

$ErrorActionPreference = "Stop"

Write-Host "WARNING: This script is deprecated. Use scripts/profile.ps1 instead." -ForegroundColor Yellow
Write-Host "  See: powershell -File scripts/profile.ps1 -Help" -ForegroundColor Yellow
Write-Host ""

if (-not (Get-Command "cargo-flamegraph" -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-flamegraph..." -ForegroundColor Yellow
    cargo install cargo-flamegraph
}

if ($Bench -ne "") {
    Write-Host "Profiling benchmark: $Bench..." -ForegroundColor Green
    cargo flamegraph --bench $Bench --output "flamegraph-$Bench.svg"
} elseif ($Binary -ne "") {
    Write-Host "Profiling binary: $Binary..." -ForegroundColor Green
    cargo flamegraph --bin $Binary --output "flamegraph-$Binary.svg"
} else {
    Write-Host "Profiling default workspace benchmarks..." -ForegroundColor Green
    cargo flamegraph --bench core_benchmarks --output "flamegraph-core.svg"
}

Write-Host "Flamegraph generated successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "TIP: Use scripts/profile.ps1 for more options (PID, freq, timestamped output)." -ForegroundColor Cyan
