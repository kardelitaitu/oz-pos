# 0.0.9

> **Goal:** Test performance optimization — reduce vitest wall-clock time from ~14s baseline.

---

## 📊 Current State

| Layer | Test files | Total tests | Status |
|-------|-----------|-------------|--------|
| Rust | 26 crates | ~800+ | ✅ All passing |
| UI/Vitest | 152 files | 2,501 | ✅ All passing (0 failures, 0 skipped) |
| Vitest duration | — | — | **~14s** (baseline) |

---

## 🎯 Test Performance Optimization Checklist

Optimize the slowest test files one by one. After each file, re-run the full
suite and record the new wall-clock time at the bottom.

### 🏆 Tier 1 — Heavy hitters (> 3s)

- [ ] **DataManagementScreen.test.tsx** — 9,128ms / 55 tests (avg 165ms)
      Split into smaller test files by feature (Export, Import, Backup).
- [ ] **RetailPosScreen.test.tsx** — 7,358ms / 49 tests (avg 150ms)
      Reduce redundant re-renders; consolidate shared mock setup.
- [ ] **PaymentModal.test.tsx** — 6,313ms / 26 tests (avg 242ms)
      Extract slow keyboard/interaction tests to a separate file.
- [ ] **PriceOverrideKeyboardEdgeCases.test.tsx** — 5,747ms / 16 tests (avg 359ms)
      Profile async userEvent chains; look for unnecessary `act()` boundaries.
- [ ] **SetupWizard.test.tsx** — 4,891ms / 24 tests (avg 203ms)
      Cache mock data; reduce Fluent bundle re-creation per test.
- [ ] **CreatePinScreen.test.tsx** — 4,759ms / 12 tests (avg 396ms)
      High avg — investigate async timer dependencies.
- [ ] **PriceOverrideModal.test.tsx** — 4,119ms / 15 tests (avg 274ms)
      Consolidate shared render wrappers; reduce IPC mock thrash.
- [ ] **StockTransfersKeyboard.test.tsx** — 3,560ms / 8 tests (avg 444ms)
      Highest avg — profile keyboard event simulation overhead.
- [ ] **SettingsPage.test.tsx** — 3,386ms / 26 tests (avg 130ms)
- [ ] **PurchaseOrderForm.test.tsx** — 3,281ms / 17 tests (avg 192ms)

### 🥈 Tier 2 — Medium (2–3s)

- [ ] **ProductLookupScreen.test.tsx** — 2,926ms / 20 tests (avg 146ms)
- [ ] **SuppliersScreen.test.tsx** — 2,354ms / 16 tests (avg 147ms)
- [ ] **FastPINOverlay.test.tsx** — 2,309ms / 19 tests (avg 121ms)
- [ ] **WorkspaceContext.test.tsx** — 2,114ms / 21 tests (avg 100ms)
- [ ] **RefundModal.test.tsx** — 2,114ms / 15 tests (avg 140ms)

### 📝 Benchmark Log

| Date | Duration | Change | Notes |
|------|----------|--------|-------|
| baseline | 13.99s | — | Current config (min 8, max 28 threads) |

---

## 🚦 Safety Rules

- **Never delete a test assertion** — only reorganize or deduplicate.
- **Run `vitest run` after every UI change**, `cargo test -p <crate>` after every Rust change.
- **Commit each completed checklist section separately** with `[skip ci]` if only
  test code changes.
- **If a test breaks**, revert to the last working commit and re-approach more carefully.
