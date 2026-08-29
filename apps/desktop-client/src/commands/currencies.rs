//! Currency-lookup command for the front-end.
//!
//! R2 Phase 3: `list_currencies` migrated to use [`modules_currency::repository::CurrencyRepository`]
//! directly. `CurrencyDto` now comes from [`modules_currency::commands`].

use serde::{Deserialize, Serialize};
use tauri::State;

use modules_currency::commands::CurrencyDto;
use modules_currency::repository::CurrencyRepository;

use crate::commands::authz::require_permission_for_session;
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

#[tauri::command]
/// Currency info.
pub async fn currency_info(code: String) -> Result<CurrencyInfo, AppError> {
    let currency: oz_core::Currency = code
        .parse()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {code}")))?;
    Ok(CurrencyInfo {
        code: String::from_utf8_lossy(&currency.0).into_owned(),
        exponent: currency.minor_unit_exponent(),
    })
}

#[tauri::command]
/// List currencies.
pub async fn list_currencies(state: State<'_, AppState>) -> Result<Vec<CurrencyDto>, AppError> {
    let db = state.db.lock().await;
    let repo = CurrencyRepository::new(&db);
    Ok(repo.list_currencies()?)
}

#[tauri::command]
/// List currencies resolved from a session token. ADR #7.
pub async fn list_currencies_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<CurrencyDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let repo = CurrencyRepository::new(&db);
    Ok(repo.list_currencies()?)
}

#[derive(Debug, Deserialize)]
/// Setdefaultcurrencyargs.
pub struct SetDefaultCurrencyArgs {
    /// Code.
    pub code: String,
}

#[tauri::command]
/// Get default currency.
pub async fn get_default_currency(state: State<'_, AppState>) -> Result<Option<String>, AppError> {
    let db = state.db.lock().await;
    let repo = CurrencyRepository::new(&db);
    Ok(repo.get_default_currency()?)
}

#[tauri::command]
/// Set default currency.
pub async fn set_default_currency(
    args: SetDefaultCurrencyArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let repo = CurrencyRepository::new(&db);
    repo.set_default_currency(&args.code)?;
    Ok(())
}

// ── Scoped variants (CUR-03) ─────────────────────────────────────────
//
// The default-currency commands above operate on the global database and
// are kept only as compatibility wrappers for single-store deployments.
// Scoped variants resolve the store from the session token and enforce
// `SETTINGS_READ` / `SETTINGS_EDIT` on the backend, so multi-store
// deployments cannot read or mutate another store's currency setting.

/// Get the default currency in the store resolved from a session token. ADR #7.
///
/// CUR-03: resolves the store from the session and enforces
/// `SETTINGS_READ` on the backend.
#[tauri::command]
pub async fn get_default_currency_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::SETTINGS_READ).await?;
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let repo = CurrencyRepository::new(&db);
    let out = repo.get_default_currency()?;
    drop(db);
    Ok(out)
}

/// Set the default currency in the store resolved from a session token. ADR #7.
///
/// CUR-03: resolves the store from the session and enforces
/// `SETTINGS_EDIT` on the backend. Validates the code is a well-formed
/// ISO-4217 code before persisting (mirror of the exchange-rate path).
#[tauri::command]
pub async fn set_default_currency_scoped(
    session_token: String,
    args: SetDefaultCurrencyArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    args.code
        .parse::<oz_core::Currency>()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {}", args.code)))?;
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::SETTINGS_EDIT).await?;
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let repo = CurrencyRepository::new(&db);
    repo.set_default_currency(&args.code)?;
    drop(db);
    Ok(())
}

/// Session-scoped variant of [`currency_info`].
#[tauri::command]
pub async fn currency_info_scoped(
    session_token: String,
    code: String,
    state: State<'_, AppState>,
) -> Result<CurrencyInfo, AppError> {
    let _session = state.resolve_session(&session_token)?;
    currency_info(code).await
}

#[cfg(test)]
#[path = "currencies_tests.rs"]
mod tests;
