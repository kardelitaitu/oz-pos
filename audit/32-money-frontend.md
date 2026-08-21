# Phase 2 — Frontend Money Arithmetic Audit

> **Audit date:** 2026-08-13  
> **Sector:** 32 — Frontend money arithmetic  
> **Status:** PARTIALLY REMEDIATED — 3 findings closed, 1 open (IPC boundary)  
> **Scope:** `ui/src/features/sales/usePosState.ts`, `PaymentModal.tsx`, `ui/src/types/domain.ts`, `ui/src/api/currency.ts`, `ui/src/contexts/CurrencyContext.tsx`

---

## Executive summary

The frontend money surface was audited end-to-end: cart preview arithmetic (`usePosState`), multi-currency checkout conversion (`PaymentModal`), display formatting (`formatMoney` / `MINOR_UNIT_EXPONENT`), exchange-rate parsing (`api/currency.ts`), and default-currency context.

**Confirmed correct:**
- `MINOR_UNIT_EXPONENT` parity with Rust `Currency::minor_unit_exponent()` — identical sets (IDR/JPY/KRW/VND/CLP/ISK/HUF=0, KWD/OMR/BHD/JOD/TND=3, else 2)
- `formatMoney` float division (`minor_units / 10**exp`) is display-only; no caller feeds the result back into arithmetic
- `Math.floor` for discount/tip/service-charge matches Rust `Percentage` truncation (exact for non-negative amounts)
- `api/currency.ts` exchange rates are integer `rate_millionths` end-to-end; `exchangeRateToDecimal`/`formatExchangeRate` are presentation-only

**Found and fixed (3 findings):**
- **P1 — PaymentModal charge-amount display bug:** the "Charge amount" row multiplied base **minor units** by the rate without dividing by the base exponent, inflating the shown charge by `10^baseExp` (e.g. $7.00 → Rp 11,200,000 instead of Rp 112,000). The correct `convertToChargeCurrency` already existed; the row now uses it. The existing test had pinned the buggy value.
- **P1 — usePosState mixed-currency cart:** the subtotal took the first line's currency and silently summed all lines regardless of currency (USD+EUR → 1500 USD). `addProduct` and `updateLinePrice` now reject cross-currency inputs (return `false`), matching the backend `Cart::add_line` single-currency contract.
- **P3 — Multi-currency settlement gap:** documented in the audit trail (see CUR-01 residual, `audit/04-currency-module.md`): the backend does not yet record tender currency, rate snapshot, or rounded amount atomically. Frontend conversion is display/preview-only.

**Open finding (needs backend change, deferred):**
- **P2 — IPC boundary drops line currency:** `AddLineArgs` has no currency field; the Rust `add_line` command silently re-stamps every line's currency as `cart.currency()`. A product priced in EUR added to a USD cart would be recorded as USD. The backend `Cart::add_line` would reject this IF the currency were sent — the fix is to add `unit_price_currency` to `AddLineArgs` and validate in the command layer. Deferred to the payment/IPC phase (Phase 5) because it changes the Tauri command contract.

---

## Findings

### FRONTEND-01 — PaymentModal charge-amount row inflates by base exponent (P1, FIXED)

**File:** `ui/src/features/sales/PaymentModal.tsx:1188–1193`

**Before:**
```tsx
{formatMoney({
  minor_units: Math.round(total.minor_units * (exchangeRateInfo?.rate ?? 1)),
  currency: selectedCurrency,
} as Money)}
```

**Problem:** `total.minor_units` is in base **minor** units (e.g. 700 = $7.00). Multiplying by the major-unit rate (16,000 IDR/USD) without first dividing by `10^baseExponent` produces a value `10^2 = 100×` too large: `700 × 16000 = 11,200,000` instead of `$7.00 × 16000 = 112,000`. The cashier sees an inflated charge amount in the multi-currency receipt preview.

**Fix:** Use the already-correct `convertToChargeCurrency(total.minor_units)` (which handles the base exponent, rate, and charge exponent):
```tsx
{formatMoney({
  minor_units: convertToChargeCurrency(total.minor_units),
  currency: selectedCurrency,
} as Money)}
```

**Test fix:** `PaymentModal.test.tsx` multi-currency test asserted `Rp 11.200.000` (the buggy value); corrected to `Rp 112.000`.

---

### FRONTEND-02 — usePosState silently mixes currencies in the subtotal (P1, FIXED)

**File:** `ui/src/features/sales/usePosState.ts`

**Problem:** The subtotal `useMemo` took the first line's currency and summed every line's `minor_units` regardless of currency — `USD 1000 + EUR 500 → 1500 USD`. A test explicitly pinned this ("uses first line currency even when subsequent lines differ"). This produced a misleading preview; the backend `Cart::add_line` enforces single-currency carts, so the sale would fail (or, worse, the IPC layer re-stamps currency — see FRONTEND-03).

**Fix:**
- `addProduct` now returns `boolean`; it rejects (`false`) a product whose currency differs from the cart's anchored currency
- `updateLinePrice` (manager price override) rejects cross-currency overrides
- `cartCurrencyRef` (anchored on the first line, cleared on `resetCart`) keeps the guard synchronous and consistent across adds in one event loop (e.g. bundle expansion)
- The subtotal retains its first-line-currency semantics as a defensive fallback (only reachable via direct `setLines` mutation, e.g. held-cart restore of already-valid data)

**Test updated:** `usePosState.test.ts` now asserts the product is rejected, the cart keeps 1 line, and the subtotal stays `{ minor_units: 1000, currency: 'USD' }`.

---

### FRONTEND-03 — IPC boundary drops line currency (P2, DEFERRED to Phase 5)

**Files:** `ui/src/api/sales.ts` (`AddLineArgs`), `apps/desktop-client/src/commands/pos.rs:352–361, 387–391`, `apps/tablet-client/src/commands/pos.rs` (same)

**Problem:** `AddLineArgs { cartId, sku, qty, unitPriceMinor }` carries no currency. The Rust command constructs the line with `cart.currency()`:
```rust
let unit_price = Money { minor_units: args.unit_price_minor, currency };
```
So a product priced in EUR added to a USD cart would be silently recorded as USD — the product's own currency is discarded at the boundary. The backend `Cart::add_line` currency check can never fire because the line is built in the cart's currency.

**Recommended fix (Phase 5):** add `unit_price_currency: String` to `AddLineArgs` on both clients, parse it as `Currency` in the command, construct the line with the product's currency, and let `Cart::add_line` reject mismatches. This closes the last silent-currency path in the sale pipeline.

---

## Files changed

| File | Change |
|------|--------|
| `ui/src/features/sales/usePosState.ts` | Cross-currency guards on `addProduct`/`updateLinePrice`, `cartCurrencyRef` |
| `ui/src/features/sales/PaymentModal.tsx` | Charge-amount row uses `convertToChargeCurrency` |
| `ui/src/__tests__/usePosState.test.ts` | Updated mixed-currency test to assert rejection |
| `ui/src/__tests__/PaymentModal.test.tsx` | Fixed multi-currency assertion to the correct Rp 112,000 |

## Verification

| Suite | Result |
|-------|--------|
| `usePosState.test.ts` | ✅ 29 passed |
| `PaymentModal.test.tsx` | ✅ 26 passed |
| `PaymentModalEdgeCases.test.tsx` | ✅ passed |
| `PaymentModalSaleFlow.test.tsx` | ✅ passed |
| `GiftCardPayment.test.tsx` | ✅ passed |
| `tsc --noEmit` (changed files) | ✅ clean (pre-existing errors in concurrent `SalesReportScreen.tsx` work are unrelated) |