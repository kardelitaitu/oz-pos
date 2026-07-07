//! Stock transfer Tauri commands.
//!
//! Exposes CRUD + send/receive lifecycle operations to the front-end.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::stock_transfer::{StockTransfer, StockTransferLine};

use crate::error::AppError;
use crate::state::AppState;

/// A received quantity for a single transfer line.
#[derive(Debug, Deserialize)]
pub struct ReceivedLineInput {
    pub line_id: String,
    pub received_qty: i64,
}

#[derive(Debug, Serialize)]
pub struct TransferWithLines {
    pub transfer: StockTransfer,
    pub lines: Vec<StockTransferLine>,
}

#[command]
#[allow(clippy::too_many_arguments)]
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ReceivedLineInput ───────────────────────────────────────────────

    #[test]
    fn received_line_input_deserialize() {
        let json = r#"{"line_id":"l1","received_qty":5}"#;
        let args: ReceivedLineInput = serde_json::from_str(json).unwrap();
        assert_eq!(args.line_id, "l1");
        assert_eq!(args.received_qty, 5);
    }

    #[test]
    fn received_line_input_debug() {
        let args = ReceivedLineInput {
            line_id: "l2".into(),
            received_qty: 10,
        };
        let d = format!("{args:?}");
        assert!(d.contains("l2"));
    }

    // ── TransferWithLines ───────────────────────────────────────────────

    #[test]
    fn transfer_with_lines_debug() {
        let transfer = StockTransfer {
            id: "t1".into(),
            transfer_number: "TRF-001".into(),
            source_location: Some("WH-A".into()),
            destination_location: Some("WH-B".into()),
            source_terminal_id: None,
            destination_terminal_id: None,
            status: "draft".into(),
            notes: String::new(),
            created_by: "admin".into(),
            received_by: None,
            sent_at: None,
            received_at: None,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let twl = TransferWithLines {
            transfer,
            lines: vec![],
        };
        let d = format!("{twl:?}");
        assert!(d.contains("TRF-001"));
    }

    #[test]
    fn transfer_with_lines_serialize() {
        let transfer = StockTransfer {
            id: "t2".into(),
            transfer_number: "TRF-002".into(),
            source_location: None,
            destination_location: None,
            source_terminal_id: None,
            destination_terminal_id: None,
            status: "in_transit".into(),
            notes: "Rush".into(),
            created_by: "user1".into(),
            received_by: None,
            sent_at: None,
            received_at: None,
            created_at: "2025-02-01T00:00:00.000Z".into(),
            updated_at: "2025-02-01T00:00:00.000Z".into(),
        };
        let twl = TransferWithLines {
            transfer,
            lines: vec![],
        };
        let json = serde_json::to_value(&twl).unwrap();
        assert_eq!(json["transfer"]["transfer_number"], "TRF-002");
        assert_eq!(json["transfer"]["status"], "in_transit");
    }
}
