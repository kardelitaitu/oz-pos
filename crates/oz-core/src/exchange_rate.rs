//! Exchange rate types.

use serde::{Deserialize, Serialize};

/// A row from the `exchange_rates` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRateRow {
    pub id: String,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: f64,
    pub source: String,
    pub effective_date: String,
    pub created_at: String,
}
