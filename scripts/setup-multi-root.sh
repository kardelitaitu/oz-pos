#!/usr/bin/env bash
# scripts/setup-multi-root.sh — bootstrap the multi-root OZ-POS layout
# (bash twin of scripts/setup-multi-root.ps1 — keep both in step).
#
#   <Base>/main/            bare repository — the ONLY git database
#   <Base>/<release>/       stable integration worktree, locked on the
#                           release branch
#   <Base>/worktrees/<name> transient agent worktrees on agent/<name>
#                           branches (ux, cargo, openapi, ...)
#
# Every path is derived from arguments or from git itself; nothing is
# anchored to the checkout this runs from. Never pushes.
#
# Usage:
#   bash scripts/setup-multi-root.sh
#   bash scripts/setup-multi-root.sh C:/dev/ozpos 0.0.33 "ux cargo openapi"
set -euo pipefail

BASE_DIR="${1:-C:/dev/ozpos}"
RELEASE_BRANCH="${2:-0.0.33}"
AGENTS="${3:-ux cargo openapi}"

ORIGIN="$(git remote get-url origin 2>/dev/null || true)"
if [ -z "$ORIGIN" ]; then
    ORIGIN="$(git rev-parse --show-toplevel)"
    echo "No origin remote — cloning from the local checkout: $ORIGIN"
else
    echo "Cloning from origin: $ORIGIN"
fi

if [ -e "$BASE_DIR" ]; then
    echo "error: refusing to continue: $BASE_DIR already exists." >&2
    echo "       Remove it or pass a different base directory." >&2
    exit 1
fi

# 1. Bare repository (the git database).
MAIN_DIR="$BASE_DIR/main"
git clone --bare "$ORIGIN" "$MAIN_DIR"

# 2. Stable worktree locked on the release branch.
RELEASE_DIR="$BASE_DIR/$RELEASE_BRANCH"
if ! git -C "$MAIN_DIR" worktree add "$RELEASE_DIR" "$RELEASE_BRANCH"; then
    git -C "$MAIN_DIR" worktree add "$RELEASE_DIR" -b "$RELEASE_BRANCH" "origin/$RELEASE_BRANCH"
fi

# 3. Agent worktrees, each on its own branch forked from the release
#    branch so worktrees never fight over a checked-out branch.
WORKTREE_ROOT="$BASE_DIR/worktrees"
mkdir -p "$WORKTREE_ROOT"
for name in $AGENTS; do
    [ -n "$name" ] || continue
    git -C "$MAIN_DIR" worktree add "$WORKTREE_ROOT/$name" -b "agent/$name" "$RELEASE_BRANCH"
done

# 4. One-time repo config (shared by every worktree).
git -C "$MAIN_DIR" config core.hooksPath .githooks

# 5. Report + next steps.
echo
echo "Layout created under $BASE_DIR :"
git -C "$MAIN_DIR" worktree list | sed 's/^/  /'
echo
echo "Next steps (per worktree, forward slashes per AGENTS.md):"
echo "  1. cd $BASE_DIR/$RELEASE_BRANCH/ui && npm ci --no-audit --no-fund"
echo "  2. repeat npm ci in any agent worktree you actively edit"
echo "  3. sccache (.cargo/config.toml) is shared automatically;"
echo "     each worktree still owns its own target/ dir"
echo "  4. dev servers: run ONE vite/tauri dev server at a time, or"
echo "     pass distinct --port values per worktree"
echo "  5. codebase-memory-mcp: index each worktree you work in"
