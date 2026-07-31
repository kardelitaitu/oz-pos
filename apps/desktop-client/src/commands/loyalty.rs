use serde::Serialize;
use tauri::State;

use oz_core::db::Store;
use oz_core::loyalty::{
    LoyaltyAccount, LoyaltyAccountWithDetails, LoyaltyTier, LoyaltyTransaction,
};
use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
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

/// Retrieves a loyalty account from the store resolved by the active session.
#[tauri::command]
pub async fn get_loyalty_account_scoped(
    session_token: String,
    customer_id: String,
    state: State<'_, AppState>,
) -> Result<Option<LoyaltyAccountWithDetails>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::LOYALTY_VIEW)?;
    Ok(store.get_loyalty_account(&customer_id)?)
}

/// Lists loyalty accounts from the store resolved by the active session.
#[tauri::command]
pub async fn list_loyalty_accounts_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<LoyaltyAccountWithDetails>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::LOYALTY_VIEW)?;
    Ok(store.list_loyalty_accounts()?)
}

/// Awards loyalty points in the store resolved by the active session.
#[tauri::command]
pub async fn earn_loyalty_points_scoped(
    session_token: String,
    customer_id: String,
    sale_id: String,
    total_minor: i64,
    state: State<'_, AppState>,
) -> Result<LoyaltyTransaction, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::LOYALTY_EARN)?;
    Ok(store.earn_points(&customer_id, &sale_id, total_minor)?)
}

/// Redeems loyalty points in the store resolved by the active session.
#[tauri::command]
pub async fn redeem_loyalty_points_scoped(
    session_token: String,
    customer_id: String,
    points: i64,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<RedeemResult, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::LOYALTY_REDEEM)?;
    let (transaction, discount_minor) = store.redeem_points(&customer_id, points, &sale_id)?;
    Ok(RedeemResult {
        transaction,
        discount_minor,
    })
}

/// Lists loyalty tiers from the store resolved by the active session.
#[tauri::command]
pub async fn list_loyalty_tiers_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<LoyaltyTier>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::LOYALTY_VIEW)?;
    Ok(store.list_tiers()?)
}

/// Updates a loyalty tier in the store resolved by the active session.
#[tauri::command]
pub async fn update_loyalty_tier_scoped(
    session_token: String,
    tier: LoyaltyTier,
    state: State<'_, AppState>,
) -> Result<LoyaltyTier, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::LOYALTY_MANAGE)?;
    Ok(store.update_tier(
        &tier.id,
        &tier.name,
        tier.min_points,
        tier.points_per_unit,
        tier.earn_multiplier,
        &tier.colour,
    )?)
}

/// Converts loyalty points into minor currency units in the active store.
#[tauri::command]
pub async fn get_points_value_scoped(
    session_token: String,
    points: i64,
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::LOYALTY_VIEW)?;
    Ok(store.get_points_value(points)?)
}

/// Retrieves or creates a loyalty account in the active store.
#[tauri::command]
pub async fn get_or_create_loyalty_account_scoped(
    session_token: String,
    customer_id: String,
    state: State<'_, AppState>,
) -> Result<LoyaltyAccount, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::LOYALTY_VIEW)?;
    Ok(store.get_or_create_loyalty_account(&customer_id)?)
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
