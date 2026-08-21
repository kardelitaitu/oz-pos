# Phase 3 — Tax & Rounding Consistency Audit

> **Audit date:** 2026-08-13  
> **Sector:** 33 — Tax computation, rounding, sale total integrity  
> **Status:** FULLY REMEDIATED — TAX-06 closed (P0), rounding equivalence verified, 5 regression tests added  
> **Scope:** `modules/tax/src/models.rs`, `crates/oz-core/src/db/sales.rs`, `sales_tests.rs`, `apps/*-client/src/commands/pos.rs`, `ui/src/api/tax.ts`, `ui/src/features/*/PosScreen.tsx`, `RetailPosScreen.tsx`, `RetailCartPanel.tsx`

---

## Executive summary

The tax surface was audited end-to-end: `RoundingMode` arithmetic, `compute_line_tax` (exclusive & inclusive), `compute_sale_tax` (per-line/rate accumulation), `compute_cart_tax` (live preview), rounding-mode settings persistence, frontend display and collection.

**Confirmed correct:**
- `RoundingMode::divide` HalfUp formula `(n + d÷2) ÷ d` is equivalent to the true "round half away from zero" for ALL non-negative inputs (verified by exhaustive brute-force over 400K cases: d=1..200 × n=0..2000)
- Rounding-mode settings round-trip correctly through serde, the typed settings layer, and Tauri IPC (snake_case wire names, unknown→HalfUp fallback)
- Inclusive tax back-calculation `base × bps ÷ (10_000 + bps)` is the correct formula for extracting embedded tax

**Critical P0 finding — exclusive tax never added to sale.total (TAX-06, FIXED):**
The default tax rate is exclusive (`is_inclusive: false`). `Sale::from_cart_with_user` set `total = cart.total()` (pre-tax, post-discount). `compute_sale_tax` updated `subtotal` and `tax_total` but **never updated `sale.total`**. The persisted `sales.total_minor` therefore excluded exclusive tax — the recorded revenue, EOD/shift reports, and payment splits all understated the collectible amount by the tax.

Additionally, the frontend displayed `cartTax` as a separate informational row ("PPN") but **never added it to the total** that the customer was asked to pay. For exclusive-tax stores, the cashier collected only the pre-tax amount.

**Other findings:**
- Payment split validation (`validate_payment_splits_cover_total`) validates against `sale.total` — after the fix, it now correctly enforces the tax-inclusive total
- The retail cart panel grand total display also excluded exclusive tax (fixed)
- Rounding-mode persistence and fallback corner cases are documented and tested

---

## Findings

### TAX-06 — Exclusive tax excluded from sale.total and payable total (P0, FIXED)

**Files:** `crates/oz-core/src/db/sales.rs:1953–2099`, `apps/*-client/src/commands/pos.rs` (both), `ui/src/api/tax.ts`, `ui/src/features/*/PosScreen.tsx`, `RetailPosScreen.tsx`, `RetailCartPanel.tsx`

**Before:**
1. `Sale::from_cart_with_user` → `total = cart.total()` (pre-tax discounted subtotal)
2. `compute_sale_tax` → sets `subtotal` (line sum) and `tax_total` but **never touches `sale.total`**
3. `create_sale` → persists `total_minor` = pre-tax (revenue understated)
4. Frontend `usePosState.total` → subtotal − discount + service + tip (no tax)
5. Frontend `computeCartTax` → returned just a number, no exclusivity info
6. PaymentModal → total prop = pre-tax, splits sent = pre-tax
7. For EXCLUSIVE tax (default!): **tax was computed, displayed, but NEVER collected**

**Fix (backend):**
- `compute_sale_tax` tracks `exclusive_tax` across all line/rate pairs. After the loop, if any exclusive tax was accumulated, it's added to `sale.total` via checked arithmetic: `sale.total = sale.total.checked_add(exclusive_tax)?`.
- `compute_cart_tax` now returns `CartTaxResult { tax_minor, has_exclusive }` instead of plain `Money`, so the frontend knows whether rates are exclusive.

**Fix (frontend IPC):**
- `compute_cart_tax_scoped` (desktop + tablet) returns `CartTaxResult` with `tax_minor` and `has_exclusive` (snake_case → camelCase via serde)
- `api/tax.ts` `computeCartTax` → returns `{ taxMinor, hasExclusive }`
- Dev mock updated

**Fix (frontend UI):**
- `PosScreen` and `RetailPosScreen`: when `hasExclusive && cartTax > 0`, the `total` passed to `PaymentModal` includes the tax (`total.minor_units + cartTax`)
- `RetailCartPanel`: the grand total display includes exclusive tax via `grandTotal(totals)` helper; `CartTotalsData.cartTaxExclusive` flag added

**Impact:** For exclusive-tax stores, the recorded sale total now matches the collectible amount, payment splits are validated against the correct total, and the frontend collects the right amount. For inclusive-tax stores, behavior is unchanged (tax already embedded in prices).

**Regression tests added:**
- `compute_tax_exclusive_adds_tax_to_sale_total` (total 700 + tax 70 = 770)
- `compute_tax_inclusive_does_not_inflate_sale_total` (total stays 700, tax 63 extracted)
- `compute_tax_no_rates_keeps_total_unchanged`
- `compute_cart_tax_reports_has_exclusive` (true for exclusive, false for no-rate)
- `compute_cart_tax_reports_has_exclusive_false_for_inclusive`

---

### TAX-05 — Rounding equivalence verified (P4, VERIFIED)

**File:** `modules/tax/tests/boundary_contract.rs`

The `HalfUp.divide(n, d)` implementation `(n + d/2) / d` was verified against the true "round half away from zero" formula `(2n + d) / (2d)` for ALL combinations of n ∈ [0, 2000] and d ∈ [1, 200] (400,200 test cases). The integer idiom is exact for both even and odd divisors — the earlier analysis confirmed no mismatch can occur because the fractional part `m/d` never falls in the ambiguous window for odd `d`. The brute-force test pins this conclusively.

---

### Open items (documented, deferred)

| Item | Sev | Description | Phase |
|------|-----|-------------|-------|
| Frontend tip/service-charge recording | P2 | Tip/service charge are displayed and collected (included in PaymentModal total) but not persisted to the backend (documented preview-only) | Phase 5 (Payment/Cash) |
| Discount-tax base order | P3 | Tax is computed on the pre-discount line total (`line.line_total`). Most jurisdictions compute tax on the discounted amount. Changing this would alter all golden test values | Phase 5 |
| Provide currency in `AddLineArgs` | P2 | The IPC `add_line` command silently re-stamps line currency as cart currency; product's own currency is lost | Phase 5 |

## Files changed

| File | Change |
|------|--------|
| `crates/oz-core/src/db/sales.rs` | TAX-06: exclusive tax added to sale.total; CartTaxResult struct; compute_cart_tax returns has_exclusive |
| `crates/oz-core/src/db/sales_tests.rs` | 5 new regression tests + `.minor_units` → `.tax_minor` updates |
| `apps/desktop-client/src/commands/pos.rs` | compute_cart_tax_scoped returns CartTaxResult |
| `apps/tablet-client/src/commands/pos.rs` | compute_cart_tax_scoped returns CartTaxResult |
| `ui/src/api/tax.ts` | computeCartTax returns { taxMinor, hasExclusive } |
| `ui/src/dev-mock/tauri-api.ts` | Updated mok return |
| `ui/src/features/sales/PosScreen.tsx` | cartTaxExclusive state; tax-adjusted total to PaymentModal |
| `ui/src/features/retail/RetailPosScreen.tsx` | Same fix + cartTaxExclusive flag in totals |
| `ui/src/features/retail/RetailCartPanel.tsx` | CartTotalsData.cartTaxExclusive; grandTotal() helper for display |
| `ui/src/\_\_tests\_\_/api-tax-contract.test.ts` | Updated mok returns |
| `ui/src/\_\_tests\_\_/RetailCartPanel.test.tsx` | cartTaxExclusive field |
| `modules/tax/tests/boundary_contract.rs` | Rounding equivalence exhaustive tests |

## Verification

| Suite | Tests | Status |
|-------|-------|--------|
| `oz-core` lib (sales) | 109 | ✅ PASS |
| `oz-core` refund tax integration | 21 | ✅ PASS |
| `modules-tax` unit + boundary | 49 + 11 + 1 | ✅ PASS |
| `oz-pos-app`, `oz-pos-tablet` | compile | ✅ PASS |
| `foundation` lib | 452 + 23 doctests | ✅ PASS |
| PaymentModal (4 files) | 84 | ✅ PASS |
| `api-tax-contract` | 9 | ✅ PASS |
| `RetailCartPanel` | 19 | ✅ PASS |
| `usePosState` | 29 | ✅ PASS |
| `tsc --noEmit` (changed files) | ✅ clean |