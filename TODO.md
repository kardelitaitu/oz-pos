# TODO — OZ-POS v0.0.3

## Top Priority — Retail POS Improvements

### Backend Integration
- [ ] Persist held carts via `hold_cart` IPC (currently in-memory only)
- [ ] Wire receipt printing after payment (link `printSalesReceipt` on complete)
- [ ] Integrate tax engine (PPN/multirate) into cart totals
- [ ] Link customer selection to sale (assign customer at checkout)

### UX Polish
- [ ] Migrate hardcoded strings to Fluent i18n (en + id)
- [ ] Replace hardcoded colors with CSS design tokens (`--color-*` vars)
- [ ] Add dark mode support for Retail POS layout
- [ ] Implement product pagination / virtual scrolling for 1000+ SKUs
- [ ] Replace silent `.catch(() => {})` with proper error handling + toast feedback
- [ ] Add ARIA labels to function bar buttons and all interactive elements
- [ ] Single-tap quick-add (tap adds qty 1; long-press for qty picker)
- [ ] On-screen numpad for touch-only terminals (cash entry, PIN)
- [ ] Stock-level indicators on grid: green (ok) / yellow (low) / red (critical)
- [ ] Sound feedback: beep on scan, error tone, cha-ching on sale complete
- [ ] Show change due in large text after cash payment
- [ ] Session auto-lock after N minutes of inactivity
- [ ] Dim out-of-stock items on grid; increase gap between buttons
- [ ] Discount modal: two tabs — % discount and Rp fixed amount
- [ ] Show date + shift duration in header clock
- [ ] Price volatility hint on product card (PC parts: recently changed)

### Feature Gaps
- [ ] Integrate loyalty points display and redemption at checkout
- [ ] Validate stock availability before adding to cart
- [ ] Support multiple held carts (view/resume/delete list)
- [ ] Move authorization checks from frontend `roleId` to backend
- [ ] Duplicate scan increments qty (instead of adding duplicate row)
- [ ] Manager price override with supervisor PIN at checkout
- [ ] Quick cash tender buttons (Rp 10k, 20k, 50k, 100k, 200k) in payment modal
- [ ] Serial number capture at checkout for warranty tracking
- [ ] Reorder / low-stock banner when any product hits threshold
- [ ] Quick return from POS — scan receipt barcode to initiate refund
- [ ] USB weight scale support for produce/groceries

### Testing
- [ ] Add unit tests for RetailPosScreen, RetailOptionsScreen components
- [ ] Add integration test for full checkout flow in retail mode

## Completed Improvements

- [x] **KDS integration** — `createKdsOrderFromSale()` called from PaymentModal after `completeSale()`
- [x] **Open bill table number** — table number persisted and restored with open bills
- [x] **Toast error feedback** — silent catch blocks now surface errors via toast
- [x] **Localized multi-child fix** — KioskScreen + KdsScreen expression+text wrapped in `<span>`
- [x] **Dev banner localization** — ProductLookupScreen already uses `<Localized>` with Fluent entries
- [x] **Graceful kernel shutdown** — `Kernel::stop_all()` wired via `AppState::drop()`
- [x] **Structured error payloads** — `CoreErrorKind` + `HalErrorKind` enums, `AppError` variant refactor, TS mirror

## Animations (Restaurant POS)

- [x] **PaymentModal** — overlay fade + modal slide-up on open; fade-out + scale-down on close
- [x] **Cart line item removal** — slide-out/fade-out animation before DOM removal
- [x] **Modal exit animations** — Hold cart, held carts, shift modals (all fade in but snap out) — added missing entrance keyframes to ShiftManagementScreen
- [x] **Add-to-cart feedback** — brief pulse/green flash on RestaurantMenu cards when tapped (`restaurant-card--added` class + `restaurant-card-added` keyframe)
- [x] **Quantity change pulse** — scale-bounce on qty value when +/- is tapped (`pos-cart-qty-bounce` keyframe on `.pos-cart-qty-value`)
- [x] **Total amount change highlight** — amber pulse when subtotal changes (`pos-subtotal-flash` keyframe, re-triggered by `key` prop)
- [x] **Staggered card entrance** — RestaurantMenu grid cards cascade in with incremental delay (`animation-delay: index * 35ms` + `restaurant-card-in` keyframe)
- [x] **Discount/tip/service charge form slide-in** — form rows slide down instead of appearing instantly (`pos-discount-form-in` keyframe)
- [x] **Empty cart ↔ populated cart cross-fade** — entrance animations on both states (existing `pos-cart-line-in` on empty msg + each `CartLineItem`)
- [x] **Sale complete celebration** — animated SVG checkmark draw + staggered fade-in on PaymentModal "Done" screen

## High Priority

- [x] **Full i18n migration** — Fluent `.ftl` files for all screens
- [x] **White-label theming** — brand logo upload, primary color picker, theme preview in Settings, sidebar brand rendering
- [x] **SQLite-backed cart** — replace in-memory CartStore with SQLite persistence (`active_carts` table, JSON blob, db::cart module, both clients)

## Medium Priority

- [x] **Plugin system** — `PluginManager` with `oz.*` Lua API (`get_time`, `log`, `apply_discount`, `register_hook`), hook registry, `sale.before_complete` in pipeline
- [x] **Plugin system (Phase 4)** — hot-reload (auto file watcher + manual `reload_plugins` command)
- [ ] **Mobile build pipeline** — Android APK + iOS build scripts in CI (tablet UI complete, builds missing)

## Low Priority / New Features

- [ ] **Purchase Orders + Supplier Management** — supplier domain type, PO migration, CRUD, screens
- [ ] **Physical Inventory / Stock Counting** — cycle counting, reconciliation
- [ ] **Stock Transfers between terminals/stores**
- [ ] **Gift Cards** — issue, redeem, balance check


