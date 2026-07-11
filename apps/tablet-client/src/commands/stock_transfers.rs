//! Stock transfer Tauri commands (tablet).
//!
//! Mirrors the desktop client's stock transfer commands exactly.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::stock_transfer::{StockTransfer, StockTransferLine};

use crate::error::AppError;
use crate::state::AppState;

/// A received quantity for a single transfer line.
#[derive(Debug, Deserialize)]
pub struct ReceivedLineInput {
    /// ID of the associated line.
    pub line_id: String,
    /// Received Qty.
    pub received_qty: i64,
}

#[derive(Debug, Serialize)]
/// Transferwithlines.
pub struct TransferWithLines {
    /// Transfer.
    pub transfer: StockTransfer,
    /// Lines.
    pub lines: Vec<StockTransferLine>,
}

#[command]
/// Create stock transfer.
pub async fn create_stock_transfer(
    source_location: Option<String>,
    destination_location: Option<String>,
    source_terminal_id: Option<String>,
    destination_terminal_id: Option<String>,
    notes: String,
    created_by: String,
    lines: Vec<StockTransferLine>,
    state: State<'_, AppState>,
) -> Result<StockTransfer, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.create_transfer(
        source_location.as_deref(),
        destination_location.as_deref(),
        source_terminal_id.as_deref(),
        destination_terminal_id.as_deref(),
        &notes,
        &created_by,
        &lines,
    )?;
    drop(db);
    Ok(result)
}

#[command]
/// Get stock transfer.
pub async fn get_stock_transfer(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<TransferWithLines>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let transfer = store.get_transfer(&id)?;
    let lines = if transfer.is_some() {
        store.get_transfer_lines(&id)?
    } else {
        vec![]
    };
    drop(db);
    Ok(transfer.map(|t| TransferWithLines { transfer: t, lines }))
}

#[command]
/// List stock transfers.
pub async fn list_stock_transfers(
    state: State<'_, AppState>,
) -> Result<Vec<StockTransfer>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.list_transfers()?;
    drop(db);
    Ok(result)
}

#[command]
/// Get stock transfer lines.
pub async fn get_stock_transfer_lines(
    transfer_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<StockTransferLine>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.get_transfer_lines(&transfer_id)?;
    drop(db);
    Ok(result)
}

#[command]
/// Add stock transfer line.
pub async fn add_stock_transfer_line(
    transfer_id: String,
    sku: String,
    product_name: String,
    qty: i64,
    state: State<'_, AppState>,
) -> Result<StockTransferLine, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.add_transfer_line(&transfer_id, &sku, &product_name, qty)?;
    drop(db);
    Ok(result)
}

#[command]
/// Remove stock transfer line.
pub async fn remove_stock_transfer_line(
    line_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.remove_transfer_line(&line_id)?;
    drop(db);
    Ok(())
}

#[command]
/// Send stock transfer.
pub async fn send_stock_transfer(
    id: String,
    state: State<'_, AppState>,
) -> Result<StockTransfer, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.send_transfer(&id)?;
    drop(db);
    Ok(result)
}

#[command]
/// Receive stock transfer.
pub async fn receive_stock_transfer(
    id: String,
    received_by: String,
    received_lines: Vec<ReceivedLineInput>,
    state: State<'_, AppState>,
) -> Result<StockTransfer, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rls: Vec<oz_core::db::stock_transfers::ReceivedLine> = received_lines
        .into_iter()
        .map(|rl| oz_core::db::stock_transfers::ReceivedLine {
            line_id: rl.line_id,
            received_qty: rl.received_qty,
        })
        .collect();
    let result = store.receive_transfer(&id, &received_by, &rls)?;
    drop(db);
    Ok(result)
}

#[command]
/// Cancel stock transfer.
pub async fn cancel_stock_transfer(
    id: String,
    state: State<'_, AppState>,
) -> Result<StockTransfer, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let result = store.cancel_transfer(&id)?;
    drop(db);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn received_line_input_deserialize() {
        let json = r#"{"line_id":"l1","received_qty":5}"#;
        let input: ReceivedLineInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.line_id, "l1");
        assert_eq!(input.received_qty, 5);
    }

    #[test]
    fn received_line_input_debug() {
        let input = ReceivedLineInput {
            line_id: "l2".into(),
            received_qty: 10,
        };
        let debug = format!("{:?}", input);
        assert!(debug.contains("l2"));
        assert!(debug.contains("10"));
    }

    #[test]
    fn transfer_with_lines_serialize() {
        let transfer = StockTransfer {
            id: "t1".into(),
            source_location: Some("Warehouse".into()),
            destination_location: Some("Store A".into()),
            source_terminal_id: None,
            destination_terminal_id: None,
            status: "draft".into(),
            notes: "test transfer".into(),
            created_by: "admin".into(),
            sent_at: None,
            received_at: None,
            received_by: None,
            created_at: "2026-01-15T10:00:00Z".into(),
            updated_at: "2026-01-15T10:00:00Z".into(),
        };
        let lines: Vec<StockTransferLine> = vec![];
        let twl = TransferWithLines { transfer, lines };
        let json = serde_json::to_value(&twl).unwrap();
        assert_eq!(json["transfer"]["id"], "t1");
    }

    #[test]
    fn transfer_with_lines_debug() {
        let transfer = StockTransfer {
            id: "t2".into(),
            source_location: None,
            destination_location: None,
            source_terminal_id: None,
            destination_terminal_id: None,
            status: "draft".into(),
            notes: String::new(),
            created_by: "admin".into(),
            sent_at: None,
            received_at: None,
            received_by: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let twl = TransferWithLines {
            transfer,
            lines: vec![],
        };
        let debug = format!("{:?}", twl);
        assert!(debug.contains("t2"));
    }
}
