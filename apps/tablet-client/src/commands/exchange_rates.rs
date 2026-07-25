//! Exchange rate commands.
//!
//! R2 Phase 2: DTO types moved to [`modules_currency::commands`].
//! The command handler functions themselves remain here because they
//! depend on Tauri-specific types (`State`, `#[command]`) and the
//! app-level `AppState` / `AppError`.

use tauri::{State, command};

use modules_currency::commands::{CreateExchangeRateArgs, ExchangeRateDto};
use modules_currency::repository::CurrencyRepository;

use crate::error::AppError;
use crate::state::AppState;

#[command]
/// List exchange rates.
pub async fn list_exchange_rates(
    state: State<'_, AppState>,
) -> Result<Vec<ExchangeRateDto>, AppError> {
    let db = state.db.lock().await;
    let repo = CurrencyRepository::new(&db);
    let rows = repo.list_exchange_rates()?;
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
    if args.rate_millionths <= 0 {
        return Err(AppError::Invalid(
            "rate must be strictly positive (zero and negative are not valid exchange rates)"
                .into(),
        ));
    }
    let db = state.db.lock().await;
    let repo = CurrencyRepository::new(&db);
    let date = args
        .effective_date
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let source = args.source.unwrap_or_else(|| "manual".to_string());
    let row = repo.create_exchange_rate(
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
    let repo = CurrencyRepository::new(&db);
    repo.delete_exchange_rate(&id)?;
    Ok(())
}
