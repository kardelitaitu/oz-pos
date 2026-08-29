#!/usr/bin/env bash
# scripts/poll-pr-checks.sh — Poll CI checks every 30s with fail-fast early exit
set -euo pipefail

PR="${1:-}"
INTERVAL="${2:-30}"

if [ -z "$PR" ]; then
    PR=$(gh pr view --json number -q .number 2>/dev/null || true)
    if [ -z "$PR" ]; then
        echo "Error: Could not determine PR number for current branch. Specify PR number as argument: ./scripts/poll-pr-checks.sh <PR_NUMBER>" >&2
        exit 1
    fi
fi

echo "Monitoring checks for PR #$PR (polling every ${INTERVAL}s, fail-fast on early failure)..."

while true; do
    CHECKS=$(gh pr checks "$PR" 2>/dev/null || true)
    
    if echo "$CHECKS" | grep -E '\bfail\b' > /dev/null; then
        echo ""
        echo "❌ Early CI failure detected!"
        echo "$CHECKS" | grep -E '\bfail\b'
        echo ""
        echo "Exiting watch early to repair immediately."
        exit 1
    fi
    
    if ! echo "$CHECKS" | grep -E '\bpending\b' > /dev/null && [ -n "$CHECKS" ]; then
        echo ""
        echo "✅ All checks passed!"
        exit 0
    fi
    
    PASS_COUNT=$(echo "$CHECKS" | grep -c -E '\bpass\b' || true)
    PENDING_COUNT=$(echo "$CHECKS" | grep -c -E '\bpending\b' || true)
    TIME=$(date +"%H:%M:%S")
    echo "[$TIME] In progress: $PASS_COUNT passed, $PENDING_COUNT pending... checking in ${INTERVAL}s"
    sleep "$INTERVAL"
done
