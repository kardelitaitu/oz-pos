#!/usr/bin/env bash
# One-time setup for Rust compilation caching with sccache.
#
# Run once per machine:
#   bash scripts/setup-cache.sh
#
# What it does:
# 1. Installs sccache if missing
# 2. Sets a generous 20 GB local disk cache
# 3. Confirms sccache is wired as the rustc wrapper

set -euo pipefail

echo "==> Checking sccache…"
if ! command -v sccache &>/dev/null; then
    echo "    sccache not found. Installing via cargo…"
    cargo install sccache --locked
fi

SCCACHE_VERSION=$(sccache --version 2>&1 || true)
echo "    $SCCACHE_VERSION"

echo "==> Setting cache size to 20 GB…"
sccache --set-config cache.disk.size 20G

echo "==> Zeroing stats (fresh start)…"
sccache --zero-stats

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)/.."

echo "==> Verifying .cargo/config.toml …"
if grep -q 'rustc-wrapper.*sccache' "$ROOT_DIR/.cargo/config.toml" 2>/dev/null; then
    echo "    ✓ sccache wired as rustc-wrapper"
else
    echo "    ✗ .cargo/config.toml missing or not configured"
    echo "    The repo ships this file — make sure you're on main."
    exit 1
fi

echo ""
echo "Setup complete. Next:"
echo "  1. Run a cold build:  cargo clean && time cargo check --workspace --exclude oz-pos-app"
echo "  2. Run a warm build:  time cargo check --workspace --exclude oz-pos-app"
echo "  3. Check stats:       sccache --show-stats"
