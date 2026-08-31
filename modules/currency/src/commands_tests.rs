//! Sibling unit tests for `commands.rs` (AGENTS.md: no tests in
//! production files).

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

// ── CurrencyDto ─────────────────────────────────────────────────────

#[test]
fn currency_dto_debug() {
    let dto = CurrencyDto {
        code: "EUR".into(),
        name: "Euro".into(),
        minor_exponent: 2,
        symbol: "\u{20ac}".into(),
    };
    let d = format!("{dto:?}");
    assert!(d.contains("Euro"));
}

#[test]
fn currency_dto_serialize() {
    let dto = CurrencyDto {
        code: "JPY".into(),
        name: "Yen".into(),
        minor_exponent: 0,
        symbol: "\u{a5}".into(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["code"], "JPY");
    assert_eq!(json["minor_exponent"], 0);
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
fn create_exchange_rate_args_deserialize_full() {
    let json = r#"{
            "from_currency": "USD",
            "to_currency": "IDR",
            "rate_millionths": 16200000000,
            "source": "ecb",
            "effective_date": "2026-07-01"
        }"#;
    let args: CreateExchangeRateArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.from_currency, "USD");
    assert_eq!(args.to_currency, "IDR");
    assert_eq!(args.rate_millionths, 16_200_000_000);
    assert_eq!(args.source.as_deref(), Some("ecb"));
    assert_eq!(args.effective_date.as_deref(), Some("2026-07-01"));
}

#[test]
fn create_exchange_rate_args_rejects_missing_required_field() {
    // from_currency is required; omitting it must fail to deserialize.
    let json = r#"{"to_currency":"IDR","rate_millionths":16200000000}"#;
    assert!(serde_json::from_str::<CreateExchangeRateArgs>(json).is_err());
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
