# 0.0.11

> **Goal:** Pick up remaining a11y items + theme verification + next major features.

---

## Current State

| Layer | Tests | Status |
|-------|-------|--------|
| Rust | ~1,188 | ✅ All passing |
| UI/Vitest | ~2,654 tests | ✅ All passing |
| ESLint | — | ✅ 0 errors, 0 warnings |
| TypeScript | — | ✅ 0 errors |

---

## 🎯 Remaining A11y Items

### 📖 Understandable

- [ ] **Error messages** — All form validation errors must be clear, specific, and suggest correction
- [ ] **Consistent navigation** — Verify same UI patterns (back buttons, save flows, modals) behave identically across all screens
- [ ] **Fluent strings** — Verify all user-visible strings use `@fluent/react` `Localized` component (no hardcoded English)
  - [x] CreatePinScreen.tsx — fixed 17 hardcoded strings

### 🏗️ Robust

- [ ] **ARIA roles** — Custom components must have correct role mappings (application, tablist, tab, switch, dialog, etc.)

### 📱 Mobile / Tablet

- [ ] **Touch spacing** — ≥ 8px gap between touchable elements
- [ ] **No horizontal scroll** — Verify all screens fit within tablet viewport (768px–1024px)

---

## 🎨 Theme Verification

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
