
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
