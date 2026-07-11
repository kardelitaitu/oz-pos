//! Hardware-facing Tauri commands: cash drawer, receipt printer, and
//! barcode scanner lifecycle (start/stop/list). All commands reach into
//! the HAL via `state.registry` — they never construct a concrete driver.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, command};
use tokio::sync::oneshot;

use oz_core::{Currency, Money, Settings};
use oz_hal::BarcodeScanner;
use oz_hal::drivers::receipt;

use crate::error::AppError;
use crate::state::AppState;

// ── Cash drawer ─────────────────────────────────────────

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
    drawer.open().await?;
    Ok(OpenCashDrawerResult { opened: true })
}

// ── Raw text receipt (legacy) ───────────────────────────

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
    printer.print_receipt(&args.body).await?;
    // Emit a completion event so the front-end can show a toast.
    if let Some(ref app) = state.app {
        let _ = app.emit("receipt:printed", serde_json::json!({ "lines": n }));
    }
    Ok(PrintReceiptResult { printed_lines: n })
}

// ── Structured sales receipt ────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PrintSalesReceiptArgs {
    pub date: String,
    pub receipt_number: String,
    pub items: Vec<LineItemDto>,
    pub subtotal: MoneyDto,
    pub tax: Option<MoneyDto>,
    pub total: MoneyDto,
    pub payments: Vec<PaymentDto>,
    #[serde(default)]
    pub table_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LineItemDto {
    pub name: String,
    pub quantity: u32,
    pub unit_price: MoneyDto,
    pub total_price: MoneyDto,
    #[serde(default)]
    pub tax_amount: Option<MoneyDto>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentDto {
    pub method: String,
    pub amount: MoneyDto,
    pub change: Option<MoneyDto>,
}

/// Flat serialisable representation of Money — the front-end sends
/// these instead of a nested Money object for simplicity.
#[derive(Debug, Deserialize)]
pub struct MoneyDto {
    pub minor_units: i64,
    pub currency: String,
}

impl MoneyDto {
    fn to_money(&self) -> Result<Money, AppError> {
        let currency: Currency = self
            .currency
            .parse()
            .map_err(|_| AppError::Invalid(format!("invalid currency code '{}'", self.currency)))?;
        Ok(Money {
            minor_units: self.minor_units,
            currency,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PrintSalesReceiptResult {
    pub printed: bool,
}

#[command]
pub async fn print_sales_receipt(
    args: PrintSalesReceiptArgs,
    state: State<'_, AppState>,
) -> Result<PrintSalesReceiptResult, AppError> {
    let printer = state
        .registry
        .printer("default")
        .await
        .ok_or_else(|| AppError::Invalid("no receipt printer registered".into()))?;

    // Load store info + display settings from the DB.
    let conn = state.db.lock().await;
    let store_name = Settings::get_store_name(&conn)?.unwrap_or_else(|| "OZ-POS Store".into());
    let store_address = Settings::get_store_address(&conn)?.unwrap_or_default();
    let store_tax_id = Settings::get_store_tax_id(&conn)?;
    let decimals = Settings::get_receipt_decimal_separator(&conn)?;
    let decimal_separator = match decimals.as_str() {
        "comma" => receipt::DecimalSeparator::Comma,
        "none" => receipt::DecimalSeparator::None,
        _ => receipt::DecimalSeparator::Dot,
    };
    let paper_width = match Settings::get_receipt_paper_width(&conn)?.as_str() {
        "narrow" => receipt::PaperWidth::Narrow,
        _ => receipt::PaperWidth::Standard,
    };
    let config = receipt::ReceiptConfig {
        paper_width,
        show_currency: Settings::get_receipt_show_currency(&conn)?,
        decimal_separator,
        show_tax: Settings::get_receipt_show_tax(&conn)?,
        footer: {
            let f = Settings::get_receipt_footer(&conn)?;
            if f.is_empty() { None } else { Some(f) }
        },
        show_table_number: Settings::get_receipt_show_table_number(&conn)?,
    };
    drop(conn); // release lock before printing

    let receipt = receipt::SalesReceipt {
        store: receipt::StoreInfo {
            name: store_name,
            address: store_address,
            tax_id: store_tax_id,
        },
        date: args.date,
        receipt_number: args.receipt_number,
        table_number: args.table_number,
        items: args
            .items
            .into_iter()
            .map(|i| {
                Ok::<_, AppError>(receipt::LineItem {
                    name: i.name,
                    quantity: i.quantity,
                    unit_price: i.unit_price.to_money()?,
                    total_price: i.total_price.to_money()?,
                    tax_amount: i.tax_amount.map(|t| t.to_money()).transpose()?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        subtotal: args.subtotal.to_money()?,
        tax: args.tax.map(|t| t.to_money()).transpose()?,
        total: args.total.to_money()?,
        payments: args
            .payments
            .into_iter()
            .map(|p| {
                Ok::<_, AppError>(receipt::PaymentInfo {
                    method: p.method,
                    amount: p.amount.to_money()?,
                    change: p.change.map(|c| c.to_money()).transpose()?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    };

    let data = receipt::format_sales_receipt(&receipt, &config);
    let line_count = receipt.items.len() + 6;

    printer.print_raw(&data).await?;

    if let Some(ref app) = state.app {
        let _ = app.emit(
            "receipt:printed",
            serde_json::json!({ "lines": line_count }),
        );
    }

    Ok(PrintSalesReceiptResult { printed: true })
}

// ── Barcode scanner ──────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ScannerInfo {
    pub id: String,
}

/// List all registered barcode scanners.
#[command]
pub async fn list_scanners(state: State<'_, AppState>) -> Result<Vec<ScannerInfo>, AppError> {
    let ids = state.registry.scanner_ids().await;
    Ok(ids.into_iter().map(|id| ScannerInfo { id }).collect())
}

/// Start a background polling task for the named scanner.
///
/// Every decoded barcode is emitted as a `barcode:scanned` event
/// with shape `{ code: String, symbology: String }`. Calling
/// `start_scanner` while a scanner is already running stops the
/// previous one first.
#[command]
pub async fn start_scanner(scanner_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    // Stop any existing scanner first.
    {
        let mut cancel = state.scanner_cancel.lock().await;
        if let Some(sender) = cancel.take() {
            let _ = sender.send(());
        }
    }

    let driver: Arc<dyn BarcodeScanner> = state
        .registry
        .scanner(&scanner_id)
        .await
        .ok_or_else(|| AppError::Invalid(format!("no scanner registered as '{scanner_id}'")))?;

    let app = state
        .app
        .clone()
        .ok_or_else(|| AppError::Internal("AppHandle unavailable".into()))?;

    let (tx, mut rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        // Attempt to connect (idempotent – a second connect is a no-op).
        let mut scanner = match driver.connect().await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(scanner = %scanner_id, error = %e, "scanner connect failed");
                let _ = app.emit(
                    "barcode:error",
                    serde_json::json!({ "error": e.to_string() }),
                );
                return;
            }
        };

        tracing::info!(scanner = %scanner_id, "barcode scanner started");

        loop {
            tokio::select! {
                _ = &mut rx => {
                    tracing::info!(scanner = %scanner_id, "barcode scanner stopped");
                    break;
                }
                result = scanner.poll(300) => {
                    match result {
                        Ok(Some(barcode)) => {
                            let payload = serde_json::json!({
                                "code": barcode.code,
                                "symbology": format!("{:?}", barcode.symbology),
                            });
                            let _ = app.emit("barcode:scanned", payload);
                        }
                        Ok(None) => {
                            // Timeout — loop again.
                        }
                        Err(e) => {
                            tracing::warn!(scanner = %scanner_id, error = %e, "scanner poll error");
                            let _ = app.emit("barcode:error", serde_json::json!({ "error": e.to_string() }));
                            // Keep trying after a brief backoff.
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }
        }
    });

    // Store the cancel-sender so a subsequent start_scanner or stop_scanner can shut it down.
    state.scanner_cancel.lock().await.replace(tx);

    Ok(())
}

/// Stop the active barcode scanner background task (if any).
#[command]
pub async fn stop_scanner(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut cancel = state.scanner_cancel.lock().await;
    if let Some(sender) = cancel.take() {
        let _ = sender.send(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_receipt_args_deserialise() {
        let json = r#"{"body":"COFFEE\n3.50\n"}"#;
        let args: PrintReceiptArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.body.lines().count(), 2);
    }

    #[test]
    fn money_dto_to_money() {
        let dto = MoneyDto {
            minor_units: 1550,
            currency: "USD".into(),
        };
        let m = dto.to_money().unwrap();
        assert_eq!(m.minor_units, 1550);
    }

    #[test]
    fn money_dto_invalid_currency() {
        let dto = MoneyDto {
            minor_units: 100,
            currency: "INVALID".into(),
        };
        assert!(dto.to_money().is_err());
    }

    #[test]
    fn print_sales_receipt_args_deserialise() {
        let json = r#"{
            "date": "01 Jan 2026",
            "receipt_number": "REC-001",
            "items": [
                {
                    "name": "Coffee",
                    "quantity": 1,
                    "unit_price": { "minor_units": 350, "currency": "USD" },
                    "total_price": { "minor_units": 350, "currency": "USD" }
                }
            ],
            "subtotal": { "minor_units": 350, "currency": "USD" },
            "total": { "minor_units": 350, "currency": "USD" },
            "payments": [
                {
                    "method": "CASH",
                    "amount": { "minor_units": 500, "currency": "USD" },
                    "change": { "minor_units": 150, "currency": "USD" }
                }
            ]
        }"#;
        let args: PrintSalesReceiptArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.date, "01 Jan 2026");
        assert_eq!(args.items.len(), 1);
        assert_eq!(args.payments.len(), 1);
    }

    // -- DTO struct tests --

    #[test]
    fn open_cash_drawer_args_default_device() {
        let json = r#"{}"#;
        let args: OpenCashDrawerArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.device_id, None);
    }

    #[test]
    fn open_cash_drawer_args_with_device() {
        let json = r#"{"device_id":"drawer-1"}"#;
        let args: OpenCashDrawerArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.device_id.as_deref(), Some("drawer-1"));
    }

    #[test]
    fn open_cash_drawer_args_debug() {
        let args = OpenCashDrawerArgs {
            device_id: Some("d".into()),
        };
        let d = format!("{args:?}");
        assert!(d.contains("d"));
    }

    #[test]
    fn open_cash_drawer_result_serialize() {
        let result = OpenCashDrawerResult { opened: true };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["opened"], true);
    }

    #[test]
    fn print_receipt_result_serialize() {
        let result = PrintReceiptResult { printed_lines: 42 };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["printed_lines"], 42);
    }

    #[test]
    fn scanner_info_serialize() {
        let info = ScannerInfo {
            id: "scanner-1".into(),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], "scanner-1");
    }

    #[test]
    fn scanner_info_debug() {
        let info = ScannerInfo { id: "s".into() };
        let d = format!("{info:?}");
        assert!(d.contains("s"));
    }

    #[test]
    fn print_receipt_args_deserialize() {
        let json = r#"{"body":"Hello\nWorld"}"#;
        let args: PrintReceiptArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.body.lines().count(), 2);
    }

    #[test]
    fn print_receipt_args_debug() {
        let args = PrintReceiptArgs {
            body: "test".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("test"));
    }

    #[test]
    fn line_item_dto_deserialize() {
        let json = r#"{"name":"Coffee","quantity":2,"unit_price":{"minor_units":350,"currency":"USD"},"total_price":{"minor_units":700,"currency":"USD"}}"#;
        let item: LineItemDto = serde_json::from_str(json).unwrap();
        assert_eq!(item.name, "Coffee");
        assert_eq!(item.quantity, 2);
        assert!(item.tax_amount.is_none());
    }

    #[test]
    fn payment_dto_deserialize() {
        let json = r#"{"method":"CASH","amount":{"minor_units":500,"currency":"USD"},"change":{"minor_units":150,"currency":"USD"}}"#;
        let p: PaymentDto = serde_json::from_str(json).unwrap();
        assert_eq!(p.method, "CASH");
        assert!(p.change.is_some());
    }
}
