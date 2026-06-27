//! Money and currency primitives.
//!
//! Money is **always** stored as integer minor units (e.g., cents for USD,
//! sen for IDR, paise for INR). Pair with an ISO-4217 currency code for
//! display. Floating point is forbidden anywhere in the money path — see
//! the `rust-backend` skill for the full rationale and the
//! checked-arithmetic patterns.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// A monetary amount in the smallest unit of a currency.
///
/// `minor_units` is the integer count of the smallest currency unit
/// (cents for USD, sen for IDR, paise for INR). It is always paired
/// with a [`Currency`] so the unit is unambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// Amount in the smallest currency unit (e.g., cents for USD).
    pub minor_units: i64,
    /// ISO-4217 currency code, e.g. "USD", "IDR", "EUR".
    pub currency: Currency,
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
///
/// Wrapping the code in a newtype lets the type system enforce the
/// 3-character shape; we never pass a `String` where a currency is
/// expected.
///
/// # Wire format
///
/// Serialized as a 3-byte ASCII string (`"USD"`, not `[85, 83, 68]`)
/// so the JSON boundary with the React/TypeScript front-end matches
/// `ui/src/types/domain.ts`. The custom `Serialize`/`Deserialize`
/// impls below enforce this; the inner `[u8; 3]` stays in place so
/// the type remains `Copy` and downstream code is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Currency(pub [u8; 3]);

impl Currency {
    /// The number of decimal places used by this currency.
    ///
    /// Returns the ISO-4217 minor-unit exponent for known codes (2 for
    /// USD/EUR/IDR, 0 for JPY/KRW, 3 for KWD/OMR/BHD). Unknown codes
    /// default to 2, the most common case; this matches the behaviour
    /// of the rest of the project until a full ISO-4217 table is wired
    /// up.
    pub fn minor_unit_exponent(&self) -> u32 {
        match &self.0 {
            b"JPY" | b"KRW" | b"VND" | b"CLP" | b"ISK" | b"HUF" => 0,
            b"KWD" | b"OMR" | b"BHD" | b"JOD" | b"TND" => 3,
            _ => 2,
        }
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

    /// Construct from a major-unit amount (e.g., dollars). Multiplies by
    /// the currency's exponent before storing.
    ///
    /// Returns `None` if the resulting minor-unit amount would overflow
    /// `i64` (e.g. `from_major(i64::MAX, KWD)` — 3 decimal places).
    #[must_use]
    pub fn from_major(major: i64, currency: Currency) -> Option<Self> {
        let exp = currency.minor_unit_exponent();
        major.checked_mul(10_i64.pow(exp)).map(|minor_units| Self {
            minor_units,
            currency,
        })
    }

    /// Add two Money values. Returns `None` if the currencies differ
    /// (caller must convert first) or if the sum overflows `i64`.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_zero_is_zero() {
        let usd: Currency = "USD".parse().unwrap();
        assert_eq!(Money::zero(usd).minor_units, 0);
    }

    #[test]
    fn money_from_major_dollars() {
        let usd: Currency = "USD".parse().unwrap();
        let m = Money::from_major(12, usd).unwrap();
        assert_eq!(m.minor_units, 1200);
        assert_eq!(m.currency, usd);
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
        let usd: Currency = "USD".parse().unwrap();
        let a = Money::from_major(5, usd).unwrap();
        let b = Money::from_major(7, usd).unwrap();
        assert_eq!(a.checked_add(b).unwrap().minor_units, 1200);
    }

    #[test]
    fn checked_add_different_currency_returns_none() {
        let usd: Currency = "USD".parse().unwrap();
        let eur: Currency = "EUR".parse().unwrap();
        let a = Money::from_major(5, usd).unwrap();
        let b = Money::from_major(5, eur).unwrap();
        assert!(a.checked_add(b).is_none());
    }

    #[test]
    fn checked_add_overflow_returns_none() {
        let usd: Currency = "USD".parse().unwrap();
        let a = Money {
            minor_units: i64::MAX,
            currency: usd,
        };
        let b = Money::from_major(1, usd).unwrap();
        assert!(a.checked_add(b).is_none());
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
        assert_eq!("USD".parse::<Currency>().unwrap().minor_unit_exponent(), 2);
        assert_eq!("JPY".parse::<Currency>().unwrap().minor_unit_exponent(), 0);
        assert_eq!("KWD".parse::<Currency>().unwrap().minor_unit_exponent(), 3);
    }
}
