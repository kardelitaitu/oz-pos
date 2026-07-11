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

/// Void an active (completed) sale.
///
/// Restores inventory for each line item and writes an audit log entry.
/// Requires `sales:void` permission.
/// Returns the updated sale with status `Voided`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn void_sale_args_deserialize() {
        let json = r#"{"sale_id":"s1","user_id":"u1","reason":"customer cancelled"}"#;
        let args: VoidSaleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sale_id, "s1");
        assert_eq!(args.user_id, "u1");
        assert_eq!(args.reason, "customer cancelled");
    }

    #[test]
    fn void_sale_args_debug() {
        let args = VoidSaleArgs {
            sale_id: "s2".into(),
            user_id: "u2".into(),
            reason: "wrong item".into(),
        };
        let debug = format!("{:?}", args);
        assert!(debug.contains("s2"));
        assert!(debug.contains("wrong item"));
    }

    #[test]
    fn void_sale_args_deserialize_empty_reason() {
        let json = r#"{"sale_id":"s3","user_id":"u3","reason":""}"#;
        let args: VoidSaleArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.sale_id, "s3");
        assert_eq!(args.reason, "");
    }
}
