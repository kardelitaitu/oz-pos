# scripts/setup-multi-root.ps1 — bootstrap the multi-root OZ-POS layout.
#
# Creates the topology the team works against once the repo outgrows a
# single checkout:
#
#   <Base>/main/            bare repository — the ONLY git database
#   <Base>/<release>/       stable integration worktree, locked on the
#                           release branch (no feature work here)
#   <Base>/worktrees/<name> transient agent worktrees (ux, cargo, openapi,
#                           ...), each on its own agent/<name> branch
#
# WHY A BARE REPO: `git worktree` needs one shared object database. Making
# the database itself a bare clone keeps every checkout a throwaway
# worktree — an agent can delete worktrees/ux entirely without touching
# history.
#
# Every path below is derived from parameters or from git itself; nothing
# is anchored to the directory this repo happens to be checked out in.
# Run it from ANY clone of the repo — it only reads the source URL.
#
# Usage:
#   powershell -File scripts/setup-multi-root.ps1
#   powershell -File scripts/setup-multi-root.ps1 -BaseDir C:/dev/ozpos `
#       -ReleaseBranch 0.0.33 -Agents ux,cargo,openapi
#
# This script never pushes and never touches the checkout it runs from.

[CmdletBinding()]
param(
    # Where the topology is created. This is the one deliberate default:
    # it is the DESTINATION the team agreed on, not a source anchor.
    [string]$BaseDir = "C:/dev/ozpos",

    # Release branch the stable worktree locks onto.
    [string]$ReleaseBranch = "0.0.33",

    # Comma-separated agent worktree names to scaffold.
    [string]$Agents = "ux,cargo,openapi"
)

$ErrorActionPreference = "Stop"

# ── Resolve the source repository ────────────────────────────────────────
# Prefer the configured origin; fall back to THIS checkout's path so the
# bootstrap also works before the repo has any remote.
$origin = git remote get-url origin 2>$null
if ($LASTEXITCODE -ne 0 -or -not $origin) {
    $origin = (git rev-parse --show-toplevel).Replace('\', '/')
    Write-Host "No origin remote — cloning from the local checkout: $origin"
} else {
    Write-Host "Cloning from origin: $origin"
}

if (Test-Path $BaseDir) {
    Write-Error "Refusing to continue: $BaseDir already exists. Remove it or pass a different -BaseDir."
}

# ── 1. Bare repository (the git database) ────────────────────────────────
$mainDir = Join-Path $BaseDir "main"
git clone --bare $origin $mainDir
if ($LASTEXITCODE -ne 0) { Write-Error "git clone --bare failed" }

# ── 2. Stable worktree locked on the release branch ──────────────────────
# `git worktree add` checks the branch out WITHOUT switching anything in
# the source checkout; the bare clone owns the refs from here on.
$releaseDir = Join-Path $BaseDir $ReleaseBranch
git -C $mainDir worktree add $releaseDir $ReleaseBranch
if ($LASTEXITCODE -ne 0) {
    # Branch not present locally in the bare clone — create it from origin.
    git -C $mainDir worktree add $releaseDir -b $ReleaseBranch "origin/$ReleaseBranch"
    if ($LASTEXITCODE -ne 0) { Write-Error "could not create the $ReleaseBranch worktree" }
}

# ── 3. Agent worktrees ───────────────────────────────────────────────────
# Each gets its own branch (agent/<name>) forked from the release branch,
# so worktrees never fight over a checked-out branch.
$worktreeRoot = Join-Path $BaseDir "worktrees"
New-Item -ItemType Directory -Force -Path $worktreeRoot | Out-Null

foreach ($name in $Agents.Split(",") | ForEach-Object { $_.Trim() } | Where-Object { $_ }) {
    $branch = "agent/$name"
    $dir = Join-Path $worktreeRoot $name
    git -C $mainDir worktree add $dir -b $branch $ReleaseBranch
    if ($LASTEXITCODE -ne 0) { Write-Error "worktree add failed for $name" }
}

# ── 4. One-time repo config (shared by every worktree) ───────────────────
# The bare repo's config is common to all linked worktrees, so hooks set
# here run in each of them; the relative path resolves per-worktree.
git -C $mainDir config core.hooksPath .githooks

# ── 5. Report + next steps ───────────────────────────────────────────────
Write-Host ""
Write-Host "Layout created under $BaseDir :" -ForegroundColor Green
git -C $mainDir worktree list | ForEach-Object { Write-Host "  $_" }
Write-Host ""
Write-Host "Next steps (per worktree, forward slashes per AGENTS.md):"
Write-Host "  1. cd $BaseDir/$ReleaseBranch/ui ; npm ci --no-audit --no-fund"
Write-Host "  2. repeat npm ci in any agent worktree you actively edit"
Write-Host "  3. sccache (.cargo/config.toml) is shared automatically;"
Write-Host "     each worktree still owns its own target/ dir"
Write-Host "  4. dev servers: run ONE vite/tauri dev server at a time, or"
Write-Host "     pass distinct --port values per worktree"
Write-Host "  5. codebase-memory-mcp: index each worktree you work in"
