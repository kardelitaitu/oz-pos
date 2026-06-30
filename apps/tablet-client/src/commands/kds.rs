//! Kitchen Display System (KDS) commands.
//!
//! IPC surface for the kitchen order queue: list orders, update status,
//! create tickets from completed sales.

use tauri::{State, command};

use oz_core::db::Store;
use oz_core::{CreateKdsOrderInput, KdsOrder};

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

/// Get the kitchen queue (pending + preparing, ordered oldest first).
#[command]
pub async fn get_kds_queue(state: State<'_, AppState>) -> Result<Vec<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let orders = store.get_kds_queue()?;
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

/// Create a KDS order from a completed sale.
#[command]
pub async fn create_kds_order_from_sale(
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<KdsOrder, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let order = store.complete_sale_to_kds(&sale_id)?;
    drop(db);
    Ok(order)
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
