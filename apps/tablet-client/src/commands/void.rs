//! Void sale command — void a completed sale and restore stock.
//!
//! Delegates to [`Store::void_sale`] which handles the status transition,
//! stock restoration, and audit logging inside a single transaction.

use serde::Deserialize;
use tauri::{State, command};

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
/// Returns the updated sale with status `Voided`.
#[command]
pub async fn void_sale(
    args: VoidSaleArgs,
    state: State<'_, AppState>,
) -> Result<oz_core::Sale, AppError> {
    let db = state.db.lock().await;
    let store = oz_core::db::Store::new(&db);

    let sale = store.void_sale(&args.sale_id, &args.user_id, &args.reason)?;
    drop(db);

    tracing::info!(sale_id = %args.sale_id, reason = %args.reason, "sale voided");
    Ok(sale)
}
