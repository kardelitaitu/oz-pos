#!/usr/bin/env bash
# scripts/test-tdd.sh — Fast TDD loop: compile+test the current crate only
#
# Uses the `[profile.tdd]` from workspace Cargo.toml which inherits from
# `dev` but sets `debug = false` and `incremental = true` for the fastest
# possible edit-compile-test cycle.
#
# Usage:
#   bash scripts/test-tdd.sh                  # auto-detect crate from cwd
#   bash scripts/test-tdd.sh -p oz-core       # specific crate
#   bash scripts/test-tdd.sh --nextest        # use cargo-nextest (parallel)
#   bash scripts/test-tdd.sh --watch          # watch mode (re-run on changes)
#
# Recommended for local TDD workflow:
#   $ cd crates/oz-core
#   $ bash scripts/test-tdd.sh --watch

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

export CARGO_PROFILE=tdd

USE_NEXTEST=false
WATCH_MODE=false
TARGET_CRATE=""

# Parse flags
while [[ $# -gt 0 ]]; do
  case "$1" in
    -p) TARGET_CRATE="$2"; shift 2 ;;
    --nextest) USE_NEXTEST=true; shift ;;
    --watch) WATCH_MODE=true; shift ;;
    *) echo "Unknown flag: $1"; exit 1 ;;
  esac
done

# Auto-detect crate from current working directory
if [ -z "$TARGET_CRATE" ]; then
  CURRENT_DIR="$(pwd)"
  # Walk up from cwd to find Cargo.toml with a [package] section
  DIR="$CURRENT_DIR"
  while [ "$DIR" != "$PROJECT_ROOT" ] && [ "$DIR" != "/" ]; do
    if [ -f "$DIR/Cargo.toml" ] && grep -q '^\[package\]' "$DIR/Cargo.toml" 2>/dev/null; then
      TARGET_CRATE="$DIR"
      break
    fi
    DIR="$(dirname "$DIR")"
  done

  if [ -z "$TARGET_CRATE" ]; then
    echo "test-tdd: could not auto-detect crate from $CURRENT_DIR"
    echo "  Specify one with: bash scripts/test-tdd.sh -p oz-core"
    exit 1
  fi
fi

echo "test-tdd: profile=tdd (debug=false, incremental=true)"
echo "test-tdd: crate=$TARGET_CRATE"

if [ "$WATCH_MODE" = "true" ]; then
  echo "test-tdd: watch mode — re-running on .rs changes"
  if [ "$USE_NEXTEST" = "true" ]; then
    cargo watch -x "nextest run --manifest-path $TARGET_CRATE/Cargo.toml"
  else
    cargo watch -x "test --manifest-path $TARGET_CRATE/Cargo.toml"
  fi
elif [ "$USE_NEXTEST" = "true" ]; then
  exec cargo nextest run --manifest-path "$TARGET_CRATE/Cargo.toml"
else
  exec cargo test --manifest-path "$TARGET_CRATE/Cargo.toml"
fi
