//! Exchange rate commands.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;

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
    /// Rate.
    pub rate: f64,
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
            rate: r.rate,
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
    /// Rate.
    pub rate: f64,
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
    if args.from_currency.trim().is_empty() || args.to_currency.trim().is_empty() {
        return Err(AppError::Invalid("Currency codes must not be empty".into()));
    }
    if args.rate <= 0.0 {
        return Err(AppError::Invalid("Rate must be positive".into()));
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
        args.rate,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_rate_dto_debug() {
        let dto = ExchangeRateDto {
            id: "er1".into(),
            from_currency: "USD".into(),
            to_currency: "IDR".into(),
            rate: 15600.0,
            source: "manual".into(),
            effective_date: "2026-01-15".into(),
            created_at: "2026-01-15T10:00:00Z".into(),
        };
        let debug = format!("{:?}", dto);
        assert!(debug.contains("USD"));
        assert!(debug.contains("15600"));
    }

    #[test]
    fn exchange_rate_dto_serialize() {
        let dto = ExchangeRateDto {
            id: "er1".into(),
            from_currency: "EUR".into(),
            to_currency: "USD".into(),
            rate: 1.08,
            source: "api".into(),
            effective_date: "2026-01-15".into(),
            created_at: "2026-01-15T10:00:00Z".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["id"], "er1");
        assert_eq!(json["from_currency"], "EUR");
        assert_eq!(json["rate"], 1.08);
    }

    #[test]
    fn create_exchange_rate_args_deserialize() {
        let json = r#"{"from_currency":"USD","to_currency":"IDR","rate":15600.0,"source":"api","effective_date":"2026-01-15"}"#;
        let args: CreateExchangeRateArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.from_currency, "USD");
        assert_eq!(args.rate, 15600.0);
        assert_eq!(args.source.unwrap(), "api");
    }

    #[test]
    fn create_exchange_rate_args_debug() {
        let args = CreateExchangeRateArgs {
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate: 0.92,
            source: None,
            effective_date: None,
        };
        let debug = format!("{:?}", args);
        assert!(debug.contains("0.92"));
    }
}
