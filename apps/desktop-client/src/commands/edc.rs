/*
last audited 31-08-26 by DSH-Agent (EDC commands rewired onto the HAL registry)
crate: oz-pos-app | status: SAFE | lint: CLEAN
findings: previously read a hardcoded AppState field holding an armed MockEdcTerminal, so any operator with SALES_PROCESS could call edc_sale and receive success:true with a Visa last4 and an auth code while no card terminal existed — the response shape gave the caller no way to tell it was fake. Nothing in ui/ imports edcSale yet, so it was latent rather than live. Now resolves through the registry and fails closed with HalErrorKind::NotFound. The hand-rolled five-arm status match is gone: TerminalStatus derives Serialize with rename_all camelCase, so the wire labels are produced by the compiler and a new variant cannot silently fall through to an unhandled arm.
next: accept a terminal_id argument once more than one terminal can be configured | perf: N/A
*/
//! EDC card-terminal commands.
//!
//! Card-present payment through whatever terminal the operator configured.
//! The terminal is resolved from [`AppState`]'s HAL registry rather than
//! held on a field, so a card tender can only reach hardware that exists.
//!
//! With no terminal registered every command here fails with
//! [`HalErrorKind::NotFound`]. That is deliberate: the drivers behind this
//! surface are stubs until a vendor protocol ships, and a payment result
//! that looks approved is worse than one that fails.

use serde::Serialize;
use tauri::State;

use oz_hal::{EdcPaymentResult, EdcTerminal, HalErrorKind, TerminalStatus};

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

/// Registry id the card tender uses.
///
/// Single-terminal for now: `edc_terminals` CRUD is still a stub, so there
/// is no per-row id to address yet. When configuration lands this becomes an
/// argument on each command.
pub const DEFAULT_TERMINAL_ID: &str = "default";

/// Terminal status, serialisable for the front-end.
///
/// `TerminalStatus` is `#[serde(rename_all = "camelCase")]`, so this emits
/// the same `"ready" | "busy" | "offline" | "paperError" | "error"` strings
/// the previous hand-written match produced and `ui/src/api/edc.ts` expects.
#[derive(Debug, Serialize)]
pub struct EdcStatusDto {
    /// Current status as the terminal reported it.
    pub status: TerminalStatus,
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

impl From<EdcPaymentResult> for EdcResultDto {
    fn from(r: EdcPaymentResult) -> Self {
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

/// Resolve the configured card terminal, or fail closed.
///
/// The error is `Hardware`/`NotFound` rather than `Invalid`: nothing the
/// caller sent was wrong, the register simply has no card reader.
async fn resolve_terminal(state: &AppState) -> Result<std::sync::Arc<dyn EdcTerminal>, AppError> {
    state
        .registry
        .terminal(DEFAULT_TERMINAL_ID)
        .await
        .ok_or_else(|| AppError::Hardware {
            sub_kind: HalErrorKind::NotFound,
            message: "no card terminal configured — add one under Settings › Hardware".into(),
        })
}

/// Parse a minor-units amount and currency from the front-end.
fn parse_amount(amount_minor: i64, currency: &str) -> Result<foundation::Money, AppError> {
    let parsed = currency
        .parse::<foundation::Currency>()
        .map_err(|_| AppError::Invalid(format!("invalid currency code: {currency}")))?;
    Ok(foundation::Money {
        minor_units: amount_minor,
        currency: parsed,
    })
}

/// Query the EDC terminal's current status.
#[tauri::command]
pub async fn edc_terminal_status(state: State<'_, AppState>) -> Result<EdcStatusDto, AppError> {
    let terminal = resolve_terminal(&state).await?;
    Ok(EdcStatusDto {
        status: terminal.status().await?,
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
    let amount = parse_amount(amount_minor, &currency)?;
    let terminal = resolve_terminal(&state).await?;
    Ok(terminal.sale(amount).await?.into())
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
    let amount = parse_amount(amount_minor, &currency)?;
    let terminal = resolve_terminal(&state).await?;
    Ok(terminal.refund(&transaction_id, Some(amount)).await?.into())
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
    let terminal = resolve_terminal(&state).await?;
    Ok(terminal.void(&transaction_id).await?.into())
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
