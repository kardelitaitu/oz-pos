# TODO — OZ-POS v0.0.3

## Retail POS Improvements

### Feature Gaps
- [x] Serial number capture at checkout for warranty tracking
- [x] Quick return from POS — scan receipt barcode to initiate refund
- [x] USB weight scale support for produce/groceries
- [x] Integrate loyalty points display and redemption at checkout
- [x] Move authorization checks from frontend `roleId` to backend
- [x] Duplicate scan increments qty (instead of adding duplicate row)
- [x] Manager price override with supervisor PIN at checkout
- [x] Quick cash tender buttons (Rp 10k, 20k, 50k, 100k, 200k) in payment modal
- [x] Validate stock availability before adding to cart (incl. cart `+` button)
- [x] Support multiple held carts (view/resume/delete list)
- [x] Reorder / low-stock banner when any product hits threshold

### UX Polish
- [x] Fluent i18n (en + ID) — 160+ strings, 143 FTL IDs across retail screens
- [x] CSS design tokens — 45+ `--color-*` custom properties
- [x] Dark mode — system preference + manual toggle, persisted
- [x] Product pagination — 50/page, prev/next, resets on filter
- [x] Toast error feedback — replaced all silent `.catch(() => {})`
- [x] ARIA labels — all interactive elements
- [x] Single-tap quick-add + long-press qty picker with numpad
- [x] Stock-level indicators on grid: green / yellow / red
- [x] Sound feedback — beep on scan, error tone, cha-ching
- [x] Large change-due display after cash payment
- [x] Session auto-lock after inactivity
- [x] Dim out-of-stock items; increased grid gap
- [x] Discount modal — % and Rp tabs with cross-conversion
- [x] Date + shift duration in header clock
- [x] Price volatility hint (needs backend `price_updated_at`)

### Testing
- [x] Unit tests for RetailPosScreen, RetailOptionsScreen
- [x] Integration test for full retail checkout flow

## Animations (Restaurant POS)
- [x] PaymentModal overlay fade + slide-up / scale-down
- [x] Cart line item removal slide-out
- [x] Hold cart / held carts / shift modal entrance keyframes
- [x] Add-to-cart green flash pulse on cards
- [x] Qty +/- scale-bounce
- [x] Subtotal amber pulse on change
- [x] Staggered card entrance (35ms cascade)
- [x] Discount/tip/service form slide-in
- [x] Empty ↔ populated cart cross-fade
- [x] Sale complete SVG checkmark draw + celebration

## High Priority
- [x] Full Fluent i18n migration (all screens)
- [x] White-label theming — logo, primary color, preview
- [x] SQLite-backed `active_carts` table

## Medium Priority
- [x] Lua plugin system — `oz.*` API, hooks, `sale.before_complete`
- [x] Plugin hot-reload — file watcher + `reload_plugins` command
## Earlier Shipments
- [x] Purchase Orders + Supplier management
- [x] Physical Inventory / Stock Counting / Stock Transfers
- [x] Gift Cards — issue, redeem, top-up, freeze, balance checks
- [x] Receipt printing, Tax engine, Customer selection, KDS, Open bills
- [x] Structured error payloads (`CoreErrorKind`, `HalErrorKind`, TS mirror)
- [x] Graceful kernel shutdown (`Kernel::stop_all()` via `AppState::drop()`)
- [x] Localized multi-child fixes for KioskScreen + KdsScreen
- [x] Dev banner localization (ProductLookupScreen)
