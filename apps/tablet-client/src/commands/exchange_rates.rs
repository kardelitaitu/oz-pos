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
    // CUR-05: field-level validation — a same-currency pair, a non-ISO-4217
    // code, or a malformed effective date must fail here with a
    // field-specific error before any write (mirror of the desktop client).
    if args.from_currency == args.to_currency {
        return Err(AppError::Invalid(
            "from_currency and to_currency must differ".into(),
        ));
    }
    for field in ["from_currency", "to_currency"] {
        let code: &str = if field == "from_currency" {
            &args.from_currency
        } else {
            &args.to_currency
        };
        code.parse::<oz_core::Currency>().map_err(|_| {
            AppError::Invalid(format!("{field}: not a valid ISO-4217 currency code"))
        })?;
    }
    if let Some(date) = &args.effective_date {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            AppError::Invalid(format!("effective_date: must be YYYY-MM-DD, got {date}"))
        })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager as _;

    fn args(from: &str, to: &str, effective_date: Option<&str>) -> CreateExchangeRateArgs {
        CreateExchangeRateArgs {
            from_currency: from.into(),
            to_currency: to.into(),
            rate_millionths: 920_000,
            source: None,
            effective_date: effective_date.map(String::from),
        }
    }

    // ── CUR-05: field-level validation on create_exchange_rate ──

    #[tokio::test]
    async fn create_exchange_rate_rejects_same_currency_pair() {
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test())
            .build(tauri::generate_context!())
            .unwrap();
        let err = create_exchange_rate(args("USD", "USD", None), app.state())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::Invalid(msg) if msg.contains("from_currency")
        ));
    }

    #[tokio::test]
    async fn create_exchange_rate_rejects_non_iso_currency_code() {
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test())
            .build(tauri::generate_context!())
            .unwrap();
        let err = create_exchange_rate(args("US1", "IDR", None), app.state())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::Invalid(msg) if msg.contains("from_currency")
        ));
    }

    #[tokio::test]
    async fn create_exchange_rate_rejects_malformed_effective_date() {
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test())
            .build(tauri::generate_context!())
            .unwrap();
        let err = create_exchange_rate(args("USD", "IDR", Some("2026-02-30")), app.state())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::Invalid(msg) if msg.contains("effective_date")
        ));
    }

    #[tokio::test]
    async fn create_exchange_rate_accepts_valid_input() {
        let conn = oz_core::migrations::fresh_db();
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();
        let dto = create_exchange_rate(args("USD", "IDR", Some("2026-08-11")), app.state())
            .await
            .unwrap();
        assert_eq!(dto.from_currency, "USD");
        assert_eq!(dto.to_currency, "IDR");
        assert_eq!(dto.rate_millionths, 920_000);
    }
}
