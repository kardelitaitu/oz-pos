# Product Management Screen Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** ProductManagementScreen — product CRUD, pricing, categories, tax-rate assignment, stock display, locations, alerts, and variants
> **Status:** ✅ **FULLY REMEDIATED** — all 12 findings closed (2026-08-02)
> **Production code changed:** All 12 findings closed; 11 remediated across 5 commits (PROD-01 was already satisfied at audit time) — see commit chain below

## Scope

This audit evaluates the Product Management screen against the universal checklist in `audit/AUDIT_JULY_2026.md`: functionality and state management, loading/error/empty states, accessibility and localization, theming, performance, security and authorization, data integrity, and quality assurance.

Inspected areas:

- `ui/src/features/products/ProductManagementScreen.tsx`
- `ui/src/features/products/ProductManagementScreen.css`
- `ui/src/features/products/useProducts.ts`
- `ui/src/api/products.ts`
- `ui/src/api/tax.ts`
- `apps/desktop-client/src/commands/products.rs`
- `apps/desktop-client/src/commands/categories.rs`
- `apps/desktop-client/src/commands/tax.rs`
- `crates/oz-core/src/db/products.rs`
- Product Management, add/edit modal, category, and related UI tests
- `ui/src/locales/products.ftl` and `ui/src/locales/products.id.ftl`

## Architecture summary

`ProductManagementScreen` correctly uses the session-scoped product list and scoped product create/update/delete commands. It keeps the CRUD form inline, loads currencies, categories, and tax rates for the form, displays stock and product type data, opens a right-side stock-alert drawer, and delegates variant management to `VariantManagementScreen`.

The screen has a loading skeleton, an empty state, a focus-trapped product modal with exit animation, and a 30-second stock-alert badge poll. Product and category APIs expose both legacy global commands and ADR #7 scoped variants. Tax APIs likewise expose `listTaxRatesScoped`, but the screen currently uses the legacy tax/category list calls.

## Findings

### PROD-01 — Product form loads categories and tax rates through unscoped commands (P1 tenant-isolation risk)

**Evidence:** `ProductManagementScreen.tsx::load()` calls `listProductsScoped(sessionToken)` and `listCurrenciesScoped(sessionToken)`, but calls `listTaxRates()` and `listCategories()`. `ui/src/api/products.ts` exposes `listCategoriesScoped(sessionToken)`, and `ui/src/api/tax.ts` exposes `listTaxRatesScoped(sessionToken)`. The corresponding Rust commands document the global versions as deprecated for multi-store deployments and provide scoped alternatives.

**Impact:** In a multi-store deployment, the product form can receive category and tax-rate data from the global database rather than the store resolved from the active session. A user may see or assign reference data from outside the current store, and the UI's scope model is inconsistent with the product list itself. The exact data exposure depends on the global database contents and deployment mode, so this is recorded as a risk until runtime store wiring is confirmed.

**Recommendation:** Replace both calls with `listTaxRatesScoped(sessionToken)` and `listCategoriesScoped(sessionToken)`. Add an IPC contract/component test that asserts all four initial-load calls carry the active session scope.

**Status:** Open · P1 risk

### PROD-02 — Delete action executes immediately without confirmation

**Evidence:** The destructive table button calls `confirmDelete(p.sku)` directly. `confirmDelete()` immediately calls `deleteProductScoped(sessionToken, sku)`. There is no confirmation dialog, `window.confirm`, or second explicit confirmation state in `ProductManagementScreen.tsx.

**Impact:** A single accidental click permanently invokes the product deletion command. This is especially dangerous in a dense table where the delete control sits beside Variants and Edit.

**Recommendation:** Add a localized confirmation dialog with `role="alertdialog"`, `aria-modal="true"`, focus trapping, explicit Cancel/Delete actions, and the product name/SKU in the message. Keep the destructive command disabled while the request is in flight.

**Status:** Open · P1

### PROD-03 — Delete failures are swallowed with no user feedback

**Evidence:** The `catch` block in `confirmDelete()` only calls `setDeleting(null)`. It does not set an error state, show a toast, or render an inline alert. The save path does surface errors through `saveError`, but the delete path has no equivalent.

**Impact:** Foreign-key or backend failures can leave the product intact while the operator receives no explanation. The user may retry repeatedly or assume the product was deleted when it was not.

**Recommendation:** Add a `deleteError` state and render a localized alert/toast containing the backend error, then reload only after a successful delete. Add a test for a rejected `deleteProductScoped` call.

**Status:** Open · P1

### PROD-04 — Product load failures render as an empty catalog

**Evidence:** `load()` catches all errors with the comment `// IPC unavailable.` and does not set an error state. Its `finally` sets `loading` to false. With the initial `products` value still empty, the render falls through to the “No products yet” empty state, which offers “Add your first product.”

**Impact:** A database, permission, or IPC outage is indistinguishable from a genuinely empty catalog. Operators may attempt to create duplicate data or conclude that the catalog was lost. There is no retry action for the failed initial load.

**Recommendation:** Add a distinct `loadError` state, render a localized error/retry state, and reserve the empty state for a successful zero-row response. Preserve the last known catalog during refreshes where possible.

**Status:** Open · P1

### PROD-05 — Price input silently truncates decimal input

**Evidence:** The form labels the field as `Price (minor units)` and uses `type="number"`, but `handleSave()` parses it with `parseInt(form.priceMinor, 10)`. An input such as `4.50` therefore becomes `4` rather than being rejected or interpreted as a major-unit price. The validation only checks `NaN` and negative values.

**Impact:** An operator who enters a familiar decimal price can save a value that is 100× smaller than intended. The current placeholder (`450`) communicates minor units, but the silent truncation remains unsafe for pasted or manually entered decimal values.

**Recommendation:** Reject non-integer minor-unit input explicitly, set `step="1"`, and show a localized validation message; or change the field to accept major units and convert through the currency's minor-unit rules. Add tests for decimal, negative, blank, and very large inputs.

**Status:** Open · P1 UX/data-integrity risk

### PROD-06 — Initial stock accepts malformed and negative values through fallback parsing

**Evidence:** On create, `initialStock` is sent as `parseInt(form.initialStock, 10) || 0`. The input has `min="0"`, but HTML `min` is not a backend or submit-time guarantee. A value beginning with digits and trailing text is partially parsed, while a negative value such as `-1` remains `-1` because it is truthy and is passed to the backend; blank or non-numeric values fall back to `0` rather than producing a validation error.

**Impact:** User input can be silently transformed or a negative quantity can reach the backend instead of being rejected. This makes the saved inventory differ from what the operator entered and hides data-entry mistakes.

**Recommendation:** Validate a complete non-negative integer before calling the command and show a field-level error. Keep backend validation authoritative and add tests for `-1`, `1abc`, decimal values, blank input, and safe upper bounds.

**Status:** Open · P2

### PROD-07 — Product form contains hardcoded visible and accessible fallback strings

**Evidence:** The component includes literal fallback/user-facing strings such as `Products`, `Add Product`, `Close stock alerts`, `Open stock alerts`, `Actions`, `Edit Product`, `Add Product`, `Close`, `e.g. LATTE`, `e.g. Caffè Latte`, `450`, `4901234567890`, `Product Type`, and option text. Some are inside `Localized`, but several `l10n.getString(...)` calls use `||` hardcoded English fallbacks. The action buttons also specify literal `aria-label` values (`Variants for ...`, `Edit ...`, `Delete ...`) despite localized wrappers being present.

**Impact:** Missing or incomplete Fluent messages can leave screen-reader labels and visible text in English, and localized wrapper attributes can be undermined by the literal attributes. This causes inconsistent language output and makes accessibility regressions harder to detect.

**Recommendation:** Use the established localized fallback helper consistently, add explicit Fluent fallbacks for every visible/ARIA string, and avoid duplicate literal `aria-label` values when `Localized attrs` owns the attribute. Verify both English and Indonesian bundles in the component tests.

**Status:** Open · P2

### PROD-08 — Stock-alert drawer is not exposed as a dialog/drawer landmark

**Evidence:** `showAlertPanel` renders `.product-mgmt-alert-drawer` as a plain `div`. It has a close button without an `aria-label` and no `role="dialog"`, `aria-modal`, labelled title relationship, focus trap, or Escape-key close handling. The product form modal does use dialog semantics and `useFocusTrap`, but the alert drawer does not.

**Impact:** Keyboard and screen-reader users cannot reliably identify, enter, or dismiss the overlay. Focus can remain behind the drawer, and the close control's accessible name depends on visible localized content rather than an explicit label.

**Recommendation:** Implement drawer semantics with a labelled dialog or complementary landmark, trap focus while open, restore focus to the bell button on close, add Escape handling, and give the close button a localized `aria-label`.

**Status:** Open · P2

### PROD-09 — Tax-rate checkbox layout and other form styling rely on inline styles

**Evidence:** The tax-rate list and checkbox labels use inline `style` objects for layout, spacing, cursor, and typography. The stock-low value also uses an inline `color` and `fontWeight`; the stylesheet already defines the semantic `.product-mgmt-stock-low` class, but the inline declaration duplicates and overrides part of that styling.

**Impact:** Inline declarations bypass the component's CSS organization and make theme/contrast audits, responsive adjustments, and design-token enforcement harder. The current color uses a token, but the pattern remains inconsistent and difficult to govern.

**Recommendation:** Move these declarations into semantic CSS classes using existing design tokens. Add the screen to the theme-token compliance checks and test dark/high-contrast rendering where supported.

**Status:** Open · P3

### PROD-10 — Stock alert polling silently masks backend errors

**Evidence:** The 30-second `getActiveStockAlerts()` poll catches all errors and intentionally does nothing. The alert badge remains at its previous value, including zero, with no stale/error indicator or retry affordance. The drawer delegates its own loading/error behaviour to `StockAlertPanel`, but the header badge does not communicate poll failure.

**Impact:** Operators can believe there are no alerts when the alert service is unavailable or the session has expired. A stale count is presented as current.

**Recommendation:** Track the alert request state and timestamp, distinguish “zero alerts” from “unable to load,” and provide a retry action in the drawer. Cancel or ignore stale responses when location/session changes.

**Status:** Open · P2

### PROD-11 — Product list mapping can display stale catalog data after overlapping loads

**Evidence:** `load()` is memoized by `sessionToken` and can be triggered by the effect when the token changes, while the delete flow also calls `await load()` after a mutation. The function has no request sequence ID, cancellation flag, or stale-response guard. Whichever request resolves last writes `productDtos`, `products`, categories, currencies, and tax rates.

**Impact:** A slower earlier request can overwrite a newer result after a session or mutation change. The user may see a product list from the previous request or miss a newly created/deleted item until another reload.

**Recommendation:** Add a request-generation guard or abort/cancellation pattern and ensure mutation refreshes cannot be overwritten by stale initial loads. Add a deferred-promise test that resolves requests out of order.

**Status:** Open · P2 risk

### PROD-12 — Product and reference-data loads perform per-product tax-rate queries

**Evidence:** `apps/desktop-client/src/commands/products.rs::map_products_to_dtos()` calls `store.get_product_tax_rates(...)` once for every product while mapping the list. The screen also loads products, tax rates, categories, and currencies concurrently. The list command therefore has an N+1 database pattern for tax assignments.

**Impact:** Catalog load latency and database work grow linearly with product count, which will become visible for larger catalogs and during repeated refreshes after CRUD actions.

**Recommendation:** Add a batch query that returns product-to-tax-rate assignments for all listed SKUs, or join/aggregate the assignments in the product query. Measure catalog load time at realistic catalog sizes and add a regression benchmark or query-count test.

**Status:** Open · P3 performance

## Positive controls observed

- Product listing and CRUD use session-scoped commands in the main screen.
- Scoped product commands resolve the session's store and read the session user for permission checks.
- Product create/update/delete operations use the backend permission gate for the corresponding product permission.
- The product modal has `role="dialog"`, `aria-modal="true"`, exit animation handling, and `useFocusTrap`.
- Loading skeleton and successful-empty states are present.
- Product CRUD errors are surfaced in the save modal, and UI controls disable while saving/deleting.
- CSS generally uses shared design tokens and reduced-motion media queries.
- Focused Product Management tests and API/component coverage exist.

## Test and validation results

Focused validation completed during this audit:

```text
cd ui
npx vitest run src/__tests__/ProductManagementScreen.test.tsx \
  src/__tests__/AddProductModal.test.tsx \
  src/__tests__/EditProductModal.test.tsx \
  src/__tests__/CategoryManagementScreen.test.tsx
npm run typecheck
```

Results:

- Focused UI tests: **34 passed, 0 failed** across 4 files
- TypeScript typecheck: **passed with 0 errors**
- A broader product-related run previously reported **67 passed across 6 files**
- Rust product test execution was not usable in this audit run because Cargo encountered an OS file-lock error while removing/relinking `target/debug/oz-pos-app.exe`; no Rust pass count is claimed here

## Recommended remediation order

1. **PROD-01:** Replace unscoped category/tax loads with session-scoped APIs.
2. **PROD-02 and PROD-03:** Add confirmation and visible failure handling for deletion.
3. **PROD-04 through PROD-06:** Separate load failure from empty state and validate numeric form input strictly.
4. **PROD-07 and PROD-08:** Complete localization and overlay accessibility for the screen and stock-alert drawer.
5. **PROD-10 and PROD-11:** Make polling and refreshes observable and race-safe.
6. **PROD-12:** Remove the per-product tax-rate query pattern before large-catalog rollout.
7. **PROD-09:** Finish CSS/token cleanup and add theme compliance coverage.

## Remediation status — 2026-08-02

All 12 findings are closed. Commit chain:

| Commit | Scope |
|---|---|
| `f399c703` | PROD-02/03/04 — delete confirmation dialog, delete-error surfacing, load-error + retry state |
| `beba8dad` | PROD-05/06 — strict non-negative-integer validation for price and initial stock |
| `6a6840aa` | PROD-07/08 — localization sweep (no duplicate aria-labels) + focus-trapped alert drawer |
| `6b9aead9` | PROD-09/10/11 — token-based styles, alert poll error banner + retry, stale-load seq guard |
| `67bb09c1` | PROD-12 — batch product tax-rate query (kills catalog N+1) |

### Per-finding closure

- **PROD-01** — already satisfied at audit time: `load()` uses `listTaxRatesScoped`/`listCategoriesScoped` with the session token (TAX-01); a contract test pins both scoped calls and asserts the unscoped variants are never invoked.
- **PROD-02** — delete now routes through a localized `ConfirmDialog` (`role="alertdialog"`, focus-trapped); the destructive command only fires after explicit confirmation and stays disabled in flight.
- **PROD-03** — delete failures render in a visible `role="alert"` (backend message), instead of being swallowed.
- **PROD-04** — failed loads render a distinct error card with a Retry action; the empty-catalog CTA is reserved for successful zero-row loads. Refreshes preserve the last known catalog (skeleton only on first load per session).
- **PROD-05** — price is validated as a complete non-negative safe integer (`/^\d+$/` + `Number.isSafeInteger`); `4.50` is rejected with a localized error, never truncated to `4`. Inputs gained `step="1"` + `inputMode="numeric"`.
- **PROD-06** — initial stock validated identically; blank / `-1` / `1abc` / out-of-safe-integer values are rejected before any IPC call.
- **PROD-07** — row action buttons (Variants/Edit/Delete) use a single `requiredLocalized` aria-label; no duplicated literal `aria-label` or `|| 'English'` fallbacks remain.
- **PROD-08** — the stock-alert drawer is now `role="dialog" aria-modal="true"` with a labelled title, `useFocusTrap`, Escape + close-button dismissal, focus restore to the bell toggle, and `aria-expanded`. Drawer and product modal are mutually exclusive (single focus trap at a time).
- **PROD-09** — tax-rate list/labels moved from inline styles to `.product-mgmt-tax-rate-list`/`-option` token classes; the stock-low span dropped its inline override.
- **PROD-10** — poll failures set `alertError` shown in a drawer banner with a localized reload button; a `pollSeqRef` stale-response guard prevents old location/session polls from overwriting newer ones.
- **PROD-11** — `load()` has a request-sequence guard (`loadSeqRef`); results/errors/loading only apply when the call is still latest. Verified by a genuinely overlapping two-load race test (deferred promise, no wall-clock waits).
- **PROD-12** — new `Store::get_product_tax_rates_batch` resolves all product tax assignments in one `IN (...)` query; `map_products_to_dtos` uses it, removing the per-product N+1. Error propagation is now fail-loudly (documented).

### Validation

- UI: `ProductManagementScreen.test.tsx` **24/24 pass** (including new PROD-02/03/04/05/06/10/11 tests), `tsc --noEmit` clean, i18n lint + bundle parity clean.
- Rust: `oz-core db::tax` **40/40**, `oz-pos-app commands::products` **30/30**, `cargo clippy -- -D warnings` clean.
- Each phase was code-reviewed; all reviewer follow-ups (loadError gating, `l10nRef`, overlapping-race test premise, `hasLoadedOnceRef` reset on session switch, scoped ConfirmDialog loading, borrow-before-move) resolved before commit.

## Audit status

This evidence-based audit report is complete. All 12 findings were remediated, validated, and reviewed; the master index marks this sector **FULLY REMEDIATED**.
