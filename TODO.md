# TODO — OZ-POS v0.0.3

## Completed Improvements

- [x] **KDS integration** — `createKdsOrderFromSale()` called from PaymentModal after `completeSale()`
- [x] **Open bill table number** — table number persisted and restored with open bills
- [x] **Toast error feedback** — silent catch blocks now surface errors via toast
- [x] **Localized multi-child fix** — KioskScreen + KdsScreen expression+text wrapped in `<span>`

## Animations (Restaurant POS)

- [x] **PaymentModal** — overlay fade + modal slide-up on open; fade-out + scale-down on close
- [x] **Cart line item removal** — slide-out/fade-out animation before DOM removal
- [ ] **Modal exit animations** — Hold cart, held carts, shift modals (all fade in but snap out)
- [ ] **Add-to-cart feedback** — brief pulse/green flash on RestaurantMenu/ProductLookup cards when tapped
- [ ] **Quantity change pulse** — scale-bounce on qty value when +/- is tapped
- [ ] **Total amount change highlight** — amber pulse when discount/tip/service charge updates total
- [ ] **Staggered card entrance** — RestaurantMenu grid cards cascade in with incremental delay
- [ ] **Discount/tip/service charge form slide-in** — form rows slide down instead of appearing instantly
- [ ] **Empty cart ↔ populated cart cross-fade** — smooth transition between bag icon and line items
- [ ] **Sale complete celebration** — checkmark draw or confetti on PaymentModal "Done" screen

## High Priority

- [x] **Full i18n migration** — Fluent `.ftl` files for all screens
- [ ] **White-label theming** — brand logo upload, primary color picker, theme preview in Settings (infra already in place)
- [x] **SQLite-backed cart** — replace in-memory CartStore with SQLite persistence (`active_carts` table, JSON blob, db::cart module, both clients)

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
