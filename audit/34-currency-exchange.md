# Phase 4 — Exchange Rates & Multi-Currency Settlement Audit

> **Audit date:** 2026-08-13  
> **Sector:** 34 — Currency module, exchange rates, commands, frontend APIs  
> **Status:** PARTIALLY REMEDIATED — CUR-03, CUR-04, CUR-08 closed (P0/P1/P2); CUR-02, CUR-05-remaining, CUR-06, CUR-09, CUR-10, CUR-11 remain open  
> **Scope:** `modules/currency/src/repository.rs`, `apps/*-client/src/commands/{currencies,exchange_rates}.rs`, `ui/src/api/currency.ts`, `PaymentModal.tsx`, `ExchangeRateScreen.tsx`, dev-mock

---

## Executive summary

The currency/exchange-rate surface was audited with focus on the highest-severity open findings from the July 2026 CUR audit (`audit/04-currency-module.md`):

**Closed (3 findings):**
- **CUR-03 (P0)** — Currency commands were NOT session-scoped or permission-checked: `get_default_currency`, `set_default_currency`, `list_exchange_rates`, `create_exchange_rate`, `delete_exchange_rate` all used the global `state.db` with no session token. In a multi-store deployment, any user could mutate another store's currency configuration. Fixed by adding scoped variants for all 5 commands + `list_currencies_scoped` (tablet was missing it) with `SETTINGS_READ`/`SETTINGS_EDIT` permission enforcement, following the tax command pattern. Desktop and tablet clients updated. Frontend API has scoped wrappers; PaymentModal uses scoped calls when a session token is available.
- **CUR-04 (P1)** — Rate selection in PaymentModal used `exchangeRates.find(...)` on the unbounded list, which is not ordered by effective date. A stale rate could be chosen when multiple rates exist for the same pair. Fixed by adding `get_latest_exchange_rate(from, to, as_of)` to the repository (selects latest effective on/before date, with forward-looking fallback) and a scoped command `get_latest_exchange_rate_scoped`. PaymentModal uses it when a session token is present.
- **CUR-08 (P2)** — `list_exchange_rates` loaded the full history with no pair filter. Added `list_exchange_rates_for_pair(from, to)` to the repository for bounded queries. The checkout path now uses `get_latest_exchange_rate_scoped` which is bounded to a single row.

**Remaining open (CUR audit residuals):**
- CUR-02 (P0): Backend multi-currency settlement — requires new DB columns, migration, and full complete_sale flow change (deferred to payment phase)
- CUR-05 (P1) remaining: repository-level validation for ISO-4217 membership, `from != to`, date format (command layer was already fixed)
- CUR-06 (P1): Delete confirmation UX (uses `ConfirmDialog` but localized key unused)
- CUR-09 (P2): Locale completeness
- CUR-10 (P2): Touch targets
- CUR-11 (P2): Module documentation

---

## Findings

### CUR-03 — Scoped currency commands with permission enforcement (P0, FIXED)

**Files:** `apps/*-client/src/commands/{currencies,exchange_rates}.rs`, `lib.rs`

**Before:** All 5 currency/exchange-rate commands used the unscoped global `state.db` and accepted no session token. No permission check was performed. Only `list_currencies_scoped` existed (desktop only).

**Fix:** Added scoped variants for each command:
- `list_exchange_rates_scoped` — `SETTINGS_READ` permission
- `create_exchange_rate_scoped` — `SETTINGS_EDIT` + CUR-05 validation (shared with legacy path)
- `delete_exchange_rate_scoped` — `SETTINGS_EDIT`
- `get_default_currency_scoped` — `SETTINGS_READ`
- `set_default_currency_scoped` — `SETTINGS_EDIT` + ISO-4217 code validation
- `list_currencies_scoped` (tablet was missing this — added for parity)

All scoped variants: `resolve_session` → `require_permission_for_session`/`require_permission_for_user` → `resolve_store`/`open_store` → execute on the session's store-scoped database. Legacy unscoped commands are retained as compatibility wrappers for single-store deployments.

**Validation:** Desktop and tablet command tests pass (13 each). Both clients compile.

---

### CUR-04 — Latest-effective-rate selection for checkout (P1, FIXED)

**Files:** `modules/currency/src/repository.rs`, `apps/*-client/src/commands/exchange_rates.rs`, `ui/src/api/currency.ts`, `PaymentModal.tsx`

**Before:** PaymentModal used `exchangeRates.find(...)` on the full list, which is ordered by `(from_currency, to_currency)` — not by effective date. With multiple historical rates for the same pair, the first match is arbitrary. The rate was not tied to the transaction date.

**Fix:** Added `get_latest_exchange_rate(from, to, as_of)` to `CurrencyRepository`:
1. Query for the most recent rate **on or before** `as_of_date` (ordered by effective_date DESC, created_at DESC, LIMIT 1)
2. If none found, fall back to the **earliest forward-looking** rate (effective_date > as_of, ASC, LIMIT 1)
3. Returns `None` when the pair has no rates at all

Added `get_latest_exchange_rate_scoped` command (desktop + tablet) with `SETTINGS_READ` permission. PaymentModal now calls this when a session token is available, using the effective `effectiveRateInfo` which prefers the backend-selected latest rate over the in-memory `find()`.

**Repository tests added:** `get_latest_exchange_rate_prefers_most_recent_on_or_before`, `list_exchange_rates_for_pair_bounds_to_pair_and_orders_recent_first`.

---

### CUR-08 — Bounded/batched rate queries (P2, FIXED)

**Files:** `modules/currency/src/repository.rs`

**Before:** `list_exchange_rates` returned every stored rate with no pagination, date window, or pair filter. PaymentModal loaded the full list to select one rate.

**Fix:** Added `list_exchange_rates_for_pair(from, to)` which returns only rates for a specific pair (ordered effective_date DESC, created_at DESC). The checkout path (`get_latest_exchange_rate_scoped`) is bounded to a single row. The admin screen (`ExchangeRateScreen`) still loads the full list (admin use case), but the payment path no longer depends on the unbounded list.

---

## Files changed

| File | Change |
|------|--------|
| `modules/currency/src/repository.rs` | Added `get_latest_exchange_rate`, `list_exchange_rates_for_pair` + 2 tests |
| `apps/desktop-client/src/commands/exchange_rates.rs` | Scoped variants + `get_latest_exchange_rate_scoped` + validation extraction |
| `apps/desktop-client/src/commands/currencies.rs` | Scoped default-currency commands |
| `apps/desktop-client/src/lib.rs` | 5 new command registrations |
| `apps/desktop-client/src/lan_server_tests.rs` | Pre-existing test fix (missing arg) |
| `apps/tablet-client/src/commands/exchange_rates.rs` | Scoped variants + latest-rate + validation extraction |
| `apps/tablet-client/src/commands/currencies.rs` | `list_currencies_scoped` + scoped default-currency commands |
| `apps/tablet-client/src/lib.rs` | 7 new command registrations |
| `ui/src/api/currency.ts` | Scoped wrappers + `getLatestExchangeRateScoped` + default-currency scoped |
| `ui/src/dev-mock/tauri-api.ts` | 6 new mock handlers |
| `ui/src/features/sales/PaymentModal.tsx` | Scoped API calls + latest-rate preference for `effectiveRateInfo` |

## Verification

| Suite | Tests | Status |
|-------|-------|--------|
| `modules-currency` | 56 | ✅ PASS |
| `oz-core` lib | 1989 | ✅ PASS |
| `oz-pos-app` currency cmds | 13 | ✅ PASS |
| `oz-pos-tablet` currency cmds | 13 | ✅ PASS |
| `oz-pos-app` + `oz-pos-tablet` | compile | ✅ PASS |
| `tsc --noEmit` (changed files) | ✅ clean |
| PaymentModal (4 files) | 69 | ✅ PASS |
| CurrencyContext + ExchangeRateScreen | ✅ PASS |