#!/usr/bin/env bash
# ── OZ-POS Dev Up (Linux / macOS) ───────────────────────────────────
#
# One-command local development startup:
#   1. Generates JWT secret if not set
#   2. Starts PostgreSQL, Redis, license-server, cloud-server via Docker
#   3. Waits for all health checks to pass
#   4. Prints service URLs + next steps
#
# Usage:
#   bash scripts/dev-up.sh              # SQLite mode (default)
#   bash scripts/dev-up.sh --pg         # PostgreSQL mode
#   bash scripts/dev-up.sh --build      # Rebuild images before starting
#   bash scripts/dev-up.sh --down       # Stop and clean volumes

set -euo pipefail

# ── Parse flags ───────────────────────────────────────────────────
PG_MODE=false
BUILD=false
DOWN=false

for arg in "$@"; do
  case "$arg" in
    --pg)    PG_MODE=true ;;
    --build) BUILD=true ;;
    --down)  DOWN=true ;;
    *)       echo "Unknown flag: $arg"; echo "Usage: dev-up.sh [--pg] [--build] [--down]"; exit 1 ;;
  esac
done

# cd to repo root
cd "$(dirname "$0")/.."

# ── Tear-down mode ────────────────────────────────────────────────
if $DOWN; then
  echo "👋 Tearing down OZ-POS dev environment..."
  if $PG_MODE; then
    docker compose --profile pg down -v
  else
    docker compose down -v
  fi
  echo "✅ Done. Volumes removed."
  exit 0
fi

# ── Prerequisites check ───────────────────────────────────────────
if ! command -v docker &>/dev/null; then
  echo "❌ Docker is required. Install from https://docker.com"
  exit 1
fi

# ── Generate JWT secret if not set ────────────────────────────────
if [ -z "${OZ_API_SECRET:-}" ]; then
  export OZ_API_SECRET=$(openssl rand -hex 32 2>/dev/null || uuidgen | tr -d '-' | head -c 64)
  echo "🔑 Generated OZ_API_SECRET (64-char hex)"
fi

# ── Check license key ─────────────────────────────────────────────
LICENSE_KEY_PATH="crates/oz-core/oz-license-private.pem"
if [ -z "${OZ_LICENSE_PRIVATE_KEY:-}" ]; then
  if [ -f "$LICENSE_KEY_PATH" ]; then
    export OZ_LICENSE_PRIVATE_KEY=$(cat "$LICENSE_KEY_PATH")
    echo "🔑 Loaded OZ_LICENSE_PRIVATE_KEY from $LICENSE_KEY_PATH"
  else
    echo "⚠️  OZ_LICENSE_PRIVATE_KEY not set and $LICENSE_KEY_PATH not found."
    echo "   Generate keys: bash scripts/generate-license-keys.sh"
  fi
fi

# ── Build (optional) ──────────────────────────────────────────────
if $BUILD; then
  echo "🔨 Building Docker images..."
  if $PG_MODE; then
    docker compose --profile pg build
  else
    docker compose build
  fi
fi

# ── Start services ────────────────────────────────────────────────
echo "🚀 Starting OZ-POS backend services..."
if $PG_MODE; then
  docker compose --profile pg up -d
else
  docker compose up -d
fi

# ── Wait for health checks ────────────────────────────────────────
echo "⏳ Waiting for services to become healthy..."

SERVICES="redis license-server pos-cloud-server"
if $PG_MODE; then SERVICES="redis pos-cloud-db license-server pos-cloud-server"; fi

TIMEOUT=120
ELAPSED=0
INTERVAL=3

while [ $ELAPSED -lt $TIMEOUT ]; do
  ALL_HEALTHY=true
  for SVC in $SERVICES; do
    STATUS=$(docker compose ps --format json "$SVC" 2>/dev/null | grep -o '"Health":"[^"]*"' | cut -d'"' -f4)
    if [ "$STATUS" != "healthy" ]; then
      ALL_HEALTHY=false
      break
    fi
  done
  if $ALL_HEALTHY; then break; fi
  sleep $INTERVAL
  ELAPSED=$((ELAPSED + INTERVAL))
done

if [ $ELAPSED -ge $TIMEOUT ]; then
  echo "⚠️  Health check timeout after ${TIMEOUT}s. Check logs: docker compose logs"
else
  echo "✅ All services healthy (${ELAPSED}s)"
fi

# ── Print service URLs ────────────────────────────────────────────
API_PORT="${OZ_API_PORT:-3099}"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  OZ-POS Backend — Ready                                  ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Cloud Server:    http://localhost:$API_PORT/api/health  ║"
echo "║  License Server:  http://localhost:8080/api/health       ║"
echo "║  Redis:           localhost:6379                         ║"
if $PG_MODE; then
  echo "║  PostgreSQL:      localhost:5432 (ozpos/ozpos)           ║"
fi
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Start desktop app: ./start-desktop.bat (or cargo run)   ║"
echo "║  Stop services:    bash scripts/dev-up.sh --down         ║"
echo "║  View logs:        docker compose logs -f                ║"
echo "╚══════════════════════════════════════════════════════════╝"
