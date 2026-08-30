use serde::Deserialize;
use tauri::State;

use oz_core::Store;
use oz_core::product_bundle::{BundleItem, BundleWithItems, ProductBundle};

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;
use oz_core::permissions;

/// Arguments for creating a bundle.
#[derive(Debug, Deserialize)]
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

// ── Tests ──────────────────────────────────────────────────────────────

// ── Scoped variants (ADR #7) ────────────────────────────────────

/// Scoped variant of `list_bundles` (ADR #7).
#[tauri::command]
pub async fn list_bundles_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<BundleWithItems>, AppError> {
    let (session, _conn) = state.resolve_scope(&session_token)?;
    // F-017: enforce per-domain permission on this scoped command.
    require_permission_for_session(&state, &session, permissions::PRODUCTS_READ).await?;
    let db = _conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    Ok(store.list_bundles()?)
}

/// Scoped variant of `get_bundle` (ADR #7).
#[tauri::command]
pub async fn get_bundle_scoped(
    id: String,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Option<BundleWithItems>, AppError> {
    let (session, _conn) = state.resolve_scope(&session_token)?;
    // F-017: enforce per-domain permission on this scoped command.
    require_permission_for_session(&state, &session, permissions::PRODUCTS_READ).await?;
    let db = _conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    Ok(store.get_bundle(&id)?)
}

/// Scoped variant of `update_bundle` (ADR #7).
#[tauri::command]
pub async fn update_bundle_scoped(
    bundle: BundleWithItems,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<BundleWithItems, AppError> {
    let (session, _conn) = state.resolve_scope(&session_token)?;
    // F-017: enforce per-domain permission on this scoped command.
    require_permission_for_session(&state, &session, permissions::PRODUCTS_UPDATE).await?;
    let db = _conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    let mut updated = bundle.bundle;
    updated.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    Ok(store.update_bundle(&updated, &bundle.items)?)
}

/// Scoped variant of `delete_bundle` (ADR #7).
#[tauri::command]
pub async fn delete_bundle_scoped(
    id: String,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (session, _conn) = state.resolve_scope(&session_token)?;
    // F-017: enforce per-domain permission on this scoped command.
    require_permission_for_session(&state, &session, permissions::PRODUCTS_DELETE).await?;
    let db = _conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.delete_bundle(&id)?;
    Ok(())
}

/// Scoped variant of `lookup_bundle_by_sku` (ADR #7).
#[tauri::command]
pub async fn lookup_bundle_by_sku_scoped(
    sku: String,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Option<BundleWithItems>, AppError> {
    let (session, _conn) = state.resolve_scope(&session_token)?;
    // F-017: enforce per-domain permission on this scoped command.
    require_permission_for_session(&state, &session, permissions::PRODUCTS_READ).await?;
    let db = _conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    Ok(store.get_bundle_by_sku(&sku)?)
}

/// Create a new bundle (scoped).
#[tauri::command]
pub async fn create_bundle_scoped(
    args: CreateBundleArgs,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<BundleWithItems, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    // F-017: enforce per-domain permission on this scoped command.
    require_permission_for_session(&state, &session, permissions::PRODUCTS_CREATE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
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

#[cfg(test)]
#[path = "bundles_tests.rs"]
mod tests;
