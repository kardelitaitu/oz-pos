#!/bin/bash
# Find oldest modified .md files in the project - AI-friendly JSON output
# Usage: ./find-oldest-md.sh [N] [PATH]
#
# N (optional): Number of results to show (default: 10)
# PATH (optional): Directory to search (default: current directory)
# Output format: JSON array for easy parsing by AI/LLMs
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

files = list(Path(search_path).rglob('*.md'))
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

files = list(Path(search_path).rglob('*.md'))
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
