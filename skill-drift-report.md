# Skill drift report — 29-08-26

## Repaired this run (commit 0534617f)

```
hal/ -> crates/oz-hal/ prefix:
  hal-drivers/SKILL.md (12), onboarding-guide/SKILL.md (3),
  rust-backend/SKILL.md (1), skill-drift-guard/SKILL.md (5)
docs/ -> docs/archived/ (guides archived 2026-08-29, commit d0fe7481):
  docs-auditor/SKILL.md (api-reference, QUICKSTART),
  tdd/SKILL.md (api-reference, user-guide),
  skill-drift-guard/SKILL.md (QUICKSTART)
scripts/ -> .agents/skills/<skill>/scripts/ full paths:
  skill-drift-guard/SKILL.md (detect.sh x2, run-tests.sh),
  onboarding-guide/SKILL.md (detect.sh)
tauri-ipc/SKILL.md:
  commands/sales.rs -> commands/pos.rs (diagram + example)
  useCart.ts -> usePosState.ts (diagram + example header)
  @/api/pos -> @/api/sales; function useCart -> usePosState
ui-components/SKILL.md:
  en-US.ftl -> per-feature bundles (sales.ftl/sales.id.ftl)
  ui/src/styles/tokens.css -> ui/src/frontend/themes/ layout
  locale-add instruction -> ui/src/i18n/ + ui/src/main.tsx
```

## Remaining findings (manual / planned / intentional)

```
docs-auditor/SKILL.md -> docs/adr, docs/specs/_approved
  INTENTIONAL: stamp explicitly documents these do NOT exist in this repo.
docs-auditor/SKILL.md -> scripts/check-orphans.py (in stamp comment, historical)
hal-drivers/SKILL.md -> crates/oz-hal/src/drivers/honeywell_barcode.rs
  ILLUSTRATIVE example file name (stamp F3 notes actual drivers are generic
  usb/bt/serial/tcp, no vendor-specific names)
project-scaffold/SKILL.md -> crates/oz- (truncated regex), src/lib.rs (generic
  example), docs/i18n-contributor-guide (branch-name example),
  docs/specs/_active/0042-cart-discount-engine (planned spec)
rust-backend/SKILL.md -> crates/oz-core/migrations/NNN_ (placeholder)
skill-drift-guard/SKILL.md -> crates/oz- (truncated regex),
  crates/oz-hal/src/drivers/customer_display.rs (planned path, pitfall example),
  scripts/detect.sh + run-tests.sh (in stamp comment, historical),
  scripts/lib.sh (relative-path convention for future lib.sh extraction)
tdd/SKILL.md -> ui/src/__tests__/api- (glob pattern api-*-contract.test.ts)
ui-components/SKILL.md -> ui/src/__tests__/features/sales/CartLine.test.tsx
  (illustrative teaching example; CartLine component does not exist in code)
```

## crates

```
5 crates in workspace but not mentioned in any skill (manual — add to
onboarding-guide router + relevant skills):
  - oz-api
  - oz-crypto
  - oz-media
  - oz-notification
  - oz-plugin
```

## fluent

```
ui-components/SKILL.md: 4 Fluent ids referenced in teaching example but not
in ui/src/locales/:
  - inventory-sku-error
  - inventory-sku-label
  - sku-error
  - sku-input
  (illustrative example ids; either add to bundles or annotate as example-only)
```

## audit-date / audit-format / doc-audit

```
Clean: no stale dates (>30d), no format violations in skills or project docs.
```

## Summary

- Auto-patched / mechanically repaired: 21 path refs across 7 skills
- Manual review needed (judgment calls, not broken links):
  1. Document 5 undocumented workspace crates (oz-api, oz-crypto, oz-media,
     oz-notification, oz-plugin) in skills + onboarding-guide
  2. Decide on 4 example-only Fluent ids in ui-components
  3. Confirm illustrative examples (honeywell_barcode, CartLine, NNN_,
     customer_display) are acceptable as teaching material
- False positives / intentional: ~10 (stamp comments, regex truncation,
  branch-name examples, planned paths)
