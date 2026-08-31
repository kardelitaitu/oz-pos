/*
last audited 25-07-26 by RSA-Agent (modules-tax slice A: models deep read)
crate: modules-tax | status: SAFE | lint: CLEAN
findings: exemplary — TAX-05 integer-only rounding with HalfUp default (jurisdiction-defensible) and legacy Truncate documented for backward compat; overflow-checked divide; bps math with rejection tests
next: none | perf: N/A
*/
//! Tax domain models.

use serde::{Deserialize, Serialize};

/// Rounding policy applied to fractional tax amounts.
///
/// TAX-05: tax is computed in integer minor units, so a per-line/rate
/// result like `333.5` must be reduced to a whole minor unit. The legacy
/// behavior silently truncated toward zero (understating tax); [`HalfUp`](Self::HalfUp)
/// is the recommended, jurisdiction-defensible default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundingMode {
    /// Truncate toward zero — legacy behavior, kept for backward
    /// compatibility with previously recorded sales.
    Truncate,
    /// Round half away from zero (`0.5` → `1`) — recommended default.
    ///
    /// The integer implementation assumes non-negative numerators, which
    /// is the only domain tax math produces (base and rate are both ≥ 0).
    #[default]
    HalfUp,
}

impl RoundingMode {
    /// Stable wire/storage name (matches the `serde` `snake_case` form
    /// declared by `#[serde(rename_all = "snake_case")]` on this enum —
    /// keep the two in sync if the rename changes).
    #[must_use]
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Truncate => "truncate",
            Self::HalfUp => "half_up",
        }
    }

    /// Divide `numerator / divisor` and round per this mode using
    /// integer-only arithmetic (no floats).
    ///
    /// `HalfUp` computes `(numerator + divisor/2) / divisor`, which rounds
    /// ties away from zero for non-negative inputs (the only domain tax
    /// math produces). `divisor` must be strictly positive.
    ///
    /// Returns `None` if the intermediate `numerator + divisor/2` would
    /// overflow `i64`.
    #[must_use]
    pub fn divide(self, numerator: i64, divisor: i64) -> Option<i64> {
        debug_assert!(divisor > 0, "divisor must be positive");
        match self {
            Self::Truncate => Some(numerator / divisor),
            Self::HalfUp => numerator.checked_add(divisor / 2).map(|n| n / divisor),
        }
    }
}

/// A named tax rate, stored in basis points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRate {
    /// Internal row id (UUID v4).
    pub id: String,
    /// Display name (e.g. "Sales Tax", "VAT 20%").
    pub name: String,
    /// Rate in basis points — 1 bps = 0.01 %.
    pub rate_bps: i64,
    /// Whether this is the default tax rate for the store.
    pub is_default: bool,
    /// Whether tax is included in the displayed price (true) or added at checkout (false).
    pub is_inclusive: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl TaxRate {
    /// Create a new tax rate.
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

    /// Mark this rate as inclusive.
    #[must_use]
    pub fn with_inclusive(mut self) -> Self {
        self.is_inclusive = true;
        self
    }

    /// Get display percentage string.
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

    // ── RoundingMode::divide ────────────────────────────────────────

    #[test]
    fn halfup_exact_division() {
        assert_eq!(RoundingMode::HalfUp.divide(10, 2).unwrap(), 5);
    }

    #[test]
    fn halfup_rounds_half_away_from_zero() {
        // 5 / 2 = 2.5 → rounds to 3 (half away from zero)
        assert_eq!(RoundingMode::HalfUp.divide(5, 2).unwrap(), 3);
    }

    #[test]
    fn halfup_rounds_exactly_half_to_upper() {
        // 15 / 10 = 1.5 → rounds to 2
        assert_eq!(RoundingMode::HalfUp.divide(15, 10).unwrap(), 2);
    }

    #[test]
    fn halfup_truncates_when_below_half() {
        // 14 / 10 = 1.4 → rounds to 1
        assert_eq!(RoundingMode::HalfUp.divide(14, 10).unwrap(), 1);
    }

    #[test]
    fn halfup_zero_numerator() {
        assert_eq!(RoundingMode::HalfUp.divide(0, 7).unwrap(), 0);
    }

    #[test]
    fn halfup_numerator_less_than_divisor() {
        // 3 / 10 = 0.3 → rounds to 0
        assert_eq!(RoundingMode::HalfUp.divide(3, 10).unwrap(), 0);
    }

    #[test]
    fn halfup_large_values() {
        // Typical tax scenario: 33333 / 3 (e.g. 333.33 minor units / 3 items)
        assert_eq!(RoundingMode::HalfUp.divide(33333, 3).unwrap(), 11111);
    }

    #[test]
    fn halfup_overflow_returns_none() {
        // i64::MAX / 2 would overflow when adding divisor/2
        let result = RoundingMode::HalfUp.divide(i64::MAX, 2);
        assert!(result.is_none());
    }

    #[test]
    fn truncate_exact_division() {
        assert_eq!(RoundingMode::Truncate.divide(10, 2).unwrap(), 5);
    }

    #[test]
    fn truncate_rounds_down() {
        // 5 / 2 = 2.5 → truncates to 2
        assert_eq!(RoundingMode::Truncate.divide(5, 2).unwrap(), 2);
    }

    #[test]
    fn truncate_rounds_toward_zero() {
        // 15 / 10 = 1.5 → truncates to 1
        assert_eq!(RoundingMode::Truncate.divide(15, 10).unwrap(), 1);
    }

    #[test]
    fn truncate_zero_numerator() {
        assert_eq!(RoundingMode::Truncate.divide(0, 7).unwrap(), 0);
    }

    // ── RoundingMode::wire_name ─────────────────────────────────────

    #[test]
    fn wire_name_matches_serde_representation() {
        assert_eq!(RoundingMode::HalfUp.wire_name(), "half_up");
        assert_eq!(RoundingMode::Truncate.wire_name(), "truncate");
    }

    // ── RoundingMode serde roundtrip ────────────────────────────────

    #[test]
    fn rounding_mode_serde_roundtrip() {
        for mode in [RoundingMode::HalfUp, RoundingMode::Truncate] {
            let json = serde_json::to_string(&mode).unwrap();
            let back: RoundingMode = serde_json::from_str(&json).unwrap();
            assert_eq!(back, mode);
        }
    }

    #[test]
    fn rounding_mode_default_is_halfup() {
        assert_eq!(RoundingMode::default(), RoundingMode::HalfUp);
    }

    // ── TaxRate ─────────────────────────────────────────────────────

    #[test]
    fn tax_rate_new_sets_fields() {
        let rate = TaxRate::new("VAT", 2100); // 21%
        assert_eq!(rate.name, "VAT");
        assert_eq!(rate.rate_bps, 2100);
        assert!(!rate.is_default);
        assert!(!rate.is_inclusive);
    }

    #[test]
    fn tax_rate_new_trims_name() {
        let rate = TaxRate::new("  Sales Tax  ", 825);
        assert_eq!(rate.name, "Sales Tax");
    }

    #[test]
    #[should_panic(expected = "tax rate name must not be empty")]
    fn tax_rate_new_rejects_empty_name() {
        TaxRate::new("  ", 100);
    }

    #[test]
    #[should_panic(expected = "rate_bps must be non-negative")]
    fn tax_rate_new_rejects_negative_rate() {
        TaxRate::new("Bad", -1);
    }

    #[test]
    fn tax_rate_new_allows_zero_rate() {
        let rate = TaxRate::new("Zero", 0);
        assert_eq!(rate.rate_bps, 0);
    }

    #[test]
    fn tax_rate_with_default() {
        let rate = TaxRate::new("VAT", 2100).with_default();
        assert!(rate.is_default);
    }

    #[test]
    fn tax_rate_with_inclusive() {
        let rate = TaxRate::new("VAT", 2100).with_inclusive();
        assert!(rate.is_inclusive);
    }

    #[test]
    fn tax_rate_new_generates_unique_id() {
        let a = TaxRate::new("A", 100);
        let b = TaxRate::new("B", 200);
        assert_ne!(a.id, b.id);
    }

    // ── TaxRate::display_rate ───────────────────────────────────────

    #[test]
    fn display_rate_whole_percent() {
        let rate = TaxRate::new("VAT", 2100); // 21.00%
        assert_eq!(rate.display_rate(), "21%");
    }

    #[test]
    fn display_rate_fractional_percent() {
        let rate = TaxRate::new("Tax", 825); // 8.25%
        assert_eq!(rate.display_rate(), "8.25%");
    }

    #[test]
    fn display_rate_small_fraction() {
        let rate = TaxRate::new("Tax", 150); // 1.50%
        assert_eq!(rate.display_rate(), "1.50%");
    }

    #[test]
    fn display_rate_zero_percent() {
        let rate = TaxRate::new("Zero", 0);
        assert_eq!(rate.display_rate(), "0%");
    }

    #[test]
    fn display_rate_one_percent() {
        let rate = TaxRate::new("One", 100);
        assert_eq!(rate.display_rate(), "1%");
    }

    // ── TaxRate serde roundtrip ─────────────────────────────────────

    #[test]
    fn tax_rate_serde_roundtrip() {
        let rate = TaxRate::new("VAT", 2100).with_default().with_inclusive();
        let json = serde_json::to_string(&rate).unwrap();
        let back: TaxRate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "VAT");
        assert_eq!(back.rate_bps, 2100);
        assert!(back.is_default);
        assert!(back.is_inclusive);
    }
}
