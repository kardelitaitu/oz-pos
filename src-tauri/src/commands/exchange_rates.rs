//! Exchange rate commands.

use serde::{Deserialize, Serialize};
use tauri::{command, State};

use oz_core::db::Store;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ExchangeRateDto {
    pub id: String,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: f64,
    pub source: String,
    pub effective_date: String,
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
pub struct CreateExchangeRateArgs {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: f64,
    pub source: Option<String>,
    pub effective_date: Option<String>,
}

#[command]
pub async fn list_exchange_rates(
    state: State<'_, AppState>,
) -> Result<Vec<ExchangeRateDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.list_exchange_rates()?;
    Ok(rows.into_iter().map(ExchangeRateDto::from).collect())
}

#[command]
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
    let date = args.effective_date.unwrap_or_else(|| {
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    });
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
pub async fn delete_exchange_rate(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_exchange_rate(&id)?;
    Ok(())
}
