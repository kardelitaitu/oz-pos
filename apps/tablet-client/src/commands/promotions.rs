//! Promotion management commands (tablet mirror).

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

#[command]
pub async fn list_promotions(state: State<'_, AppState>) -> Result<Vec<Promotion>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.list_promotions()?)
}

#[command]
pub async fn get_promotion(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<Promotion>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_promotion(&id)?)
}

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

#[command]
pub async fn apply_promotion(
    user_id: String,
    sale_id: String,
    promotion_id: String,
    state: State<'_, AppState>,
) -> Result<PromotionApplication, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

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
        promotion_id,
        sale_id,
        discount_minor,
        description: format!("{}: {:.2} off", promo.name, discount_minor as f64 / 100.0),
        created_at: now,
    };

    Ok(store.record_promotion_application(&app)?)
}

#[command]
pub async fn get_sale_promotions(
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PromotionApplication>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    Ok(store.get_promotion_applications_for_sale(&sale_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_promotion_args_deserialize_minimal() {
        let json = r#"{"name":"Summer Sale","promo_type":"percentage","value_minor":10}"#;
        let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "Summer Sale");
        assert_eq!(args.promo_type, "percentage");
        assert_eq!(args.value_minor, 10);
        assert!(args.active);
        assert_eq!(args.min_order_minor, 0);
        assert_eq!(args.description, "");
    }

    #[test]
    fn create_promotion_args_deserialize_all_fields() {
        let json = r#"{"name":"Flash Deal","description":"Limited time","promo_type":"fixed_amount","value_minor":500,"min_qty":2,"trigger_sku":"SKU-A","reward_sku":"SKU-B","reward_qty":1,"starts_at":"2026-01-01","ends_at":"2026-12-31","min_order_minor":5000,"category_id":"cat-1","active":true}"#;
        let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.name, "Flash Deal");
        assert_eq!(args.min_order_minor, 5000);
        assert_eq!(args.category_id.unwrap(), "cat-1");
    }

    #[test]
    fn create_promotion_args_explicit_inactive() {
        let json = r#"{"name":"Draft","promo_type":"percentage","value_minor":5,"active":false}"#;
        let args: CreatePromotionArgs = serde_json::from_str(json).unwrap();
        assert!(!args.active);
    }

    #[test]
    fn create_promotion_args_debug() {
        let args = CreatePromotionArgs {
            name: "Test".into(),
            description: "Desc".into(),
            promo_type: "percentage".into(),
            value_minor: 10,
            min_qty: None,
            trigger_sku: None,
            reward_sku: None,
            reward_qty: None,
            starts_at: None,
            ends_at: None,
            min_order_minor: 0,
            category_id: None,
            active: true,
        };
        let debug = format!("{:?}", args);
        assert!(debug.contains("Test"));
    }
}
