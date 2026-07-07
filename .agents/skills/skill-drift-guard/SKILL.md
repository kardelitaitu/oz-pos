---
name: skill-drift-guard
description: Meta-skill that detects and patches drift in the other OZ-POS skills. Use when a code change is made that touches a path, type, trait, or convention referenced in a skill; when onboarding a new contributor who might have added a crate or module; or as a periodic CI check. Always run before merging a change that touches `oz-*` crates, `apps/desktop-client/`, or `ui/`.
---

# Skill Drift Guard

A skill is a **claim about the code**. When the code changes and the skill doesn't, the skill becomes a lie. Future agents read the lie, write code that matches the lie, and the lie propagates.

The drift guard audits each skill against the code it describes, classifies the drift, and either auto-patches it (mechanical changes) or files a `fix(docs):` PR for the rest.

---

## When to run

- After any PR that changes a public API in an `oz-*` crate.
- After any rename, move, or delete in `apps/desktop-client/`, `ui/`, `hal/`, or `crates/`.
- After a dependency bump (Tauri, React, `rusqlite`, etc.).
- After a change to `AGENTS.md` (golden rules).
- **As a CI job** that runs nightly or on changes to `.agents/skills/**`.

---

## Taxonomy of drift

Nine concrete kinds. Each has a detection strategy and a patch strategy.

| # | Drift | Detection | Patch |
|---|-------|-----------|-------|
| 1 | **File path no longer exists** | Glob each path the skill mentions | Manual (renames are usually intentional) |
| 2 | **Crate removed from workspace** | Parse `Cargo.toml` `members` and cross-check the crate list in each skill | Auto: remove the crate from the skill's text |
| 3 | **Crate added to workspace** | Diff `members` against the skill's crate list | Manual (needs new content) |
| 4 | **Public API signature changed** | `cargo doc` + AST diff vs the skill's code example | Manual (need to rewrite the example) |
| 5 | **Dependency version outdated** | Parse `Cargo.toml` for actual versions; grep skill for quoted versions | Auto: replace the version string |
| 6 | **Golden rule changed in `AGENTS.md`** | Diff key phrases (`Money is always i64`, `use thiserror`, …) | Manual (judgment call on impact) |
| 7 | **Fluent ID drift** | Every `<Localized id="...">` in a skill must exist in `ui/src/locales/*.ftl` (one-way) | Manual (decide whether to add the id or remove the reference) |
| 8 | **Cross-reference broken** | For every `\`<skill-name>\`` mention, verify the skill directory exists | Auto: remove the reference or rename |
| 9 | **`last audited` date stale (>30 days)** | Grep the footer line | Auto: bump the date and the auditor name |

If a change is **not** in this list, the drift guard does not auto-patch it. File an issue instead.

---

## Detection workflow

Run these checks in order. Each is a fast, mechanical pass. Stop after each pass to triage the output before running the next. (Checks 1–8 are implemented in `scripts/detect.sh`. Inline Check 2 covers taxonomy kinds 2 and 3 — the "removed" and "added" cases are both detected from the same `members` diff.)

**Pre-code state:** when the corresponding code does not yet exist, each check silently no-ops:
- Checks 2–4 (crates, API, dep versions) skip if `Cargo.toml` is missing.
- Check 7 (Fluent) skips if `ui/src/locales/` is missing.
- Checks 1, 5, 6, 8 (paths, golden rules, refs, audit date) always run.

Once the Rust workspace and UI scaffold land, all checks become active without any change to the script.

### Check 1 — File path inventory

```bash
# For each skill, extract every path-looking token and verify it exists
for skill in .agents/skills/*/SKILL.md; do
  grep -oE '[a-zA-Z_-]+(/[a-zA-Z0-9_.-]+)+' "$skill" \
    | sort -u \
    | while read -r path; do
        # skip web URLs and obvious non-paths
        case "$path" in
          http*|https*|file://*) continue ;;
        esac
        # check the repo
        if [ ! -e "$path" ] && [ ! -d "$path" ]; then
          echo "MISSING: $skill references $path (no such file or dir)"
        fi
      done
done
```

**Output:** a list of `MISSING:` lines, one per broken reference. Each is a candidate `DOC DRIFT` finding.

### Check 2 — Crate inventory

```bash
# List all crates the skills claim exist
for skill in .agents/skills/*/SKILL.md; do
  grep -oE 'oz-[a-z-]+' "$skill" | sort -u
done | sort -u > /tmp/skills-claim.txt

# List all crates actually in the workspace
grep -oE '"crates/oz-[a-z-]+"' Cargo.toml | sort -u \
  | sed 's|"crates/||;s|"||' > /tmp/workspace-has.txt

diff /tmp/skills-claim.txt /tmp/workspace-has.txt
```

**Output:** lines starting with `<` are claimed by a skill but missing from the workspace; lines starting with `>` are in the workspace but not mentioned in any skill (also drift — onboarding-guide should know about them).

### Check 3 — API signature diff

For each public type that a skill's code example uses, confirm the type still has the same shape.

```bash
# Extract the public items from oz-core
cargo doc --no-deps --document-private-items 2>/dev/null
grep -E '^pub (struct|enum|fn|trait) ' crates/oz-core/src/lib.rs \
  | sed 's|{.*||;s|;.*||' > /tmp/core-public.txt

# Compare to what rust-backend/SKILL.md implies
# (manual: the skill should mention each public type it uses)
```

**Output:** a list of types the skill references that are not in the public API (renamed, removed, or made private). Each is `CODE DRIFT`.

### Check 4 — Dependency version drift

```bash
# Versions declared in workspace
grep -E '^[a-z_-]+ = ' Cargo.toml | sort -u > /tmp/workspace-deps.txt

# Versions mentioned in skills
grep -hoE '"[0-9]+\.[0-9]+(\.[0-9]+)?"' .agents/skills/*/SKILL.md \
  | sort -u > /tmp/skills-versions.txt

diff /tmp/workspace-deps.txt /tmp/skills-versions.txt
```

**Output:** versions in skills that no longer match the workspace. Auto-patchable.

### Check 5 — Golden rule alignment

```bash
# Extract the golden-rule sentences from AGENTS.md
sed -n '/^## /,/^## /p' AGENTS.md \
  | grep -E '^- ' > /tmp/agents-rules.txt

# Extract the rule sentences from each skill
for skill in .agents/skills/*/SKILL.md; do
  echo "=== $skill ==="
  sed -n '/Golden rules/,/^## /p' "$skill" | grep -E '^\| [0-9]+ \|'
done > /tmp/skills-rules.txt

# Manual diff: read both, look for contradictions
```

**Output:** a manual review file. The guard does not auto-merge contradictory rules — a human must decide.

### Check 6 — Cross-reference integrity

```bash
# Every <skill-name> reference in onboarding-guide must point to an existing skill
for ref in $(grep -oE 'rust-backend\|tauri-ipc\|ui-components\|hal-drivers\|project-scaffold\|onboarding-guide\|skill-drift-guard' .agents/skills/onboarding-guide/SKILL.md | sort -u); do
  if [ ! -d ".agents/skills/$ref" ]; then
    echo "BROKEN REF: onboarding-guide mentions $ref but no such skill exists"
  fi
done
```

**Output:** a list of broken skill-to-skill references.

### Check 7 — Fluent ID alignment

```bash
# Every <Localized id="..."> in a skill must exist in the active FTL files.
# One-way check — FTL files can have undocumented ids.
for skill in .agents/skills/*/SKILL.md; do
  grep -hoE 'id="[^"]+"' "$skill" | sort -u | \
    sed 's/id="//;s/"$//' | \
    while read -r ftl_id; do
      if ! grep -rqE "^${ftl_id}\s*=" ui/src/locales/ 2>/dev/null; then
        echo "MISSING: $skill references Fluent id '$ftl_id' (not in ui/src/locales/)"
      fi
    done
done
```

**Output:** a list of `Localized id` references in skills that have no matching entry in any `.ftl` file. *One-way check (skill → FTL): the reverse is not checked so FTL files can legitimately contain ids that no skill has documented yet.* Skip silently if `ui/src/locales/` does not exist (pre-UI state).

### Check 8 — Audit-date freshness

```bash
# Find the last-audited line in each skill
for skill in .agents/skills/*/SKILL.md; do
  last=$(grep -oE 'last audited [0-9-]+' "$skill" | tail -1 | awk '{print $3}')
  today=$(date +%d-%m-%y)
  # bash arithmetic: parse the date
  echo "$skill: last audited $last"
done
```

**Output:** skills older than 30 days. Bump them with the patch step.

---

## Patch workflow

For each finding, classify and act:

| Finding type | Action |
|--------------|--------|
| Crate removed from workspace | **Auto-patch:** remove the crate from the skill's crate list. |
| Dependency version outdated | **Auto-patch:** replace the old version with the new one. |
| Audit date stale | **Auto-patch:** replace the date and append `by skill-drift-guard`. |
| Cross-reference broken | **Auto-patch:** remove the broken reference. |
| File path renamed | **Manual:** open an issue; the rename is usually intentional. |
| Public API changed | **Manual:** rewrite the example to match the new API. |
| Golden rule changed | **Manual:** update the skill's rules to match. |
| New crate added | **Manual:** add the crate to the relevant skills and `onboarding-guide`. |

**Rule:** never auto-patch something that would change the meaning of the skill. Version numbers, dates, and explicit cross-references are safe. Prose and code examples are not.

### Auto-patch implementation

```bash
# Example: bump the audit date on every skill
for skill in .agents/skills/*/SKILL.md; do
  today=$(date +%d-%m-%y)
  sed -i "s/^> last audited .* by .*/> last audited $today by skill-drift-guard/" "$skill"
done
```

```bash
# Example: replace an old version with the new one
sed -i 's|rusqlite = { version = "0.31"|rusqlite = { version = "0.32"|' .agents/skills/project-scaffold/SKILL.md
```

Always show the diff before committing. The drift guard never pushes.

---

## Drift report format

After running detection, produce a single report:

```markdown
# Skill drift report — <DD-MM-YY>

## Auto-patched (<n>)

- `project-scaffold/SKILL.md`: bumped `rusqlite` 0.31 → 0.32 (matches workspace).
- `rust-backend/SKILL.md`: bumped audit date 26-06-26 → 28-06-26.
- `onboarding-guide/SKILL.md`: removed broken reference to `oauth-integration` (skill does not exist).

## Manual review needed (<n>)

- `tauri-ipc/SKILL.md`: example uses `cart.add_line(sku, qty)` but `oz-core` now exposes `Cart::add_line_with_discount(sku, qty, discount)`. The example compiles but uses the old API.
- `hal-drivers/SKILL.md`: new device `customer-display` was added to `hal/src/traits/`, but the skill does not list it. Add a row to the layout diagram.
- `AGENTS.md` now requires `cargo audit` in CI. `project-scaffold/SKILL.md` does not mention it. Add to the security workflow.

## False positives (<n>)

- `tauri-ipc/SKILL.md` references `Cargo.lock` — the warning is intentional (binary crates must commit it).

## Skipped (<n>)

- `ui-components/SKILL.md` uses `formatMoney` from a not-yet-existent utility. Not drift; the file is a roadmap.
```

Open a `fix(docs): sync skills with code drift report <DD-MM-YY>` PR for everything in the "Manual review needed" section.

---

## CI integration

Add a job to `.github/workflows/ci.yml` that runs the mechanical checks nightly and on changes to `.agents/skills/**`.

```yaml
skill-drift:
  name: Skill drift
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Detect drift
      run: bash .agents/skills/skill-drift-guard/scripts/detect.sh
    - name: Upload report
      if: always()
      uses: actions/upload-artifact@v4
      with:
        name: skill-drift-report
        path: skill-drift-report.md
```

The detection script is a thin wrapper around the seven checks above. Keep it under 200 lines and make every check individually skippable via a flag (`SKIP=api ./detect.sh`) so contributors can iterate quickly.

### Local run

```bash
# Run all checks, no patches
bash .agents/skills/skill-drift-guard/scripts/detect.sh

# Run a single check
bash .agents/skills/skill-drift-guard/scripts/detect.sh --check=paths

# Auto-patch the safe categories
bash .agents/skills/skill-drift-guard/scripts/detect.sh --auto-patch

# Dry-run with a report
bash .agents/skills/skill-drift-guard/scripts/detect.sh --report
```

---

## What this skill explicitly does NOT do

- It does **not** read the project's other `.md` files (`README.md`, `WHITEPAPER.md`, `ARCHITECTURE.md`, `ROADMAP.md`) for drift. Those files are human-maintained documentation, not agent skills. Drift there is a separate concern.
- It does **not** generate new skills. Creating a new skill is a deliberate act; the onboarding guide (`onboarding-guide`) decides what to add.
- It does **not** delete skills. A skill that becomes irrelevant should be removed by the `onboarding-guide` maintainer, not silently.
- It does **not** judge whether a code change is correct. The drift guard checks consistency, not correctness.

---

## When to escalate

- A skill is **factually wrong** about the code (e.g., says `Money::new()` but the function is `Money::zero()`). **File a `fix(docs):` PR immediately.** This is a `CODE DRIFT` finding.
- A skill's `last audited` date is **>90 days** old. **Bump the date** (auto-patch) and add a note to the next sprint to re-audit by hand.
- The `onboarding-guide` references a skill that **does not exist**. **Auto-remove the reference** and create a follow-up issue.
- A **new crate** is added to the workspace. **Manual patch:** add the crate to the relevant skills and the onboarding guide's router table. This is the most common drift class.

---

## Adding a new drift check

When you find a kind of drift this skill doesn't cover:

1. Add a row to the taxonomy table at the top.
2. Write a new `check-N.sh` snippet in the detection workflow.
3. Add a row to the drift report template.
4. If the patch is mechanical, add it to the auto-patch implementation. If not, add a "manual" entry.
5. Bump the audit date.

The drift guard should be self-extending: every discovery becomes a new check, so the next run catches the same class of problem.

---

## Common pitfalls

1. **Auto-patching code examples.** A broken example might be wrong in 3 ways; a script can only fix one. Manual review required.
2. **Treating "missing file" as drift.** A skill may describe a planned path that doesn't exist yet (`hal/src/drivers/customer_display.rs` before the trait lands). Cross-check with the roadmap before flagging.
3. **Skipping the report.** Even if you auto-patch, produce the report. The next contributor needs the audit trail.
4. **Running on `main` only.** Run the drift guard on every PR that touches `.agents/skills/**` or a referenced path. Catch drift at PR time, not after merge.
5. **Trusting the workspace `members` list as ground truth.** It isn't. A crate in `members` can be an empty stub with no real code yet. The drift guard checks *what the code says*, not what the build manifest claims.
6. **Comparing `last audited` dates as strings.** They're `dd-mm-yy`, which doesn't sort lexicographically. Parse them or use ISO-8601 (`2026-06-28`) and convert for display.
7. **Patching the onboarding-guide's router table** when a skill is added. Yes, do this — but also patch every skill that mentions the new skill as a "see also" cross-reference. The graph is bidirectional.

---

> last audited 28-06-26 by skill-drift-guard
