//! Stock transfer Tauri commands.
//!
//! Exposes CRUD + send/receive lifecycle operations to the front-end.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::db::Store;
use oz_core::stock_transfer::{StockTransfer, StockTransferLine};

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

/// Verify inventory-transfer permission against the global identity database.
async fn require_inventory_permission(state: &AppState, user_id: &str) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, user_id, oz_core::permissions::INVENTORY_TRANSFER)
}

/// Validate that a client-supplied location belongs to this store database.
fn validate_location(
    db: &rusqlite::Connection,
    location_id: Option<&str>,
    field: &'static str,
) -> Result<(), AppError> {
    let Some(location_id) = location_id else {
        return Ok(());
    };
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM inventory_locations WHERE id = ?1 AND is_active = 1)",
        [location_id],
        |row| row.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(AppError::Invalid(format!(
            "{field} location '{location_id}' is not active in the current store"
        )))
    }
}

/// Validate an optional terminal identifier against the active terminals in the
/// resolved store database. Transfer terminal foreign keys are store-local.
fn validate_terminal(
    db: &rusqlite::Connection,
    terminal_id: Option<&str>,
    field: &'static str,
) -> Result<(), AppError> {
    let Some(terminal_id) = terminal_id else {
        return Ok(());
    };
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM terminals WHERE id = ?1 AND is_active = 1)",
        [terminal_id],
        |row| row.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(AppError::Invalid(format!(
            "{field} terminal '{terminal_id}' is not active in the current store"
        )))
    }
}

/// A received quantity for a single transfer line.
#[derive(Debug, Deserialize)]
pub struct ReceivedLineInput {
    /// ID of the associated line.
    pub line_id: String,
    /// Received Qty.
    pub received_qty: i64,
}

#[derive(Debug, Serialize)]
/// Transferwithlines.
pub struct TransferWithLines {
    /// Transfer.
    pub transfer: StockTransfer,
    /// Lines.
    pub lines: Vec<StockTransferLine>,
}

// ── Session-scoped commands (ADR #7) ─────────────────────────────────

/// Create a stock transfer in the store resolved from the session token.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_stock_transfer_scoped(
    session_token: String,
    source_location: Option<String>,
    destination_location: Option<String>,
    source_terminal_id: Option<String>,
    destination_terminal_id: Option<String>,
    notes: String,
    lines: Vec<StockTransferLine>,
    state: State<'_, AppState>,
) -> Result<StockTransfer, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    validate_location(&db, source_location.as_deref(), "source")?;
    validate_location(&db, destination_location.as_deref(), "destination")?;
    validate_terminal(&db, source_terminal_id.as_deref(), "source")?;
    validate_terminal(&db, destination_terminal_id.as_deref(), "destination")?;
    let store = Store::new(&db);
    Ok(store.create_transfer(
        source_location.as_deref(),
        destination_location.as_deref(),
        source_terminal_id.as_deref(),
        destination_terminal_id.as_deref(),
        &notes,
        &session.user_id,
        &lines,
    )?)
}

/// Get a stock transfer from the session-scoped store.
#[tauri::command]
pub async fn get_stock_transfer_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<TransferWithLines>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let transfer = store.get_transfer(&id)?;
    let lines = if transfer.is_some() {
        store.get_transfer_lines(&id)?
    } else {
        vec![]
    };
    Ok(transfer.map(|t| TransferWithLines { transfer: t, lines }))
}

/// List stock transfers from the session-scoped store.
#[tauri::command]
pub async fn list_stock_transfers_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<StockTransfer>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).list_transfers()?)
}

/// List in-transit transfers with their line items in one batch request.
///
/// The transit audit screen previously listed all transfers and then fetched
/// lines one transfer at a time (N+1). This command returns the lines in two
/// SQL queries so the whole audit view loads in a single IPC round-trip.
///
/// The status filter is intentionally `in_transit` only: this mirrors the
/// legacy screen's behavior, and partially-received transfers (`received_partial`)
/// continue to be received on the StockTransfersScreen, not the transit audit.
#[tauri::command]
pub async fn list_in_transit_transfers_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<TransferWithLines>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db)
        .list_transfers_with_lines_by_status("in_transit")?
        .into_iter()
        .map(|(transfer, lines)| TransferWithLines { transfer, lines })
        .collect())
}

/// Get transfer lines from the session-scoped store.
#[tauri::command]
pub async fn get_stock_transfer_lines_scoped(
    session_token: String,
    transfer_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<StockTransferLine>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).get_transfer_lines(&transfer_id)?)
}

/// Add a transfer line in the session-scoped store.
#[tauri::command]
pub async fn add_stock_transfer_line_scoped(
    session_token: String,
    transfer_id: String,
    sku: String,
    product_name: String,
    qty: i64,
    state: State<'_, AppState>,
) -> Result<StockTransferLine, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).add_transfer_line(&transfer_id, &sku, &product_name, qty)?)
}

/// Remove a transfer line in the session-scoped store.
#[tauri::command]
pub async fn remove_stock_transfer_line_scoped(
    session_token: String,
    line_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Store::new(&db).remove_transfer_line(&line_id)?;
    Ok(())
}

/// Send a transfer in the session-scoped store.
#[tauri::command]
pub async fn send_stock_transfer_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<StockTransfer, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).send_transfer(&id)?)
}

/// Receive a transfer, attributing the actor to the authenticated session.
#[tauri::command]
pub async fn receive_stock_transfer_scoped(
    session_token: String,
    id: String,
    received_lines: Vec<ReceivedLineInput>,
    state: State<'_, AppState>,
) -> Result<StockTransfer, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let received_lines = received_lines
        .into_iter()
        .map(|line| oz_core::db::stock_transfers::ReceivedLine {
            line_id: line.line_id,
            received_qty: line.received_qty,
        })
        .collect::<Vec<_>>();
    Ok(Store::new(&db).receive_transfer(&id, &session.user_id, &received_lines)?)
}

/// Cancel a transfer in the session-scoped store.
#[tauri::command]
pub async fn cancel_stock_transfer_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<StockTransfer, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_inventory_permission(&state, &session.user_id).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    Ok(Store::new(&db).cancel_transfer(&id)?)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)] #[path = "stock_transfers_tests.rs"] mod tests;
