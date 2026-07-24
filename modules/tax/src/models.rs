//! Tax domain models.

use serde::{Deserialize, Serialize};

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
