# 0.0.10

> **Goal:** UI accessibility audit + theme consistency across all pages.

---

## Current State

| Layer | Tests | Status |
|-------|-------|--------|
| Rust | ~800+ | ✅ All passing |
| UI/Vitest | 164 files, ~2,561 tests | ✅ All passing |
| ESLint | — | ✅ 0 errors, 0 warnings |
| TypeScript | — | ✅ 0 errors |

---

## 🎯 UI Accessibility Audit

Build on the [existing a11y compliance doc](./docs/a11y.md) — close all gaps.

### ♿ Perceivable

- [x] **Colour contrast audit** — Run automated contrast check across all 3 themes (default glassmorphism, light, dark). WCAG 2.1 AA: 4.5:1 normal text, 3:1 large text.
  - [x] Build a `contrastCompliance.test.ts` that reads CSS tokens and verifies contrast ratios programmatically
  - [x] Check `--color-fg` on `--color-bg`, `--color-accent` on `--color-bg`, `--color-danger` on `--color-danger-bg`, etc.
  - [x] Fix any failing contrast pairs in `tokens.css`
- [x] **Image `alt` text audit** — Scan all `<img>` tags for missing `alt` or `aria-hidden="true"`
- [x] **Non-text content** — Verify SVG icons in shared components have `aria-hidden="true"`

### 🖱️ Operable

- [x] **Keyboard navigation** — Every interactive element must be reachable and operable via keyboard alone
  - [x] Audit custom components (toggle switches, slider selects, chip groups) for missing `tabIndex` / `role`
  - [x] Verify all modal dialogs trap focus and close on Escape
- [x] **Focus indicators** — Every `:focus-visible` must have a visible outline/ring (use `focusVisibleCompliance.test.ts`)
  - [x] Fix any remaining violations from the compliance scanner
- [x] **Touch targets (WCAG 2.5.5)** — Every interactive element must be ≥ 44×44px on touch devices
  - [x] Audit all 55+ feature screens for controls not covered by the shared `@media (pointer: coarse)` rule in `components.css`
  - [x] Add `min-height` / `min-width` overrides per-screen where the global rule doesn't apply
- [x] **`prefers-reduced-motion`** — All animations must be gated behind this media query
  - [x] Scan every CSS file for `@keyframes` / `animation` without a `prefers-reduced-motion: no-preference` wrapper
  - [x] Fix unguarded animations

### 📖 Understandable

- [ ] **Error messages** — All form validation errors must be clear, specific, and suggest correction
- [ ] **Consistent navigation** — Verify same UI patterns (back buttons, save flows, modals) behave identically across all screens
- [ ] **Fluent strings** — Verify all user-visible strings use `@fluent/react` `Localized` component (no hardcoded English)
  - [x] CreatePinScreen.tsx — fixed 17 hardcoded strings

### 🏗️ Robust

- [ ] **ARIA roles** — Custom components must have correct role mappings (application, tablist, tab, switch, dialog, etc.)
- [x] **Semantic HTML** — Use `<button>`, `<nav>`, `<table>`, `<dialog>` where appropriate instead of `<div>` + ARIA
  - [x] StaffLoginScreen.tsx — added keyboard handler to click-to-focus wrapper div

### 📱 Mobile / Tablet

- [ ] **Touch spacing** — ≥ 8px gap between touchable elements
- [ ] **No horizontal scroll** — Verify all screens fit within tablet viewport (768px–1024px)

---

## 🎨 Theme Consistency Audit

All screens must use **design tokens only** — never hardcoded colours, sizes, or shadows.

### 🔍 CSS Token Compliance Scan

- [x] **Build a scanner test** (`themeTokenCompliance.test.ts`) that:
  - [x] Reads every CSS file in `ui/src/features/*/` and `ui/src/frontend/*/`
  - [x] Flags any colour value (`#...`, `rgb(`, `rgba(`) that is **not** a `var(--)` reference
  - [x] Flags any hardcoded font-size, border-radius, box-shadow, or spacing that should use a token
  - [x] Exempt legitimate exceptions (gradient backgrounds, `transparent`, `currentColor`, `inset 0`)
- [ ] **Fix violations per screen** — Track progress in the table below
  - [x] AuditLogScreen.css
  - [x] CreatePinScreen.css
  - [x] LicenseActivationScreen.css
  - [x] InventoryReportScreen.css
  - [x] StaffLoginScreen.css
  - [x] CategoryManagementScreen.css
  - [x] DesignSystem.css
  - [x] GiftCardsScreen.css
  - [x] InventoryAdjustmentScreen.css
  - [x] StockCountDetail.css (~30 violations)
  - [x] StockCountForm.css (~15 violations)
  - [x] StockCountHistory.css (~25 violations)
  - [x] StockCountsScreen.css (~15 violations)
  - [x] KdsScreen.css (~30 violations)
  - [x] KioskScreen.css (~20 violations)
  - [x] LoyaltyManagementScreen.css (2 violations)
  - [x] ProductLookupScreen.css (4 violations)
  - [x] ProductManagementScreen.css (5 violations)
  - [x] PromotionManagementScreen.css (6 violations)
  - [x] PurchaseOrderForm.css (3 violations)
  - [x] PurchaseOrdersScreen.css (1 violation)
  - [x] SuppliersScreen.css (1 violation)
  - [ ] RestaurantMenu.css, RetailPosScreen.css, etc. (528 remaining)
  - [ ] sales/ CSS files (PaymentModal, RefundModal, etc.)
  - [ ] settings/ CSS files (SettingsPage, DataManagement, etc.)
  - [ ] frontend/ CSS files
  - [ ] components/ CSS files

### 🌗 Theme Switching

- [ ] **Verify all 3 themes render correctly** on every screen type:
  - [ ] Settings pages (tabbed layout, forms, toggles, selects)
  - [ ] POS screens (RetailPosScreen, RestaurantMenu, CartPanel)
  - [ ] Modals (PaymentModal, RefundModal, PriceOverrideModal, ConfirmDialog)
  - [ ] Auth screens (StaffLoginScreen, LicenseActivationScreen, CreatePinScreen)
  - [ ] Report screens (DashboardScreen, SalesReportScreen, InventoryReportScreen)
  - [ ] Management screens (Products, Categories, Staff, Customers, Tables, Terminals)
- [ ] **Smooth theme transitions** — Verify `html.is-theme-transitioning` class is applied during theme switches
- [ ] **Persist theme preference** — Verify theme choice survives page reload

### 🧪 Theme Regression Tests

- [ ] Write snapshot or visual tests for each theme on key screens
- [ ] Verify `--color-*` tokens resolve correctly in JSDOM (`getComputedStyle`)

---

## 📝 Audit Log

| Date | Screen / File | Issue | Status |
|------|-------------|-------|--------|
| — | — | — | ⬜ |

---

## 🚦 Safety Rules

- **Never delete a test assertion** — only reorganize or deduplicate.
- **Run `vitest run` after every UI change**, `cargo test` after every Rust change.
- **Commit each completed section separately** with descriptive messages.
- **If a test breaks**, revert to the last working commit and re-approach.
- **All new CSS must use `var(--token)` exclusively** — no hardcoded values.
- **All new animations must be gated behind `@media (prefers-reduced-motion: no-preference)`**.
