//! Promotion management commands.
//!
//! CRUD for promotion rules and recording promotion applications against sales.

use serde::Deserialize;
use tauri::{State, command};

use oz_core::{Promotion, PromotionApplication, Store};

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreatePromotionArgs {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub promo_type: String,
    pub value_minor: i64,
    pub min_qty: Option<i64>,
    pub trigger_sku: Option<String>,
    pub reward_sku: Option<String>,
    pub reward_qty: Option<i64>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    #[serde(default)]
    pub min_order_minor: i64,
    pub category_id: Option<String>,
    #[serde(default = "default_true")]
    pub active: bool,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_promotion_args_deserialize_minimal() {
        let json = r#"{"name":"10% Off","promo_type":"percentage","value_minor":10}"#;
        let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "10% Off");
        assert_eq!(args.description, "");
        assert_eq!(args.promo_type, "percentage");
        assert_eq!(args.value_minor, 10);
        assert!(args.min_qty.is_none());
        assert!(args.trigger_sku.is_none());
        assert!(args.reward_sku.is_none());
        assert!(args.reward_qty.is_none());
        assert!(args.starts_at.is_none());
        assert!(args.ends_at.is_none());
        assert_eq!(args.min_order_minor, 0);
        assert!(args.category_id.is_none());
        assert!(args.active);
    }

    #[test]
    fn create_promotion_args_deserialize_all_fields() {
        let json = r#"{
            "name": "Buy 2 Get 1",
            "description": "Buy two coffees, get one free",
            "promo_type": "buy_x_get_y",
            "value_minor": 100,
            "min_qty": 2,
            "trigger_sku": "COFFEE",
            "reward_sku": "COFFEE",
            "reward_qty": 1,
            "starts_at": "2026-01-01T00:00:00.000Z",
            "ends_at": "2026-12-31T23:59:59.000Z",
            "min_order_minor": 1000,
            "category_id": "cat-drinks",
            "active": true
        }"#;
        let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "Buy 2 Get 1");
        assert_eq!(args.description, "Buy two coffees, get one free");
        assert_eq!(args.promo_type, "buy_x_get_y");
        assert_eq!(args.value_minor, 100);
        assert_eq!(args.min_qty, Some(2));
        assert_eq!(args.trigger_sku.as_deref(), Some("COFFEE"));
        assert_eq!(args.reward_sku.as_deref(), Some("COFFEE"));
        assert_eq!(args.reward_qty, Some(1));
        assert_eq!(args.min_order_minor, 1000);
        assert_eq!(args.category_id.as_deref(), Some("cat-drinks"));
        assert!(args.active);
    }

    #[test]
    fn create_promotion_args_active_defaults_true() {
        let json = r#"{"name":"test","promo_type":"fixed_amount","value_minor":500}"#;
        let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
        assert!(args.active, "active should default to true");
    }

    #[test]
    fn create_promotion_args_explicit_inactive() {
        let json =
            r#"{"name":"Disabled Promo","promo_type":"percentage","value_minor":5,"active":false}"#;
        let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
        assert!(!args.active);
    }

    #[test]
    fn create_promotion_args_debug() {
        let args = CreatePromotionArgs {
            name: "Flash Sale".into(),
            description: "Limited time".into(),
            promo_type: "percentage".into(),
            value_minor: 20,
            min_qty: None,
            trigger_sku: None,
            reward_sku: None,
            reward_qty: None,
            starts_at: Some("2026-07-01T00:00:00.000Z".into()),
            ends_at: Some("2026-07-07T23:59:59.000Z".into()),
            min_order_minor: 0,
            category_id: None,
            active: true,
        };
        let debug = format!("{args:?}");
        assert!(debug.contains("Flash Sale"));
        assert!(debug.contains("percentage"));
        assert!(debug.contains("2026-07-01"));
    }

    #[test]
    fn promotions_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }
}
/// List all promotions.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_promotions_scoped`.
#[command]
pub async fn list_promotions(state: State<'_, AppState>) -> Result<Vec<Promotion>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.list_promotions()?)
}

/// List promotions for the store resolved from a session token. ADR #7.
#[command]
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

/// Get a single promotion by id.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_promotion_scoped`.
#[command]
pub async fn get_promotion(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<Promotion>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_promotion(&id)?)
}

/// Get a promotion from the store resolved from a session token. ADR #7.
#[command]
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

/// Create a new promotion.
///
/// **Deprecated for multi-store (ADR #7):** Use `create_promotion_scoped`.
#[command]
pub async fn create_promotion(
    user_id: String,
    args: CreatePromotionArgs,
    state: State<'_, AppState>,
) -> Result<Promotion, AppError> {
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

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::PROMOTIONS_CREATE)?;

    Ok(store.create_promotion(&promo)?)
}

/// Create a promotion in the store resolved from a session token. ADR #7.
#[command]
pub async fn create_promotion_scoped(
    session_token: String,
    args: CreatePromotionArgs,
    state: State<'_, AppState>,
) -> Result<Promotion, AppError> {
    let session = state.resolve_session(&session_token)?;
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
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::PROMOTIONS_CREATE,
    )?;
    let result = store.create_promotion(&promo)?;
    drop(db);
    Ok(result)
}

/// Update an existing promotion.
///
/// **Deprecated for multi-store (ADR #7):** Use `update_promotion_scoped`.
#[command]
pub async fn update_promotion(
    user_id: String,
    promotion: Promotion,
    state: State<'_, AppState>,
) -> Result<Promotion, AppError> {
    let mut p = promotion;
    p.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::PROMOTIONS_EDIT)?;

    Ok(store.update_promotion(&p)?)
}

/// Update a promotion in the store resolved from a session token. ADR #7.
#[command]
pub async fn update_promotion_scoped(
    session_token: String,
    promotion: Promotion,
    state: State<'_, AppState>,
) -> Result<Promotion, AppError> {
    let session = state.resolve_session(&session_token)?;
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
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::PROMOTIONS_EDIT,
    )?;
    let result = store.update_promotion(&p)?;
    drop(db);
    Ok(result)
}

/// Delete a promotion by id.
///
/// **Deprecated for multi-store (ADR #7):** Use `delete_promotion_scoped`.
#[command]
pub async fn delete_promotion(
    user_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::PROMOTIONS_DELETE)?;

    Ok(store.delete_promotion(&id)?)
}

/// Delete a promotion in the store resolved from a session token. ADR #7.
#[command]
pub async fn delete_promotion_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(
        &store,
        &session.user_id,
        oz_core::permissions::PROMOTIONS_DELETE,
    )?;
    store.delete_promotion(&id)?;
    drop(db);
    Ok(())
}

/// Apply a promotion to a sale.
///
/// **Deprecated for multi-store (ADR #7):** Use `apply_promotion_scoped`.
#[command]
pub async fn apply_promotion(
    user_id: String,
    sale_id: String,
    promotion_id: String,
    state: State<'_, AppState>,
) -> Result<PromotionApplication, AppError> {
    let db = state.db.lock().await;
    run_apply_promotion(&db, &sale_id, &promotion_id, &user_id)
}

/// Apply a promotion in the store resolved from a session token. ADR #7.
#[command]
pub async fn apply_promotion_scoped(
    session_token: String,
    sale_id: String,
    promotion_id: String,
    state: State<'_, AppState>,
) -> Result<PromotionApplication, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_apply_promotion(&db, &sale_id, &promotion_id, &session.user_id)
}

/// Shared business logic for applying a promotion.
fn run_apply_promotion(
    db: &rusqlite::Connection,
    sale_id: &str,
    promotion_id: &str,
    user_id: &str,
) -> Result<PromotionApplication, AppError> {
    let store = Store::new(db);

    require_permission_for_user(&store, user_id, oz_core::permissions::PROMOTIONS_APPLY)?;

    let promo = store
        .get_promotion(promotion_id)?
        .ok_or_else(|| AppError::Invalid(format!("promotion {promotion_id} not found")))?;

    let sale = store
        .get_sale(sale_id)?
        .ok_or_else(|| AppError::Invalid(format!("sale {sale_id} not found")))?;

    if !promo.active {
        return Err(AppError::Invalid("promotion is not active".into()));
    }

    if let Some(ref starts_at) = promo.starts_at {
        let now = chrono::Utc::now();
        let start = chrono::DateTime::parse_from_rfc3339(starts_at)
            .map_err(|e| AppError::Invalid(format!("invalid starts_at: {e}")))?;
        if now < start {
            return Err(AppError::Invalid("promotion has not started yet".into()));
        }
    }
    if let Some(ref ends_at) = promo.ends_at {
        let now = chrono::Utc::now();
        let end = chrono::DateTime::parse_from_rfc3339(ends_at)
            .map_err(|e| AppError::Invalid(format!("invalid ends_at: {e}")))?;
        if now > end {
            return Err(AppError::Invalid("promotion has expired".into()));
        }
    }

    if sale.total.minor_units < promo.min_order_minor {
        return Err(AppError::Invalid(format!(
            "sale total {} is below minimum order {}",
            sale.total.minor_units, promo.min_order_minor
        )));
    }

    let discount_minor = match promo.promo_type.as_str() {
        "percentage" => {
            let total = sale.total.minor_units;
            (total * promo.value_minor) / 100
        }
        "fixed_amount" => promo.value_minor.min(sale.total.minor_units),
        "buy_x_get_y" => {
            let trigger_sku = promo.trigger_sku.as_deref().unwrap_or_default();
            let reward_sku = promo.reward_sku.as_deref().unwrap_or(trigger_sku);
            let min_qty = promo.min_qty.unwrap_or(1);
            let reward_qty = promo.reward_qty.unwrap_or(1);

            let trigger_qty: i64 = sale
                .lines
                .iter()
                .filter(|l| l.sku == trigger_sku)
                .map(|l| l.qty)
                .sum();

            let reward_lines: Vec<&oz_core::SaleLine> =
                sale.lines.iter().filter(|l| l.sku == reward_sku).collect();

            if trigger_qty < min_qty || reward_lines.is_empty() {
                0
            } else {
                let cheapest = reward_lines
                    .iter()
                    .map(|l| l.unit_price.minor_units)
                    .min()
                    .unwrap_or(0);
                let applicable = reward_qty.min(reward_lines.iter().map(|l| l.qty).sum::<i64>());
                (cheapest * applicable * promo.value_minor) / 100
            }
        }
        _ => {
            return Err(AppError::Invalid(format!(
                "unknown promo_type: {}",
                promo.promo_type
            )));
        }
    };

    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let app = PromotionApplication {
        id: uuid::Uuid::now_v7().to_string(),
        promotion_id: promotion_id.to_string(),
        sale_id: sale_id.to_string(),
        discount_minor,
        description: format!("{}: {:.2} off", promo.name, discount_minor as f64 / 100.0),
        created_at: now,
    };

    Ok(store.record_promotion_application(&app)?)
}

/// List all promotion applications for a sale.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_sale_promotions_scoped`.
#[command]
pub async fn get_sale_promotions(
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PromotionApplication>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_promotion_applications_for_sale(&sale_id)?)
}

/// Get sale promotions from the store resolved from a session token. ADR #7.
#[command]
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
