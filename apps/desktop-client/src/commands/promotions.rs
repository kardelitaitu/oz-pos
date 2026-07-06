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

/// List all promotions.
#[command]
pub async fn list_promotions(state: State<'_, AppState>) -> Result<Vec<Promotion>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.list_promotions()?)
}

/// Get a single promotion by id.
#[command]
pub async fn get_promotion(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<Promotion>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_promotion(&id)?)
}

/// Create a new promotion.
#[command]
pub async fn create_promotion(
    user_id: String,
    args: CreatePromotionArgs,
    state: State<'_, AppState>,
) -> Result<Promotion, AppError> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let promo = Promotion {
        id: uuid::Uuid::new_v4().to_string(),
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

    // Authorize
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::PROMOTIONS_CREATE)?;

    Ok(store.create_promotion(&promo)?)
}

/// Update an existing promotion.
#[command]
pub async fn update_promotion(
    user_id: String,
    promotion: Promotion,
    state: State<'_, AppState>,
) -> Result<Promotion, AppError> {
    let mut p = promotion;
    p.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    // Authorize
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::PROMOTIONS_EDIT)?;

    Ok(store.update_promotion(&p)?)
}

/// Delete a promotion by id.
#[command]
pub async fn delete_promotion(
    user_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Authorize
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::PROMOTIONS_DELETE)?;

    Ok(store.delete_promotion(&id)?)
}

/// Apply a promotion to a sale.
///
/// Calculates the discount based on the promotion type, validates time
/// and minimum-order constraints, and records the application.
#[command]
pub async fn apply_promotion(
    user_id: String,
    sale_id: String,
    promotion_id: String,
    state: State<'_, AppState>,
) -> Result<PromotionApplication, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Authorize
    require_permission_for_user(&store, &user_id, oz_core::permissions::PROMOTIONS_APPLY)?;

    let promo = store
        .get_promotion(&promotion_id)?
        .ok_or_else(|| AppError::Invalid(format!("promotion {promotion_id} not found")))?;

    let sale = store
        .get_sale(&sale_id)?
        .ok_or_else(|| AppError::Invalid(format!("sale {sale_id} not found")))?;

    if !promo.active {
        return Err(AppError::Invalid("promotion is not active".into()));
    }

    // Time validation.
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

    // Minimum order validation.
    if sale.total.minor_units < promo.min_order_minor {
        return Err(AppError::Invalid(format!(
            "sale total {} is below minimum order {}",
            sale.total.minor_units, promo.min_order_minor
        )));
    }

    // Calculate discount based on promo_type.
    let discount_minor = match promo.promo_type.as_str() {
        "percentage" => {
            let total = sale.total.minor_units;
            (total * promo.value_minor) / 100
        }
        "fixed_amount" => promo.value_minor.min(sale.total.minor_units),
        "buy_x_get_y" => {
            // Simple calculation: find qualifying trigger items, apply discount
            // to the cheapest reward item(s).
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
                // Find the cheapest reward line unit price.
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
        id: uuid::Uuid::new_v4().to_string(),
        promotion_id: promotion_id.clone(),
        sale_id: sale_id.clone(),
        discount_minor,
        description: format!("{}: {:.2} off", promo.name, discount_minor as f64 / 100.0),
        created_at: now,
    };

    Ok(store.record_promotion_application(&app)?)
}

/// List all promotion applications for a sale.
#[command]
pub async fn get_sale_promotions(
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PromotionApplication>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_promotion_applications_for_sale(&sale_id)?)
}
