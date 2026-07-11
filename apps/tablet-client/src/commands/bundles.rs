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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_bundle_args_deserialize() {
        let json = r#"{"bundle_sku":"BUNDLE-001","name":"Breakfast Combo","description":"Coffee + croissant","bundle_price_minor":1500,"currency":"USD","items":[{"sku":"SKU-1","qty":2,"unit_price_minor":500}]}"#;
        let args: CreateBundleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.bundle_sku, "BUNDLE-001");
        assert_eq!(args.name, "Breakfast Combo");
        assert_eq!(args.description.unwrap(), "Coffee + croissant");
        assert_eq!(args.bundle_price_minor.unwrap(), 1500);
        assert_eq!(args.items.len(), 1);
    }

    #[test]
    fn create_bundle_args_debug() {
        let args = CreateBundleArgs {
            bundle_sku: "B-TEST".into(),
            name: "Test".into(),
            description: None,
            bundle_price_minor: None,
            currency: None,
            items: vec![],
        };
        let debug = format!("{:?}", args);
        assert!(debug.contains("B-TEST"));
    }

    #[test]
    fn create_bundle_item_arg_deserialize() {
        let json = r#"{"sku":"SKU-A","qty":3,"unit_price_minor":200}"#;
        let item: CreateBundleItemArg = serde_json::from_str(json).unwrap();
        assert_eq!(item.sku, "SKU-A");
        assert_eq!(item.qty, 3);
        assert_eq!(item.unit_price_minor.unwrap(), 200);
    }

    #[test]
    fn create_bundle_item_arg_debug() {
        let item = CreateBundleItemArg {
            sku: "SKU-X".into(),
            qty: 1,
            unit_price_minor: Some(100),
        };
        let debug = format!("{:?}", item);
        assert!(debug.contains("SKU-X"));
        assert!(debug.contains("100"));
    }
}
