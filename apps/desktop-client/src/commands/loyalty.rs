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

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::session::SessionContext;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

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

    #[tokio::test]
    async fn permission_check_uses_global_identity_db() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        // Narrow custom role: loyalty:view but NOT loyalty:manage — the new
        // role-staff preset grants both (0048 retirement sweep).
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-lite', 'Lite', 'Limited loyalty view', '[\"loyalty:view\"]', '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-lite', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
        )
        .unwrap();
        let state = AppState::for_test_with_conn(conn);

        assert!(
            require_loyalty_permission(&state, "user-cashier", permissions::LOYALTY_VIEW)
                .await
                .is_ok()
        );
        assert!(matches!(
            require_loyalty_permission(&state, "user-cashier", permissions::LOYALTY_MANAGE).await,
            Err(AppError::PermissionDenied(_))
        ));
    }

    #[tokio::test]
    async fn permission_check_rejects_missing_user() {
        let conn = oz_core::migrations::fresh_db();
        let state = AppState::for_test_with_conn(conn);

        assert!(matches!(
            require_loyalty_permission(&state, "missing-user", permissions::LOYALTY_VIEW).await,
            Err(AppError::PermissionDenied(_))
        ));
    }

    #[tokio::test]
    async fn scoped_command_rejects_invalid_session() {
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test())
            .build(tauri::generate_context!())
            .unwrap();

        let result = list_loyalty_accounts_scoped("missing-token".into(), app.state()).await;
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[tokio::test]
    async fn scoped_command_denies_user_without_loyalty_permission() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-custom', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        state.session_store.write().unwrap().insert(
            "cashier-token".into(),
            SessionContext::new(
                "user-cashier".into(),
                "role-owner".into(),
                "terminal-1".into(),
                "store-cashier".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = list_loyalty_accounts_scoped("cashier-token".into(), app.state()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn scoped_command_reads_only_the_session_store() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        for (token, store_id) in [("store-a-token", "store-a"), ("store-b-token", "store-b")] {
            state.session_store.write().unwrap().insert(
                token.into(),
                SessionContext::new(
                    "user-owner".into(),
                    "role-owner".into(),
                    "terminal-1".into(),
                    store_id.into(),
                    "instance-1".into(),
                    "pos".into(),
                    None,
                    0,
                ),
            );
        }

        {
            let store_a_conn = state.db_manager.open_store("store-a").unwrap();
            let store_a_db = store_a_conn.lock().unwrap();
            store_a_db
                .execute(
                    "INSERT INTO customers (id, name) VALUES ('customer-a', 'Store A Customer')",
                    [],
                )
                .unwrap();
            Store::new(&store_a_db)
                .get_or_create_loyalty_account("customer-a")
                .unwrap();
        }

        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let store_a_accounts = list_loyalty_accounts_scoped("store-a-token".into(), app.state())
            .await
            .unwrap();
        let store_b_accounts = list_loyalty_accounts_scoped("store-b-token".into(), app.state())
            .await
            .unwrap();
        assert_eq!(store_a_accounts.len(), 1);
        assert_eq!(store_a_accounts[0].account.customer_id, "customer-a");
        assert!(
            store_b_accounts.is_empty(),
            "store B must not see store A loyalty data"
        );
    }
}
