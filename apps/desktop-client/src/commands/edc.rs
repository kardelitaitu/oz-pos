//! EDC payment terminal commands.
//!
//! Card-present payment via the EDC terminal wired into [`AppState`]
//! (currently a success-mode `MockEdcTerminal`). These commands are the
//! IPC surface for the POS screen's card tender.
//!
//! The terminal exposes the [`EdcTerminal`] trait from `oz-payment`; a
//! real wired/wireless terminal replaces the mock without changing this
//! file.

use serde::Serialize;
use tauri::State;

use oz_payment::drivers::edc::TerminalStatus;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

/// Terminal status, serialisable for the front-end.
#[derive(Debug, Serialize)]
pub struct EdcStatusDto {
    /// One of: ready, busy, offline, paperError, error.
    pub status: String,
}

/// Result of a card-present sale/refund/void.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdcResultDto {
    /// Whether the transaction was approved.
    pub success: bool,
    /// Gateway / acquirer transaction id (present on success).
    pub transaction_id: Option<String>,
    /// Authorisation code from the card network.
    pub auth_code: Option<String>,
    /// Card scheme (e.g. "Visa").
    pub card_scheme: Option<String>,
    /// Last 4 digits of the card.
    pub card_last4: Option<String>,
    /// Human-readable message.
    pub message: String,
}

impl From<oz_payment::drivers::edc::PaymentResult> for EdcResultDto {
    fn from(r: oz_payment::drivers::edc::PaymentResult) -> Self {
        Self {
            success: r.success,
            transaction_id: r.transaction_id,
            auth_code: r.auth_code,
            card_scheme: r.card_scheme,
            card_last4: r.card_last4,
            message: r.message,
        }
    }
}

/// Query the EDC terminal's current status.
#[tauri::command]
pub async fn edc_terminal_status(state: State<'_, AppState>) -> Result<EdcStatusDto, AppError> {
    let status = state
        .edc_terminal
        .status()
        .await
        .map_err(|e| AppError::Internal(format!("EDC status: {e}")))?;
    let label = match status {
        TerminalStatus::Ready => "ready",
        TerminalStatus::Busy => "busy",
        TerminalStatus::Offline => "offline",
        TerminalStatus::PaperError => "paperError",
        TerminalStatus::Error => "error",
    };
    Ok(EdcStatusDto {
        status: label.into(),
    })
}

/// Process a card-present sale (authorize + capture in one call).
///
/// `amount_minor` is in the currency's minor units (e.g. cents for USD,
/// rupiah for IDR). `currency` is an ISO-4217 code.
#[tauri::command]
pub async fn edc_sale(
    session_token: String,
    state: State<'_, AppState>,
    amount_minor: i64,
    currency: String,
) -> Result<EdcResultDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::SALES_PROCESS).await?;
    let currency = currency
        .parse::<foundation::Currency>()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {currency}")))?;
    let amount = foundation::Money {
        minor_units: amount_minor,
        currency,
    };
    let result = state
        .edc_terminal
        .sale(amount)
        .await
        .map_err(|e| AppError::Internal(format!("EDC sale: {e}")))?;
    Ok(result.into())
}

/// Refund a previously captured card transaction.
#[tauri::command]
pub async fn edc_refund(
    session_token: String,
    state: State<'_, AppState>,
    transaction_id: String,
    amount_minor: i64,
    currency: String,
) -> Result<EdcResultDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::SALES_REFUND).await?;
    let currency = currency
        .parse::<foundation::Currency>()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {currency}")))?;
    let amount = foundation::Money {
        minor_units: amount_minor,
        currency,
    };
    let result = state
        .edc_terminal
        .refund(&transaction_id, Some(amount))
        .await
        .map_err(|e| AppError::Internal(format!("EDC refund: {e}")))?;
    Ok(result.into())
}

/// Void a pending authorisation before capture.
#[tauri::command]
pub async fn edc_void(
    session_token: String,
    state: State<'_, AppState>,
    transaction_id: String,
) -> Result<EdcResultDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::SALES_VOID).await?;
    let result = state
        .edc_terminal
        .void(&transaction_id)
        .await
        .map_err(|e| AppError::Internal(format!("EDC void: {e}")))?;
    Ok(result.into())
}

/// Session-scoped variant of [`edc_terminal_status`].
#[tauri::command]
pub async fn edc_terminal_status_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<EdcStatusDto, AppError> {
    let _session = state.resolve_session(&session_token)?;
    edc_terminal_status(state).await
}

#[cfg(test)]
#[path = "edc_tests.rs"]
mod tests;
