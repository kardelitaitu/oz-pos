# TODO — OZ-POS v0.0.3

## High Priority

- [ ] **Full i18n migration** — scan all `.tsx` files for hardcoded English strings and move to Fluent `.ftl` files (known: ProductLookupScreen fallback notice)
- [ ] **White-label theming** — brand logo upload, primary color picker, theme preview in Settings (infra already in place)
- [ ] **SQLite-backed cart** — replace in-memory CartStore with SQLite persistence (TODOs in `apps/*/state.rs`)

## Medium Priority

- [ ] **Plugin system** — hot-reload, third-party HAL driver registration, plugin sandboxing (crate exists, needs finalization)
- [ ] **Mobile build pipeline** — Android APK + iOS build scripts in CI (tablet UI complete, builds missing)

## Low Priority / New Features

- [ ] **Purchase Orders + Supplier Management** — supplier domain type, PO migration, CRUD, screens
- [ ] **Physical Inventory / Stock Counting** — cycle counting, reconciliation
- [ ] **Stock Transfers between terminals/stores**
- [ ] **Gift Cards** — issue, redeem, balance check

## Quick Wins

- [ ] **Graceful kernel shutdown** — implement `stop_all()` in kernel (`apps/*/lib.rs`)
- [ ] **Structured error payloads** — two TODOs in `apps/*/error.rs`
- [ ] **Dev banner localization** — one-line fix in ProductLookupScreen.tsx
