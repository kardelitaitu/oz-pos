#!/usr/bin/env bash
# import-portal.sh — stage the built docs portal into the website's public/ tree.
#
# The portal (mdBook, built by scripts/build-docs.sh at the repo root) contains
# the three doc surfaces we want to serve deep inside the website:
#
#   1. Docs     — hand-written guides + ADRs          (docs/book/guides, decisions)
#   2. Rust     — cargo doc API reference             (docs/book/api/rust)
#   3. TypeScript — TypeDoc API reference             (docs/book/api/ts)
#
# Each of those HTML trees carries its own components (css/js/fonts/search
# index), so we copy the whole built `docs/book/` tree — not just the three
# index.html files — into `website/public/docs-portal/`. Astro copies
# `public/` verbatim to the build output, so the portal becomes reachable at
# `/docs-portal/…` on the deployed site and during `astro dev`.
#
# Usage:
#   bash website/scripts/import-portal.sh        # copy (fails if portal not built)
#   bash website/scripts/import-portal.sh --if-exists   # skip silently if absent
#
# Run build-docs.sh first:  bash scripts/build-docs.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEBSITE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$WEBSITE_ROOT/.." && pwd)"

PORTAL_SRC="$REPO_ROOT/docs/book"
PORTAL_DEST="$WEBSITE_ROOT/public/docs-portal"

if [ ! -f "$PORTAL_SRC/index.html" ]; then
    if [ "${1:-}" = "--if-exists" ]; then
        echo "import-portal: portal not built ($PORTAL_SRC missing) — skipping (--if-exists)"
        exit 0
    fi
    echo "error: portal not built — run 'bash scripts/build-docs.sh' first (expected $PORTAL_SRC)" >&2
    exit 1
fi

echo "import-portal: staging portal into website..."
rm -rf "$PORTAL_DEST"
mkdir -p "$(dirname "$PORTAL_DEST")"
cp -r "$PORTAL_SRC" "$PORTAL_DEST"

echo "✔ portal staged: $PORTAL_DEST"
echo "  served at:     /docs-portal/  (astro dev + astro build)"
echo "  size:          $(du -sh "$PORTAL_DEST" | cut -f1)"
