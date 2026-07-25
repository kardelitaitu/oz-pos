//! Shared DTO types for exchange-rate Tauri commands.
//!
//! Moved here during R2 Phase 2 so desktop and tablet clients can
//! import the same types instead of duplicating them.

use serde::{Deserialize, Serialize};

use crate::ExchangeRateRow;

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
mod tests {
    use super::*;

    // ── ExchangeRateDto ─────────────────────────────────────────────────

    #[test]
    fn exchange_rate_dto_debug() {
        let dto = ExchangeRateDto {
            id: "e1".into(),
            from_currency: "USD".into(),
            to_currency: "IDR".into(),
            rate_millionths: 16_200_000_000, // 16200.0
            source: "manual".into(),
            effective_date: "2025-01-01".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("USD"));
        assert!(d.contains("IDR"));
    }

    #[test]
    fn exchange_rate_dto_serialize() {
        let dto = ExchangeRateDto {
            id: "e2".into(),
            from_currency: "EUR".into(),
            to_currency: "USD".into(),
            rate_millionths: 1_080_000, // 1.08
            source: "api".into(),
            effective_date: "2025-02-01".into(),
            created_at: "2025-02-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["from_currency"], "EUR");
        assert_eq!(json["rate_millionths"].as_i64().unwrap(), 1_080_000);
    }

    #[test]
    fn exchange_rate_dto_from_row() {
        let row = ExchangeRateRow {
            id: "e3".into(),
            from_currency: "JPY".into(),
            to_currency: "USD".into(),
            rate_millionths: 7_000, // 0.007
            source: "manual".into(),
            effective_date: "2025-03-01".into(),
            created_at: "2025-03-01T00:00:00.000Z".into(),
        };
        let dto = ExchangeRateDto::from(row);
        assert_eq!(dto.from_currency, "JPY");
        assert_eq!(dto.rate_millionths, 7_000);
    }

    // ── CreateExchangeRateArgs ──────────────────────────────────────────

    #[test]
    fn create_exchange_rate_args_deserialize_minimal() {
        let json =
            r#"{"from_currency":"USD","to_currency":"IDR","rate_millionths":16200000000}"#;
        let args: CreateExchangeRateArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.from_currency, "USD");
        assert_eq!(args.rate_millionths, 16_200_000_000);
        assert_eq!(args.source, None);
        assert_eq!(args.effective_date, None);
    }

    #[test]
    fn create_exchange_rate_args_debug() {
        let args = CreateExchangeRateArgs {
            from_currency: "F".into(),
            to_currency: "T".into(),
            rate_millionths: 1_000_000, // 1.0
            source: Some("api".into()),
            effective_date: None,
        };
        let d = format!("{args:?}");
        assert!(d.contains("F"));
    }
}
