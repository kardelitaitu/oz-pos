/*
last audited 12-07-26 by RSA-Agent
crate: oz-pos-app | status: SAFE | lint: CLEAN
findings: closed C-1 (Epic X-3, see audit doc §11); no remaining findings in this file | next: re-audit on next material change | perf: not a hot path
*/

//! Exchange rate commands.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;

use foundation::validate_not_empty;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
/// Exchangeratedto.
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

impl From<oz_core::exchange_rate::ExchangeRateRow> for ExchangeRateDto {
    fn from(r: oz_core::exchange_rate::ExchangeRateRow) -> Self {
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

#[derive(Debug, Deserialize)]
/// Createexchangerateargs.
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

#[command]
/// List exchange rates.
pub async fn list_exchange_rates(
    state: State<'_, AppState>,
) -> Result<Vec<ExchangeRateDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.list_exchange_rates()?;
    Ok(rows.into_iter().map(ExchangeRateDto::from).collect())
}

#[command]
/// Create exchange rate.
pub async fn create_exchange_rate(
    args: CreateExchangeRateArgs,
    state: State<'_, AppState>,
) -> Result<ExchangeRateDto, AppError> {
    validate_not_empty("from_currency", &args.from_currency)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("to_currency", &args.to_currency)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    if args.rate_millionths <= 0 {
        return Err(AppError::Invalid(
            "rate must be strictly positive (zero and negative are not valid exchange rates)"
                .into(),
        ));
    }
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let date = args
        .effective_date
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let source = args.source.unwrap_or_else(|| "manual".to_string());
    let row = store.create_exchange_rate(
        &args.from_currency,
        &args.to_currency,
        args.rate_millionths,
        &source,
        &date,
    )?;
    Ok(ExchangeRateDto::from(row))
}

#[command]
/// Delete exchange rate.
pub async fn delete_exchange_rate(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_exchange_rate(&id)?;
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::exchange_rate::ExchangeRateRow;

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
        let json = r#"{"from_currency":"USD","to_currency":"IDR","rate_millionths":16200000000}"#;
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
