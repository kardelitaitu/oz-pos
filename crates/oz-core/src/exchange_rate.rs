//! Exchange rate types.

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
    /// Conversion rate (multiply `from` amount by this to get `to` amount).
    pub rate: f64,
    /// Source of the rate (e.g. "manual", "ECB", "OANDA").
    pub source: String,
    /// ISO-8601 date this rate is effective from.
    pub effective_date: String,
    /// ISO-8601 row creation timestamp.
    pub created_at: String,
}
