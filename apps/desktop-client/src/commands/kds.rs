//! Kitchen Display System (KDS) commands.
//!
//! IPC surface for the kitchen order queue: list orders, update status,
//! create tickets from completed sales.
//!
//! All KDS commands require `kds:view` or `kds:update` permission.

use tauri::{State, command};

use oz_core::KdsOrder;
use oz_core::db::Store;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

/// List KDS orders from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_kds_orders_scoped`.
#[command]
pub async fn list_kds_orders(
    user_id: String,
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::KDS_VIEW)?;
    let orders = store.list_kds_orders(status.as_deref())?;
    drop(db);
    Ok(orders)
}

/// List KDS orders for the store resolved from a session token. ADR #7.
#[command]
pub async fn list_kds_orders_scoped(
    session_token: String,
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::KDS_VIEW)?;
    let orders = store.list_kds_orders(status.as_deref())?;
    drop(db);
    Ok(orders)
}

/// Get the kitchen queue from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_kds_queue_scoped`.
#[command]
pub async fn get_kds_queue(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::KDS_VIEW)?;
    let orders = store.get_kds_queue()?;
    drop(db);
    Ok(orders)
}

/// Get the kitchen queue for the store resolved from a session token. ADR #7.
#[command]
pub async fn get_kds_queue_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::KDS_VIEW)?;
    let orders = store.get_kds_queue()?;
    drop(db);
    Ok(orders)
}

/// Update a KDS order's status in the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `update_kds_status_scoped`.
#[command]
pub async fn update_kds_status(
    user_id: String,
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<KdsOrder, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::KDS_UPDATE)?;
    let order = store.update_kds_status(&id, &status)?;
    drop(db);
    Ok(order)
}

/// Update a KDS order's status in the store resolved from a session token. ADR #7.
#[command]
pub async fn update_kds_status_scoped(
    session_token: String,
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<KdsOrder, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::KDS_UPDATE)?;
    let order = store.update_kds_status(&id, &status)?;
    drop(db);
    Ok(order)
}

/// Create a KDS order from a completed sale in the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `create_kds_order_from_sale_scoped`.
#[command]
pub async fn create_kds_order_from_sale(
    user_id: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Option<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::KDS_UPDATE)?;
    let order = store.complete_sale_to_kds(&sale_id)?;
    drop(db);
    Ok(order)
}

/// Create a KDS order in the store resolved from a session token. ADR #7.
#[command]
pub async fn create_kds_order_from_sale_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Option<KdsOrder>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::KDS_UPDATE)?;
    let order = store.complete_sale_to_kds(&sale_id)?;
    drop(db);
    Ok(order)
}

/// Get a KDS order by id from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_kds_order_scoped`.
#[command]
pub async fn get_kds_order(
    user_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::KDS_VIEW)?;
    let order = store.get_kds_order(&id)?;
    drop(db);
    Ok(order)
}

/// Get a KDS order from the store resolved from a session token. ADR #7.
#[command]
pub async fn get_kds_order_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<KdsOrder>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::KDS_VIEW)?;
    let order = store.get_kds_order(&id)?;
    drop(db);
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kds_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }
}
