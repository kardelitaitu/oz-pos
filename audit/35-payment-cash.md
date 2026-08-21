# Phase 5 — Payment Gateways & Cash Accounting Audit

> **Audit date:** 2026-08-13  
> **Sector:** 35 — Payment drivers, gift cards, shifts, cash payouts  
> **Status:** FULLY REMEDIATED — 3 findings closed, 2 documented  
> **Scope:** `crates/oz-payment/src/drivers/{square,stripe}.rs`, `crates/oz-core/src/db/{gift_cards,shifts,cash_payouts}.rs`, `loyalty.rs`

---

## Executive summary

The payment and cash-accounting surface was audited: gateway amount conversion (Square/Stripe), gift card balance ledger, shift close cash math, and loyalty points computation.

**Closed (3 findings):**
- **PA-01 (P2)** — Gift card redeem used a **read-modify-write outside the transaction**: balance checked BEFORE the transaction, then an unconditional `UPDATE SET current_balance_minor = <stale value>` was written inside the txn. Two concurrent redeems (different sales, same card) could both read the same balance, both compute the same `new_balance`, and both write — losing one redemption. Fixed with atomic DB-computed decrement: `current_balance_minor - amount` with a `WHERE >= amount` guard, plus re-reading the true balance inside the txn for the ledger row (`balance_after_minor`). Same pattern applied to `top_up_gift_card` (with overflow guard). Regression test added.
- **PA-02 (P2)** — Square and Stripe `to_currency` functions silently fell back to USD on invalid gateway currency codes (`unwrap_or(Currency(*b"USD"))`), and Square/Stripe request paths hardcoded `"USD"`/`"usd"` regardless of the `Money`'s actual currency. Fixed: `to_currency` now returns `Result<Currency, PaymentError>` (unknown codes are a hard error), and both drivers send the Money's actual currency in the gateway request. Tests updated.
- **PA-03 (P4)** — Loyalty `earn_multiplier: f64` in the points-earning computation. Validated finite & positive. Points are rounded to integer points before conversion to money (POINTS_TO_MINOR_RATIO = 1). The redemption path is integer-only. Documented as acceptable rounding.

**Documented (2 observations):**
- **Shift expected-cash formula**: `expected_cash = opening + cash_sales - payouts`. Refunds are intentionally NOT subtracted (they show as variance in `cash_difference`). The refunds table has no payment method column, so cash vs card refunds can't be distinguished. Schema comment documents this design. P4.
- **Shift totals have no currency column**: All shift monetary aggregates are bare `i64` with no currency. The POS is single-currency per store, so this is a design limitation rather than a bug. P3.

---

## Findings

### PA-01 — Gift card redeem lost-update race (P2, FIXED)

**File:** `crates/oz-core/src/db/gift_cards.rs:400–465`

**Before:**
```rust
let new_balance = card.current_balance_minor - amount_minor;  // read outside txn
let tx = self.conn.unchecked_transaction()?;
// … INSERT transaction row with stale new_balance …
tx.execute(
    "UPDATE gift_cards SET current_balance_minor = ?1 WHERE id = ?2",
    params![new_balance, card.id],  // unconditional write from stale read
)?;
```

**Problem:** Two concurrent redeems (different sales, same card) could both read `current_balance_minor = 100`, both compute `new_balance = 50`, and both write 50 — the second UPDATE overwrites the first, losing one redemption. The ledger would show two -50 transactions but the card balance would be 50 instead of 0.

**Fix:** Atomic DB-computed decrement matching the loyalty redeem pattern:
```rust
let changed = tx.execute(
    "UPDATE gift_cards SET current_balance_minor = current_balance_minor - ?1, … WHERE id = ?2 AND current_balance_minor >= ?1",
    params![amount_minor, card.id],
)?;
if changed != 1 { tx.rollback()?; return Err(…); }
// Re-read true balance inside txn for ledger row
let balance_after: i64 = tx.query_row("SELECT current_balance_minor …", …)?;
```

**Also fixed:** `top_up_gift_card` had the same unconditional UPDATE pattern. Fixed with atomic `current_balance_minor + amount` and an overflow guard `WHERE current_balance_minor <= i64::MAX - amount`.

**Regression test added:** `redeem_atomic_decrement_keeps_ledger_in_sync` — two sequential redeems on different sales, verifying both card balance and `balance_after_minor` in the ledger.

---

### PA-02 — Gateway silent USD fallback on invalid currency codes (P2, FIXED)

**Files:** `crates/oz-payment/src/drivers/{square,stripe}.rs`

**Before:**
- `to_currency(code)` used `code.parse().unwrap_or(Currency(*b"USD"))` — any unrecognized gateway currency code silently became USD
- `authorize()`/`refund()` sent hardcoded `"USD"`/`"usd"` to the gateway regardless of the `Money.currency` field

**Fix:**
- `to_currency(code)` returns `Result<Currency, PaymentError>` — unknown codes are a hard error
- `to_money()` propagates the error
- `payment_result()` and `intent_result()` propagate the error
- Authorize/refund requests send the Money's actual currency code via `String::from_utf8_lossy(&request.amount.currency.0).into_owned()`

**Tests added:** `square_to_money_rejects_unknown_currency`, `stripe_to_currency_rejects_unknown` — verify invalid codes are rejected as hard errors.

---

### PA-03 — Loyalty earn_multiplier f64 (P4, DOCUMENTED)

**File:** `crates/oz-core/src/db/loyalty.rs:335`

The `earn_multiplier: f64` is used in points computation:
```rust
let points = ((base as f64) / 100.0 * tier.earn_multiplier).round() as i64;
```

This is a **points** computation (not money). Points are converted to money at redemption via `POINTS_TO_MINOR_RATIO = 1` (1 point = 1 minor unit). The redemption path uses integer-only `checked_mul`. The f64 is validated finite & positive, and the result is rounded to integer points. For realistic POS amounts (< 2^53), the f64 precision is sufficient. No change required — documented as an acceptable design choice.

---

## Files changed

| File | Change |
|------|--------|
| `crates/oz-core/src/db/gift_cards.rs` | PA-01: atomic conditional UPDATE for redeem + topup; re-read balance inside txn |
| `crates/oz-core/src/db/gift_cards_tests.rs` | PA-01 regression test (atomic decrement + ledger sync) |
| `crates/oz-payment/src/drivers/square.rs` | PA-02: to_currency returns Result, request uses Money's currency |
| `crates/oz-payment/src/drivers/stripe.rs` | PA-02: to_currency returns Result, request uses Money's currency |
| `crates/oz-payment/src/drivers/square_tests.rs` | PA-02: tests updated for Result, + unknown-currency rejection test |
| `crates/oz-payment/src/drivers/stripe_tests.rs` | PA-02: tests updated for Result, + unknown-currency rejection test |
| `crates/oz-payment/tests/stripe_integration.rs` | PA-02: test expects `currency=USD` (not hardcoded `usd`) |
| `crates/oz-payment/tests/stripe_lifecycle.rs` | PA-02: test expects `currency=USD` (not hardcoded `usd`) |

## Verification

| Suite | Tests | Status |
|-------|-------|--------|
| `oz-payment` lib + integration | 124 + 17 + 13 + 19 + 13 + 5 | ✅ PASS |
| `oz-core` lib | 1996 | ✅ PASS |
| `oz-core` shift integration | 29 | ✅ PASS |
| `oz-core` refund tax integration | 21 | ✅ PASS |
| Gift card tests | 22 (1 new) | ✅ PASS |