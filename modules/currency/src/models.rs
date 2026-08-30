//! Currency exchange-rate domain model.
//!
//! Rates are stored as **integer minor units** (`rate_millionths: i64`) at a
//! 6-decimal fixed-point scale to keep the exchange-rate domain out of the
//! float-arithmetic error class. `0.92` is represented as `920_000`; the
//! backoffice display path uses [`ExchangeRateRow::display_rate`].
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
    /// boundary `<= 0` (negative or zero) is rejected at the repository layer,
    /// not here; this helper formats whatever was persisted.
    pub fn display_rate(&self) -> String {
        format_rate(self.rate_millionths)
    }
}

/// Format an `i64` rate-millionths value as a display string with up to 6
/// fractional digits and trailing zeros trimmed.
fn format_rate(millionths: i64) -> String {
    let int_part = millionths / 1_000_000;
    let frac_part = (millionths % 1_000_000).abs();
    let sign = if millionths < 0 { "-" } else { "" };
    // Use unsigned abs for display: int_part is negative via truncation-
    // toward-zero (e.g., -1_000_000 / 1_000_000 = -1), and the sign
    // string already carries the sign — so "-1" would become "--1".
    let display_int = int_part.unsigned_abs();
    if frac_part == 0 {
        return format!("{sign}{display_int}");
    }
    // Pad to 6 digits, trim trailing zeros.
    let mut s = format!("{frac_part:06}");
    while s.ends_with('0') {
        s.pop();
    }
    format!("{sign}{display_int}.{s}")
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
