//! Unit tests for `Percentage` — sibling test file per AGENTS.md
//! (tests must never live inside production `.rs` files; COR-33).
//!
//! Wired from `percentage.rs` via `#[cfg(test)] #[path = "percentage_tests.rs"]
//! mod tests;`.

use super::*;
use crate::Currency;

fn usd() -> Currency {
    "USD".parse().unwrap()
}

// ── Construction ─────────────────────────────────────────────

#[test]
fn new_zero() {
    let p = Percentage::new(0).unwrap();
    assert_eq!(p.get(), 0);
}

#[test]
fn new_hundred() {
    let p = Percentage::new(100).unwrap();
    assert_eq!(p.get(), 100);
}

#[test]
fn new_mid_range() {
    let p = Percentage::new(37).unwrap();
    assert_eq!(p.get(), 37);
}

#[test]
fn new_above_100_returns_none() {
    assert!(Percentage::new(101).is_none());
}

#[test]
fn new_255_returns_none() {
    assert!(Percentage::new(255).is_none());
}

// ── apply_to ─────────────────────────────────────────────────

#[test]
fn apply_to_zero_pct() {
    let m = Money::from_major(50, usd()).unwrap();
    let result = Percentage::new(0).unwrap().apply_to(m).unwrap();
    assert_eq!(result.minor_units, 0);
}

#[test]
fn apply_to_100_pct() {
    let m = Money::from_major(50, usd()).unwrap();
    let result = Percentage::new(100).unwrap().apply_to(m).unwrap();
    assert_eq!(result.minor_units, 5000);
}

#[test]
fn apply_to_10_pct() {
    let m = Money::from_major(20, usd()).unwrap();
    let result = Percentage::new(10).unwrap().apply_to(m).unwrap();
    assert_eq!(result.minor_units, 200); // 10% of 2000¢
}

#[test]
fn apply_to_truncates_fractional() {
    let m = Money {
        minor_units: 100,
        currency: usd(),
    };
    // 33% of 100¢ = 33¢ (integer division truncates)
    let result = Percentage::new(33).unwrap().apply_to(m).unwrap();
    assert_eq!(result.minor_units, 33);
}

#[test]
fn apply_to_preserves_currency() {
    let jpy: Currency = "JPY".parse().unwrap();
    let m = Money::from_major(100, jpy).unwrap();
    let result = Percentage::new(50).unwrap().apply_to(m).unwrap();
    assert_eq!(result.currency, jpy);
}

#[test]
fn apply_to_100_pct_of_i64_max_returns_max() {
    // Overflow-free decomposition: 100% of i64::MAX is i64::MAX.
    // The old checked_mul(100) implementation spuriously returned None
    // because the intermediate product overflowed even though the
    // result fits.
    let m = Money {
        minor_units: i64::MAX,
        currency: usd(),
    };
    let result = Percentage::new(100).unwrap().apply_to(m).unwrap();
    assert_eq!(result.minor_units, i64::MAX);
}

#[test]
fn apply_to_max_div_100_succeeds() {
    // i64::MAX / 100 * 100 → just under i64::MAX, no overflow
    let m = Money {
        minor_units: i64::MAX / 100,
        currency: usd(),
    };
    let result = Percentage::new(100).unwrap().apply_to(m).unwrap();
    assert_eq!(result.minor_units, (i64::MAX / 100) * 100 / 100);
}

#[test]
fn apply_to_i64_max_with_1_pct_succeeds() {
    // i64::MAX * 1 does not overflow, then / 100 fits
    let m = Money {
        minor_units: i64::MAX,
        currency: usd(),
    };
    let result = Percentage::new(1).unwrap().apply_to(m).unwrap();
    assert_eq!(result.minor_units, i64::MAX / 100);
}

#[test]
fn apply_to_zero_amount() {
    let m = Money {
        minor_units: 0,
        currency: usd(),
    };
    let result = Percentage::new(50).unwrap().apply_to(m).unwrap();
    assert_eq!(result.minor_units, 0);
}

// ── complement_apply_to ─────────────────────────────────────

#[test]
fn complement_apply_to_zero_pct() {
    let m = Money::from_major(50, usd()).unwrap();
    let result = Percentage::new(0).unwrap().complement_apply_to(m).unwrap();
    assert_eq!(result.minor_units, 5000); // 100% of 5000¢
}

#[test]
fn complement_apply_to_100_pct() {
    let m = Money::from_major(50, usd()).unwrap();
    let result = Percentage::new(100)
        .unwrap()
        .complement_apply_to(m)
        .unwrap();
    assert_eq!(result.minor_units, 0); // 0% of 5000¢
}

#[test]
fn complement_apply_to_10_pct() {
    let m = Money::from_major(20, usd()).unwrap();
    let result = Percentage::new(10).unwrap().complement_apply_to(m).unwrap();
    assert_eq!(result.minor_units, 1800); // 90% of 2000¢
}

#[test]
fn complement_apply_to_100_pct_of_i64_max_is_zero() {
    // 0% complement = 100% of i64::MAX = i64::MAX, then 100% of that
    // must be zero. The old checked_mul(100) implementation would have
    // failed on the intermediate product; the decomposition handles it.
    let m = Money {
        minor_units: i64::MAX,
        currency: usd(),
    };
    let result = Percentage::new(0).unwrap().complement_apply_to(m).unwrap();
    assert_eq!(result.minor_units, i64::MAX);
    let zero = Percentage::new(100)
        .unwrap()
        .complement_apply_to(result)
        .unwrap();
    assert_eq!(zero.minor_units, 0);
}

#[test]
fn complement_apply_to_near_overflow_succeeds() {
    // 1% complement = 99% of i64::MAX/99 → safely within i64::MAX
    let m = Money {
        minor_units: i64::MAX / 99,
        currency: usd(),
    };
    let result = Percentage::new(1).unwrap().complement_apply_to(m).unwrap();
    // 99% of (i64::MAX / 99) ≈ i64::MAX * 99 / 99 / 100 (truncated)
    assert_eq!(result.minor_units, (i64::MAX / 99) * 99 / 100);
}

#[test]
fn complement_apply_to_zero_amount() {
    let m = Money {
        minor_units: 0,
        currency: usd(),
    };
    let result = Percentage::new(50).unwrap().complement_apply_to(m).unwrap();
    assert_eq!(result.minor_units, 0);
}

// ── zero / hundred / default ─────────────────────────────────

#[test]
fn zero_helper() {
    assert_eq!(Percentage::zero(), Percentage::new(0).unwrap());
}

#[test]
fn hundred_helper() {
    assert_eq!(Percentage::hundred(), Percentage::new(100).unwrap());
}

#[test]
fn default_is_zero() {
    assert_eq!(Percentage::default(), Percentage::zero());
}

// ── Display ──────────────────────────────────────────────────

#[test]
fn display_formats_with_percent_sign() {
    assert_eq!(Percentage::new(10).unwrap().to_string(), "10%");
    assert_eq!(Percentage::new(0).unwrap().to_string(), "0%");
    assert_eq!(Percentage::new(100).unwrap().to_string(), "100%");
}

// ── Serde ────────────────────────────────────────────────────

#[test]
fn serde_roundtrip() {
    let p = Percentage::new(42).unwrap();
    let json = serde_json::to_string(&p).unwrap();
    let back: Percentage = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn serde_happy_path() {
    let json = "75";
    let p: Percentage = serde_json::from_str(json).unwrap();
    assert_eq!(p.get(), 75);
}

#[test]
fn serde_rejects_above_100() {
    let result: Result<Percentage, _> = serde_json::from_str("101");
    assert!(result.is_err());
}
