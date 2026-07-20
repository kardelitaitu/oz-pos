#!/usr/bin/env bash
# Flamegraph Profiling Helper for Linux/macOS
# Usage: ./scripts/flamegraph.sh [--bench <bench_name>] [--bin <bin_name>]

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
