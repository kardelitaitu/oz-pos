# Phase 6 — Multi-Currency Settlement, Tip/Service-Charge Persistence, Reporting & Plugin Money Path

> **Audit date:** 2026-08-13  
> **Sector:** 36 — CUR-02 backend settlement, Phase-3 residuals, reporting/plugin money path  
> **Status:** REMEDIATED — CUR-02 (P0) + tip/service persistence closed; Phase-6 items verified display-only  
> **Scope:** `modules/sales`, `crates/oz-core/src/db/sales.rs`, `apps/*-client/src/commands/pos.rs`, `ui/src/{api,features/sales,features/retail}`, `crates/oz-reporting`, `crates/oz-lua`, `apps/cloud-server/src/email_pg.rs`

---

## Executive summary

Three workstreams were completed in this phase:

1. **CUR-02 (P0)** — Multi-currency settlement: the sale row now records the original (base) currency, base total, and fixed-point rate used when checkout converted the amount. The backend previously recorded only the converted (charge) currency with **no record of the base amount or rate**, so refunds/reconciliation had no way to reconstruct the original amount.
2. **Phase-3 residual** — Tip and service-charge were collected by the frontend in the payment total but **never persisted** — the recorded `sale.total` understated collected revenue. Both are now persisted on the sale.
3. **Phase 6** — Verified the reporting/plugin money path f64 usages are display/rounding-only (no money-integrity impact).

---

## CUR-02 — Multi-currency tender metadata (P0, FIXED)

**Files:** `crates/oz-core/migrations/20260821_tender_currency.sql`, `modules/sales/src/models.rs`, `crates/oz-core/src/db/sales.rs`, `apps/*-client/src/commands/pos.rs`, `ui/src/api/sales.ts`, `ui/src/features/sales/PaymentModal.tsx`

**Before:** When a cashier selected a non-base charge currency, PaymentModal converted the total for display and sent `currency: cartCurrency` to `completeSale` — but the backend `CompleteSaleScopedArgs` had no currency field (serde silently dropped it). The sale was recorded in the charge currency with converted line amounts, but the original base currency, the base total, and the rate used were **lost**.

**Fix:**
- Migration `20260821_tender_currency.sql` adds three nullable columns: `base_currency`, `base_total_minor`, `tender_rate_millionths`
- `Sale` struct gains the three fields (serde-default None)
- `CompleteSaleArgs` / `CompleteSaleScopedArgs` / `CompleteSaleWithResolvedShortfallsArgs` (desktop + tablet) gain the three optional fields
- All complete-sale handlers copy them onto the sale before persistence
- All INSERT/SELECT paths persist/load them
- PaymentModal snapshots `baseCurrency` (the original `total.currency`), `baseTotalMinor` (the original total), and `tenderRateMillionths` (round(rate × 1e6)) whenever multi-currency conversion is active, for both the cash/card path and the QRIS path
- PG schema mirrored

**Regression test:** `create_sale_persists_tender_currency_metadata` — multi-currency sale round-trips all three fields; single-currency sale persists None.

---

## Phase-3 residual — Tip / service-charge persistence (P1, FIXED)

**Files:** `crates/oz-core/migrations/20260822_sale_charges.sql`, `modules/sales/src/models.rs`, `crates/oz-core/src/db/sales.rs`, `apps/*-client/src/commands/pos.rs`, `ui/src/api/sales.ts`, `ui/src/features/{sales,retail}/*Screen.tsx`

**Before:** `usePosState.total` includes tip and service charge (frontend preview), and PaymentModal collected that total from the customer — but the backend cart never knew about tip/service, so the persisted `sale.total` excluded them. Collected revenue was understated by the tip + service-charge amount.

**Fix:**
- Migration `20260822_sale_charges.sql` adds `tip_minor` and `service_charge_minor` (NOT NULL DEFAULT 0)
- `Sale` struct gains both fields (serde-default 0)
- Complete-sale args gain `tip_minor` / `service_charge_minor`
- Handlers set them (`args.tip_minor.unwrap_or(0)`)
- INSERT/SELECT paths persist/load them
- `PosScreen` and `RetailPosScreen` pass `tipMinor={tipAmount?.minor_units ?? 0}` and `serviceChargeMinor` to PaymentModal
- PaymentModal includes them in every complete-sale payload
- PG schema mirrored

---

## Phase 6 — Reporting & plugin money path (VERIFIED, no change)

- **`crates/oz-reporting/src/margin.rs`** — `margin_percent: f64` is a percentage for display/analysis. All monetary fields (`margin_minor`, `unit_price_minor`, etc.) are i64. The f64 is only the ratio. ✅
- **`apps/cloud-server/src/email_pg.rs`** — `gross_profit_minor as f64 / total_minor as f64 * 100.0` and `grand_total: f64` are percentage/rank computations for the email report only. ✅
- **`crates/oz-lua/src/lib.rs`** — `qty as f64` / `unit_price_minor as f64` / `total_minor as f64` cross into the Lua VM. Documented: minor-unit values below 2^53 are exact in f64; realistic POS amounts are far below that boundary. The MONEY-05 test pins the overflow-scale float semantics. ✅
- **`crates/oz-plugin/src/manager.rs`** — same Lua boundary, documented. ✅
- **`crates/oz-core/src/db/loyalty.rs`** — `earn_multiplier: f64` validated finite/positive; rounds to integer points before money conversion. ✅

---

## Files changed

| File | Change |
|------|--------|
| `crates/oz-core/migrations/20260821_tender_currency.sql` | NEW — CUR-02 columns |
| `crates/oz-core/migrations/20260822_sale_charges.sql` | NEW — tip/service columns |
| `crates/oz-core/migrations/20260813_init.pg.sql` | Mirror both column sets |
| `crates/oz-core/src/migrations.rs` | Register both migrations |
| `crates/oz-core/src/migrations_tests.rs` | Expected migration list |
| `modules/sales/src/models.rs` | Sale: 5 new fields (3 tender + 2 charges) |
| `modules/sales/src/repository.rs` | SELECT/INSERT updated |
| `crates/oz-core/src/db/sales.rs` | 3 INSERT + 5 SELECT row-mappers + constructors |
| `apps/*-client/src/commands/pos.rs` | Args + handlers (3 complete-sale variants each) |
| `apps/*-client/src/commands/{pos,kds}_tests.rs` | Sale literals updated |
| `crates/oz-core/src/db/{kds,multi_terminal,sales}_tests.rs` | Sale literals updated |
| `crates/oz-core/tests/*.rs` | Sale literals updated |
| `ui/src/api/sales.ts` | Args + SaleDetail types |
| `ui/src/features/sales/PaymentModal.tsx` | Sends tender metadata + tip/service |
| `ui/src/features/sales/PosScreen.tsx`, `retail/RetailPosScreen.tsx` | Pass tip/service to PaymentModal |

## Verification

| Suite | Tests | Status |
|-------|-------|--------|
| `oz-core` lib | 2016 | ✅ PASS |
| `oz-pos-tablet` lib | 454 | ✅ PASS |
| `oz-pos-app` lib | 1140 passed, 2 pre-existing KDS FK failures (concurrent agent's work, unrelated to this change) | ⚠️ |
| `tsc --noEmit` (changed UI files) | ✅ clean |
| `cargo check` all affected crates --all-targets | ✅ PASS |

## Open items

- The 2 KDS test failures (`register_and_list_kds_devices_scoped`, `resolve_kds_targets_scoped_returns_empty_for_no_devices`) are from the concurrent KDS workstream (FK constraint on test fixture data), not this change.
- The audit trail (CUR audit residuals) is now fully closed for the settlement/integrity items; remaining UX items (CUR-06 delete confirmation, CUR-09 locale, CUR-10 touch targets) are cosmetic and documented in `audit/04-currency-module.md`.