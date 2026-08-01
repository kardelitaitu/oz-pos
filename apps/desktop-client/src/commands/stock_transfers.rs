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

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::session::SessionContext;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

    // ── ReceivedLineInput ───────────────────────────────────────────────

    #[test]
    fn received_line_input_deserialize() {
        let json = r#"{"line_id":"l1","received_qty":5}"#;
        let args: ReceivedLineInput = serde_json::from_str(json).unwrap();
        assert_eq!(args.line_id, "l1");
        assert_eq!(args.received_qty, 5);
    }

    #[test]
    fn received_line_input_debug() {
        let args = ReceivedLineInput {
            line_id: "l2".into(),
            received_qty: 10,
        };
        let d = format!("{args:?}");
        assert!(d.contains("l2"));
    }

    // ── TransferWithLines ───────────────────────────────────────────────

    #[test]
    fn transfer_with_lines_debug() {
        let transfer = StockTransfer {
            id: "t1".into(),
            transfer_number: "TRF-001".into(),
            source_location: Some("WH-A".into()),
            destination_location: Some("WH-B".into()),
            source_terminal_id: None,
            destination_terminal_id: None,
            status: "draft".into(),
            notes: String::new(),
            created_by: "admin".into(),
            received_by: None,
            sent_at: None,
            received_at: None,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let twl = TransferWithLines {
            transfer,
            lines: vec![],
        };
        let d = format!("{twl:?}");
        assert!(d.contains("TRF-001"));
    }

    #[test]
    fn transfer_with_lines_serialize() {
        let transfer = StockTransfer {
            id: "t2".into(),
            transfer_number: "TRF-002".into(),
            source_location: None,
            destination_location: None,
            source_terminal_id: None,
            destination_terminal_id: None,
            status: "in_transit".into(),
            notes: "Rush".into(),
            created_by: "user1".into(),
            received_by: None,
            sent_at: None,
            received_at: None,
            created_at: "2025-02-01T00:00:00.000Z".into(),
            updated_at: "2025-02-01T00:00:00.000Z".into(),
        };
        let twl = TransferWithLines {
            transfer,
            lines: vec![],
        };
        let json = serde_json::to_value(&twl).unwrap();
        assert_eq!(json["transfer"]["transfer_number"], "TRF-002");
        assert_eq!(json["transfer"]["status"], "in_transit");
    }

    fn seed_identity(conn: &rusqlite::Connection, user_id: &str, role_id: &str) {
        let store = Store::new(conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active,
                                created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?2, ?3, 1,
                     '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            rusqlite::params![user_id, user_id, role_id],
        )
        .unwrap();
    }

    fn scoped_test_app() -> tauri::App<tauri::test::MockRuntime> {
        let global = oz_core::migrations::fresh_db();
        seed_identity(&global, "transfer-owner", "role-owner");
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let _keep_temp_dir = Box::leak(Box::new(temp_dir));
        let mut state = AppState::for_test_with_conn(global);
        state.db_manager = StoreDatabaseManager::new(temp_path, oz_core::migrations::ALL);
        state.session_store.write().unwrap().insert(
            "transfer-token".into(),
            SessionContext::new(
                "transfer-owner".into(),
                "role-owner".into(),
                "terminal-1".into(),
                "store-a".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );

        tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap()
    }

    #[tokio::test]
    async fn scoped_create_derives_created_by_from_session() {
        let app = scoped_test_app();
        let transfer = create_stock_transfer_scoped(
            "transfer-token".into(),
            None,
            None,
            None,
            None,
            "session actor test".into(),
            vec![],
            app.state(),
        )
        .await
        .unwrap();

        assert_eq!(transfer.created_by, "transfer-owner");

        // Authentication is global; the store ledger must not manufacture a
        // local users row merely to satisfy the historical FK.
        let state = app.state::<AppState>();
        let (_, conn) = state.resolve_scope("transfer-token").unwrap();
        let db = conn.lock().unwrap();
        let local_users: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM users WHERE id = 'transfer-owner'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            local_users, 0,
            "transfer writes must not clone global auth users"
        );
    }

    #[tokio::test]
    async fn scoped_transfer_reads_are_isolated_between_store_sessions() {
        let global = oz_core::migrations::fresh_db();
        seed_identity(&global, "transfer-owner", "role-owner");
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let _keep_temp_dir = Box::leak(Box::new(temp_dir));
        let mut state = AppState::for_test_with_conn(global);
        state.db_manager = StoreDatabaseManager::new(temp_path, oz_core::migrations::ALL);
        for (token, store_id) in [("store-a-token", "store-a"), ("store-b-token", "store-b")] {
            state.session_store.write().unwrap().insert(
                token.into(),
                SessionContext::new(
                    "transfer-owner".into(),
                    "role-owner".into(),
                    "terminal-1".into(),
                    store_id.into(),
                    "instance-1".into(),
                    "pos".into(),
                    None,
                    0,
                ),
            );
        }
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        create_stock_transfer_scoped(
            "store-a-token".into(),
            None,
            None,
            None,
            None,
            "store A only".into(),
            vec![],
            app.state(),
        )
        .await
        .unwrap();

        let store_a = list_stock_transfers_scoped("store-a-token".into(), app.state())
            .await
            .unwrap();
        let store_b = list_stock_transfers_scoped("store-b-token".into(), app.state())
            .await
            .unwrap();
        assert_eq!(store_a.len(), 1);
        assert!(store_b.is_empty());
    }

    #[tokio::test]
    async fn scoped_transfer_denies_user_without_transfer_permission() {
        let global = oz_core::migrations::fresh_db();
        seed_identity(&global, "transfer-cashier", "role-cashier");
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let _keep_temp_dir = Box::leak(Box::new(temp_dir));
        let mut state = AppState::for_test_with_conn(global);
        state.db_manager = StoreDatabaseManager::new(temp_path, oz_core::migrations::ALL);
        state.session_store.write().unwrap().insert(
            "cashier-transfer-token".into(),
            SessionContext::new(
                "transfer-cashier".into(),
                "role-cashier".into(),
                "terminal-1".into(),
                "store-cashier".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result =
            list_stock_transfers_scoped("cashier-transfer-token".into(), app.state()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }
}
