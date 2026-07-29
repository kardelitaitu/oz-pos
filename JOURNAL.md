<!-- Audit stamp: 2026-07-29 · Codebuff · status: UPDATED — July 29 full-codebase i18n audit session appended -->

# OZ-POS Development Journal

## 2026-07-29 — Full-Codebase i18n Audit & FTL Sweep

### Attribute-Only FTL Sweep (TODO #3)
**Problem:** ~268 attribute-only FTL messages (`.aria-label = ...` with no message value) silently returned `undefined` when accessed via `l10n.getString()`, causing empty aria-labels and placeholders across 25 files.

**Solution:** Cross-referenced all 1212 `l10n.getString()` calls against the 268 attribute-only keys. Found 75 keys used without fallbacks across 25 files. Verified `<Localized>` usage: 72 keys safe to convert to simple `key = value` format (125 conversions, 16 bundles via `scripts/convert-safe-attr-ftl.py`). 3 keys shared with `<Localized attrs>` received `||` fallbacks in code.

**Commits:** `104c4891`, `ee5a4f96`

### RestaurantMenu.tsx Audit (TODO #2)
**Problem:** 795-line restaurant/KDS screen was completely un-audited, with 11 missing FTL keys and 2 hardcoded English strings (`aria-label="Menu items"`, hex color codes as aria-labels).

**Solution:** Added 13 FTL keys (en + id): search-aria, search-clear-aria, context-pin/unpin, context-available/unavailable, card-pin-title, sort-manual/a-z/date/popularity, menu-items-aria, color-swatch-aria. Localized the grid aria-label and color swatch labels. CSS audit: 0 hardcoded hex, all tokens. Hooks: all cleanup + deps correct.

**Commits:** `b3307810`, `446a88f3`

### SettingsPage.tsx Audit (TODO #1)
**Problem:** Largest UI file (1081 lines) was surprisingly clean — 244 CSS tokens, zero hardcoded hex, correct hook deps. Only 2 hardcoded strings: `placeholder="Search"` and Suspense fallback `Loading...`.

**Solution:** Added `settings-search-placeholder` and `settings-section-loading` FTL keys (en + id). Localized both strings with `l10n.getString()` and `<Localized>`.

**Commits:** `533247bc`, `de1517dc`

### PosScreen.tsx Audit
**Problem:** Largest file in codebase (2212 lines TSX + 682 CSS). 26 attribute-only bugs already fixed by the FTL sweep. After sweep: 216 CSS tokens (all hex in var() fallbacks), 40 hooks with correct deps, ESLint zero errors.

**Bugs found:** 5 hardcoded strings missed by the sweep:
- Course fire button: `aria-label={`Fire ${course.label} (${holdCount} items)`}` — not inside `<Localized>`
- Fire All button: `<span>Fire All</span>` — not wrapped in `<Localized>`
- Override button in CartLineItem: `aria-label={...}` and `Override` text — both hardcoded
- Missing FTL keys: `pos-cart-course-fire-aria`, `pos-cart-course-btn--all`, `pos-cart-line-override`, `pos-cart-line-override-aria`

**Commit:** `0796d835`

### ProductManagementScreen + CategoryManagementScreen Audit
**Problem:** Two ~640-line screens flagged in the original audit for hardcoded aria-labels. Both were clean after the attribute-only sweep: 92+135 CSS tokens, zero true hardcoded hex.

**Bugs found:** 3 hardcoded strings:
- CategoryManagementScreen: `aria-label={`Edit category ${cat.name}`}` — not inside `<Localized>`
- ProductManagementScreen: Stock alert bell aria-label in English
- ProductManagementScreen: Product type dropdown options (Retail/Restaurant/Service) — not localized

**Commit:** `13023004`

### Session Totals
| Metric | Count |
|--------|-------|
| Bugs fixed | **88** |
| FTL keys added | **28** (en + id) |
| Files changed | **25** |
| Commits | **5** fix + **4** docs |
| Tests | **3324/3324 passing, 221/221 files** |
| TypeScript | Clean (0 errors) |
| Bundle parity | 0 missing keys |


## 2026-07-02 — i18n Migration & Test Fixes

### Test Infrastructure Fixes
- **SettingsPage.test.tsx**: Wrapped with `AuthProvider` context + added `get_brand_settings` mock to fix pre-existing failures.
- **SetupWizard.test.tsx**: Corrected Launch button test to use `onLaunch` prop instead of `onSkip`.
- **CSS Extraction Tests**: Removed duplicate/dead CSS classes in `CartPanelActions.css`, added `url()` stripping in `extractClassSelectors` to fix `w3` false positive, added `externalClasses` support.
- **WorkspaceEntry.test.tsx**: Fixed unused `screen` import and `registerNavItem` import path (was pointing to `page-registry` instead of `menu-registry`).
- **Fluent missing-ID warnings**: Added 15 missing `setup-feature-*-label` IDs to `settings.ftl`.

### i18n Migration — Wrapped hardcoded aria-labels with `<Localized attrs>`

| Component | Labels wrapped |
|-----------|---------------|
| **SalesHistoryScreen.tsx** | 16 — date from/to, cashier select, table, actions th, pagination nav/prev/next/per-page, void overlay/close/reason, detail overlay/close/lines/refund-lines |
| **VoidOrdersScreen.tsx** | 3 — search input, status filter radiogroup, custom reason input |
| **PaymentModal.tsx** | 17 — dialog overlay, close button, currency label/select, exchange notice, receipt currency, other-input, customer-name (was fully hardcoded), tendered-input, quick-tender (with vars), exact button, QRIS button, split-evenly, split-add, split-other, split-amount, split-remove |
| **TaxConfigurationScreen.tsx** | 9 — tax rates table, category tax rates table, tax name label, edit/delete/cat-edit buttons, tax-rate modal, tax-type radiogroup, category-tax modal |
| **CustomerManagementScreen.tsx** | 5 — customers table, name/email/phone/notes inputs |
| **LoyaltyManagementScreen.tsx** | 8 — accounts table, actions th, transactions table, 5 tier form inputs |

### FTL Files Modified
- `sales.ftl` — added 21 new IDs for sales history + void orders + payment modal
- `settings.ftl` — added 15 setup-feature-label IDs
- `tax.ftl` — added 3 new IDs (table-aria, cat-table-aria, field-name-aria)
- `customers.ftl` — added 5 new IDs (table-aria, 4 field aria)
- `loyalty.ftl` — modified `loyalty-table-actions` to `.aria-label` format + added 7 new tier/table IDs

## 2026-07-02 — White-Label Theming Improvements

### Changes Made

1. **BrandContext created** (`ui/src/contexts/BrandContext.tsx`) — New React context providing brand/white-label settings and a `refreshBrandSettings()` function to the entire app tree. Loads from backend on mount.

2. **ThemeProvider cleaned up** — Removed `BrandInfo` interface, `brand`/`updateBrand` state (now handled by BrandContext), and the direct `getBrandSettings` effect. Now uses `useBrand()` from BrandContext to reactively apply the accent palette whenever `primary_colour` changes.

3. **AppLayout sidebar header** — Replaced hardcoded "OZ-POS" with dynamic brand logo (if set) + store name (fallback to "OZ-POS"). Also sets `document.title` reactively to the store name.

4. **AppearanceSettings** — Replaced `useTheme().updateBrand` with `useBrand().refreshBrandSettings()`. `handlePickLogo` now also refreshes brand settings immediately so the sidebar shows the new logo without waiting for "Save".

5. **AppLayout.css** — Added `.app-sidebar-logo-img` (32×32, object-fit contain) and collapsed variant (28×28) styles.

6. **App.tsx** — Wrapped app with `<BrandProvider>` above `<ThemeProvider>`.

### TypeScript
Clean (0 errors).

## 2026-07-02 — Modal Exit Animations

**Problem:** Hold cart, held carts, and shift modals had entrance animations but snapped out on close — no exit animation.

**Solution:** Created reusable `useAnimatedModal` hook that manages entering/exiting phases. When `show` becomes `false`, the modal stays mounted for 200ms with `exiting=true` before unmounting, allowing CSS exit animations to play.

**Changes made:**
- NEW `ui/src/hooks/useAnimatedModal.ts` — animation phase management hook
- `PosScreen.css` — added `@keyframes pos-overlay-out` (fade) + `pos-modal-out` (fade+translate), `.pos-overlay-exit`/`.pos-modal-exit` classes
- `ShiftManagementScreen.css` — added identical shift-prefixed exit keyframes + classes
- `PosScreen.tsx` — applied hook to 5 modals (hold cart, held carts, close shift, shift summary, open shift)
- `ShiftManagementScreen.tsx` — applied hook to 5 modals (open, payout, close, closed summary, detail)
- Reduced-motion overrides extended to cover exit classes

**Null-safety:** Used IIFE pattern (`{mX && (() => { const s = nullable!; return ( ... ); })()}`) where hook conditions couldn't be tracked across the hook boundary.

### Bugs Fixed During Migration
- Nested `<label>` bug in PaymentModal currency selector (invalid HTML)
- `key` prop on quick-tender buttons moved to outermost `<Localized>` component
- Stale `l10n.getString()` call on loyalty `<th>` after converting ftl to attribute format
- Missing `</Localized>` closing tags for void and detail overlay wrappers

### Test Results
- **TypeScript**: Clean (0 errors)
- **Tests**: 261 passed / 15 failed (down from 31 failing pre-migration — all remaining failures are pre-existing FSI/PDI marker issues and structural WorkspaceEntry module-not-found)
