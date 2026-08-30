//! Promotion management commands.
//!
//! CRUD for promotion rules and recording promotion applications against sales.

use serde::Deserialize;
use tauri::State;

use oz_core::{Promotion, PromotionApplication, Store, format_minor};

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
/// Createpromotionargs.
pub struct CreatePromotionArgs {
    /// Display name.
    pub name: String,
    #[serde(default)]
    /// Human-readable description.
    pub description: String,
    /// Promo Type.
    pub promo_type: String,
    /// Value Minor.
    pub value_minor: i64,
    /// Min Qty.
    pub min_qty: Option<i64>,
    /// Trigger Sku.
    pub trigger_sku: Option<String>,
    /// Reward Sku.
    pub reward_sku: Option<String>,
    /// Reward Qty.
    pub reward_qty: Option<i64>,
    /// Starts At.
    pub starts_at: Option<String>,
    /// Ends At.
    pub ends_at: Option<String>,
    #[serde(default)]
    /// Min Order Minor.
    pub min_order_minor: i64,
    /// ID of the associated category.
    pub category_id: Option<String>,
    #[serde(default = "default_true")]
    /// Whether this record is active.
    pub active: bool,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
#[path = "promotions_tests.rs"]
mod tests;

/// List promotions for the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn list_promotions_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<Promotion>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let promos = store.list_promotions()?;
    drop(db);
    Ok(promos)
}

/// Get a promotion from the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn get_promotion_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<Promotion>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let promo = store.get_promotion(&id)?;
    drop(db);
    Ok(promo)
}

/// Create a promotion in the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn create_promotion_scoped(
    session_token: String,
    args: CreatePromotionArgs,
    state: State<'_, AppState>,
) -> Result<Promotion, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::PROMOTIONS_CREATE)
        .await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let promo = Promotion {
        id: uuid::Uuid::now_v7().to_string(),
        name: args.name,
        description: args.description,
        promo_type: args.promo_type,
        value_minor: args.value_minor,
        min_qty: args.min_qty,
        trigger_sku: args.trigger_sku,
        reward_sku: args.reward_sku,
        reward_qty: args.reward_qty,
        starts_at: args.starts_at,
        ends_at: args.ends_at,
        min_order_minor: args.min_order_minor,
        category_id: args.category_id,
        active: args.active,
        created_at: now.clone(),
        updated_at: now,
    };

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let result = store.create_promotion(&promo)?;
    drop(db);
    Ok(result)
}

/// Update a promotion in the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn update_promotion_scoped(
    session_token: String,
    promotion: Promotion,
    state: State<'_, AppState>,
) -> Result<Promotion, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::PROMOTIONS_EDIT).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let mut p = promotion;
    p.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let result = store.update_promotion(&p)?;
    drop(db);
    Ok(result)
}

/// Delete a promotion in the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn delete_promotion_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::PROMOTIONS_DELETE)
        .await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.delete_promotion(&id)?;
    drop(db);
    Ok(())
}

/// Apply a promotion in the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn apply_promotion_scoped(
    session_token: String,
    sale_id: String,
    promotion_id: String,
    state: State<'_, AppState>,
) -> Result<PromotionApplication, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::PROMOTIONS_APPLY)
        .await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_apply_promotion_unchecked(&db, &sale_id, &promotion_id)
}

fn run_apply_promotion_unchecked(
    db: &rusqlite::Connection,
    sale_id: &str,
    promotion_id: &str,
) -> Result<PromotionApplication, AppError> {
    let store = Store::new(db);

    let promo = store
        .get_promotion(promotion_id)?
        .ok_or_else(|| AppError::Invalid(format!("promotion {promotion_id} not found")))?;

    let sale = store
        .get_sale(sale_id)?
        .ok_or_else(|| AppError::Invalid(format!("sale {sale_id} not found")))?;

    // Category scope resolution: SKU -> product category (PROMO-6).
    // Only consulted when the promotion carries a category_id.
    let category_of = |sku: &str| {
        store
            .get_product(sku)
            .ok()
            .flatten()
            .and_then(|p| p.product.category_id)
    };

    let discount_minor = oz_core::compute_discount(&promo, &sale, chrono::Utc::now(), category_of)?;

    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let app = PromotionApplication {
        id: uuid::Uuid::now_v7().to_string(),
        promotion_id: promotion_id.to_string(),
        sale_id: sale_id.to_string(),
        discount_minor,
        description: format!(
            "{}: {} off",
            promo.name,
            format_minor(discount_minor, sale.currency)
        ),
        created_at: now,
    };

    Ok(store.record_promotion_application(&app)?)
}

/// Get sale promotions from the store resolved from a session token. ADR #7.
#[tauri::command]
pub async fn get_sale_promotions_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PromotionApplication>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let apps = store.get_promotion_applications_for_sale(&sale_id)?;
    drop(db);
    Ok(apps)
}
