//! Void sale command — void a completed sale and restore stock.
//!
//! Delegates to [`Store::void_sale`] which handles the status transition,
//! stock restoration, and audit logging inside a single transaction.

use serde::Deserialize;
use tauri::{State, command};

use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct VoidSaleArgs {
    pub sale_id: String,
    pub user_id: String,
    pub reason: String,
}

/// Args for `void_sale_scoped` — identical to `VoidSaleArgs` but without
/// `user_id` (read from the session token instead).
#[derive(Debug, Deserialize)]
pub struct VoidSaleScopedArgs {
    pub sale_id: String,
    pub reason: String,
}

/// Void an active (completed) sale using the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `void_sale_scoped`
/// with a `session_token` instead. The `user_id` is read from the
/// resolved session, not passed as a frontend parameter.
#[command]
pub async fn void_sale(
    args: VoidSaleArgs,
    state: State<'_, AppState>,
) -> Result<oz_core::Sale, AppError> {
    let db = state.db.lock().await;
    let store = oz_core::db::Store::new(&db);

    // Permission check: caller must have sales:void (derived from user_id).
    require_permission_for_user(&store, &args.user_id, permissions::SALES_VOID)?;

    let sale = store.void_sale(&args.sale_id, &args.user_id, &args.reason)?;
    drop(db);

    tracing::info!(sale_id = %args.sale_id, reason = %args.reason, "sale voided");
    Ok(sale)
}

/// Void a sale within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `void_sale`. The `user_id` for permission
/// checks and the void operation is read from the resolved `SessionContext`.
#[command]
pub async fn void_sale_scoped(
    session_token: String,
    args: VoidSaleScopedArgs,
    state: State<'_, AppState>,
) -> Result<oz_core::Sale, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = oz_core::db::Store::new(&db);

    require_permission_for_user(&store, &session.user_id, permissions::SALES_VOID)?;

    let sale = store.void_sale(&args.sale_id, &session.user_id, &args.reason)?;
    drop(db);

    tracing::info!(sale_id = %args.sale_id, reason = %args.reason, "sale voided (scoped)");
    Ok(sale)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn void_sale_args_deserialize() {
        let json = r##"{"sale_id":"s1","user_id":"u1","reason":"Wrong item"}"##;
        let args: VoidSaleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sale_id, "s1");
        assert_eq!(args.user_id, "u1");
        assert_eq!(args.reason, "Wrong item");
    }

    #[test]
    fn void_sale_args_debug() {
        let args = VoidSaleArgs {
            sale_id: "s2".into(),
            user_id: "u2".into(),
            reason: "Test".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("s2"));
        assert!(d.contains("Test"));
    }

    #[test]
    fn void_sale_scoped_args_deserialize() {
        let json = r##"{"sale_id":"s1","reason":"Wrong item"}"##;
        let args: VoidSaleScopedArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sale_id, "s1");
        assert_eq!(args.reason, "Wrong item");
    }

    #[test]
    fn void_sale_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[test]
    fn void_sale_args_deserialize_empty_reason() {
        let json = r##"{"sale_id":"s3","user_id":"u3","reason":""}"##;
        let args: VoidSaleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sale_id, "s3");
        assert_eq!(args.reason, "");
    }
}
