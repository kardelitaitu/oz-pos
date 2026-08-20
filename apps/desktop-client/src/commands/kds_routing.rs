//! KDS routing resolution command.
//!
//! Resolves which KDS devices should receive an order based on line items,
//! topology station assignments, and device station bindings.

use tauri::State;

use oz_core::db::Store;
use oz_core::kds::KdsDevice;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

/// Resolve which KDS device IDs should receive an order based on its
/// line items and the registered device station bindings.
///
/// Returns a list of device IDs. The caller is responsible for
/// filtering or pushing events to those devices.
///
/// Uses the 3-phase algorithm from `oz_core::kds::resolve_kds_targets`:
/// 1. Station-based targeting — match line item SKU → topology station → device
/// 2. Broadcast fallback — devices with empty station_ids get everything
/// 3. Catch-all — if any station has no claiming device, broadcast to all
#[tauri::command]
pub async fn resolve_kds_targets_scoped(
    session_token: String,
    order_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::KDS_VIEW).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    // Get the order's line items.
    let order = store
        .get_kds_order(&order_id)?
        .ok_or_else(|| AppError::Invalid(format!("KDS order not found: {order_id}")))?;
    let line_items = store.get_kds_order_lines(&order_id)?;

    // Get active devices for this restaurant.
    let resto_id = order.store_id.as_deref().unwrap_or(&session.terminal_id);
    let devices = store.list_kds_devices_for_restaurant(resto_id)?;

    // Filter to active only.
    let active_devices: Vec<KdsDevice> = devices.into_iter().filter(|d| d.is_active).collect();

    // Build a SKU → kitchen_zone map from the product catalog.
    // The product's `kitchen_zone` field serves as the "station" for routing.
    // Devices declare which zones they handle via `station_ids`.
    use std::collections::HashMap;
    let mut sku_to_station: HashMap<String, Option<String>> = HashMap::new();
    for item in &line_items {
        if !sku_to_station.contains_key(&item.sku) {
            let zone = store.product_kitchen_zone_by_sku(&item.sku)?;
            sku_to_station.insert(item.sku.clone(), zone);
        }
    }

    // Use the pure routing function with the real zone lookup.
    let targets = oz_core::kds::resolve_kds_targets(&line_items, &active_devices, |sku| {
        sku_to_station.get(sku).cloned().flatten()
    });

    Ok(targets)
}
