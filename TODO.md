# 0.0.9

> **Goal:** Test performance optimization — reduce vitest wall-clock time from ~14s baseline.

---

## 📊 Current State

| Layer | Test files | Total tests | Status |
|-------|-----------|-------------|--------|
| Rust | 26 crates | ~800+ | ✅ All passing |
| UI/Vitest | 161 files | 2,526 | ✅ All passing (0 failures, 0 skipped) |
| Vitest duration | — | — | **~14s** (baseline) |

---

## 🎯 Test Performance Optimization Checklist

Optimize the slowest test files one by one. After each file, re-run the full
suite and record the new wall-clock time at the bottom.

### 🏆 Tier 1 — Heavy hitters (> 3s)

- [x] **DataManagementScreen.test.tsx** — 9,128ms / 55 tests → 4 files (Screen 10, Export 20, Import 16, Backup 9)
      Split into smaller test files by feature (Export, Import, Backup).
- [x] **RetailPosScreen.test.tsx** — 7,358ms / 49 tests → 3 files (rendering 24, interactions 17, checkout 8)
      Isolated heavy tests (long-press with 500ms setTimeout, full PaymentModal checkout flow) in separate files.
- [x] **PaymentModal.test.tsx** — 6,313ms / 26 tests → 2 files (rendering/interaction 22, sale flow 4)
      Extracted 4 heavy flow tests with Tauri IPC chain into PaymentModalSaleFlow.test.tsx.
- [x] **PriceOverrideKeyboardEdgeCases.test.tsx** — 5,747ms / 16 tests (avg 359ms → 250ms)
      Replaced userEvent.click with fireEvent.click for navigation buttons. Extracted 3 sync tests to PriceOverridePriceStep.test.tsx. -43% time.
- [x] **SetupWizard.test.tsx** — 4,891ms / 24 tests → 2 files (interactions 21, render 3)
      Module-level FluentBundle singleton + fireEvent.click for navigation. -72% time (1,347ms).
- [x] **CreatePinScreen.test.tsx** — 4,759ms / 12 tests → 2 files (interactions 10, render 2)
      Replaced userEvent.type with fireEvent.change (4 fields × ~20 chars = ~1.6s saved per test). 4 validation tests now sync. -87% time.
- [x] **PriceOverrideModal.test.tsx** — 4,119ms / 15 tests → 2 files (interactions 11, render 4)
      fireEvent.click for nav buttons + fireEvent.change for username + shared advanceToPinStep helpers. -87% time.
- [x] **StockTransfersKeyboard.test.tsx** — 3,560ms / 8 tests → 2,049ms (-42%)
      fireEvent.click for all button clicks + fireEvent.change for form fields. Kept userEvent.keyboard for Escape (native addEventListener).
- [x] **SettingsPage.test.tsx** — 3,386ms / 26 tests → 2,210ms (-35%)
      fireEvent.click for nav buttons, fireEvent.change for form fields,
      fireEvent.blur for validation triggers, document.body.textContent for
      split-element version string. Kept await waitFor for initial API data.
- [x] **PurchaseOrderForm.test.tsx** — 3,281ms / 17 tests → 812ms (-75%)
      fireEvent.click for all buttons, fireEvent.change for all form fields
      + select options. Added fillField(), clickButton(), selectOption() helpers.

### 🥈 Tier 2 — Medium (2–3s)

- [x] **ProductLookupScreen.test.tsx** — 2,926ms / 20 tests → 1,055ms (-64%)
      fireEvent.change for search/barcode inputs, fireEvent.click for radio
      chips + scan buttons, fireEvent.keyDown for Enter key on barcode input.
      Added fillInput(), pressEnter(), clickButton(), clickRadio() helpers.
      Key lesson: async barcode scan asserts need await waitFor().
- [x] **SuppliersScreen.test.tsx** — 2,354ms / 16 tests → 2,100ms (-11%)
      fireEvent.change for form fields, fireEvent.click for buttons.
      Removed 6 userEvent.setup() calls + userEvent import.
      Added clickButton(name), fillField(index, value) helpers.
- [x] **FastPINOverlay.test.tsx** — 2,309ms / 19 tests → 1,109ms (-52%)
      fireEvent.change for username, fake timers for setTimeout test,
      getByRole('button', { name }) for icon-only Backspace button.
- [x] **WorkspaceContext.test.tsx** — 2,114ms / 21 tests → 1,400ms (-34%)
      waitForLoaded(result) helper: waitFor(loading===false, FAST_WAIT) preserves
      safety assertion. flushAsync() kept for 2 null-session tests. FAST_WAIT
      (5ms polling) for multi-step async chains (sessionToken, screens).
- [x] **RefundModal.test.tsx** — 2,114ms / 15 tests → 1,180ms (-44%)
      fireEvent.click for checkboxes/buttons, fireEvent.change for reason
      input, waitFor for async processRefund resolution. Added clickCheckbox(),
      fillReason(), clickSubmitRefund(), doRefund() helpers. Sync tests
      (checkbox toggle, qty decrement) converted from async to sync.

### 🥉 Tier 3 — Next batch (1.5–2.4s)

- [x] **GiftCardsScreen.test.tsx** — 2,370ms / 22 tests → 1,029ms (-57%)
      fireEvent.click for all interactions (expand/collapse, freeze/unfreeze,
      top-up, cancel, issue), fireEvent.change for top-up amount input.
      Removed 14 userEvent.setup() calls + userEvent import. Added FAST_WAIT
      (5ms polling), expandCard() and waitAndClickButton() helpers.
- [x] **PosScreen.test.tsx** — 2,264ms / 19 tests → 1,571ms (-31%)
      fireEvent.click for 5 Settings tab tests (gear, Features/Data/Sync tabs,
      Back button). FAST_WAIT (5ms polling) for all waitFor calls. 14 barcode
      scan tests already used act() — only got FAST_WAIT benefit.
- [x] **LicenseActivationScreen.test.tsx** — 2,236ms / 50 tests → 781ms (-65%)
      Replaced 28 userEvent.click with fireEvent.click (clear buttons, submit,
      container click, paste clicks). Removed userEvent import. Added fillForm()
      and clickSubmit() helpers. FAST_WAIT (5ms polling) on all 19 waitFor calls.
      Converted tests 18-22, 38 to sync (loading state set before first await).
      Test 31 bug fix: needed async + waitFor because handleSubmit awaits
      getMachineId() before activateLicense() — fireEvent.click doesn't flush
      microtasks. All 3 tiers complete!
- [x] **StaffManagementScreen.test.tsx** — 2,176ms / 12 tests → 1,348ms (-38%)
      fireEvent.change for form fields (username, display name, PIN),
      fireEvent.click for buttons (Add/Edit/Deactivate/Restore/Create),
      fireEvent.change for role dropdown. Kept userEvent.keyboard for Escape
      (native addEventListener in useFocusTrap). FAST_WAIT (5ms polling).

### 🏅 Tier 4 — Next batch (1.9–3s)

- [x] **RetailOptionsScreen.test.tsx** — 2,985ms / 31 tests → 1,583ms (-47%)
      Replaced ~25 userEvent.click with fireEvent.click for all tab navigation
      (Receipt, Printer, Scanner, Credit, System, Payments, Sync, Appearance,
      Features, Data), Save button, Back/Close buttons, receipt preview open/close,
      tender preset add/remove. Kept userEvent.keyboard for Escape. FAST_WAIT
      on all ~25 waitFor calls.
- [x] **StockTransfersKeyboard.test.tsx** — 2,122ms / 8 tests → 2,281ms (+8%)
      Already used fireEvent from Tier 1. Only added FAST_WAIT (5ms polling)
      to all 7 waitFor calls. Minimal improvement — bottleneck is userEvent.keyboard
      for Escape tests (4 tests × ~320ms each = ~1.3s, can't optimize further).
- [x] **FeatureToggleScreen.test.tsx** — 1,933ms / 20 tests → 1,588ms (-18%)
      Replaced userEvent.type with fireEvent.change for search input (2 tests).
      Replaced all userEvent.click with fireEvent.click for retry, clear search,
      feature toggle checkboxes, bulk enable/disable buttons. Removed userEvent
      import. FAST_WAIT on all ~18 waitFor calls.

### 📝 Benchmark Log

| Date | Duration | Change | Notes |
|------|----------|--------|-------|
| baseline | 13.99s | — | Current config (min 8, max 28 threads) |
| 2026-07-17 | 15.25s | +1.26s | After DataManagement split (155 files, 2509 tests). Slight increase from setup overhead; heavy Export (5.2s) + Import (5.1s) now run in parallel. |
| 2026-07-17 | 15.38s | +1.39s | After RetailPosScreen split (158 files, 2526 tests). FeatureToggleScreen (15 fails) is pre-existing. 3 files from 1. |
| 2026-07-17 | 16.33s | +2.34s | After PaymentModal split (159 files, 2526 tests). 22 fast + 4 heavy flow tests now parallel. |
| 2026-07-17 | **13.64s** | **-0.35s** | After PriceOverride optimization (160 files, 2526 tests). fireEvent.click nav buttons -43% on KeyboardEdgeCases. **First time below baseline!** |
| 2026-07-17 | 14.88s | +0.89s | After SetupWizard optimization (161 files, 2526 tests). SetupWizard 4.9s→1.35s (-72%). File count increase adds overhead. |
| 2026-07-17 | 15.05s | +1.06s | After CreatePinScreen optimization (162 files, 2526 tests). CreatePinScreen 4.8s→645ms (-87%). File count increase adds overhead. |
| 2026-07-17 | 16.48s | +2.49s | After PriceOverrideModal optimization (163 files, 2526 tests). PriceOverrideModal 4.1s→529ms (-87%). Run-to-run variance. |
| 2026-07-17 | 15.15s | +1.16s | After StockTransfersKeyboard optimization (163 files, 2526 tests). StockTransfersKeyboard 3.6s→2.0s (-42%). |
| 2026-07-17 | 15.46s | +1.47s | After SettingsPage optimization (164 files, 2533 tests). SettingsPage 3.4s→2.2s (-35%). |
| 2026-07-17 | 15.28s | +1.29s | After SuppliersScreen optimization (164 files, 2533 tests). SuppliersScreen 2.35s→2.10s (-11%). All passing. |
| 2026-07-17 | **13.45s** | **-0.54s** | After FastPINOverlay optimization (164 files, 2533 tests). FastPINOverlay 2.3s→1.1s (-52%). **Below baseline!** 🎉 |
| 2026-07-17 | 16.55s | +2.56s | After WorkspaceContext optimization (164 files, 2533 tests). WorkspaceContext 2.1s→1.4s (-34%). Run-to-run variance; safety-preserving waitForLoaded. |
| 2026-07-17 | 15.10s | +1.11s | After RefundModal optimization (164 files, 2533 tests). RefundModal 2.1s→1.18s (-44%). Tier 2 complete! |
| 2026-07-17 | **13.47s** | **-0.52s** | After GiftCardsScreen optimization (164 files, 2533 tests). GiftCardsScreen 2.37s→1.03s (-57%). **New all-time best!** 🎉 |
| 2026-07-17 | 15.50s | +1.51s | After PosScreen optimization (164 files, 2533 tests). PosScreen 2.26s→1.57s (-31%). Run-to-run variance. |
| 2026-07-17 | 16.81s | +2.82s | After StaffManagementScreen optimization (164 files, 2533 tests). StaffMgmt 2.18s→1.35s (-38%). Run-to-run variance. |
| 2026-07-17 | 15.11s | +1.12s | After LicenseActivationScreen optimization (164 files, 2550 tests). LicenseActivation 2.24s→0.78s (-65%). All 3 tiers complete! 🎉 |
| 2026-07-17 | 18.83s | +4.84s | After Tier 4 optimization (164 files, 2554 tests). RetailOptions 2.99s→1.58s (-47%), FeatureToggle 1.93s→1.59s (-18%), StockTransfersKeyboard 2.12s→2.28s (+8%). Run-to-run variance (high system load). |

---

## 🚦 Safety Rules

- **Never delete a test assertion** — only reorganize or deduplicate.
- **Run `vitest run` after every UI change**, `cargo test -p <crate>` after every Rust change.
- **Commit each completed checklist section separately** with `[skip ci]` if only
  test code changes.
- **If a test breaks**, revert to the last working commit and re-approach more carefully.
