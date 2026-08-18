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

/// Verify a loyalty permission against the global identity database.
///
/// Users and roles are global authentication records; loyalty business data
/// is read from the store-scoped connection after this check succeeds.
async fn require_loyalty_permission(
    state: &AppState,
    user_id: &str,
    permission: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, user_id, permission)
}

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
    require_loyalty_permission(&state, &session.user_id, permissions::LOYALTY_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    Ok(store.get_loyalty_account(&customer_id)?)
}

/// Lists loyalty accounts from the store resolved by the active session.
#[tauri::command]
pub async fn list_loyalty_accounts_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<LoyaltyAccountWithDetails>, AppError> {
    let (session, conn) = state.resolve_scope(&session_token)?;
    require_loyalty_permission(&state, &session.user_id, permissions::LOYALTY_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
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
    require_loyalty_permission(&state, &session.user_id, permissions::LOYALTY_EARN).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
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
    require_loyalty_permission(&state, &session.user_id, permissions::LOYALTY_REDEEM).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
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
    require_loyalty_permission(&state, &session.user_id, permissions::LOYALTY_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
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
    require_loyalty_permission(&state, &session.user_id, permissions::LOYALTY_MANAGE).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
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
    require_loyalty_permission(&state, &session.user_id, permissions::LOYALTY_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
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
    require_loyalty_permission(&state, &session.user_id, permissions::LOYALTY_VIEW).await?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    Ok(store.get_or_create_loyalty_account(&customer_id)?)
}

#[cfg(test)] #[path = "loyalty_tests.rs"] mod tests;
