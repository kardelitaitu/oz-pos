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
