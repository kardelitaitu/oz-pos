#!/usr/bin/env bash
# One-time setup for Rust compilation caching with sccache.
#
# Run once per machine:
#   bash scripts/setup-cache.sh
#
# What it does:
# 1. Installs sccache if missing
# 2. Sets a generous 20 GB local disk cache
# 3. Verifies sccache is active as the rustc wrapper

set -euo pipefail

echo "==> Checking sccache..."
if ! command -v sccache &>/dev/null; then
    echo "    sccache not found. Installing via cargo..."
    cargo install sccache --locked
fi

SCCACHE_VERSION=$(sccache --version 2>&1 || true)
echo "    $SCCACHE_VERSION"

echo "==> Setting cache size to 20 GB..."
sccache --set-config cache.disk.size 20G

echo "==> Zeroing stats (fresh start)..."
sccache --zero-stats

echo "==> Verifying sccache is enabled (not commented) in .cargo/config.toml ..."
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CONFIG="$ROOT_DIR/.cargo/config.toml"
if grep -q '^rustc-wrapper.*sccache' "$CONFIG" 2>/dev/null; then
    echo "    ✓ sccache enabled as rustc-wrapper (uncommented)"
else
    echo "    ✗ sccache not wired or still commented in .cargo/config.toml"
    echo "    The repo ships this file uncommented -- make sure you have the latest version."
    exit 1
fi

echo ""
echo "Setup complete. Next:"
echo "  1. Run a cold build:  cargo clean && time cargo check --workspace --exclude oz-pos-app"
echo "  2. Run a warm build:  time cargo check --workspace --exclude oz-pos-app"
echo "  3. Check stats:       sccache --show-stats"
