#!/usr/bin/env bash
# scripts/verify-no-raw-params.sh — ADR #7 Phase 4 enforcement gate.
#
# Ensures no Tauri #[command] function in the desktop client accepts
# store_id or user_id as a direct parameter. Struct fields (pub) and
# tablet-client commands (not yet migrated) are excluded.
#
# After all 28 desktop domain commands were migrated to the session
# token pattern, this lint prevents regressions.
#
# Usage:  bash scripts/verify-no-raw-params.sh
#         (run from the workspace root)

set -euo pipefail

cd "$(dirname "$0")/.."

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

violations=0
target_dir="apps/desktop-client/src/commands"

if [ ! -d "$target_dir" ]; then
    echo "target directory not found: $target_dir" >&2
    exit 1
fi

while IFS= read -r file; do
    # Exclude struct field definitions (pub keyword) and comments.
    # Match only function parameter lines with store_id/user_id.
    while IFS= read -r line; do
        line_num=$(echo "$line" | cut -d: -f1)
        content=$(echo "$line" | cut -d: -f2-)
        
        # Skip struct fields (pub keyword) and comments.
        if echo "$content" | grep -qE '^\s*(pub|//|///|\*)'; then
            continue
        fi
        
        echo -e "${RED}VIOLATION in ${file}:${line_num}: ${content}${NC}" >&2
        violations=$((violations + 1))
    done < <(grep -n 'store_id: String\|user_id: String' "$file" 2>/dev/null || true)
done < <(find "$target_dir" -name '*.rs' -type f)

if [ "$violations" -gt 0 ]; then
    echo -e "${RED}FAIL: ${violations} raw store_id/user_id parameter(s) found in desktop command signatures.${NC}" >&2
    echo "ADR #7 Phase 4: All commands must use session_token + resolve_scope() instead." >&2
    echo "See docs/decisions/2026-07-10-data-scope-guard.md for the migration pattern." >&2
    exit 1
fi

echo -e "${GREEN}PASS: no raw store_id or user_id parameters in desktop command signatures${NC}"
