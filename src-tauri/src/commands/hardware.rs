//! Hardware-facing Tauri commands: cash drawer, receipt printer, and the
//! barcode subscription event. All commands reach into the HAL via
//! `state.registry` — they never construct a concrete driver.

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, command};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct OpenCashDrawerArgs {
    /// Optional device id; defaults to "default" which is the mock drawer
    /// registered at startup.
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OpenCashDrawerResult {
    pub opened: bool,
}

#[command]
pub async fn open_cash_drawer(
    args: OpenCashDrawerArgs,
    state: State<'_, AppState>,
) -> Result<OpenCashDrawerResult, AppError> {
    let id = args.device_id.as_deref().unwrap_or("default");
    let drawer = state
        .registry
        .cash_drawer(id)
        .await
        .ok_or_else(|| AppError::Invalid(format!("no cash drawer registered as '{id}'")))?;
    drawer
        .open()
        .await
        .map_err(|e| AppError::Hardware(e.to_string()))?;
    Ok(OpenCashDrawerResult { opened: true })
}

#[derive(Debug, Deserialize)]
pub struct PrintReceiptArgs {
    /// Raw receipt text (lines separated by '\n'). ESC/POS commands are
    /// added by the printer driver; the command layer only knows about
    /// plain text.
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct PrintReceiptResult {
    pub printed_lines: usize,
}

#[command]
pub async fn print_receipt(
    args: PrintReceiptArgs,
    state: State<'_, AppState>,
) -> Result<PrintReceiptResult, AppError> {
    let printer = state
        .registry
        .printer("default")
        .await
        .ok_or_else(|| AppError::Invalid("no receipt printer registered".into()))?;
    let lines: Vec<&str> = args.body.lines().collect();
    let n = lines.len();
    printer
        .print_receipt(&args.body)
        .await
        .map_err(|e| AppError::Hardware(e.to_string()))?;
    // Emit a completion event so the front-end can show a toast.
    let _ = state
        .app
        .emit("receipt:printed", serde_json::json!({ "lines": n }));
    Ok(PrintReceiptResult { printed_lines: n })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_receipt_args_deserialise() {
        let json = r#"{"body":"COFFEE\n3.50\n"}"#;
        let args: PrintReceiptArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.body.lines().count(), 3);
    }
}
