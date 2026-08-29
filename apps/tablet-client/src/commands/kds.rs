//! Kitchen Display System (KDS) commands.
//!
//! IPC surface for the kitchen order queue: list orders, update status,
//! create tickets from completed sales.

use tauri::{State, command};

use oz_core::KdsOrder;
use oz_core::db::Store;

use crate::error::AppError;
use crate::state::AppState;

/// List KDS orders, optionally filtered by status.
#[command]
pub async fn list_kds_orders(
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let orders = store.list_kds_orders(status.as_deref())?;
    drop(db);
    Ok(orders)
}

/// Get the kitchen queue (pending + preparing + ready, ordered oldest first).
/// Optionally filtered by kitchen zone.
#[command]
pub async fn get_kds_queue(
    kds_zone: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let orders = store.get_kds_queue(kds_zone.as_deref())?;
    drop(db);
    Ok(orders)
}

/// Update a KDS order's status. Sets the appropriate timestamp automatically.
#[command]
pub async fn update_kds_status(
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<KdsOrder, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let order = store.update_kds_status(&id, &status)?;
    drop(db);
    Ok(order)
}

/// Create KDS orders from a completed sale. Returns one order per kitchen zone.
#[command]
pub async fn create_kds_order_from_sale(
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let orders = store.complete_sale_to_kds(&sale_id, None)?;
    drop(db);
    Ok(orders)
}

/// Get a KDS order by id.
#[command]
pub async fn get_kds_order(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let order = store.get_kds_order(&id)?;
    drop(db);
    Ok(order)
}

/// Session-scoped variant of `list_kds_orders`.
#[allow(clippy::needless_borrow, dropping_references)]
#[command]
pub async fn list_kds_orders_scoped(
    session_token: String,
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    let orders = store.list_kds_orders(status.as_deref())?;
    drop(db);
    Ok(orders)
}

/// Session-scoped variant of `get_kds_queue`.
#[allow(clippy::needless_borrow, dropping_references)]
#[command]
pub async fn get_kds_queue_scoped(
    session_token: String,
    kds_zone: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    let orders = store.get_kds_queue(kds_zone.as_deref())?;
    drop(db);
    Ok(orders)
}

/// Session-scoped variant of `update_kds_status`.
#[allow(clippy::needless_borrow, dropping_references)]
#[command]
pub async fn update_kds_status_scoped(
    session_token: String,
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<KdsOrder, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    let order = store.update_kds_status(&id, &status)?;
    drop(db);
    Ok(order)
}

/// Session-scoped variant of `create_kds_order_from_sale`.
#[allow(clippy::needless_borrow, dropping_references)]
#[command]
pub async fn create_kds_order_from_sale_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    let orders = store.complete_sale_to_kds(&sale_id, None)?;
    drop(db);
    Ok(orders)
}

/// Session-scoped variant of `get_kds_order`.
#[allow(clippy::needless_borrow, dropping_references)]
#[command]
pub async fn get_kds_order_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<KdsOrder>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    let order = store.get_kds_order(&id)?;
    drop(db);
    Ok(order)
}
