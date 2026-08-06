//! Kitchen Display System (KDS) commands.
//!
//! IPC surface for the kitchen order queue: list orders, update status,
//! create tickets from completed sales.
//!
//! All KDS commands require `kds:view` or `kds:update` permission.

use tauri::{Emitter, State};

use oz_core::KdsOrder;
use oz_core::db::Store;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

/// List KDS orders from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_kds_orders_scoped`.
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
pub async fn get_kds_queue(
    user_id: String,
    kds_zone: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::KDS_VIEW)?;
    let orders = store.get_kds_queue(kds_zone.as_deref())?;
    drop(db);
    Ok(orders)
}

/// Get the kitchen queue for the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn get_kds_queue_scoped(
    session_token: String,
    kds_zone: Option<String>,
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
    let orders = store.get_kds_queue(kds_zone.as_deref())?;
    drop(db);
    Ok(orders)
}

/// Update a KDS order's status in the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `update_kds_status_scoped`.
#[tauri::command]
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

    // Push real-time update to all KDS displays (1a: real-time push).
    if let Some(app) = state.app.as_ref() {
        let _ = app.emit("kds:orders-changed", ());
    }

    Ok(order)
}

/// Update the items (summary + count) on an existing KDS order.
///
/// **Deprecated for multi-store (ADR #7):** Use `update_kds_order_items_scoped`.
#[tauri::command]
pub async fn update_kds_order_items(
    user_id: String,
    args: oz_core::UpdateKdsOrderItemsInput,
    state: State<'_, AppState>,
) -> Result<KdsOrder, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::KDS_UPDATE)?;
    let order = store.update_kds_order_items(args)?;
    drop(db);

    // Push real-time update to all KDS displays.
    if let Some(app) = state.app.as_ref() {
        let _ = app.emit("kds:orders-changed", ());
    }

    Ok(order)
}

/// Update the items on a KDS order in the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn update_kds_order_items_scoped(
    session_token: String,
    args: oz_core::UpdateKdsOrderItemsInput,
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
    let order = store.update_kds_order_items(args)?;
    drop(db);

    // Push real-time update to all KDS displays.
    if let Some(app) = state.app.as_ref() {
        let _ = app.emit("kds:orders-changed", ());
    }

    Ok(order)
}

/// Update a KDS order's status in the store resolved from a session token. ADR #7.
#[tauri::command]
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

    // Push real-time update to all KDS displays (1a: real-time push).
    if let Some(app) = state.app.as_ref() {
        let _ = app.emit("kds:orders-changed", ());
    }

    Ok(order)
}

/// Create KDS orders from a completed sale. Returns one order per kitchen zone.
///
/// **Deprecated for multi-store (ADR #7):** Use `create_kds_order_from_sale_scoped`.
#[tauri::command]
pub async fn create_kds_order_from_sale(
    user_id: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    // Scope-limit the DB access so Store (which borrows from the MutexGuard)
    // is dropped before any .await point — required for Tauri's Send bound.
    let orders = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        require_permission_for_user(&store, &user_id, permissions::KDS_UPDATE)?;
        store.complete_sale_to_kds(&sale_id, None)?
    }; // db + store dropped here

    // Push real-time update to all KDS displays — skip if no kitchen items.
    if !orders.is_empty()
        && let Some(app) = state.app.as_ref()
    {
        let _ = app.emit("kds:orders-changed", ());
    }

    // Auto-print kitchen chits (3c: printer HAL — best-effort).
    try_auto_print_kds_chits(&orders, &state.registry, state.app.as_ref()).await;

    Ok(orders)
}

/// Create KDS orders in the store resolved from a session token. ADR #7.
///
/// Passes the session's `store_id` so the KDS order carries store identity
/// for defense-in-depth filtering on KDS tablets (ADR #8). Returns one
/// KDS order per kitchen zone; an empty vec when no restaurant items exist.
#[tauri::command]
pub async fn create_kds_order_from_sale_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<KdsOrder>, AppError> {
    let session = state.resolve_session(&session_token)?;
    // Scope-limit the DB access so Store is dropped before .await.
    let orders = {
        let conn = state
            .db_manager
            .open_store(&session.store_id)
            .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);
        require_permission_for_user(&store, &session.user_id, permissions::KDS_UPDATE)?;
        store.complete_sale_to_kds(&sale_id, Some(&session.store_id))?
    }; // conn, db, store dropped here

    // Push real-time update to all KDS displays — skip if no kitchen items.
    if !orders.is_empty()
        && let Some(app) = state.app.as_ref()
    {
        let _ = app.emit("kds:orders-changed", ());
    }

    // Auto-print kitchen chits (3c: printer HAL — best-effort).
    try_auto_print_kds_chits(&orders, &state.registry, state.app.as_ref()).await;

    Ok(orders)
}

/// Get a KDS order by id from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_kds_order_scoped`.
#[tauri::command]
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
#[tauri::command]
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

// ── Kitchen chit printing ───────────────────────────────

/// Print a kitchen chit for a single KDS order.
///
/// Tries the "kitchen" printer first; falls back to the "default"
/// receipt printer. Silently skips when no printer is registered
/// (the kitchen may not have a dedicated printer).
///
/// Returns `true` when the chit was printed, `false` when skipped.
pub async fn print_kds_chit_for_order(
    order: &KdsOrder,
    registry: &oz_hal::DriverRegistry,
    app: Option<&tauri::AppHandle>,
) -> bool {
    // Find the best available printer — try "kitchen" first, then "default".
    let printer = match registry.printer("kitchen").await {
        Some(p) => Some(p),
        None => registry.printer("default").await,
    };
    let printer = match printer {
        Some(p) => p,
        None => {
            tracing::trace!(
                order_id = %order.id,
                "kitchen chit: no printer available, skipping"
            );
            return false;
        }
    };

    // Format the chit.
    let chit = oz_hal::drivers::kds_chit::format_kds_chit(
        order.display_number,
        order.table_number.as_deref(),
        &order.items_summary,
        order.item_count,
        &order.notes,
        &order.received_at,
    );

    // Print it.
    match printer.print_raw(&chit.data).await {
        Ok(_) => {
            tracing::info!(
                order_id = %order.id,
                display_number = ?order.display_number,
                "kitchen chit printed"
            );
            if let Some(app) = app {
                let _ = app.emit(
                    "kds:chit-printed",
                    serde_json::json!({
                        "orderId": order.id,
                        "displayNumber": order.display_number,
                    }),
                );
            }
            true
        }
        Err(e) => {
            tracing::warn!(
                order_id = %order.id,
                error = %e,
                "kitchen chit print failed"
            );
            false
        }
    }
}

/// Print a kitchen chit for a specific KDS order by ID (scoped — ADR #7).
///
/// Useful for manual re-print from the KDS screen when a chit was lost
/// or damaged. Returns`true` if the chit was printed, `false` if the
/// order was not found or no printer was available.
#[tauri::command]
pub async fn print_kds_chit_scoped(
    session_token: String,
    order_id: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let session = state.resolve_session(&session_token)?;
    // Scope-limit the DB access so Store is dropped before .await.
    let order = {
        let conn = state
            .db_manager
            .open_store(&session.store_id)
            .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);
        require_permission_for_user(&store, &session.user_id, permissions::KDS_UPDATE)?;
        store.get_kds_order(&order_id)?
    }; // conn, db, store dropped here

    let Some(order) = order else {
        return Ok(false);
    };

    let printed = print_kds_chit_for_order(&order, &state.registry, state.app.as_ref()).await;
    Ok(printed)
}

// ── KDS line items (TODO 2a) ────────────────────────────

/// Get all line items for a KDS order (scoped — ADR #7).
///
/// Returns structured line items with course and modifier data,
/// ordered by course priority then line position.
#[tauri::command]
pub async fn get_kds_order_lines_scoped(
    session_token: String,
    order_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<oz_core::KdsLineItem>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let order = {
        let conn = state
            .db_manager
            .open_store(&session.store_id)
            .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        let store = Store::new(&db);
        require_permission_for_user(&store, &session.user_id, permissions::KDS_VIEW)?;
        store.get_kds_order_lines(&order_id)?
    };
    Ok(order)
}

/// Update the status of a single KDS line item in the store resolved
/// from a session token. ADR #7.
///
/// Returns the updated line item with the new status and timestamp.
#[tauri::command]
pub async fn update_kds_line_item_status_scoped(
    session_token: String,
    item_id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<oz_core::KdsLineItem, AppError> {
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
    let item = store.update_kds_line_item_status(&item_id, &status)?;
    drop(db);

    // Push real-time update to all KDS displays.
    if let Some(app) = state.app.as_ref() {
        let _ = app.emit("kds:orders-changed", ());
    }

    Ok(item)
}

/// Try to print kitchen chits for every order in the slice.
///
/// Best-effort: logs failures but does not return errors.
/// Called automatically after KDS order creation.
///
/// Takes owned clones of registry and app so the caller can drop any
/// Tauri state borrows before the first `.await`.
pub async fn try_auto_print_kds_chits(
    orders: &[KdsOrder],
    registry: &oz_hal::DriverRegistry,
    app: Option<&tauri::AppHandle>,
) {
    if orders.is_empty() {
        return;
    }
    for order in orders {
        print_kds_chit_for_order(order, registry, app).await;
    }
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
