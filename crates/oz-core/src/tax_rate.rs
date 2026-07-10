//! Tax rate domain type — named percentages for tax configuration.
//!
//! A [`TaxRate`] stores a human-readable name and a rate in basis
//! points (e.g. 825 = 8.25%). One rate may be marked as the default.
//! Rates are stored in the `tax_rates` table (migration `009_tax_rates.sql`).

use serde::{Deserialize, Serialize};

/// A named tax rate, stored in basis points.
///
/// # Schema mapping
///
/// Maps 1:1 to the `tax_rates` table. The `rate_bps` field uses
/// integer basis points (1/100th of a percent) to avoid floating
/// point altogether.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRate {
    /// Internal row id (UUID v4).
    pub id: String,

    /// Display name (e.g. "Sales Tax", "VAT 20%").
    pub name: String,

    /// Rate in basis points — 1 bps = 0.01 %.
    /// E.g. 825 = 8.25 %, 2000 = 20 %, 0 = tax-exempt.
    pub rate_bps: i64,

    /// Whether this is the default tax rate for the store.
    pub is_default: bool,

    /// Whether tax is included in the displayed price (true) or
    /// added at checkout (false). Defaults to exclusive.
    pub is_inclusive: bool,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl TaxRate {
    /// Create a new tax rate with the given name and basis-point rate.
    ///
    /// Generates a fresh UUID for `id`.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty after trimming, or if `rate_bps` is negative.
    pub fn new(name: impl Into<String>, rate_bps: i64) -> Self {
        let name = name.into().trim().to_owned();
        assert!(!name.is_empty(), "tax rate name must not be empty");
        assert!(rate_bps >= 0, "rate_bps must be non-negative");

        Self {
            id: uuid::Uuid::now_v7().to_string(),
            name,
            rate_bps,
            is_default: false,
            is_inclusive: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// Mark this rate as the store default.
    #[must_use]
    pub fn with_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// Mark this rate as inclusive (tax included in displayed price).
    #[must_use]
    pub fn with_inclusive(mut self) -> Self {
        self.is_inclusive = true;
        self
    }

    /// Get the rate as a display percentage string (e.g. `"8.25%"`).
    pub fn display_rate(&self) -> String {
        let major = self.rate_bps / 100;
        let frac = self.rate_bps % 100;
        if frac == 0 {
            format!("{major}%")
        } else {
            format!("{major}.{frac:02}%")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tax_rate() {
        let t = TaxRate::new("Sales Tax", 825);
        assert_eq!(t.name, "Sales Tax");
        assert_eq!(t.rate_bps, 825);
        assert!(!t.is_default);
    }

    #[test]
    #[should_panic(expected = "tax rate name must not be empty")]
    fn panics_on_empty_name() {
        TaxRate::new("   ", 500);
    }

    #[test]
    #[should_panic(expected = "rate_bps must be non-negative")]
    fn panics_on_negative_rate() {
        TaxRate::new("Bad", -1);
    }

    #[test]
    fn with_default_flag() {
        let t = TaxRate::new("VAT", 2000).with_default();
        assert!(t.is_default);
    }

    #[test]
    fn display_rate_whole_percent() {
        let t = TaxRate::new("VAT", 2000);
        assert_eq!(t.display_rate(), "20%");
    }

    #[test]
    fn display_rate_fractional() {
        let t = TaxRate::new("Sales Tax", 825);
        assert_eq!(t.display_rate(), "8.25%");
    }

    #[test]
    fn display_rate_zero() {
        let t = TaxRate::new("Tax Exempt", 0);
        assert_eq!(t.display_rate(), "0%");
    }

    #[test]
    fn serde_roundtrip() {
        let t = TaxRate::new("VAT", 2000).with_default();
        let json = serde_json::to_string(&t).unwrap();
        let back: TaxRate = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}
