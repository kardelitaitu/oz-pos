use serde::Serialize;
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::loyalty::{
    LoyaltyAccount, LoyaltyAccountWithDetails, LoyaltyTier, LoyaltyTransaction,
};

use crate::error::AppError;
use crate::state::AppState;

/// The result of a successful loyalty points redemption.
#[derive(Debug, Serialize)]
pub struct RedeemResult {
    /// The ledger transaction recording the points deduction.
    pub transaction: LoyaltyTransaction,
    /// The calculated discount amount in minor currency units.
    pub discount_minor: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_txn() -> LoyaltyTransaction {
        LoyaltyTransaction {
            id: "txn-1".into(),
            account_id: "acct-1".into(),
            sale_id: Some("sale-1".into()),
            points: -100,
            txn_type: "redeem".into(),
            description: "Redeemed 100 points".into(),
            created_at: "2025-01-01T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn redeem_result_debug() {
        let result = RedeemResult {
            transaction: sample_txn(),
            discount_minor: 100,
        };
        let debug = format!("{result:?}");
        assert!(debug.contains("redeem"));
        assert!(debug.contains("100"));
    }

    #[test]
    fn redeem_result_serialize() {
        let result = RedeemResult {
            transaction: sample_txn(),
            discount_minor: 50,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["discount_minor"], 50);
        assert!(json["transaction"].is_object());
        assert_eq!(json["transaction"]["txn_type"], "redeem");
        assert_eq!(json["transaction"]["points"], -100);
    }

    #[test]
    fn redeem_result_zero_discount() {
        let result = RedeemResult {
            transaction: sample_txn(),
            discount_minor: 0,
        };
        assert_eq!(result.discount_minor, 0);
    }
}

/// Retrieves the loyalty account details and tier information for a specific customer.
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

/// Lists all registered loyalty accounts along with their tier details.
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

/// Awards loyalty points to a customer based on the total value of a completed sale.
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

/// Redeems loyalty points for a customer against a specific sale and returns the discount value.
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

/// Lists all defined loyalty tiers and their thresholds.
#[command]
pub async fn list_loyalty_tiers(state: State<'_, AppState>) -> Result<Vec<LoyaltyTier>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.list_tiers()?;
    drop(db);
    Ok(result)
}

/// Updates the definition, multipliers, thresholds, or styling of a loyalty tier.
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

/// Converts a given amount of loyalty points into its equivalent monetary value in minor units.
#[command]
pub async fn get_points_value(points: i64, state: State<'_, AppState>) -> Result<i64, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.get_points_value(points);
    drop(db);
    Ok(result)
}

/// Retrieves an existing loyalty account for a customer, or creates a new one if it does not exist.
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
