#!/usr/bin/env bash
# Regression test for dev-ci.yml's path router.
#
# Why this exists: commit c5ec6381 ("ci: route dev-ci jobs by changed paths")
# shipped ONLY the workflow YAML. The 0.0.36 release notes claim the routing
# logic "is tested by extracting the real shell body from the YAML and running it
# against 11 synthetic diffs" -- but no such test was committed, so the claim
# described something that existed nowhere reproducible. This file makes it true.
#
# Design: the shell body is EXTRACTED from the workflow rather than copied, so
# the test cannot drift from what CI actually runs. A copied fixture would keep
# passing after someone edited the YAML -- the exact failure mode this prevents.
#
# Pure bash, deliberately. A Python driver shelling out with piped stdio hangs in
# this repo's Windows sandbox (see AGENTS.md notes on named pipes).
#
# Usage: bash scripts/test-ci-routing.sh

set -uo pipefail

WF=".github/workflows/dev-ci.yml"
KEYS="rust ui i18n website docs"

[ -f "$WF" ] || { echo "FATAL: $WF not found (run from repo root)"; exit 1; }

# ── Extract the Route step's shell body ─────────────────────────────
# From the line after `run: |` under `id: route`, take the indented block,
# stopping at the first line that is not blank and not indented at least 10
# spaces (i.e. the next YAML key).
BODY="$(awk '
  /^        id: route[[:space:]]*$/ { inroute=1; next }
  inroute && /^        run: \|[[:space:]]*$/ { capture=1; next }
  capture {
    if ($0 ~ /^          /) { sub(/^          /, ""); print; next }
    if ($0 ~ /^[[:space:]]*$/) { print; next }
    exit
  }
' "$WF")"

if [ -z "${BODY//[[:space:]]/}" ]; then
  echo "FATAL: could not extract the Route step body from $WF"
  echo "       (the test breaks loudly rather than silently testing nothing)"
  exit 1
fi

# ── Run one synthetic diff through the real body ────────────────────
# The body calls `git diff --name-only` and appends key=value to $GITHUB_OUTPUT.
# A shell function named `git` shadows the binary, so no repository is needed.
# The body's all() helper calls `exit 0`, hence a subshell per case.
run_case() {
  listing="$1"
  event="${2:-pull_request}"
  out="$(mktemp)"
  : > "$out"

  (
    GITHUB_OUTPUT="$out"
    BASE=base-sha
    HEAD_SHA=head-sha
    EVENT="$event"
    export GITHUB_OUTPUT BASE HEAD_SHA EVENT

    git() { printf '%s\n' "$listing"; }

    # shellcheck disable=SC2154  # BODY is expanded intentionally
    eval "$BODY"
  ) >/dev/null 2>&1

  cat "$out"
  rm -f "$out"
}

get_val() { # get_val <outputs> <key>
  printf '%s\n' "$1" | grep "^$2=" | head -1 | cut -d= -f2-
}

pass=0; fail=0

# check <description> <expected-string "k=v ..."> <files> [event]
check() {
  desc="$1"; want="$2"; files="$3"; event="${4:-pull_request}"
  got="$(run_case "$files" "$event")"
  bad=""
  for k in $KEYS; do
    w="$(printf '%s' "$want" | tr ' ' '\n' | grep "^$k=" | cut -d= -f2-)"
    g="$(get_val "$got" "$k")"
    [ -z "$w" ] && continue
    if [ "$g" != "$w" ]; then bad="$bad $k:want=$w,got=${g:-<unset>}"; fi
  done
  if [ -n "$bad" ]; then
    fail=$((fail+1))
    printf '  FAIL  %-26s %s\n' "$desc" "$bad"
  else
    pass=$((pass+1))
    line=""
    for k in $KEYS; do line="$line $k=$(get_val "$got" "$k")"; done
    printf '  ok    %-26s %s\n' "$desc" "${line# }"
  fi
}

echo "dev-ci.yml path router: $(printf '%s' "$BODY" | wc -l | tr -d ' ') body lines extracted"
echo

# Each bucket must fire ALONE -- a router that over-triggers wastes exactly the
# runner minutes this mechanism exists to save.
check "rust crate"          "rust=true ui=false i18n=false website=false docs=false" "crates/oz-core/src/db.rs"
check "rust lockfile"       "rust=true ui=false i18n=false website=false docs=false" "Cargo.lock"
check "ui tsx"              "rust=false ui=true i18n=true website=false docs=false"  "ui/src/features/reports/DashboardScreen.tsx"
check "ui lockfile"         "rust=false ui=true i18n=false website=false docs=false" "ui/package-lock.json"
check "ftl bundle"          "rust=false ui=true i18n=true website=false docs=false"  "ui/src/locales/en/reports.ftl"
check "website only"        "rust=false ui=false i18n=false website=true docs=false" "website/src/pages/index.astro"
check "i18n script"         "rust=false ui=false i18n=true website=false docs=false" "scripts/verify-bundle-parity.py"
# Docs must route to the drift checker: a docs-only PR is precisely the change
# that can make CI docs lie, and before this output existed it ran nothing.
check "docs only"           "rust=false ui=false i18n=false website=false docs=true" "docs/operations/ci-pipeline.md"
check "release doc"         "rust=false ui=false i18n=false website=false docs=true" "docs/releases/checklist.md"
check "gate manifest"       "rust=false ui=false i18n=false website=false docs=true" "scripts/gates.json"
check "drift checker"       "rust=false ui=false i18n=false website=false docs=true" "scripts/verify-ci-docs-drift.py"
# The workflow gating everything must never be able to route itself away.
check "this workflow"       "rust=true ui=true i18n=true website=true docs=true"     ".github/workflows/dev-ci.yml"
check "mixed rust+website"  "rust=true ui=false i18n=false website=true docs=false"  "$(printf 'crates/oz-api/src/lib.rs\nwebsite/src/site.css')"
check "unrelated file"      "rust=false ui=false i18n=false website=false docs=false" "README.md"
# Non-PR events must always run the full matrix.
check "dispatch event"      "rust=true ui=true i18n=true website=true docs=true"     "README.md" "workflow_dispatch"

echo
echo "$pass/$((pass+fail)) routing cases correct"
if [ "$fail" -ne 0 ]; then
  echo "FAIL: $fail routing case(s) wrong"
  exit 1
fi
echo "PASS: dev-ci.yml path router behaves as specified"
