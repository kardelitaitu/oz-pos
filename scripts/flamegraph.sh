#!/usr/bin/env bash
# DEPRECATED — Use scripts/profile.sh instead.
# This script is kept for backward compatibility but will be removed
# in a future release. scripts/profile.sh supports all options plus
# PID profiling, frequency control, timestamped output, listing, and
# sudo elevation.
#
# Flamegraph Profiling Helper for Linux/macOS
# Usage: ./scripts/flamegraph.sh [--bench <bench_name>] [--bin <bin_name>]

echo -e "\033[1;33mWARNING: This script is deprecated. Use scripts/profile.sh instead.\033[0m"
echo -e "\033[1;33m  See: bash scripts/profile.sh --help\033[0m"
echo ""

set -euo pipefail

if ! command -v cargo-flamegraph &> /dev/null; then
    echo "Installing cargo-flamegraph..."
    cargo install cargo-flamegraph
fi

BENCH=""
BIN=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --bench)
            BENCH="$2"
            shift 2
            ;;
        --bin)
            BIN="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

if [ -n "$BENCH" ]; then
    echo "Profiling benchmark: $BENCH..."
    cargo flamegraph --bench "$BENCH" --output "flamegraph-$BENCH.svg"
elif [ -n "$BIN" ]; then
    echo "Profiling binary: $BIN..."
    cargo flamegraph --bin "$BIN" --output "flamegraph-$BIN.svg"
else
    echo "Profiling default core benchmarks..."
    cargo flamegraph --bench core_benchmarks --output "flamegraph-core.svg"
fi

echo "Flamegraph generated successfully!"
echo ""
echo -e "TIP: Use bash scripts/profile.sh for more options (PID, freq, timestamped output)."
