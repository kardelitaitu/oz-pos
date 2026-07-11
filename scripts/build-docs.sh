#!/usr/bin/env bash
# build-docs.sh — Build and open the OZ-POS documentation portal
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "\033[36m==========================================\033[0m"
echo -e "\033[36m Building OZ-POS Documentation Portal...\033[0m"
echo -e "\033[36m==========================================\033[0m"

echo -e "\n\033[33m[1/3] Generating Rust Workspace API Docs (cargo doc)...\033[0m"
cd "$WORKSPACE_ROOT"
cargo doc --workspace --no-deps --document-private-items
echo -e "\033[32m✔ Rust documentation generated in target/doc/\033[0m"

echo -e "\n\033[33m[2/3] Generating Frontend TypeScript Docs (TypeDoc)...\033[0m"
cd "$WORKSPACE_ROOT/ui"
if command -v npx &> /dev/null; then
    npx -y typedoc --skipErrorChecking --entryPointStrategy expand ./src/api ./src/types ./src/hooks --out ../docs/html/ui-docs 2>/dev/null || true
    if [ -f "../docs/html/ui-docs/index.html" ]; then
        echo -e "\033[32m✔ Frontend TypeScript documentation generated in docs/html/ui-docs/\033[0m"
    else
        echo -e "\033[33m⚠ TypeDoc generation skipped (install typedoc inside ui/ if needed via 'npm i -D typedoc')\033[0m"
    fi
else
    echo -e "\033[33m⚠ npx not found on PATH, skipping TypeDoc generation.\033[0m"
fi

echo -e "\n\033[33m[3/3] Verifying Documentation Portal Hub...\033[0m"
PORTAL_INDEX="$WORKSPACE_ROOT/docs/html/index.html"
if [ -f "$PORTAL_INDEX" ]; then
    echo -e "\033[32m✔ Master Documentation Hub ready at: $PORTAL_INDEX\033[0m"
    if [ "$1" == "--open" ]; then
        if command -v xdg-open &> /dev/null; then
            xdg-open "$PORTAL_INDEX"
        elif command -v open &> /dev/null; then
            open "$PORTAL_INDEX"
        fi
    fi
else
    echo -e "\033[31mError: Documentation portal index.html not found at $PORTAL_INDEX\033[0m" >&2
    exit 1
fi

echo -e "\n\033[36m==========================================\033[0m"
echo -e "\033[36m Documentation Build Complete!\033[0m"
echo -e "\033[36m==========================================\033[0m"
