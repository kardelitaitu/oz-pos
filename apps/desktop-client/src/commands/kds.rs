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

/// List KDS orders, optionally filtered by status.
///
/// Requires `kds:view` permission.
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

/// Get the kitchen queue (pending + preparing, ordered oldest first).
///
/// Requires `kds:view` permission.
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

/// Update a KDS order's status. Sets the appropriate timestamp automatically.
///
/// Requires `kds:update` permission.
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

/// Create a KDS order from a completed sale.
///
/// Requires `kds:update` permission.
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

/// Get a KDS order by id.
///
/// Requires `kds:view` permission.
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
