#!/bin/bash
# Find oldest modified .md files in the codebase - AI-friendly JSON output
# Usage: ./find-oldest-md.sh [N] [PATH]
#
# N (optional): Number of results to show (default: 10)
# PATH (optional): Directory to search (default: current directory)
# Output format: JSON array for easy parsing by AI/LLMs
#
# Non-codebase directories are skipped: generated artifacts (graphify-out,
# target, node_modules, dist, gen), tooling caches (.git, .vite, .cache,
# .idea, .vscode, __pycache__), and test/playwright output.
# NOTE: 'coverage' is intentionally NOT ignored — docs/coverage/ is a real
# tracked doc directory (the generated /coverage/ and /ui/coverage/ report
# dirs contain no .md files anyway).
# NOTE: if SEARCH_PATH is itself an ignored dir (e.g. ./find-oldest-md.sh 5
# target/), every file is filtered out and the script prints empty output.
#
# Examples:
#   ./find-oldest-md.sh                    # Top 10 oldest in current dir
#   ./find-oldest-md.sh 5 docs/            # Top 5 oldest in docs/
#   ./find-oldest-md.sh 20 . --json-lines  # Stream for large repos

set -euo pipefail

NUM_RESULTS=${1:-10}
SEARCH_PATH="${2:-$(pwd)}"

echo "Finding oldest modified .md files in: $SEARCH_PATH"
echo "Showing top $NUM_RESULTS results..."
echo "========================================"

# Use Python for reliable JSON output with proper escaping and metadata
python3 -c "
import os
from pathlib import Path
import json
from datetime import datetime

search_path = '$SEARCH_PATH'
num_results = $NUM_RESULTS

# Directory names that are not part of the codebase (generated output,
# build artifacts, tooling caches, vendor deps). A path is skipped if any
# of its components matches. 'coverage' is intentionally NOT listed:
# docs/coverage/ is a real tracked doc directory (the generated /coverage/
# and /ui/coverage/ report dirs contain no .md files anyway).
IGNORED_DIRS = {
    '.git', '.vite', '.cache', '.idea', '.vscode', '.turbo', '__pycache__',
    'node_modules', 'target', 'dist', 'build', 'graphify-out',
    'gen', 'playwright-report', 'test-results', '.next', '.nuxt', 'out',
}

def is_codebase(path: Path) -> bool:
    # True when no component of the path is a non-codebase directory.
    return not any(part in IGNORED_DIRS for part in path.parts)

files = [f for f in Path(search_path).rglob('*.md') if is_codebase(f)]
oldest_files = sorted(files, key=lambda x: x.stat().st_mtime)

results = []
for f in oldest_files[:num_results]:
    stat = f.stat()
    results.append({
        'mtime': stat.st_mtime,
        'mtime_iso': datetime.fromtimestamp(stat.st_mtime).isoformat(),
        'size_bytes': stat.st_size,
        'relative_path': str(f.relative_to(search_path)),
        'absolute_path': str(f),
        'directory_depth': len(str(f.relative_to(search_path)).split(os.sep))
    })

print(json.dumps(results, indent=2))
" 2>/dev/null || \
python -c "
import os
from pathlib import Path
import json
from datetime import datetime

search_path = '$SEARCH_PATH'
num_results = $NUM_RESULTS

IGNORED_DIRS = {
    '.git', '.vite', '.cache', '.idea', '.vscode', '.turbo', '__pycache__',
    'node_modules', 'target', 'dist', 'build', 'graphify-out',
    'gen', 'playwright-report', 'test-results', '.next', '.nuxt', 'out',
}

def is_codebase(path: Path) -> bool:
    # True when no component of the path is a non-codebase directory.
    return not any(part in IGNORED_DIRS for part in path.parts)

files = [f for f in Path(search_path).rglob('*.md') if is_codebase(f)]
oldest_files = sorted(files, key=lambda x: x.stat().st_mtime)

results = []
for f in oldest_files[:num_results]:
    stat = f.stat()
    results.append({
        'mtime': stat.st_mtime,
        'mtime_iso': datetime.fromtimestamp(stat.st_mtime).isoformat(),
        'size_bytes': stat.st_size,
        'relative_path': str(f.relative_to(search_path)),
        'absolute_path': str(f),
        'directory_depth': len(str(f.relative_to(search_path)).split(os.sep))
    })

print(json.dumps(results, indent=2))
"

echo "========================================"
