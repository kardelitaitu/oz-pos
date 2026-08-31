//! Void sale command — void a completed sale and restore stock.
//!
//! Delegates to `Store::void_sale` which handles the status transition,
//! stock restoration, and audit logging inside a single transaction.

use serde::Deserialize;
use tauri::State;

use oz_core::permissions;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Voidsaleargs.
pub struct VoidSaleArgs {
    /// ID of the associated sale.
    pub sale_id: String,
    /// ID of the associated user.
    pub user_id: String,
    /// Reason.
    pub reason: String,
}

/// Args for `void_sale_scoped` — identical to `VoidSaleArgs` but without
/// `user_id` (read from the session token instead).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidSaleScopedArgs {
    /// ID of the associated sale.
    pub sale_id: String,
    /// Reason.
    pub reason: String,
}

/// Void a sale within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `void_sale`. The `user_id` for permission
/// checks and the void operation is read from the resolved `SessionContext`.
#[tauri::command]
pub async fn void_sale_scoped(
    session_token: String,
    args: VoidSaleScopedArgs,
    state: State<'_, AppState>,
) -> Result<oz_core::Sale, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::SALES_VOID).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = oz_core::db::Store::new(&db);

    let sale = store.void_sale(&args.sale_id, &session.user_id, &args.reason)?;
    drop(db);

    tracing::info!(sale_id = %args.sale_id, reason = %args.reason, "sale voided (scoped)");
    Ok(sale)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "void_tests.rs"]
mod tests;
