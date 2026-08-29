# Skill drift report — 29-08-26

## paths

```
docs-auditor/SKILL.md -> docs/api-reference.md (archived to docs/archived/; current live twin is not referenced)
docs-auditor/SKILL.md -> docs/QUICKSTART.md (archived to docs/archived/)
docs-auditor/SKILL.md -> scripts/check-orphans.py (actual: .agents/skills/docs-auditor/scripts/check-orphans.py)
hal-drivers/SKILL.md -> hal/... (6 refs: crate is crates/oz-hal, not hal/)
onboarding-guide/SKILL.md -> hal/src/drivers/mock.rs (crate is crates/oz-hal)
onboarding-guide/SKILL.md -> scripts/detect.sh (actual: .agents/skills/skill-drift-guard/scripts/detect.sh)
project-scaffold/SKILL.md -> src/lib.rs (generic example, not a real path)
rust-backend/SKILL.md -> hal/src/drivers/mock.rs (crate is crates/oz-hal)
skill-drift-guard/SKILL.md -> docs/QUICKSTART.md (archived to docs/archived/)
skill-drift-guard/SKILL.md -> hal/src/drivers/customer_display.rs (planned path; crate is crates/oz-hal)
skill-drift-guard/SKILL.md -> scripts/detect.sh (actual: .agents/skills/skill-drift-guard/scripts/detect.sh)
skill-drift-guard/SKILL.md -> scripts/lib.sh (does not exist anywhere)
skill-drift-guard/SKILL.md -> scripts/run-tests.sh (actual: .agents/skills/skill-drift-guard/scripts/run-tests.sh)
tauri-ipc/SKILL.md -> apps/desktop-client/src/commands/sales.rs (renamed to pos.rs)
tauri-ipc/SKILL.md -> ui/src/features/sales/useCart.ts (does not exist)
tdd/SKILL.md -> docs/api-reference.md (archived to docs/archived/)
tdd/SKILL.md -> docs/user-guide.md (archived to docs/archived/)
ui-components/SKILL.md -> ui/src/__tests__/features/sales/CartLine.test.tsx (does not exist)
ui-components/SKILL.md -> ui/src/locales/en-US.ftl (split into per-feature .ftl bundles)
ui-components/SKILL.md -> ui/src/styles/tokens.css (moved to ui/src/frontend/themes/tokens.css)
```

## crates

```
5 crates in workspace but not mentioned in any skill:
  - oz-api
  - oz-crypto
  - oz-media
  - oz-notification
  - oz-plugin
```

## fluent

```
ui-components/SKILL.md: 4 Fluent ids referenced but not found in ui/src/locales/:
  - inventory-sku-error
  - inventory-sku-label
  - sku-error
  - sku-input
```

## audit-date

```
No stale audit dates found (all skills < 30 days old; oldest: tdd 22 days, docs-auditor 21 days)
```

## audit-format

```
No format violations found (all footers match ^> last audited [0-9]{2}-[0-9]{2}-[0-9]{2} by <name>$)
```

## doc-audit

```
No project-doc audit-footer format violations found
```

## Manual review needed

1. **hal/ prefix drift** — 6 skills reference `hal/...` paths but the crate is
   `crates/oz-hal`. Update all refs to `crates/oz-hal/...`.
2. **docs/ archived refs** — docs-auditor, tdd, skill-drift-guard SKILLs
   reference `docs/*.md` that moved to `docs/archived/` (commit d0fe7481).
   Repoint to `docs/archived/`.
3. **Relative script paths** — skill-drift-guard and docs-auditor reference
   `scripts/...` but their scripts live in `.agents/skills/<skill>/scripts/`.
   Use full relative paths from repo root.
4. **tauri-ipc/SKILL.md** — references removed `commands/sales.rs` (now
   `pos.rs`) and non-existent `ui/src/features/sales/useCart.ts`.
5. **ui-components/SKILL.md** — 4 Fluent ids not found in `ui/src/locales/`
   (inventory-sku-error, inventory-sku-label, sku-error, sku-input). Either
   add the ids to the bundles or remove the references.
6. **New crates not documented** — oz-api, oz-crypto, oz-media,
   oz-notification, oz-plugin exist in the workspace but are not mentioned in
   any skill. Add to onboarding-guide router + relevant skills.
7. **project-scaffold `src/lib.rs`** — generic placeholder, not a repo path;
   confirm intended as illustrative.

## Auto-patchable

- None (all findings require manual review per the taxonomy; audit-date and
  cross-ref categories are clean).

---

Findings: 4 categories active (paths, crates, fluent), ~20 distinct refs across 7 skills.
Manual review needed: 7 items.
Auto-patched: 0.
