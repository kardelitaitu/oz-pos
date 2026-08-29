use serde::Deserialize;
use tauri::{State, command};

use oz_core::Store;
use oz_core::product_bundle::{BundleItem, BundleWithItems, ProductBundle};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
/// Createbundleargs.
pub struct CreateBundleArgs {
    /// Bundle Sku.
    pub bundle_sku: String,
    /// Display name.
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Bundle Price Minor.
    pub bundle_price_minor: Option<i64>,
    /// ISO-4217 currency code.
    pub currency: Option<String>,
    /// Items.
    pub items: Vec<CreateBundleItemArg>,
}

#[derive(Debug, Deserialize)]
/// Createbundleitemarg.
pub struct CreateBundleItemArg {
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Quantity.
    pub qty: i64,
    /// Unit Price Minor.
    pub unit_price_minor: Option<i64>,
}

#[command]
/// List bundles.
pub async fn list_bundles(state: State<'_, AppState>) -> Result<Vec<BundleWithItems>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.list_bundles()?)
}

#[command]
/// Get bundle.
pub async fn get_bundle(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<BundleWithItems>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_bundle(&id)?)
}

#[command]
/// Create bundle.
pub async fn create_bundle(
    args: CreateBundleArgs,
    state: State<'_, AppState>,
) -> Result<BundleWithItems, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let bundle = ProductBundle {
        id: id.clone(),
        bundle_sku: args.bundle_sku,
        name: args.name,
        description: args.description.unwrap_or_default(),
        bundle_price_minor: args.bundle_price_minor,
        currency: args.currency.unwrap_or_else(|| "USD".into()),
        active: true,
        created_at: now.clone(),
        updated_at: now,
    };

    let items: Vec<BundleItem> = args
        .items
        .into_iter()
        .map(|i| BundleItem {
            id: uuid::Uuid::now_v7().to_string(),
            bundle_id: id.clone(),
            sku: i.sku,
            qty: i.qty,
            unit_price_minor: i.unit_price_minor,
        })
        .collect();

    Ok(store.create_bundle(&bundle, &items)?)
}

#[command]
/// Update bundle.
pub async fn update_bundle(
    bundle: BundleWithItems,
    state: State<'_, AppState>,
) -> Result<BundleWithItems, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let mut updated = bundle.bundle;
    updated.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    Ok(store.update_bundle(&updated, &bundle.items)?)
}

#[command]
/// Delete bundle.
pub async fn delete_bundle(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_bundle(&id)?;
    Ok(())
}

#[command]
/// Lookup bundle by sku.
pub async fn lookup_bundle_by_sku(
    sku: String,
    state: State<'_, AppState>,
) -> Result<Option<BundleWithItems>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_bundle_by_sku(&sku)?)
}

/// Session-scoped variant of `list_bundles`.
#[command]
pub async fn list_bundles_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<BundleWithItems>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    Ok(store.list_bundles()?)
}

/// Session-scoped variant of `get_bundle`.
#[command]
pub async fn get_bundle_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<BundleWithItems>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    Ok(store.get_bundle(&id)?)
}

/// Session-scoped variant of `create_bundle`.
#[command]
pub async fn create_bundle_scoped(
    session_token: String,
    args: CreateBundleArgs,
    state: State<'_, AppState>,
) -> Result<BundleWithItems, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);

    let id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let bundle = ProductBundle {
        id: id.clone(),
        bundle_sku: args.bundle_sku,
        name: args.name,
        description: args.description.unwrap_or_default(),
        bundle_price_minor: args.bundle_price_minor,
        currency: args.currency.unwrap_or_else(|| "USD".into()),
        active: true,
        created_at: now.clone(),
        updated_at: now,
    };

    let items: Vec<BundleItem> = args
        .items
        .into_iter()
        .map(|i| BundleItem {
            id: uuid::Uuid::now_v7().to_string(),
            bundle_id: id.clone(),
            sku: i.sku,
            qty: i.qty,
            unit_price_minor: i.unit_price_minor,
        })
        .collect();

    Ok(store.create_bundle(&bundle, &items)?)
}

/// Session-scoped variant of `update_bundle`.
#[command]
pub async fn update_bundle_scoped(
    session_token: String,
    bundle: BundleWithItems,
    state: State<'_, AppState>,
) -> Result<BundleWithItems, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);

    let mut updated = bundle.bundle;
    updated.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    Ok(store.update_bundle(&updated, &bundle.items)?)
}

/// Session-scoped variant of `delete_bundle`.
#[command]
pub async fn delete_bundle_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    store.delete_bundle(&id)?;
    Ok(())
}

/// Session-scoped variant of `lookup_bundle_by_sku`.
#[command]
pub async fn lookup_bundle_by_sku_scoped(
    session_token: String,
    sku: String,
    state: State<'_, AppState>,
) -> Result<Option<BundleWithItems>, AppError> {
    let (_session, conn_arc) = state.resolve_scope(&session_token)?;
    let db_guard = conn_arc
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let db = &*db_guard;
    let store = Store::new(&db);
    Ok(store.get_bundle_by_sku(&sku)?)
}

#[cfg(test)]
#[path = "bundles_tests.rs"]
mod tests;
