//! Money and currency primitives.
//!
//! Money is **always** stored as integer minor units (e.g., cents for USD,
//! sen for IDR, paise for INR). Pair with an ISO-4217 currency code for
//! display. Floating point is forbidden anywhere in the money path.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// A monetary amount in the smallest unit of a currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// Amount in the smallest currency unit (e.g., cents for USD).
    pub minor_units: i64,
    /// ISO-4217 currency code, e.g. "USD", "IDR", "EUR".
    pub currency: Currency,
}

impl Default for Money {
    fn default() -> Self {
        Self {
            minor_units: 0,
            currency: Currency(*b"USD"),
        }
    }
}

/// Error returned by [`Currency`]'s [`FromStr`] impl when the input is
/// not a valid ISO-4217 alpha-3 code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidCurrencyCode;

impl std::fmt::Display for InvalidCurrencyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("expected a 3-letter ISO-4217 currency code")
    }
}

impl std::error::Error for InvalidCurrencyCode {}

/// An ISO-4217 alpha-3 currency code stored as 3 raw ASCII bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Currency(pub [u8; 3]);

impl Currency {
    /// The number of decimal places used by this currency.
    pub fn minor_unit_exponent(&self) -> u32 {
        match &self.0 {
            b"JPY" | b"KRW" | b"VND" | b"CLP" | b"ISK" | b"HUF" => 0,
            b"KWD" | b"OMR" | b"BHD" | b"JOD" | b"TND" => 3,
            _ => 2,
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = std::str::from_utf8(&self.0).unwrap_or("???");
        f.write_str(s)
    }
}

impl FromStr for Currency {
    type Err = InvalidCurrencyCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        if bytes.len() != 3 || !bytes.iter().all(|b| b.is_ascii_alphabetic()) {
            return Err(InvalidCurrencyCode);
        }
        let mut out = [0u8; 3];
        out.copy_from_slice(bytes);
        Ok(Self(out))
    }
}

impl Serialize for Currency {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let s = std::str::from_utf8(&self.0)
            .map_err(|e| serde::ser::Error::custom(format!("invalid currency bytes: {e}")))?;
        ser.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for Currency {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        s.parse::<Currency>().map_err(serde::de::Error::custom)
    }
}

impl Money {
    /// Construct a zero amount in the given currency.
    #[must_use]
    pub fn zero(currency: Currency) -> Self {
        Self {
            minor_units: 0,
            currency,
        }
    }

    /// Construct from a major-unit amount (e.g., dollars).
    #[must_use]
    pub fn from_major(major: i64, currency: Currency) -> Option<Self> {
        let exp = currency.minor_unit_exponent();
        major.checked_mul(10_i64.pow(exp)).map(|minor_units| Self {
            minor_units,
            currency,
        })
    }

    /// Add two Money values. Returns `None` if currencies differ or overflow.
    #[must_use]
    pub fn checked_add(self, other: Money) -> Option<Money> {
        if self.currency != other.currency {
            return None;
        }
        self.minor_units
            .checked_add(other.minor_units)
            .map(|v| Self {
                minor_units: v,
                currency: self.currency,
            })
    }

    /// Subtract another Money value. Returns `None` if currencies differ
    /// or underflow.
    #[must_use]
    pub fn checked_sub(self, other: Money) -> Option<Money> {
        if self.currency != other.currency {
            return None;
        }
        self.minor_units
            .checked_sub(other.minor_units)
            .map(|v| Self {
                minor_units: v,
                currency: self.currency,
            })
    }

    /// Multiply the minor-units amount by an integer scalar. Keeps the
    /// same currency. Returns `None` on overflow.
    #[must_use]
    pub fn checked_mul(self, scalar: i64) -> Option<Money> {
        self.minor_units.checked_mul(scalar).map(|v| Self {
            minor_units: v,
            currency: self.currency,
        })
    }

    /// Divide the minor-units amount by an integer divisor. Keeps the
    /// same currency. Returns `None` on overflow or division by zero.
    #[must_use]
    pub fn checked_div(self, divisor: i64) -> Option<Money> {
        self.minor_units.checked_div(divisor).map(|v| Self {
            minor_units: v,
            currency: self.currency,
        })
    }

    /// Negate the amount. Positive becomes negative and vice versa.
    /// Same currency.
    ///
    /// ⚠️ **Panics on `i64::MIN` in debug mode** (wraps in release) —
    /// same behaviour as `i64::neg`. Use [`checked_sub`](Self::checked_sub)
    /// on `Money::zero()` if you need overflow safety.
    #[must_use]
    pub fn negate(self) -> Money {
        Money {
            minor_units: -self.minor_units,
            currency: self.currency,
        }
    }

    /// Absolute value of the amount. Same currency.
    ///
    /// ⚠️ **Panics on `i64::MIN` in debug mode** (wraps in release) —
    /// same behaviour as [`i64::abs`].
    #[must_use]
    pub fn abs(self) -> Money {
        Money {
            minor_units: self.minor_units.abs(),
            currency: self.currency,
        }
    }

    /// Check whether the amount is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.minor_units == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    #[test]
    fn money_zero_is_zero() {
        assert_eq!(Money::zero(usd()).minor_units, 0);
    }

    #[test]
    fn money_from_major_dollars() {
        let m = Money::from_major(12, usd()).unwrap();
        assert_eq!(m.minor_units, 1200);
    }

    #[test]
    fn money_from_major_jpy_has_no_exponent() {
        let jpy: Currency = "JPY".parse().unwrap();
        let m = Money::from_major(12, jpy).unwrap();
        assert_eq!(m.minor_units, 12);
    }

    #[test]
    fn money_from_major_overflow_returns_none() {
        let kwd: Currency = "KWD".parse().unwrap();
        assert!(Money::from_major(i64::MAX, kwd).is_none());
    }

    #[test]
    fn checked_add_same_currency() {
        let a = Money::from_major(5, usd()).unwrap();
        let b = Money::from_major(7, usd()).unwrap();
        assert_eq!(a.checked_add(b).unwrap().minor_units, 1200);
    }

    #[test]
    fn checked_add_different_currency_returns_none() {
        let eur: Currency = "EUR".parse().unwrap();
        let a = Money::from_major(5, usd()).unwrap();
        let b = Money::from_major(5, eur).unwrap();
        assert!(a.checked_add(b).is_none());
    }

    #[test]
    fn checked_add_overflow_returns_none() {
        let a = Money {
            minor_units: i64::MAX,
            currency: usd(),
        };
        let b = Money::from_major(1, usd()).unwrap();
        assert!(a.checked_add(b).is_none());
    }

    // ── checked_sub ────────────────────────────────────────────

    #[test]
    fn checked_sub_same_currency() {
        let a = Money::from_major(10, usd()).unwrap();
        let b = Money::from_major(3, usd()).unwrap();
        let result = a.checked_sub(b).unwrap();
        assert_eq!(result.minor_units, 700);
        assert_eq!(result.currency, usd());
    }

    #[test]
    fn checked_sub_different_currency_returns_none() {
        let eur: Currency = "EUR".parse().unwrap();
        let a = Money::from_major(10, usd()).unwrap();
        let b = Money::from_major(3, eur).unwrap();
        assert!(a.checked_sub(b).is_none());
    }

    #[test]
    fn checked_sub_underflow_returns_none() {
        let a = Money {
            minor_units: i64::MIN,
            currency: usd(),
        };
        let b = Money::from_major(1, usd()).unwrap();
        assert!(a.checked_sub(b).is_none());
    }

    #[test]
    fn checked_sub_result_is_zero() {
        let a = Money::from_major(7, usd()).unwrap();
        let b = Money::from_major(7, usd()).unwrap();
        assert_eq!(a.checked_sub(b).unwrap().minor_units, 0);
    }

    // ── checked_mul ────────────────────────────────────────────

    #[test]
    fn checked_mul_by_scalar() {
        let m = Money::from_major(5, usd()).unwrap();
        let result = m.checked_mul(3).unwrap();
        assert_eq!(result.minor_units, 1500);
    }

    #[test]
    fn checked_mul_by_zero() {
        let m = Money::from_major(5, usd()).unwrap();
        let result = m.checked_mul(0).unwrap();
        assert_eq!(result.minor_units, 0);
    }

    #[test]
    fn checked_mul_by_one() {
        let m = Money::from_major(5, usd()).unwrap();
        let result = m.checked_mul(1).unwrap();
        assert_eq!(result.minor_units, 500);
    }

    #[test]
    fn checked_mul_overflow_returns_none() {
        let m = Money {
            minor_units: i64::MAX,
            currency: usd(),
        };
        assert!(m.checked_mul(2).is_none());
    }

    #[test]
    fn checked_mul_preserves_currency() {
        let jpy: Currency = "JPY".parse().unwrap();
        let m = Money::from_major(100, jpy).unwrap();
        let result = m.checked_mul(2).unwrap();
        assert_eq!(result.currency, jpy);
    }

    // ── checked_div ────────────────────────────────────────────

    #[test]
    fn checked_div_by_scalar() {
        let m = Money::from_major(10, usd()).unwrap();
        let result = m.checked_div(3).unwrap();
        assert_eq!(result.minor_units, 333); // 1000 / 3 = 333 (truncated)
    }

    #[test]
    fn checked_div_by_one() {
        let m = Money::from_major(7, usd()).unwrap();
        let result = m.checked_div(1).unwrap();
        assert_eq!(result.minor_units, 700);
    }

    #[test]
    fn checked_div_by_zero_returns_none() {
        let m = Money::from_major(5, usd()).unwrap();
        assert!(m.checked_div(0).is_none());
    }

    #[test]
    fn checked_div_negative_scalar() {
        let m = Money::from_major(10, usd()).unwrap();
        let result = m.checked_div(-2).unwrap();
        assert_eq!(result.minor_units, -500);
    }

    #[test]
    fn checked_div_preserves_currency() {
        let eur: Currency = "EUR".parse().unwrap();
        let m = Money::from_major(15, eur).unwrap();
        let result = m.checked_div(2).unwrap();
        assert_eq!(result.currency, eur);
    }

    // ── negate ─────────────────────────────────────────────────

    #[test]
    fn negate_positive_becomes_negative() {
        let m = Money::from_major(5, usd()).unwrap();
        let neg = m.negate();
        assert_eq!(neg.minor_units, -500);
        assert_eq!(neg.currency, usd());
    }

    #[test]
    fn negate_negative_becomes_positive() {
        let m = Money {
            minor_units: -500,
            currency: usd(),
        };
        let pos = m.negate();
        assert_eq!(pos.minor_units, 500);
    }

    #[test]
    fn negate_zero_stays_zero() {
        let m = Money::zero(usd()).negate();
        assert_eq!(m.minor_units, 0);
    }

    #[test]
    fn negate_twice_is_identity() {
        let m = Money::from_major(5, usd()).unwrap();
        assert_eq!(m.negate().negate(), m);
    }

    // ── abs ────────────────────────────────────────────────────

    #[test]
    fn abs_positive_is_noop() {
        let m = Money::from_major(5, usd()).unwrap();
        assert_eq!(m.abs().minor_units, 500);
    }

    #[test]
    fn abs_negative_becomes_positive() {
        let m = Money {
            minor_units: -500,
            currency: usd(),
        };
        assert_eq!(m.abs().minor_units, 500);
    }

    #[test]
    fn abs_zero_is_zero() {
        assert_eq!(Money::zero(usd()).abs().minor_units, 0);
    }

    #[test]
    fn abs_preserves_currency() {
        let jpy: Currency = "JPY".parse().unwrap();
        let m = Money {
            minor_units: -1000,
            currency: jpy,
        };
        assert_eq!(m.abs().currency, jpy);
    }

    // ── is_zero ────────────────────────────────────────────────

    #[test]
    fn is_zero_true_for_zero() {
        assert!(Money::zero(usd()).is_zero());
    }

    #[test]
    fn is_zero_false_for_non_zero() {
        let m = Money::from_major(1, usd()).unwrap();
        assert!(!m.is_zero());
    }

    #[test]
    fn is_zero_false_for_negative() {
        let m = Money {
            minor_units: -1,
            currency: usd(),
        };
        assert!(!m.is_zero());
    }

    #[test]
    fn is_zero_chained_after_arithmetic() {
        let a = Money::from_major(3, usd()).unwrap();
        let b = Money::from_major(3, usd()).unwrap();
        assert!(a.checked_sub(b).unwrap().is_zero());
    }

    #[test]
    fn currency_from_str_rejects_bad_input() {
        assert!("US".parse::<Currency>().is_err());
        assert!("USDD".parse::<Currency>().is_err());
        assert!("U2D".parse::<Currency>().is_err());
        assert!("USD".parse::<Currency>().is_ok());
    }

    #[test]
    fn minor_unit_exponent_known_codes() {
        assert_eq!(usd().minor_unit_exponent(), 2);
        assert_eq!("JPY".parse::<Currency>().unwrap().minor_unit_exponent(), 0);
        assert_eq!("KWD".parse::<Currency>().unwrap().minor_unit_exponent(), 3);
    }
}
