# TODO — OZ-POS v0.0.3

## Top Priority — Retail POS Improvements

### UX Polish
- [x] Migrate hardcoded strings to Fluent i18n (en + id)
- [x] Replace hardcoded colors with CSS design tokens (`--color-*` vars)
- [x] Add dark mode support for Retail POS layout (CSS tokens + data-theme + system preference + manual toggle in Options)
- [x] Implement product pagination / virtual scrolling for 1000+ SKUs
- [x] Replace silent `.catch(() => {})` with proper error handling + toast feedback
- [x] Add ARIA labels to function bar buttons and all interactive elements
- [x] Single-tap quick-add (tap adds qty 1; long-press for qty picker)
- [x] On-screen numpad in qty picker modal
- [x] Stock-level indicators on grid: green (ok) / yellow (low) / red (critical)
- [x] Sound feedback: beep on scan, error tone, cha-ching on sale complete
- [x] Show change due in large text after cash payment
- [x] Session auto-lock after N minutes of inactivity
- [x] Dim out-of-stock items on grid; increase gap between buttons
- [x] Discount modal: two tabs — % discount and Rp fixed amount
- [x] Show date + shift duration in header clock
- [x] Price volatility hint on product card (needs `price_updated_at` backend field)

### Feature Gaps
- [ ] Integrate loyalty points display and redemption at checkout
- [x] Validate stock availability before adding to cart (incl. cart `+` button)
- [x] Support multiple held carts (view/resume/delete list)
- [ ] Move authorization checks from frontend `roleId` to backend
- [x] Duplicate scan increments qty (instead of adding duplicate row)
- [ ] Manager price override with supervisor PIN at checkout
- [x] Quick cash tender buttons (Rp 10k, 20k, 50k, 100k, 200k) in payment modal
- [ ] Serial number capture at checkout for warranty tracking
- [x] Reorder / low-stock banner when any product hits threshold
- [ ] Quick return from POS — scan receipt barcode to initiate refund
- [ ] USB weight scale support for produce/groceries

### Testing
- [x] Add unit tests for RetailPosScreen, RetailOptionsScreen components
- [x] Add integration test for full checkout flow in retail mode

## Completed Improvements

- [x] **Persist held carts** — `hold_cart` IPC wired in RetailPosScreen, carts stored in SQLite
- [x] **Receipt printing after payment** — `printSalesReceipt` already called on `onComplete` in PaymentModal
- [x] **Tax engine integration** — PPN/multirate tax applied to cart totals
- [x] **Customer selection at checkout** — `customer_id` in sales table, customer picker UI in RetailPosScreen
- [x] **KDS integration** — `createKdsOrderFromSale()` called from PaymentModal after `completeSale()`
- [x] **Open bill table number** — table number persisted and restored with open bills
- [x] **Toast error feedback** — silent catch blocks now surface errors via toast
- [x] **Localized multi-child fix** — KioskScreen + KdsScreen expression+text wrapped in `<span>`
- [x] **Dev banner localization** — ProductLookupScreen already uses `<Localized>` with Fluent entries
- [x] **Graceful kernel shutdown** — `Kernel::stop_all()` wired via `AppState::drop()`
- [x] **Structured error payloads** — `CoreErrorKind` + `HalErrorKind` enums, `AppError` variant refactor, TS mirror
- [x] **Silent catch blocks** — 12 `.catch(() => {})` instances replaced with proper toast error messages in RetailPosScreen, RetailOptionsScreen, PaymentModal, and PosScreen
- [x] **Stock color tiers** — product grid badge shows green (>10), yellow (6-10), or red (1-5) based on stock level
- [x] **Date + shift duration in clock** — header displays weekday, date, time, and active shift duration
- [x] **Quick cash tender buttons** — denominations changed to Rp 5.000/10.000/20.000/50.000/100.000 with `Rp` symbol and `id-ID` locale formatting
- [x] **Discount modal tabs** — percentage and Rp fixed amount tabs, Rp value converted to equivalent percentage of subtotal
- [x] **Fluent i18n (retail screens)** — ~160 hardcoded strings migrated in RetailPosScreen + RetailOptionsScreen; 143 new FTL IDs added to sales.ftl and settings.ftl
- [x] **Product pagination** — page-based grid navigation (50 items/page) with prev/next controls; auto-resets on filter/category change
- [x] **Multiple held carts** — held carts modal lists all saved carts with resume (tap row) and delete (× button) actions
- [x] **CSS design tokens** — 45+ `--color-*` custom properties defined in `:root`, all retail POS colors tokenized
- [x] **Low-stock banner** — gold header banner showing count of products with stock ≤ 5, with FTL and `lowStockCount` memo
- [x] **Dim out-of-stock items** — `.retail-product-btn--out-of-stock` class at 45% opacity, click disabled; grid gap increased to 4px
- [x] **ARIA labels** — toolbar role, category `aria-pressed`, product grid `aria-label`/`aria-disabled`, cart action aria-labels, qty ±, remove, page nav, and fn bar (via visible text)
- [x] **Sound feedback** — `useSound` hook with `AudioContext` beep (scan), error tone (fail), and ascending triad (sale complete); wired into RetailPosScreen barcode handler, error toasts, and PaymentModal onComplete
- [x] **Dark mode** — full dark palette via CSS variables; auto-detects `prefers-color-scheme`; manual toggle in Options → System tab; persisted to localStorage

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


