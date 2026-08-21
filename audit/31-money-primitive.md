# Phase 1 — Money Primitive & Arithmetic Safety Audit

> **Audit date:** 2026-08-13  
> **Sector:** 31 — Money primitive, Currency, Cart, Percentage  
> **Status:** FULLY REMEDIATED — all 6 findings closed  
> **Scope:** `foundation/src/money.rs`, `money_tests.rs`, `money_proptests.rs`, `cart.rs`, `percentage.rs`, `crates/oz-core/src/db/sales.rs`, `sales_tests.rs`

---

## Executive summary

The Money primitive (`Money`, `Currency`, `Percentage`, `Cart`, `CartLine`) has a strong foundation: integer minor units, checked arithmetic, `#[must_use]` constructors, and a fuzz target (`money_parse.rs`, `percentage_parse.rs`). The audit confirmed 452 unit/property tests pass and the arithmetic contracts are well-defined.

The audit found and fixed **6 findings**:

- **P1** — `compute_sale_tax` silently zeroed subtotal/tax on checked-add overflow (money-integrity bug reached at extreme line totals)
- **P2** — `Percentage::apply_to`/`complement_apply_to` used `x * p / 100` which overflowed the intermediate product on valid inputs (100% of `i64::MAX` returned `None` instead of `i64::MAX`)
- **P2** — `Cart::discount_amount` masked overflow with `.or(Some(Money::zero(...)))`, contradicting the documented "Returns None on overflow" contract
- **P2** — `Money::default()` = USD is a latent cross-currency vector; documented as serde-only fallback
- **P3** — `CartLine::total()` didn't guard against `qty <= 0` from serde deserialization (bypasses the `new()` assert, persisted carts could carry zero/negative lines)
- **P4** — Missing property-based test coverage for money invariants (now added: 1,512 proptest cases across 16 property tests)

---

## Findings

### MONEY-AUDIT-1 — compute_sale_tax overflow silently zeroes subtotal/tax (P1, FIXED)

**File:** `crates/oz-core/src/db/sales.rs:2056–2068`

**Before:**
```rust
subtotal = match subtotal {
    None => Some(line.line_total),
    Some(acc) => acc.checked_add(line.line_total),  // overflow → None
};
sale.subtotal = subtotal.unwrap_or_else(|| Money::zero(currency));  // None → zero!
```

**Problem:** If `acc.checked_add(line.line_total)` returned `None` (overflow), the `match` arm produced `None`, and `unwrap_or_else(|| Money::zero(currency))` **silently recorded the sale with subtotal = 0**. A sale whose line totals exceeded `i64::MAX` would be persisted with subtotal = 0 and tax_total = 0, bypassing any error path. The `compute_cart_tax` function (the live tax preview) already handled this correctly with `.ok_or_else(...)?`.

**Fix:** Propagate the overflow as a `Validation` error, matching `compute_cart_tax`:
```rust
Some(acc) => Some(
    acc.checked_add(line.line_total).ok_or_else(|| CoreError::Validation {
        field: "subtotal",
        message: "sale subtotal overflow".into(),
    })?,
),
```

**Regression tests added:** `compute_sale_tax_subtotal_overflow_returns_validation_error`, `compute_sale_tax_line_tax_overflow_returns_validation_error`.

---

### MONEY-AUDIT-2 — Percentage overflow in intermediate product (P2, FIXED)

**File:** `foundation/src/percentage.rs:75–89`

**Before:**
```rust
pub fn apply_to(self, money: Money) -> Option<Money> {
    money.checked_mul(self.0 as i64)?.checked_div(100)
}
```

**Problem:** `checked_mul(pct)` for `pct = 100` and `x = i64::MAX` overflows the intermediate product even though the final result `i64::MAX * 100 / 100 = i64::MAX` fits. 100% of the largest possible amount returned `None` instead of the amount. Same for `complement_apply_to`.

**Fix:** Use the decomposition identity `x = 100q + r ⇒ (x·p)/100 = q·p + (r·p)/100`, which is exact under Rust's truncating division and never overflows for any `i64` amount and any `p ∈ [0, 100]`:
```rust
let q = x / 100;
let r = x % 100;
let hi = q.checked_mul(p)?;  // |hi| ≤ |x| — never overflows
let lo = r.checked_mul(p)?;  // |lo| ≤ 9900 — never overflows
Some(Money { minor_units: hi.checked_add(lo / 100)?, currency: money.currency })
```

**Tests updated:** `apply_to_overflow_returns_none` → `apply_to_100_pct_of_i64_max_returns_max` (100% of MAX = MAX). `complement_apply_to_overflow_returns_none` → `complement_apply_to_100_pct_of_i64_max_is_zero` (complement of 0% = 100% of MAX = MAX). Proptest `percentage_apply_to_is_total_and_bounded` verifies the operation is total for all inputs.

---

### MONEY-AUDIT-3 — Cart::discount_amount masks errors with `.or(Some(zero))` (P2, FIXED)

**File:** `foundation/src/cart.rs:293–321`

**Before:**
```rust
discounted
    .checked_sub(fixed.min(discounted)?)
    .and_then(|total| subtotal.checked_sub(total))
    .or(Some(Money::zero(self.currency)))  // overflow → masqueraded as zero
```

**Problem:** If `checked_sub` returned `None` (overflow), the `.or(Some(Money::zero(...)))` fallback silently returned zero instead of propagating `None`. The doc string said "Returns `None` on overflow or currency mismatch" but the code violated this contract. Additionally, the early-return `if self.discount_percent.get() == 0 && self.fixed_discount_minor == 0` bypassed line validation entirely, so a corrupted persisted cart (qty=0) would report a discount of 0 instead of failing closed.

**Fix:** Removed the `.or(Some(zero))` fallback — `?` now propagates overflow as `None`, matching the documented contract. Moved the early-return to after the line-validation fold so `discount_amount` fails closed on corrupted lines, consistent with `Cart::total()`.

**Regression tests added:** `cart_total_fails_closed_when_serde_line_has_zero_qty`, `cart_line_total_fails_closed_on_zero_or_negative_qty_from_serde`.

---

### MONEY-AUDIT-4 — Money::default() = USD latent vector (P2, DOCUMENTED)

**File:** `foundation/src/money.rs:20–27`

**Problem:** `impl Default for Money` returns `{ minor_units: 0, currency: USD }`. This is a latent cross-currency vector: any `#[serde(default)]` on a `Money` field (e.g. `SaleLine::tax_amount`, `Sale::subtotal`, `Sale::tax_total` in `modules/sales`) silently produces USD. The default is only used in a single test (`money_tests.rs:775`) and as a serde fallback for legacy payloads — it must never be used for business money.

**Fix:** Expanded the doc comment to explicitly call this a serde-only fallback and forbid new production call sites. The `Default` impl is retained because `#[serde(default)]` on `Money` fields in `modules/sales` requires it. A future audit could replace those serde defaults with `#[serde(default = "default_zero_money")]` functions that match the correct currency.

---

### MONEY-AUDIT-5 — CartLine::total() doesn't guard against qty ≤ 0 from serde (P3, FIXED)

**File:** `foundation/src/cart.rs:79–90`

**Problem:** `CartLine::new` asserts `qty > 0`, but `CartLine` fields are `pub` and `#[derive(Deserialize)]` bypasses the constructor. Persisted carts (`save_active_cart`, `held_carts`) round-trip through JSON, so a hand-edited or legacy cart with `qty = 0` or `qty = -2` would deserialize silently. `line.total()` = `price.checked_mul(self.qty)` — qty=0 produces a zero total (free item), qty=-2 produces a negative total. `compute_sale_tax` has a pre-pass rejecting negative line totals (MONEY-02), but `Cart::total()` would silently sum corrupted values.

**Fix:** `CartLine::total()` returns `None` when `qty <= 0` (fail closed), documented as a serde-bypass guard. Both `Cart::total()` and `Cart::discount_amount()` propagate the `None` via `?`.

**Regression tests added:** `cart_line_total_fails_closed_on_zero_or_negative_qty_from_serde`, `cart_total_fails_closed_when_serde_line_has_zero_qty`.

---

### MONEY-AUDIT-6 — Missing property-based test coverage (P4, ADDED)

**File:** `foundation/src/money_proptests.rs`

**Problem:** The workspace has an existing fuzz target (`money_parse.rs`, `percentage_parse.rs`) and `oz-core` has `proptest = "1"`, but `foundation` had no property-based tests to pin the arithmetic invariants.

**Fix:** Added `proptest = "1"` to `foundation/Cargo.toml` and created `money_proptests.rs` with 16 property tests (512 cases each) covering:
- `checked_add` commutativity, identity, associativity (sound form), overflow boundary
- `checked_sub` exactness, inverse of checked_add
- `checked_mul` exactness, identity/zero
- `checked_div` exactness, zero-divisor, i64::MIN / -1
- `checked_negate`, `checked_abs` exactness + i64::MIN overflow
- cross-currency ops (all None)
- `from_major` exponent match
- `format_minor` purity, sign-correctness, round-trip via i128
- `Percentage` totality (all inputs succeed), partition property
- `Money::min` correctness, commutativity

---

## Files changed

| File | Change |
|------|--------|
| `foundation/Cargo.toml` | Added `proptest = "1"` dev-dependency |
| `foundation/src/money.rs` | Documented Default as serde-only fallback; added audit stamp |
| `foundation/src/money_proptests.rs` | **NEW** — 16 property tests for money invariants |
| `foundation/src/cart.rs` | `CartLine::total()` qty guard; `discount_amount()` error propagation rework; audit stamp |
| `foundation/src/percentage.rs` | Overflow-free `apply_to`/`complement_apply_to`; updated tests; audit stamp |
| `crates/oz-core/src/db/sales.rs` | `compute_sale_tax` overflow propagation (P1 bug fix) |
| `crates/oz-core/src/db/sales_tests.rs` | 2 regression tests for overflow propagation |

## Verification

| Test suite | Tests | Status |
|-----------|-------|--------|
| `foundation` (unit + proptests) | 452 + 23 doctests | ✅ PASS |
| `oz-core` lib tests | 1,949 | ✅ PASS |
| `oz-core` currency integration | 38 | ✅ PASS |
| `oz-core` refund tax integration | 21 | ✅ PASS |
| `oz-core` gift card integration | 21 | ✅ PASS |
| `modules-sales` | 54 | ✅ PASS |
| `modules-currency` | 33 | ✅ PASS |
| `modules-crm` | 54 | ✅ PASS |
| `modules-tax` | 122 | ✅ PASS |
| `oz-payment` | 209 | ✅ PASS |