//! Currency-lookup command for the front-end.

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::command;

use crate::error::AppError;
use crate::state::AppState;

/// Currency info returned to the front-end for formatting.
#[derive(Debug, Serialize)]
pub struct CurrencyInfo {
    /// ISO-4217 alpha-3 code, e.g. "USD".
    pub code: String,
    /// Minor-unit exponent (decimal places), e.g. 2 for USD.
    pub exponent: u32,
}

#[command]
pub async fn currency_info(code: String) -> Result<CurrencyInfo, AppError> {
    let currency: oz_core::Currency = code
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {code}")))?;
    Ok(CurrencyInfo {
        code: String::from_utf8_lossy(&currency.0).into_owned(),
        exponent: currency.minor_unit_exponent(),
    })
}

/// A currency DTO for the front-end.
#[derive(Debug, Serialize)]
pub struct CurrencyDto {
    pub code: String,
    pub name: String,
    pub minor_exponent: u32,
    pub symbol: String,
}

#[command]
pub async fn list_currencies(state: State<'_, AppState>) -> Result<Vec<CurrencyDto>, AppError> {
    let db = state.db.lock().await;
    let store = oz_core::db::Store::new(&db);
    let rows = store.list_currencies()?;
    Ok(rows
        .into_iter()
        .map(|(code, name, minor_exponent, symbol)| CurrencyDto {
            code,
            name,
            minor_exponent,
            symbol,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
pub struct SetDefaultCurrencyArgs {
    pub code: String,
}

#[command]
pub async fn get_default_currency(state: State<'_, AppState>) -> Result<Option<String>, AppError> {
    let db = state.db.lock().await;
    let store = oz_core::db::Store::new(&db);
    Ok(store.get_default_currency()?)
}

#[command]
pub async fn set_default_currency(
    args: SetDefaultCurrencyArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = oz_core::db::Store::new(&db);
    store.set_default_currency(&args.code)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn usd_has_exponent_2() {
        let info = currency_info("USD".into()).await.unwrap();
        assert_eq!(info.exponent, 2);
    }

    #[tokio::test]
    async fn jpy_has_exponent_0() {
        let info = currency_info("JPY".into()).await.unwrap();
        assert_eq!(info.exponent, 0);
    }

    #[tokio::test]
    async fn invalid_code_is_error() {
        assert!(currency_info("XX".into()).await.is_err());
    }

    #[test]
    fn currency_info_debug_output() {
        let info = CurrencyInfo {
            code: "USD".into(),
            exponent: 2,
        };
        let d = format!("{info:?}");
        assert!(d.contains("USD"));
        assert!(d.contains("2"));
    }

    #[test]
    fn currency_info_serialize() {
        let info = CurrencyInfo {
            code: "IDR".into(),
            exponent: 0,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["code"], "IDR");
        assert_eq!(json["exponent"], 0);
    }

    #[test]
    fn currency_dto_debug() {
        let dto = CurrencyDto {
            code: "EUR".into(),
            name: "Euro".into(),
            minor_exponent: 2,
            symbol: "€".into(),
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
            symbol: "¥".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["code"], "JPY");
        assert_eq!(json["minor_exponent"], 0);
    }

    #[test]
    fn set_default_currency_args_deserialize() {
        let json = r#"{"code":"USD"}"#;
        let args: SetDefaultCurrencyArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.code, "USD");
    }

    #[test]
    fn set_default_currency_args_debug() {
        let args = SetDefaultCurrencyArgs { code: "IDR".into() };
        let d = format!("{args:?}");
        assert!(d.contains("IDR"));
    }
}
