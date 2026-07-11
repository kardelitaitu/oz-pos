use serde::Serialize;
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::loyalty::{
    LoyaltyAccount, LoyaltyAccountWithDetails, LoyaltyTier, LoyaltyTransaction,
};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct RedeemResult {
    pub transaction: LoyaltyTransaction,
    pub discount_minor: i64,
}

#[command]
pub async fn get_loyalty_account(
    customer_id: String,
    state: State<'_, AppState>,
) -> Result<Option<LoyaltyAccountWithDetails>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.get_loyalty_account(&customer_id)?;
    drop(db);
    Ok(result)
}

#[command]
pub async fn list_loyalty_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<LoyaltyAccountWithDetails>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.list_loyalty_accounts()?;
    drop(db);
    Ok(result)
}

#[command]
pub async fn earn_loyalty_points(
    customer_id: String,
    sale_id: String,
    total_minor: i64,
    state: State<'_, AppState>,
) -> Result<LoyaltyTransaction, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.earn_points(&customer_id, &sale_id, total_minor)?;
    drop(db);
    Ok(result)
}

#[command]
pub async fn redeem_loyalty_points(
    customer_id: String,
    points: i64,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<RedeemResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let (transaction, discount_minor) = store.redeem_points(&customer_id, points, &sale_id)?;
    drop(db);
    Ok(RedeemResult {
        transaction,
        discount_minor,
    })
}

#[command]
pub async fn list_loyalty_tiers(state: State<'_, AppState>) -> Result<Vec<LoyaltyTier>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.list_tiers()?;
    drop(db);
    Ok(result)
}

#[command]
pub async fn update_loyalty_tier(
    tier: LoyaltyTier,
    state: State<'_, AppState>,
) -> Result<LoyaltyTier, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.update_tier(
        &tier.id,
        &tier.name,
        tier.min_points,
        tier.points_per_unit,
        tier.earn_multiplier,
        &tier.colour,
    )?;
    drop(db);
    Ok(result)
}

#[command]
pub async fn get_points_value(points: i64, state: State<'_, AppState>) -> Result<i64, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.get_points_value(points);
    drop(db);
    Ok(result)
}

#[command]
pub async fn get_or_create_loyalty_account(
    customer_id: String,
    state: State<'_, AppState>,
) -> Result<LoyaltyAccount, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.get_or_create_loyalty_account(&customer_id)?;
    drop(db);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redeem_result_debug() {
        let tx = LoyaltyTransaction {
            id: "tx1".into(),
            account_id: "a1".into(),
            points: -200,
            txn_type: "redeem".into(),
            sale_id: Some("s1".into()),
            description: "Redeemed 200 points".into(),
            created_at: "2026-01-15T10:00:00Z".into(),
        };
        let r = RedeemResult {
            transaction: tx,
            discount_minor: 1000,
        };
        let debug = format!("{:?}", r);
        assert!(debug.contains("1000"));
    }

    #[test]
    fn redeem_result_serialize() {
        let tx = LoyaltyTransaction {
            id: "tx2".into(),
            account_id: "a2".into(),
            points: -100,
            txn_type: "redeem".into(),
            sale_id: Some("s2".into()),
            description: "Redeemed".into(),
            created_at: "2026-01-15T10:00:00Z".into(),
        };
        let r = RedeemResult {
            transaction: tx,
            discount_minor: 500,
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["discount_minor"], 500);
        assert_eq!(json["transaction"]["points"], -100);
    }

    #[test]
    fn redeem_result_zero_discount() {
        let tx = LoyaltyTransaction {
            id: "tx3".into(),
            account_id: "a3".into(),
            points: 0,
            txn_type: "check".into(),
            sale_id: None,
            description: "Balance check".into(),
            created_at: "2026-01-15T10:00:00Z".into(),
        };
        let r = RedeemResult {
            transaction: tx,
            discount_minor: 0,
        };
        assert_eq!(r.discount_minor, 0);
    }
}
