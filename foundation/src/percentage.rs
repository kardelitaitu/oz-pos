/*
last audited 25-07-26 by RSA-Agent (foundation slice A: percentage deep read)
crate: foundation (percentage.rs) | status: SAFE | lint: CLEAN
findings: exemplary — MONEY-AUDIT-2 overflow-free decomposition verified (100% of i64::MAX = i64::MAX, tested at edges); total arithmetic for any i64 x 0..=100; bounded u8 construction incl. serde path; COR-33 pattern note: ~240 lines of inline tests in production file (AGENTS.md wants sibling files)
next: move inline tests to sibling (COR-33 family) | perf: two extra mul/add
*/
//! Percentage value object — a bounded 0–100 integer type.
//!
//! Use this instead of raw `i64`/`u8` wherever a percentage discount,
//! tax rate part, or proportional amount is needed. Construction is
//! fallible so callers never deal with out-of-range values.
//!
//! # Example
//!
//! ```
//! use foundation::{Money, Currency, Percentage};
//!
//! let pct = Percentage::new(10).unwrap();
//! let usd: Currency = "USD".parse().unwrap();
//! let amount = Money::from_major(20, usd).unwrap();
//!
//! assert_eq!(pct.apply_to(amount).unwrap().minor_units, 200);   // 10% of 2000¢
//! assert_eq!(pct.complement_apply_to(amount).unwrap().minor_units, 1800); // 90% of 2000¢
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::money::Money;

/// A percentage value guaranteed to be in the range `0..=100`.
///
/// Construction via [`Percentage::new`] validates the range immediately
/// so consumers never have to check bounds.
///
/// The inner value is stored as [`u8`] because percentages never exceed
/// 100, making the type smaller and cheaper than `i64`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Percentage(u8);

impl Serialize for Percentage {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(ser)
    }
}

impl<'de> Deserialize<'de> for Percentage {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let val = u8::deserialize(de)?;
        Percentage::new(val).ok_or_else(|| {
            serde::de::Error::custom(format!("percentage must be between 0 and 100, got {val}"))
        })
    }
}

impl Percentage {
    /// Try to create a `Percentage` from a raw value.
    ///
    /// Returns `None` when `value > 100`.
    #[must_use]
    pub fn new(value: u8) -> Option<Self> {
        if value <= 100 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Return the raw percentage value (`0..=100`).
    #[must_use]
    pub fn get(&self) -> u8 {
        self.0
    }

    /// Apply this percentage to a [`Money`] amount.
    ///
    /// E.g. `Percentage::new(10).apply_to(Money::from_major(20, …))`
    /// returns `200` (10% of 2000¢ = 200¢).
    ///
    /// Computed as `(x * p) / 100` **without a full-width product**, using
    /// the identity `x = 100q + r  ⇒  (x*p)/100 = q*p + (r*p)/100`. This is
    /// exact under Rust's truncating division and never overflows for any
    /// `i64` amount and any percentage `0..=100` (both terms are bounded by
    /// `|x|` and 9900 respectively). The previous `checked_mul(p)`
    /// implementation spuriously returned `None` for amounts where the
    /// *product* overflowed but the *result* fit — e.g. 100% of `i64::MAX`
    /// (the largest representable discount) failed instead of returning
    /// `i64::MAX`.
    ///
    /// `Option` is retained for API stability; the arithmetic is total.
    #[must_use]
    pub fn apply_to(self, money: Money) -> Option<Money> {
        let x = money.minor_units;
        let p = i64::from(self.0);
        let q = x / 100;
        let r = x % 100;
        let hi = q.checked_mul(p)?; // |hi| ≤ |x| — cannot overflow
        let lo = r.checked_mul(p)?; // |lo| ≤ 9900 — cannot overflow
        Some(Money {
            minor_units: hi.checked_add(lo / 100)?,
            currency: money.currency,
        })
    }

    /// Apply the **complement** of this percentage (i.e. `100% - self`)
    /// to a [`Money`] amount.
    ///
    /// E.g. with a 10% discount, `complement_apply_to` returns 90% of the
    /// amount. This is a single combined operation: `amount × (100 - pct) / 100`.
    ///
    /// Same overflow-free decomposition as [`apply_to`](Self::apply_to);
    /// never fails for any `i64` amount and any percentage `0..=100`.
    #[must_use]
    pub fn complement_apply_to(self, money: Money) -> Option<Money> {
        let x = money.minor_units;
        let p = i64::from(100 - self.0);
        let q = x / 100;
        let r = x % 100;
        let hi = q.checked_mul(p)?; // |hi| ≤ |x| — cannot overflow
        let lo = r.checked_mul(p)?; // |lo| ≤ 9900 — cannot overflow
        Some(Money {
            minor_units: hi.checked_add(lo / 100)?,
            currency: money.currency,
        })
    }

    /// Shorthand for `Percentage::new(0).unwrap()`.
    #[must_use]
    pub fn zero() -> Self {
        Self(0)
    }

    /// Shorthand for `Percentage::new(100).unwrap()`.
    #[must_use]
    pub fn hundred() -> Self {
        Self(100)
    }
}

impl std::fmt::Display for Percentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0)
    }
}

#[cfg(test)]
mod tests {
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
}
