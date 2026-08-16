/*
last audited 12-07-26 by RSA-Agent
crate: oz-pos-app | status: SAFE | lint: CLEAN
findings: closed C-1 (Epic X-3, see audit doc §11); no remaining findings in this file | next: re-audit on next material change | perf: not a hot path
*/

//! Exchange rate commands.
//!
//! R2 Phase 2: DTO types moved to [`modules_currency::commands`].
//! The command handler functions themselves remain here because they
//! depend on Tauri-specific types (`State`, `#[command]`) and the
//! app-level `AppState` / `AppError`.

use tauri::State;

use foundation::validate_not_empty;
use modules_currency::commands::{CreateExchangeRateArgs, ExchangeRateDto};
use modules_currency::repository::CurrencyRepository;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
/// List exchange rates.
pub async fn list_exchange_rates(
    state: State<'_, AppState>,
) -> Result<Vec<ExchangeRateDto>, AppError> {
    let db = state.db.lock().await;
    let repo = CurrencyRepository::new(&db);
    let rows = repo.list_exchange_rates()?;
    Ok(rows.into_iter().map(ExchangeRateDto::from).collect())
}

#[tauri::command]
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
    // CUR-05: field-level validation. The repository/DB only rejects
    // non-empty strings and relies on FKs for currency existence — a
    // same-currency pair, a non-ISO-4217 code, or a malformed effective
    // date would otherwise persist as semantically invalid configuration
    // that the "latest effective rate" selection can never match. Fail here
    // with a field-specific error before any write.
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

#[tauri::command]
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
        // A rate from USD to USD is semantically meaningless and must fail
        // with a field-specific error, not persist (CUR-05).
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
        // "US1" is not a valid ISO-4217 code; the DB FK would accept any
        // 3-letter code the currencies table contains, so the command must
        // validate shape up front (CUR-05).
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
        // 2026-02-30 does not exist; a strict YYYY-MM-DD parse must reject
        // it with a field-specific error instead of persisting a date the
        // "latest effective rate" selection can never match (CUR-05).
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
        // Positive path: valid pair + ISO codes + strict date persist and
        // round-trip through the DTO.
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
