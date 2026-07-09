# TODO — OZ-POS v0.0.3

## Active Sprint

### 1. Product Type Separation (`product_type` enum)
Separate retail products from restaurant menu items at the data model level.

| Step | Area | What |
|------|------|------|
| 1.1 | **DB migration** | Add `product_type TEXT NOT NULL DEFAULT 'retail'` to `products` ✓ |
| 1.2 | **Rust** | Update `Product` struct + `ProductType` enum + DB queries + command args ✓ |
| 1.3 | **Rust** | Update KDS order creation to skip lines whose product has `product_type = 'retail'` ✓ |
| 1.4 | **Rust** | *(deferred)* Add optional `restaurant_meta` table (prep_time_seconds, course_number, recipe_id, modifier_group_ids) |
| 1.5 | **TS types** | Update `Product` / `ProductDto` / `CreateProductInput` with `productType` ✓ |
| 1.6 | **RetailPosScreen** | Filter product grid to `product_type IN ('retail','both')` ✓ |
| 1.7 | **RestaurantMenu** | Filter to `product_type IN ('restaurant','both')` ✓ |
| 1.8 | **ProductManagementScreen** | Add product_type dropdown in add/edit form; show badge in table ✓ |
| 1.9 | **ProductLookupScreen** | *(deferred)* Respect workspace context when filtering |
| 1.10 | **Tests** | Update existing tests; add tests for dual-type filtering |

### 2. Categorized, Collapsible Side Menu
Use the existing but unused `section` field on nav items to group them, rendered as accordion sections.

| Step | Area | What |
|------|------|------|
| 2.1 | **NavItemRegistration** | `section` field typed as `SectionName` union ✓ |
| 2.2 | **menu-registry** | All 35 nav items assigned to sections; `SECTION_LABELS` map ✓ |
| 2.3 | **AppLayout.tsx** | `groupBySection()` helper + accordion rendering with chevron ✓ |
| 2.4 | **AppLayout.tsx** | Per-section collapse persisted in `localStorage['app-sidebar-sections']` ✓ |
| 2.5 | **AppLayout.tsx** | Collapsed mode hides section headers, shows flat icons ✓ |
| 2.6 | **AppLayout.css** | Chevron rotate animation, indent items, section header hover ✓ |
| 2.7 | **i18n** | 10 section labels added (en + id FTL) ✓ |
| 2.8 | **Admin workspace** | *(verify on next Tauri launch)* |
| 2.9 | **Tests** | *(check if tests reference DOM structure)* |

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
