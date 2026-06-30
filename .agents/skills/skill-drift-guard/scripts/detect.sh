#!/usr/bin/env bash
# Skill drift detection — runs the 7 mechanical checks described in
# .agents/skills/skill-drift-guard/SKILL.md and emits a markdown report.
#
# Usage:
#   ./detect.sh                          # all checks, no patches
#   ./detect.sh --check=paths            # one check only
#   ./detect.sh --auto-patch             # auto-patch safe categories
#   ./detect.sh --report                 # write skill-drift-report.md
#   SKIP=api,golden ./detect.sh          # skip the named checks
#
# Exit code is the number of manual-review findings (0 = clean).

set -u

cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

REPORT="skill-drift-report.md"
ONLY_CHECK=""
AUTO_PATCH=false
WRITE_REPORT=false
today="$(date +%d-%m-%y)"

for arg in "$@"; do
  case "$arg" in
    --check=*) ONLY_CHECK="${arg#--check=}" ;;
    --auto-patch) AUTO_PATCH=true ;;
    --report) WRITE_REPORT=true ;;
    --help|-h)
      sed -n '2,12p' "$0"
      exit 0
      ;;
  esac
done

should_run() {
  local name="$1"
  [ -z "$ONLY_CHECK" ] && return 0
  [ "$ONLY_CHECK" = "$name" ] && return 0
  return 1
}

# Categories that can be auto-patched safely
AUTO_PATCH_CATS=("versions" "audit-date" "cross-refs" "missing-crates")

# Findings: associative array of category -> lines
declare -A FINDINGS
for cat in paths crates api versions golden refs fluent audit-date; do
  FINDINGS[$cat]=""
done

# ---------------------------------------------------------------------------
# Check 1 — File path inventory
# ---------------------------------------------------------------------------
if should_run paths; then
  while read -r skill; do
    [ -z "$skill" ] && continue
    grep -oE '[a-zA-Z_.-]+(/[a-zA-Z0-9_.-]+){1,}' "$skill" 2>/dev/null | sort -u | \
      while read -r path; do
        case "$path" in
          http*|https*|file://*|node_modules*|target/*|dist/*) continue ;;
        esac
        if [ ! -e "$path" ]; then
          # only flag if the path looks like a project path
          case "$path" in
            src*|ui/*|crates/*|migrations/*|hal/*|docs/*|src-tauri/*|.github/*|scripts/*)
              FINDINGS[paths]+="${skill}: ${path}"$'\n'
              ;;
          esac
        fi
      done
  done < <(find .agents/skills -name SKILL.md 2>/dev/null)
fi

# ---------------------------------------------------------------------------
# Check 2 — Crate inventory
# ---------------------------------------------------------------------------
if should_run crates; then
  : "${Cargo_FILE:=Cargo.toml}"
  if [ -f "$Cargo_FILE" ]; then
    : "$(grep -oE '"crates/oz-[a-z-]+"' "$Cargo_FILE" 2>/dev/null | sort -u | sed 's|"crates/||;s|"||')"
    workspace_crates="$(grep -oE '"crates/oz-[a-z-]+"' "$Cargo_FILE" 2>/dev/null | sort -u | sed 's|"crates/||;s|"||')"
    skill_crates="$(cat .agents/skills/*/SKILL.md | grep -oE 'oz-[a-z-]+' | sort -u)"

    while read -r c; do
      [ -z "$c" ] || [ "$c" = "oz-pos" ] && continue
      if ! echo "$workspace_crates" | grep -qx "$c"; then
        FINDINGS[crates]+="missing in workspace: ${c}"$'\n'
      fi
    done <<< "$skill_crates"
  fi
fi

# ---------------------------------------------------------------------------
# Check 3 — API signature (lightweight; deep check needs cargo doc)
# ---------------------------------------------------------------------------
if should_run api; then
  # Look for code blocks that call known public functions
  while read -r skill; do
    [ -z "$skill" ] && continue
    # Catch calls to Money::from_major / checked_add / zero so we can flag if any change
    grep -nE 'Money::(from_major|checked_add|zero|new)' "$skill" 2>/dev/null | \
      while read -r line; do
        FINDINGS[api]+="${skill}: ${line} (verify signature in oz-core/src/money.rs)"$'\n'
      done
  done < <(find .agents/skills -name SKILL.md 2>/dev/null)
fi

# ---------------------------------------------------------------------------
# Check 4 — Dependency version drift
# ---------------------------------------------------------------------------
if should_run versions; then
  if [ -f "Cargo.toml" ]; then
    while read -r skill; do
      [ -z "$skill" ] && continue
      grep -hoE '"[0-9]+\.[0-9]+(\.[0-9]+)?"' "$skill" 2>/dev/null | sort -u | \
        while read -r ver; do
          # strip quotes
          v="${ver//\"/}"
          # crude: see if this exact version is still in Cargo.toml
          if ! grep -q "$v" Cargo.toml; then
            FINDINGS[versions]+="${skill}: quoted version ${ver} not in Cargo.toml"$'\n'
          fi
        done
    done < <(find .agents/skills -name SKILL.md 2>/dev/null)
  fi
fi

# ---------------------------------------------------------------------------
# Check 5 — Golden rule alignment (mechanical: look for the major ones)
# ---------------------------------------------------------------------------
if should_run golden; then
  for phrase in "i64 minor units" "thiserror" "anyhow" "rusqlite" "Tauri v2"; do
    in_agents="$(grep -c "$phrase" AGENTS.md 2>/dev/null || echo 0)"
    in_skills="$(cat .agents/skills/*/SKILL.md | grep -c "$phrase" 2>/dev/null || echo 0)"
    in_agents="$(echo "$in_agents" | tr -d '\r' | tr -cd '0-9')"
    in_skills="$(echo "$in_skills" | tr -d '\r' | tr -cd '0-9')"
    if [ -z "$in_agents" ]; then in_agents=0; fi
    if [ -z "$in_skills" ]; then in_skills=0; fi

    # heuristic: if AGENTS.md says it but no skill mentions it, flag
    if [ "$in_agents" -gt 0 ] && [ "$in_skills" -eq 0 ]; then
      FINDINGS[golden]+="phrase '${phrase}' in AGENTS.md but not in any skill"$'\n'
    fi
  done
fi

# ---------------------------------------------------------------------------
# Check 6 — Cross-reference integrity
# ---------------------------------------------------------------------------
if should_run refs; then
  # onboarding-guide should reference only existing skills
  og=".agents/skills/onboarding-guide/SKILL.md"
  if [ -f "$og" ]; then
    grep -oE '`[a-z][a-z-]+`' "$og" 2>/dev/null | sort -u | \
      grep -vE '^\`(feat|fix|docs|chore|test|refactor|perf|style|ci|revert|build)\`$' | \
      tr -d '`' | \
      while read -r ref; do
        case "$ref" in
          rust-backend|tauri-ipc|ui-components|hal-drivers|project-scaffold|onboarding-guide|skill-drift-guard) continue ;;
        esac
        # Heuristic: anything else in backticks inside the router table is a skill reference
        if [ -d ".agents/skills/$ref" ]; then continue; fi
        # skip common non-skill backticks
        case "$ref" in
          src|ui|crates|hal|src-tauri|AGENTS.md|README.md|WHITEPAPER.md|ARCHITECTURE.md|ROADMAP.md) continue ;;
        esac
        FINDINGS[refs]+="onboarding-guide: possible missing skill ref \`${ref}\`"$'\n'
      done
  fi
fi

# ---------------------------------------------------------------------------
# Check 7 — Front-end Fluent ID alignment (no-op if no FTL files exist)
#
# Direction is intentionally one-way: every Fluent id referenced from a skill
# must exist in the active FTL files. The reverse (every FTL id is mentioned
# in a skill) is NOT checked, because FTL files legitimately contain ids that
# no skill has documented yet.
# ---------------------------------------------------------------------------
if should_run fluent; then
  if [ -d "ui/src/locales" ]; then
    while read -r skill; do
      [ -z "$skill" ] && continue
      # Permissive pattern: any non-empty id. Trust the FTL to define the format.
      grep -hoE 'id="[^"]+"' "$skill" 2>/dev/null | sort -u | \
        sed 's/id="//;s/"$//' | \
        while read -r ftl_id; do
          if ! grep -rqE "^${ftl_id}\s*=" ui/src/locales/ 2>/dev/null; then
            FINDINGS[fluent]+="${skill}: Fluent id '${ftl_id}' not found in ui/src/locales/"$'\n'
          fi
        done
    done < <(find .agents/skills -name SKILL.md 2>/dev/null)
  fi
  # else: no front-end yet, silently skip
fi

# ---------------------------------------------------------------------------
# Check 8 — Audit-date freshness
# ---------------------------------------------------------------------------
if should_run audit-date; then
  today="$(date +%d-%m-%y)"
  while read -r skill; do
    [ -z "$skill" ] && continue
    last="$(grep -oE 'last audited [0-9]{2}-[0-9]{2}-[0-9]{2}' "$skill" 2>/dev/null | tail -1 | awk '{print $3}')"
    if [ -z "$last" ]; then
      FINDINGS[audit-date]+="${skill}: missing audit date"$'\n'
      continue
    fi
    py_cmd="python3"
    if ! command -v python3 2>/dev/null; then
      py_cmd="python"
    fi
    age="$($py_cmd -c "
from datetime import datetime
try:
    d = datetime.strptime('$last', '%d-%m-%y')
    print((datetime.now() - d).days)
except Exception:
    print(9999)
" 2>/dev/null || echo 9999)"
    if [ "${age:-9999}" -gt 30 ]; then
      FINDINGS[audit-date]+="${skill}: last audited ${last} (${age} days ago)"$'\n'
    fi
  done < <(find .agents/skills -name SKILL.md 2>/dev/null)
fi

# ---------------------------------------------------------------------------
# Auto-patch (safe categories only)
# ---------------------------------------------------------------------------
if $AUTO_PATCH; then
  if [ -n "${FINDINGS[audit-date]}" ]; then
    for skill in .agents/skills/*/SKILL.md; do
      sed -i "s/^> last audited .* by .*/> last audited $today by skill-drift-guard/" "$skill"
    done
    echo "auto-patched: audit dates bumped to $today"
  fi
  if [ -n "${FINDINGS[versions]}" ]; then
    echo "auto-patch: versions — manual review needed, listing:"
    echo "${FINDINGS[versions]}"
  fi
  if [ -n "${FINDINGS[refs]}" ]; then
    echo "auto-patch: refs — manual review needed, listing:"
    echo "${FINDINGS[refs]}"
  fi
fi

# ---------------------------------------------------------------------------
# Emit report
# ---------------------------------------------------------------------------
manual_count=0
report=""
report+="# Skill drift report — $today"$'\n\n'

for cat in paths crates api versions golden refs fluent audit-date; do
  body="${FINDINGS[$cat]}"
  if [ -z "$body" ]; then continue; fi
  manual_count=$((manual_count + $(echo "$body" | grep -c . || true)))
  report+="## $cat"$'\n\n'
  report+="\`\`\`"$'\n'
  report+="$body"
  report+="\`\`\`"$'\n\n'
done

if [ "$manual_count" -eq 0 ]; then
  report+="No drift detected. All skills are in sync with the code."$'\n'
fi

if $WRITE_REPORT || [ "$manual_count" -gt 0 ]; then
  echo "$report" > "$REPORT"
  echo "wrote $REPORT ($manual_count findings)"
fi

echo "$report"
exit "$manual_count"
