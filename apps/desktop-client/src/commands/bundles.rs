use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::Store;
use oz_core::product_bundle::{BundleItem, BundleWithItems, ProductBundle};

use crate::error::AppError;
use crate::state::AppState;

/// Arguments for creating a bundle.
#[derive(Debug, Deserialize)]
pub struct CreateBundleArgs {
    pub bundle_sku: String,
    pub name: String,
    pub description: Option<String>,
    pub bundle_price_minor: Option<i64>,
    pub currency: Option<String>,
    pub items: Vec<CreateBundleItemArg>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBundleItemArg {
    pub sku: String,
    pub qty: i64,
    pub unit_price_minor: Option<i64>,
}

/// List all bundles with their items.
#[command]
pub async fn list_bundles(state: State<'_, AppState>) -> Result<Vec<BundleWithItems>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.list_bundles()?)
}

/// Get a single bundle by id.
#[command]
pub async fn get_bundle(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<BundleWithItems>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_bundle(&id)?)
}

/// Create a new bundle.
#[command]
pub async fn create_bundle(
    args: CreateBundleArgs,
    state: State<'_, AppState>,
) -> Result<BundleWithItems, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let id = uuid::Uuid::new_v4().to_string();
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
            id: uuid::Uuid::new_v4().to_string(),
            bundle_id: id.clone(),
            sku: i.sku,
            qty: i.qty,
            unit_price_minor: i.unit_price_minor,
        })
        .collect();

    Ok(store.create_bundle(&bundle, &items)?)
}

/// Update an existing bundle.
#[command]
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

/// Delete a bundle.
#[command]
pub async fn delete_bundle(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_bundle(&id)?;
    Ok(())
}

/// Look up a bundle by its SKU (for barcode scanning / POS lookup).
#[command]
pub async fn lookup_bundle_by_sku(
    sku: String,
    state: State<'_, AppState>,
) -> Result<Option<BundleWithItems>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_bundle_by_sku(&sku)?)
}
