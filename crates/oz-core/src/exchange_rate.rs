//! Exchange rate types.
//!
//! Rates are stored as **integer minor units** (`rate_millionths: i64`) at a
//! 6-decimal fixed-point scale to keep the exchange-rate domain out of the
//! float-arithmetic error class. `0.92` is represented as `920_000`; the
//! backoffice display path uses [`ExchangeRateRow::display_rate`].
//!
//! This is the C-1 close-out from
//! `docs/specs/_active/2026-07-12-desktop-app-audit.md` — the previous
//! `f64` representation contaminated every downstream multi-currency
//! checkout multiplier and made the `<= 0` validation sign-unstable near
//! zero.
//!
//! Scale: `rate_millionths = rate_real * 1_000_000`. 6 decimals are
//! sufficient for every fixture in the test suite (worst case 0.00025 JPY→KWD
//! = 250; largest USD→JPY ≈ 150 = 150_000_000). i64 max (~9.2 × 10¹⁸) covers
//! ~9 trillion major units scaled this way.

use serde::{Deserialize, Serialize};

/// A row from the `exchange_rates` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRateRow {
    /// Internal row id (UUID v4).
    pub id: String,
    /// ISO-4217 currency code to convert from.
    pub from_currency: String,
    /// ISO-4217 currency code to convert to.
    pub to_currency: String,
    /// Conversion rate as integer millionths: `rate = rate_millionths / 1_000_000`.
    /// Multiply [`crate::Money`] amounts in `from_currency` by this value and
    /// divide by `1_000_000` to obtain the equivalent in `to_currency`.
    pub rate_millionths: i64,
    /// Source of the rate (e.g. "manual", "ECB", "OANDA").
    pub source: String,
    /// ISO-8601 date this rate is effective from.
    pub effective_date: String,
    /// ISO-8601 row creation timestamp.
    pub created_at: String,
}

impl ExchangeRateRow {
    /// Backoffice-line formatting for logs / CLI output.
    ///
    /// Renders the rate with up to 6 fractional digits, trimming trailing
    /// zeros so `920_000` → `"0.92"` and `149_500_000` → `"149.5"`. The
    /// boundary `<= 0` (negative or zero) is rejected at the Store layer,
    /// not here; this helper formats whatever was persisted.
    pub fn display_rate(&self) -> String {
        format_rate(self.rate_millionths)
    }
}

/// Format an `i64` rate-millionths value as a display string with up to 6
/// fractional digits and trailing zeros trimmed.
///
/// Extracted from [`ExchangeRateRow::display_rate`] so it can be reused by
/// test helpers and external formatting paths without going through a
/// struct instance.
fn format_rate(millionths: i64) -> String {
    let (int_part, frac_part) = (millionths / 1_000_000, (millionths % 1_000_000).abs());
    let sign = if millionths < 0 { "-" } else { "" };
    if frac_part == 0 {
        return format!("{sign}{int_part}");
    }
    // Pad to 6 digits, trim trailing zeros.
    let mut s = format!("{frac_part:06}");
    while s.ends_with('0') {
        s.pop();
    }
    format!("{sign}{int_part}.{s}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ────────────────────────────────────────────────────

    #[test]
    fn exchange_rate_row_construction() {
        let row = ExchangeRateRow {
            id: "rate-1".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate_millionths: 920_000, // 0.92
            source: "ECB".into(),
            effective_date: "2026-06-01".into(),
            created_at: "2026-06-01T12:00:00.000Z".into(),
        };
        assert_eq!(row.id, "rate-1");
        assert_eq!(row.from_currency, "USD");
        assert_eq!(row.to_currency, "EUR");
        assert_eq!(row.rate_millionths, 920_000);
        assert_eq!(row.source, "ECB");
        assert_eq!(row.effective_date, "2026-06-01");
    }

    #[test]
    fn exchange_rate_row_serde_roundtrip() {
        let row = ExchangeRateRow {
            id: "rate-2".into(),
            from_currency: "GBP".into(),
            to_currency: "JPY".into(),
            rate_millionths: 185_420_000, // 185.42
            source: "manual".into(),
            effective_date: "2026-06-15".into(),
            created_at: "2026-06-15T08:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&row).unwrap();
        let deserialized: ExchangeRateRow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, row.id);
        assert_eq!(deserialized.from_currency, row.from_currency);
        assert_eq!(deserialized.to_currency, row.to_currency);
        assert_eq!(deserialized.rate_millionths, row.rate_millionths);
        assert_eq!(deserialized.source, row.source);
        assert_eq!(deserialized.effective_date, row.effective_date);
        assert_eq!(deserialized.created_at, row.created_at);
    }

    #[test]
    fn exchange_rate_row_json_field_names() {
        let row = ExchangeRateRow {
            id: "r1".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate_millionths: 850_000, // 0.85
            source: "ECB".into(),
            effective_date: "2026-07-01".into(),
            created_at: "2026-07-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["id"], "r1");
        assert_eq!(json["from_currency"], "USD");
        assert_eq!(json["to_currency"], "EUR");
        assert_eq!(json["rate_millionths"].as_i64().unwrap(), 850_000);
        assert_eq!(json["source"], "ECB");
        assert_eq!(json["effective_date"], "2026-07-01");
        assert_eq!(json["created_at"], "2026-07-01T00:00:00.000Z");
    }

    #[test]
    fn exchange_rate_row_partial_eq() {
        let a = ExchangeRateRow {
            id: "r1".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate_millionths: 920_000,
            source: "ECB".into(),
            effective_date: "2026-06-01".into(),
            created_at: "2026-06-01T12:00:00.000Z".into(),
        };
        let b = ExchangeRateRow {
            id: "r1".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate_millionths: 920_000,
            source: "ECB".into(),
            effective_date: "2026-06-01".into(),
            created_at: "2026-06-01T12:00:00.000Z".into(),
        };
        assert_eq!(a, b);
        let c = ExchangeRateRow {
            rate_millionths: 910_000,
            ..a.clone()
        };
        assert_ne!(a, c);
    }

    // ── display_rate formatting ────────────────────────────────────────

    #[test]
    fn display_rate_zero() {
        let row = ExchangeRateRow {
            id: "r".into(),
            from_currency: "USD".into(),
            to_currency: "USD".into(),
            rate_millionths: 0,
            source: "manual".into(),
            effective_date: "2026-01-01".into(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
        };
        assert_eq!(row.display_rate(), "0");
    }

    #[test]
    fn display_rate_integer_value() {
        let row = ExchangeRateRow {
            id: "r".into(),
            from_currency: "BTC".into(),
            to_currency: "USD".into(),
            rate_millionths: 50_000_000, // 50.0
            source: "market".into(),
            effective_date: "2026-06-20".into(),
            created_at: "2026-06-20T00:00:00.000Z".into(),
        };
        assert_eq!(row.display_rate(), "50");
    }

    #[test]
    fn display_rate_two_decimals() {
        let row = ExchangeRateRow {
            id: "r".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate_millionths: 920_000, // 0.92
            source: "ECB".into(),
            effective_date: "2026-06-28".into(),
            created_at: "2026-06-28T00:00:00.000Z".into(),
        };
        assert_eq!(row.display_rate(), "0.92");
    }

    #[test]
    fn display_rate_three_decimals() {
        let row = ExchangeRateRow {
            id: "r".into(),
            from_currency: "USD".into(),
            to_currency: "JPY".into(),
            rate_millionths: 149_500_000, // 149.5
            source: "ecb".into(),
            effective_date: "2026-06-28".into(),
            created_at: "2026-06-28T00:00:00.000Z".into(),
        };
        assert_eq!(row.display_rate(), "149.5");
    }

    #[test]
    fn display_rate_six_decimals_kept() {
        let row = ExchangeRateRow {
            id: "r".into(),
            from_currency: "JPY".into(),
            to_currency: "KWD".into(),
            rate_millionths: 250, // 0.00025
            source: "manual".into(),
            effective_date: "2026-01-01".into(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
        };
        assert_eq!(row.display_rate(), "0.00025");
    }

    #[test]
    fn display_rate_negative_is_formatted_with_sign() {
        let row = ExchangeRateRow {
            id: "r".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate_millionths: -500_000, // -0.50
            source: "manual".into(),
            effective_date: "2026-01-01".into(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
        };
        assert_eq!(row.display_rate(), "-0.5");
    }

    #[test]
    fn display_rate_large_value_trims_to_clean_form() {
        let row = ExchangeRateRow {
            id: "r".into(),
            from_currency: "BTC".into(),
            to_currency: "USD".into(),
            rate_millionths: 102_345_670_000, // 102345.67
            source: "market".into(),
            effective_date: "2026-06-20".into(),
            created_at: "2026-06-20T00:00:00.000Z".into(),
        };
        assert_eq!(row.display_rate(), "102345.67");
    }
}
