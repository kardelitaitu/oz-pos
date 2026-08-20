//! Money and currency primitives.
//!
//! Money is **always** stored as integer minor units (e.g., cents for USD,
//! paise for INR; IDR/JPY/KRW have a 0 exponent so the minor unit IS the
//! Rupiah/Yen/Won). Pair with an ISO-4217 currency code for display.
//! Floating point is forbidden anywhere in the money path.

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
    ///
    /// ISO-4217 minor-unit exponent: IDR/JPY/KRW/VND/CLP/ISK/HUF = 0,
    /// KWD/OMR/BHD/JOD/TND = 3, everything else = 2. This must stay in
    /// sync with the seeds in `crates/oz-core/migrations/006_currencies.sql`,
    /// `crates/oz-cli` (init-db) and the frontend `MINOR_UNIT_EXPONENT`
    /// (ui/src/types/domain.ts) — all treat IDR as 0 (the Rupiah has no
    /// circulating minor unit).
    pub fn minor_unit_exponent(&self) -> u32 {
        match &self.0 {
            b"IDR" | b"JPY" | b"KRW" | b"VND" | b"CLP" | b"ISK" | b"HUF" => 0,
            b"KWD" | b"OMR" | b"BHD" | b"JOD" | b"TND" => 3,
            _ => 2,
        }
    }
}

/// Render a minor-unit amount as a major-unit decimal string using the
/// currency's ISO-4217 minor-unit exponent — e.g. `1_200` USD minor →
/// `"12.00"`, `4_450_000` IDR minor → `"4450000"`, `12` KWD minor →
/// `"0.012"`. No currency symbol/code is appended; the caller adds
/// context (symbol, "off", etc.).
#[must_use]
pub fn format_minor(minor: i64, currency: Currency) -> String {
    let exp = currency.minor_unit_exponent();
    if exp == 0 {
        minor.to_string()
    } else {
        let div = 10_i64.pow(exp);
        let major = minor / div;
        // `minor % div` keeps the dividend's sign (never overflows); take
        // the unsigned magnitude so `i64::MIN` cannot panic (`abs()` would)
        // or render as a negative fraction.
        let frac = (minor % div).unsigned_abs();
        // Integer division truncates toward zero, so a negative sub-major
        // amount (e.g. -12 cents) would otherwise render as "0.12" and lose
        // its sign — prefix it explicitly.
        let sign = if minor < 0 && major == 0 { "-" } else { "" };
        format!("{sign}{major}.{:0width$}", frac, width = exp as usize)
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
        // Normalise to uppercase so e.g. "jpy" and "JPY" produce the same
        // Currency value and match the same minor_unit_exponent patterns.
        for (i, b) in bytes.iter().enumerate() {
            out[i] = b.to_ascii_uppercase();
        }
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

    /// Negate the amount. Same currency. Returns `None` on `i64::MIN`
    /// overflow (where [`negate`](Self::negate) would panic).
    #[must_use]
    pub fn checked_negate(self) -> Option<Money> {
        self.minor_units.checked_neg().map(|v| Self {
            minor_units: v,
            currency: self.currency,
        })
    }

    /// Absolute value of the amount. Same currency. Returns `None` on
    /// `i64::MIN` overflow (where [`abs`](Self::abs) would panic).
    #[must_use]
    pub fn checked_abs(self) -> Option<Money> {
        self.minor_units.checked_abs().map(|v| Self {
            minor_units: v,
            currency: self.currency,
        })
    }

    /// Negate the amount. Positive becomes negative and vice versa.
    /// Same currency.
    ///
    /// ⚠️ **Panics on `i64::MIN` in debug mode** (wraps in release) —
    /// same behaviour as `i64::neg`. Prefer
    /// [`checked_negate`](Self::checked_negate) when the amount could be
    /// `i64::MIN`.
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
    /// same behaviour as [`i64::abs`]. Prefer
    /// [`checked_abs`](Self::checked_abs) when the amount could be
    /// `i64::MIN`.
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

impl PartialOrd for Money {
    /// Compare two amounts. Returns `Some(ordering)` when the currencies
    /// match; returns `None` when they differ — the same domain-error
    /// rule as [`checked_add`](Self::checked_add), so a cross-currency
    /// comparison is incomparable rather than silently ordered.
    ///
    /// `Money` deliberately does **not** implement [`Ord`]: a total order
    /// across currencies (e.g. a derived one) would let `USD 1 < EUR 0`
    /// hold. Callers comparing amounts in different currencies must
    /// convert first.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.currency != other.currency {
            return None;
        }
        self.minor_units.partial_cmp(&other.minor_units)
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

    // ── checked_negate ─────────────────────────────────────────

    #[test]
    fn checked_negate_positive_becomes_negative() {
        let m = Money::from_major(5, usd()).unwrap();
        let neg = m.checked_negate().unwrap();
        assert_eq!(neg.minor_units, -500);
        assert_eq!(neg.currency, usd());
    }

    #[test]
    fn checked_negate_negative_becomes_positive() {
        let m = Money {
            minor_units: -500,
            currency: usd(),
        };
        let pos = m.checked_negate().unwrap();
        assert_eq!(pos.minor_units, 500);
    }

    #[test]
    fn checked_negate_zero_stays_zero() {
        let m = Money::zero(usd()).checked_negate().unwrap();
        assert_eq!(m.minor_units, 0);
        assert_eq!(m.currency, usd());
    }

    #[test]
    fn checked_negate_i64_min_returns_none() {
        // -i64::MIN overflows; must not panic like `negate()` does.
        let m = Money {
            minor_units: i64::MIN,
            currency: usd(),
        };
        assert!(m.checked_negate().is_none());
    }

    #[test]
    fn checked_negate_twice_is_identity() {
        let m = Money::from_major(5, usd()).unwrap();
        assert_eq!(m.checked_negate().and_then(|n| n.checked_negate()), Some(m));
    }

    // ── checked_abs ────────────────────────────────────────────

    #[test]
    fn checked_abs_positive_is_noop() {
        let m = Money::from_major(5, usd()).unwrap();
        let a = m.checked_abs().unwrap();
        assert_eq!(a.minor_units, 500);
        assert_eq!(a.currency, usd());
    }

    #[test]
    fn checked_abs_negative_becomes_positive() {
        let m = Money {
            minor_units: -500,
            currency: usd(),
        };
        let a = m.checked_abs().unwrap();
        assert_eq!(a.minor_units, 500);
    }

    #[test]
    fn checked_abs_zero_is_zero() {
        let a = Money::zero(usd()).checked_abs().unwrap();
        assert_eq!(a.minor_units, 0);
    }

    #[test]
    fn checked_abs_i64_min_returns_none() {
        // i64::MIN.abs() overflows; must not panic like `abs()` does.
        let m = Money {
            minor_units: i64::MIN,
            currency: usd(),
        };
        assert!(m.checked_abs().is_none());
    }

    #[test]
    fn checked_abs_preserves_currency() {
        let jpy: Currency = "JPY".parse().unwrap();
        let m = Money {
            minor_units: -1000,
            currency: jpy,
        };
        assert_eq!(m.checked_abs().unwrap().currency, jpy);
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
        assert_eq!("IDR".parse::<Currency>().unwrap().minor_unit_exponent(), 0);
        assert_eq!("KWD".parse::<Currency>().unwrap().minor_unit_exponent(), 3);
    }

    #[test]
    fn money_from_major_idr_has_no_exponent() {
        let idr: Currency = "IDR".parse().unwrap();
        // IDR minor unit IS the Rupiah (exp 0): Rp 12 → 12 minor units.
        let m = Money::from_major(12, idr).unwrap();
        assert_eq!(m.minor_units, 12);
    }

    #[test]
    fn format_minor_uses_currency_exponent() {
        let idr: Currency = "IDR".parse().unwrap();
        let usd: Currency = "USD".parse().unwrap();
        let kwd: Currency = "KWD".parse().unwrap();
        // exp 0: minor unit IS the Rupiah.
        assert_eq!(format_minor(4_450_000, idr), "4450000");
        // exp 2: 1,200 cents → $12.00.
        assert_eq!(format_minor(1_200, usd), "12.00");
        // exp 3: 12 fils → KWD 0.012.
        assert_eq!(format_minor(12, kwd), "0.012");
        // Negative keeps sign on the major part.
        assert_eq!(format_minor(-1_200, usd), "-12.00");
        // Negative sub-major amount keeps its sign too (refund/void totals).
        assert_eq!(format_minor(-12, usd), "-0.12");
        assert_eq!(format_minor(-12, kwd), "-0.012");
        assert_eq!(format_minor(-4_450_000, idr), "-4450000");
    }

    #[test]
    fn lowercase_currency_parses_with_correct_exponent() {
        // Lowercase codes should parse and produce the same exponent as uppercase.
        let jpy: Currency = "jpy".parse().unwrap();
        assert_eq!(
            jpy.minor_unit_exponent(),
            0,
            "'jpy' should have 0 exponent like 'JPY'"
        );
        let krw: Currency = "krw".parse().unwrap();
        assert_eq!(
            krw.minor_unit_exponent(),
            0,
            "'krw' should have 0 exponent like 'KRW'"
        );
        let kwd: Currency = "kwd".parse().unwrap();
        assert_eq!(
            kwd.minor_unit_exponent(),
            3,
            "'kwd' should have 3 exponent like 'KWD'"
        );
    }

    #[test]
    fn mixed_case_currency_parses_with_correct_exponent() {
        let jpy: Currency = "Jpy".parse().unwrap();
        assert_eq!(jpy.minor_unit_exponent(), 0, "'Jpy' should have 0 exponent");
    }

    // ── Additional edge-case tests ───────────────────────────────────────

    #[test]
    fn format_minor_jpy_no_decimal_places() {
        let jpy: Currency = "JPY".parse().unwrap();
        assert_eq!(format_minor(12345, jpy), "12345");
    }

    #[test]
    fn format_minor_kwd_three_decimal_places() {
        let kwd: Currency = "KWD".parse().unwrap();
        assert_eq!(format_minor(12345, kwd), "12.345");
    }

    #[test]
    fn format_minor_usd_two_decimal_places() {
        assert_eq!(format_minor(12345, usd()), "123.45");
    }

    #[test]
    fn format_minor_usd_zero() {
        assert_eq!(format_minor(0, usd()), "0.00");
    }

    #[test]
    fn format_minor_usd_one_cent() {
        assert_eq!(format_minor(1, usd()), "0.01");
    }

    #[test]
    fn format_minor_i64_min_does_not_panic() {
        // `i64::MIN.abs()` overflows (panics in debug, wraps negative in
        // release). The fractional part must render as "08", not "-8" or a
        // panic: -9_223_372_036_854_775_808 / 100 → -92_233_720_368_547_758.08.
        assert_eq!(format_minor(i64::MIN, usd()), "-92233720368547758.08");
    }

    #[test]
    fn format_minor_idr_no_decimal() {
        let idr: Currency = "IDR".parse().unwrap();
        assert_eq!(format_minor(50000, idr), "50000");
    }

    #[test]
    fn money_from_major_various_currencies() {
        let usd_val = Money::from_major(10, usd()).unwrap();
        assert_eq!(usd_val.minor_units, 1000);
        let jpy = Money::from_major(500, "JPY".parse().unwrap()).unwrap();
        assert_eq!(jpy.minor_units, 500);
    }

    #[test]
    fn money_checked_add_same_currency() {
        let a = Money {
            minor_units: 100,
            currency: usd(),
        };
        let b = Money {
            minor_units: 250,
            currency: usd(),
        };
        let sum = a.checked_add(b).unwrap();
        assert_eq!(sum.minor_units, 350);
        assert_eq!(sum.currency, usd());
    }

    #[test]
    fn money_checked_add_different_currency_returns_none() {
        let usd_val = Money {
            minor_units: 100,
            currency: usd(),
        };
        let eur_val = Money {
            minor_units: 100,
            currency: "EUR".parse().unwrap(),
        };
        assert!(usd_val.checked_add(eur_val).is_none());
    }

    #[test]
    fn money_checked_sub_underflow_returns_none() {
        let a = Money {
            minor_units: i64::MIN,
            currency: usd(),
        };
        let b = Money {
            minor_units: 1,
            currency: usd(),
        };
        assert!(a.checked_sub(b).is_none());
    }

    #[test]
    fn money_checked_mul_overflow_returns_none() {
        let m = Money {
            minor_units: i64::MAX,
            currency: usd(),
        };
        assert!(m.checked_mul(2).is_none());
    }

    #[test]
    fn money_checked_div_by_zero_returns_none() {
        let m = Money {
            minor_units: 100,
            currency: usd(),
        };
        assert!(m.checked_div(0).is_none());
    }

    #[test]
    fn money_negate_flips_sign() {
        let m = Money {
            minor_units: 500,
            currency: usd(),
        };
        assert_eq!(m.negate().minor_units, -500);
        assert_eq!(m.negate().negate().minor_units, 500);
    }

    #[test]
    fn money_abs_makes_positive() {
        let m = Money {
            minor_units: -300,
            currency: usd(),
        };
        assert_eq!(m.abs().minor_units, 300);
        assert_eq!(m.abs().currency, usd());
    }

    #[test]
    fn money_is_zero() {
        assert!(Money::zero(usd()).is_zero());
        assert!(
            !Money {
                minor_units: 1,
                currency: usd()
            }
            .is_zero()
        );
    }

    #[test]
    fn money_equality_by_minor_units_and_currency() {
        let a = Money {
            minor_units: 100,
            currency: usd(),
        };
        let b = Money {
            minor_units: 100,
            currency: usd(),
        };
        let c = Money {
            minor_units: 200,
            currency: usd(),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn money_eq_different_currency_not_equal() {
        // PartialEq: amounts in different currencies are never equal.
        // (Cross-currency ordering is incomparable — see
        // `partial_cmp_different_currency_is_none`.)
        let usd_val = Money {
            minor_units: 100,
            currency: usd(),
        };
        let eur_val = Money {
            minor_units: 100,
            currency: "EUR".parse().unwrap(),
        };
        assert_ne!(usd_val, eur_val);
    }

    #[test]
    fn money_hash_consistent_with_eq() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        let key = Money {
            minor_units: 100,
            currency: usd(),
        };
        map.insert(key, "one hundred");
        let lookup = Money {
            minor_units: 100,
            currency: usd(),
        };
        assert_eq!(map.get(&lookup), Some(&"one hundred"));
    }

    // ── PartialOrd ─────────────────────────────────────────────

    #[test]
    fn partial_cmp_same_currency_orders() {
        let low = Money::from_major(5, usd()).unwrap();
        let high = Money::from_major(7, usd()).unwrap();
        assert_eq!(low.partial_cmp(&high), Some(std::cmp::Ordering::Less));
        assert_eq!(high.partial_cmp(&low), Some(std::cmp::Ordering::Greater));
    }

    #[test]
    fn partial_cmp_same_amount_equal() {
        let a = Money::from_major(5, usd()).unwrap();
        let b = Money::from_major(5, usd()).unwrap();
        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Equal));
    }

    #[test]
    fn partial_cmp_different_currency_is_none() {
        // Cross-currency comparison is a domain error, mirroring
        // `checked_add`: USD 1 < EUR 0 must NOT be orderable.
        let usd_val = Money::from_major(1, usd()).unwrap();
        let eur_val = Money::from_major(0, "EUR".parse().unwrap()).unwrap();
        assert_eq!(usd_val.partial_cmp(&eur_val), None);
        assert_eq!(eur_val.partial_cmp(&usd_val), None);
    }

    #[test]
    fn money_lt_gt_operators_same_currency() {
        let low = Money::from_major(5, usd()).unwrap();
        let high = Money::from_major(7, usd()).unwrap();
        assert!(low < high);
        assert!(high > low);
        assert!(low <= high);
        assert!(low <= low);
        assert!(high >= low);
        assert!(high >= high);
    }

    #[test]
    fn money_lt_different_currency_is_false() {
        // PartialOrd contract: when partial_cmp is None, < / > / <= / >=
        // all return false — a cross-currency "less than" must not hold.
        let usd_val = Money::from_major(1, usd()).unwrap();
        let eur_val = Money::from_major(100, "EUR".parse().unwrap()).unwrap();
        let lt = usd_val < eur_val;
        let gt = usd_val > eur_val;
        let le = usd_val <= eur_val;
        let ge = usd_val >= eur_val;
        assert!(!lt);
        assert!(!gt);
        assert!(!le);
        assert!(!ge);
    }

    #[test]
    fn partial_cmp_negative_orders() {
        let neg = Money {
            minor_units: -500,
            currency: usd(),
        };
        let zero = Money::zero(usd());
        let pos = Money::from_major(5, usd()).unwrap();
        assert!(neg < zero);
        assert!(zero < pos);
        assert!(neg < pos);
    }

    #[test]
    fn money_min_operator_picks_lower_same_currency() {
        // `Money` is only PartialOrd (cross-currency is incomparable),
        // so the ordering APIs that assume a total order (Ord::min etc.)
        // must NOT exist — this pins the design.
        fn assert_manual_min(a: Money, b: Money) -> Money {
            if a < b { a } else { b }
        }
        let low = Money::from_major(5, usd()).unwrap();
        let high = Money::from_major(7, usd()).unwrap();
        assert_eq!(assert_manual_min(low, high), low);
    }

    #[test]
    fn currency_idr_has_zero_exponent() {
        let idr: Currency = "IDR".parse().unwrap();
        assert_eq!(idr.minor_unit_exponent(), 0);
    }

    #[test]
    fn currency_jpy_has_zero_exponent() {
        let jpy: Currency = "JPY".parse().unwrap();
        assert_eq!(jpy.minor_unit_exponent(), 0);
    }

    #[test]
    fn currency_kwd_has_three_exponent() {
        let kwd: Currency = "KWD".parse().unwrap();
        assert_eq!(kwd.minor_unit_exponent(), 3);
    }

    #[test]
    fn currency_usd_has_two_exponent() {
        assert_eq!(usd().minor_unit_exponent(), 2);
    }
}
