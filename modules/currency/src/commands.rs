/*
last audited 25-07-26 by RSA-Agent (modules-currency slice A: commands verified)
crate: modules-currency | status: SAFE | lint: CLEAN
findings: clean shared DTOs with documented millionths scale
next: none | perf: N/A
*/
//! Shared DTO types for exchange-rate Tauri commands.
//!
//! Moved here during R2 Phase 2 so desktop and tablet clients can
//! import the same types instead of duplicating them.

use serde::{Deserialize, Serialize};

use crate::ExchangeRateRow;

/// A currency DTO returned by the `list_currencies` command.
#[derive(Debug, Clone, Serialize)]
pub struct CurrencyDto {
    /// ISO-4217 alpha-3 code, e.g. "USD".
    pub code: String,
    /// Display name, e.g. "US Dollar".
    pub name: String,
    /// Minor-unit exponent (decimal places), e.g. 2 for USD, 0 for JPY.
    pub minor_exponent: u32,
    /// Symbol, e.g. "$", "€", "¥".
    pub symbol: String,
}

/// A serializable exchange-rate DTO returned by Tauri commands.
#[derive(Debug, Clone, Serialize)]
pub struct ExchangeRateDto {
    /// Unique identifier.
    pub id: String,
    /// From Currency.
    pub from_currency: String,
    /// To Currency.
    pub to_currency: String,
    /// Conversion rate as integer millionths: `rate = rate_millionths / 1_000_000`.
    pub rate_millionths: i64,
    /// Source.
    pub source: String,
    /// Effective Date.
    pub effective_date: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

impl From<ExchangeRateRow> for ExchangeRateDto {
    fn from(r: ExchangeRateRow) -> Self {
        Self {
            id: r.id,
            from_currency: r.from_currency,
            to_currency: r.to_currency,
            rate_millionths: r.rate_millionths,
            source: r.source,
            effective_date: r.effective_date,
            created_at: r.created_at,
        }
    }
}

/// Arguments for the `create_exchange_rate` Tauri command.
#[derive(Debug, Deserialize)]
pub struct CreateExchangeRateArgs {
    /// From Currency.
    pub from_currency: String,
    /// To Currency.
    pub to_currency: String,
    /// Conversion rate as integer millionths (e.g. `0.92` → `920_000`).
    pub rate_millionths: i64,
    /// Source.
    pub source: Option<String>,
    /// Effective Date.
    pub effective_date: Option<String>,
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
