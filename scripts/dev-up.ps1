# ── OZ-POS Dev Up (Windows PowerShell) ───────────────────────────────
#
# One-command local development startup:
#   1. Generates JWT secret if not set
#   2. Starts PostgreSQL, Redis, license-server, cloud-server via Docker
#   3. Waits for all health checks to pass
#   4. Prints service URLs + next steps
#
# Usage:
#   .\scripts\dev-up.ps1              # SQLite mode (default)
#   .\scripts\dev-up.ps1 -Pg          # PostgreSQL mode
#   .\scripts\dev-up.ps1 -Build       # Rebuild images before starting
#   .\scripts\dev-up.ps1 -Down        # Stop and clean volumes

param(
  [switch]$Pg,        # Enable PostgreSQL backend
  [switch]$Build,     # Rebuild Docker images
  [switch]$Down       # Tear down (docker compose down -v)
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent (Split-Path -Parent $PSCommandPath))

# ── Tear-down mode ────────────────────────────────────────────────
if ($Down) {
  Write-Host "👋 Tearing down OZ-POS dev environment..." -ForegroundColor Yellow
  if ($Pg) {
    docker compose --profile pg down -v
  } else {
    docker compose down -v
  }
  Write-Host "✅ Done. Volumes removed." -ForegroundColor Green
  exit 0
}

# ── Prerequisites check ───────────────────────────────────────────
if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
  Write-Host "❌ Docker is required. Install Docker Desktop from https://docker.com" -ForegroundColor Red
  exit 1
}

# ── Generate JWT secret if not set ────────────────────────────────
if (-not $env:OZ_API_SECRET) {
  $hexChars = '0','1','2','3','4','5','6','7','8','9','a','b','c','d','e','f'
  $secret = -join (1..64 | ForEach-Object { Get-Random -InputObject $hexChars })
  $env:OZ_API_SECRET = $secret
  Write-Host "🔑 Generated OZ_API_SECRET (64-char hex)" -ForegroundColor Cyan
}

# ── Check license key ─────────────────────────────────────────────
$licenseKeyPath = "crates\oz-core\oz-license-private.pem"
if (-not $env:OZ_LICENSE_PRIVATE_KEY) {
  if (Test-Path $licenseKeyPath) {
    $env:OZ_LICENSE_PRIVATE_KEY = (Get-Content $licenseKeyPath -Raw).Trim()
    Write-Host "🔑 Loaded OZ_LICENSE_PRIVATE_KEY from $licenseKeyPath" -ForegroundColor Cyan
  } else {
    Write-Host "⚠️  OZ_LICENSE_PRIVATE_KEY not set and $licenseKeyPath not found." -ForegroundColor Yellow
    Write-Host "   Generate keys: .\scripts\generate-license-keys.ps1" -ForegroundColor Yellow
  }
}

# ── Build (optional) ──────────────────────────────────────────────
if ($Build) {
  Write-Host "🔨 Building Docker images..." -ForegroundColor Cyan
  if ($Pg) {
    docker compose --profile pg build
  } else {
    docker compose build
  }
}

# ── Start services ────────────────────────────────────────────────
Write-Host "🚀 Starting OZ-POS backend services..." -ForegroundColor Cyan
if ($Pg) {
  docker compose --profile pg up -d
} else {
  docker compose up -d
}

# ── Wait for health checks ────────────────────────────────────────
Write-Host "⏳ Waiting for services to become healthy..." -ForegroundColor Yellow

$services = @("redis", "license-server", "pos-cloud-server")
if ($Pg) { $services = @("redis", "pos-cloud-db", "license-server", "pos-cloud-server") }

$timeout = 120
$elapsed = 0
$interval = 3

while ($elapsed -lt $timeout) {
  $allHealthy = $true
  foreach ($svc in $services) {
    $status = docker compose ps --format json $svc 2>$null | ConvertFrom-Json | Select-Object -ExpandProperty Health -ErrorAction SilentlyContinue
    if ($status -ne "healthy") {
      $allHealthy = $false
      break
    }
  }
  if ($allHealthy) { break }
  Start-Sleep -Seconds $interval
  $elapsed += $interval
}

if ($elapsed -ge $timeout) {
  Write-Host "⚠️  Health check timeout after ${timeout}s. Check logs: docker compose logs" -ForegroundColor Yellow
} else {
  Write-Host "✅ All services healthy (${elapsed}s)" -ForegroundColor Green
}

# ── Print service URLs ────────────────────────────────────────────
$apiPort = if ($env:OZ_API_PORT) { $env:OZ_API_PORT } else { "3099" }

Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║  OZ-POS Backend — Ready                                  ║" -ForegroundColor Green
Write-Host "╠══════════════════════════════════════════════════════════╣" -ForegroundColor Green
Write-Host "║  Cloud Server:    http://localhost:$apiPort/api/health        ║" -ForegroundColor Green
Write-Host "║  License Server:  http://localhost:8080/api/health       ║" -ForegroundColor Green
Write-Host "║  Redis:           localhost:6379                         ║" -ForegroundColor Green
if ($Pg) {
  Write-Host "║  PostgreSQL:      localhost:5432 (ozpos/ozpos)           ║" -ForegroundColor Green
}
Write-Host "╠══════════════════════════════════════════════════════════╣" -ForegroundColor Green
Write-Host "║  Start desktop app: .\start-desktop.bat                  ║" -ForegroundColor Green
Write-Host "║  Stop services:    .\scripts\dev-up.ps1 -Down             ║" -ForegroundColor Green
Write-Host "║  View logs:        docker compose logs -f                ║" -ForegroundColor Green
Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green
