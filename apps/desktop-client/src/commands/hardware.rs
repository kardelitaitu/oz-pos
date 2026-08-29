//! Hardware-facing Tauri commands: cash drawer, receipt printer, and
//! barcode scanner lifecycle (start/stop/list). All commands reach into
//! the HAL via `state.registry` — they never construct a concrete driver.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use tokio::sync::oneshot;

use oz_core::{Currency, Money, Settings};
use oz_hal::drivers::receipt;
use oz_hal::transport::usb::{UsbDeviceInfo, probe_all};
use oz_hal::{BarcodeScanner, DisplayContent};

use crate::error::AppError;
use crate::state::AppState;

// ── Cash drawer ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Opencashdrawerargs.
pub struct OpenCashDrawerArgs {
    /// Optional device id; defaults to "default" which is the mock drawer
    /// registered at startup.
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
/// Opencashdrawerresult.
pub struct OpenCashDrawerResult {
    /// Opened.
    pub opened: bool,
}

#[tauri::command]
/// Open cash drawer.
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
/// Printreceiptargs.
pub struct PrintReceiptArgs {
    /// Raw receipt text (lines separated by '\n'). ESC/POS commands are
    /// added by the printer driver; the command layer only knows about
    /// plain text.
    pub body: String,
}

#[derive(Debug, Serialize)]
/// Printreceiptresult.
pub struct PrintReceiptResult {
    /// Printed Lines.
    pub printed_lines: usize,
}

#[tauri::command]
/// Print receipt.
pub async fn print_receipt(
    args: PrintReceiptArgs,
    state: State<'_, AppState>,
) -> Result<PrintReceiptResult, AppError> {
    let printer = state
        .registry
        .printer("default")
        .await
        .ok_or_else(|| AppError::Invalid("no receipt printer registered".into()))?;

    // Check printer status before printing
    let status = printer.get_status().await?;
    if status.has_fault() {
        return Err(AppError::Invalid(
            "Printer is not ready: check paper supply and cover".into(),
        ));
    }
    if status.paper != oz_hal::PaperStatus::Ok {
        // Low paper — warn but continue
        tracing::warn!(
            paper = ?status.paper,
            "printer paper is low, continuing"
        );
    }

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
#[serde(rename_all = "camelCase")]
/// Printsalesreceiptargs.
pub struct PrintSalesReceiptArgs {
    /// Date.
    pub date: String,
    /// Receipt Number.
    pub receipt_number: String,
    /// Items.
    pub items: Vec<LineItemDto>,
    /// Subtotal.
    pub subtotal: MoneyDto,
    /// Tax.
    pub tax: Option<MoneyDto>,
    /// Total amount in minor currency units.
    pub total: MoneyDto,
    /// Payments.
    pub payments: Vec<PaymentDto>,
    #[serde(default)]
    /// Table Number.
    pub table_number: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Lineitemdto.
pub struct LineItemDto {
    /// Display name.
    pub name: String,
    /// Quantity.
    pub quantity: u32,
    /// Unit price in minor currency units.
    pub unit_price: MoneyDto,
    /// Total Price.
    pub total_price: MoneyDto,
    #[serde(default)]
    /// Tax Amount.
    pub tax_amount: Option<MoneyDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Paymentdto.
pub struct PaymentDto {
    /// Method.
    pub method: String,
    /// Amount.
    pub amount: MoneyDto,
    /// Change.
    pub change: Option<MoneyDto>,
}

/// Flat serialisable representation of Money — the front-end sends
/// these instead of a nested Money object for simplicity.
#[derive(Debug, Deserialize)]
pub struct MoneyDto {
    /// Minor Units.
    pub minor_units: i64,
    /// ISO-4217 currency code.
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
/// Printsalesreceiptresult.
pub struct PrintSalesReceiptResult {
    /// Printed.
    pub printed: bool,
}

#[tauri::command]
/// Print sales receipt.
///
/// **Deprecated for multi-store (ADR #7):** Use `print_sales_receipt_scoped`.
pub async fn print_sales_receipt(
    args: PrintSalesReceiptArgs,
    state: State<'_, AppState>,
) -> Result<PrintSalesReceiptResult, AppError> {
    let (config, store_info) = {
        let db = state.db.lock().await;
        read_receipt_config(&db)?
    }; // MutexGuard dropped here before any .await
    run_print_receipt_inner(args, config, store_info, state).await
}

/// Print sales receipt for the store resolved from a session token. ADR #7.
/// Settings (store name, address, receipt config) are loaded from the
/// store-scoped database, while the printer hardware itself is not
/// store-specific.
#[tauri::command]
pub async fn print_sales_receipt_scoped(
    session_token: String,
    args: PrintSalesReceiptArgs,
    state: State<'_, AppState>,
) -> Result<PrintSalesReceiptResult, AppError> {
    let (config, store_info) = {
        let conn = state.resolve_store(&session_token)?;
        let db = conn
            .lock()
            .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
        read_receipt_config(&db)?
    }; // MutexGuard dropped here before any .await
    run_print_receipt_inner(args, config, store_info, state).await
}

/// Read receipt configuration and store info from the DB (synchronous — no async).
fn read_receipt_config(
    conn: &rusqlite::Connection,
) -> Result<(receipt::ReceiptConfig, receipt::StoreInfo), AppError> {
    let store_name = Settings::get_store_name(conn)?.unwrap_or_else(|| "OZ-POS Store".into());
    let store_address = Settings::get_store_address(conn)?.unwrap_or_default();
    let store_tax_id = Settings::get_store_tax_id(conn)?;
    let decimals = Settings::get_receipt_decimal_separator(conn)?;
    let decimal_separator = match decimals.as_str() {
        "comma" => receipt::DecimalSeparator::Comma,
        "none" => receipt::DecimalSeparator::None,
        _ => receipt::DecimalSeparator::Dot,
    };
    let paper_width = match Settings::get_receipt_paper_width(conn)?.as_str() {
        "narrow" => receipt::PaperWidth::Narrow,
        _ => receipt::PaperWidth::Standard,
    };
    let config = receipt::ReceiptConfig {
        paper_width,
        show_currency: Settings::get_receipt_show_currency(conn)?,
        decimal_separator,
        show_tax: Settings::get_receipt_show_tax(conn)?,
        footer: {
            let f = Settings::get_receipt_footer(conn)?;
            if f.is_empty() { None } else { Some(f) }
        },
        show_table_number: Settings::get_receipt_show_table_number(conn)?,
        barcode_enabled: false,
        payment_link_template: None,
    };
    let store_info = receipt::StoreInfo {
        name: store_name,
        address: store_address,
        tax_id: store_tax_id,
    };
    Ok((config, store_info))
}

/// Async inner: format and print receipt (no DB reference — all config already loaded).
pub async fn run_print_receipt_inner(
    args: PrintSalesReceiptArgs,
    config: receipt::ReceiptConfig,
    store_info: receipt::StoreInfo,
    state: State<'_, AppState>,
) -> Result<PrintSalesReceiptResult, AppError> {
    let printer = state
        .registry
        .printer("default")
        .await
        .ok_or_else(|| AppError::Invalid("no receipt printer registered".into()))?;

    // Check printer status before printing
    let status = printer.get_status().await?;
    if status.has_fault() {
        return Err(AppError::Invalid(
            "Printer is not ready: check paper supply and cover".into(),
        ));
    }
    if status.paper != oz_hal::PaperStatus::Ok {
        tracing::warn!(
            paper = ?status.paper,
            "printer paper is low, continuing"
        );
    }

    let receipt = receipt::SalesReceipt {
        store: store_info,
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
/// Scannerinfo.
pub struct ScannerInfo {
    /// Unique identifier.
    pub id: String,
}

/// List all registered barcode scanners.
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
pub async fn stop_scanner(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut cancel = state.scanner_cancel.lock().await;
    if let Some(sender) = cancel.take() {
        let _ = sender.send(());
    }
    Ok(())
}

// ── Customer Display ───────────────────────────────────

/// List all registered customer displays.
#[tauri::command]
pub async fn list_displays(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    Ok(state.registry.display_ids().await)
}

#[derive(Debug, Deserialize)]
/// Displayshowargs.
pub struct DisplayShowArgs {
    /// ID of the associated display.
    pub display_id: String,
    /// Line1.
    pub line1: String,
    /// Line2.
    pub line2: String,
}

/// Show content on a customer-facing pole display.
#[tauri::command]
pub async fn display_show(
    args: DisplayShowArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let display = state
        .registry
        .display(&args.display_id)
        .await
        .ok_or_else(|| {
            AppError::Invalid(format!("no display registered as '{}'", args.display_id))
        })?;
    let content = DisplayContent {
        line1: args.line1,
        line2: args.line2,
    };
    display.connect().await?;
    display.show(&content).await?;
    Ok(())
}

/// Discover all connected USB hardware devices (scanners, printers, scales).
///
/// Calls `oz_hal::transport::usb::probe_all()` to enumerate known USB
/// devices. Returns an empty vec (not an error) when no USB hardware is
/// found — the front-end can fall back to manual configuration.
#[tauri::command]
pub async fn discover_hardware() -> Result<Vec<UsbDeviceInfo>, AppError> {
    // probe_all is synchronous USB enumeration — no blocking issues for
    // a one-shot discovery call. On Windows/macOS the rusb context init
    // is fast; on Linux it depends on udev being available.
    match probe_all() {
        Ok(devices) => Ok(devices),
        Err(e) => Err(AppError::Internal(format!(
            "hardware discovery failed: {e}"
        ))),
    }
}

/// Clear a customer-facing pole display.
#[tauri::command]
pub async fn display_clear(display_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let display = state
        .registry
        .display(&display_id)
        .await
        .ok_or_else(|| AppError::Invalid(format!("no display registered as '{display_id}'")))?;
    display.clear().await?;
    Ok(())
}

// ── Scoped variants (ADR #7) ────────────────────────────────────

/// Open cash drawer (scoped — requires valid session).
#[tauri::command]
pub async fn open_cash_drawer_scoped(
    args: OpenCashDrawerArgs,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<OpenCashDrawerResult, AppError> {
    state.resolve_scope(&session_token)?;
    let id = args.device_id.as_deref().unwrap_or("default");
    let drawer = state
        .registry
        .cash_drawer(id)
        .await
        .ok_or_else(|| AppError::Invalid(format!("no cash drawer registered as '{id}'")))?;
    drawer.open().await?;
    Ok(OpenCashDrawerResult { opened: true })
}

/// Print receipt (scoped — requires valid session).
#[tauri::command]
pub async fn print_receipt_scoped(
    args: PrintReceiptArgs,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<PrintReceiptResult, AppError> {
    state.resolve_scope(&session_token)?;
    let printer = state
        .registry
        .printer("default")
        .await
        .ok_or_else(|| AppError::Invalid("no receipt printer registered".into()))?;
    let status = printer.get_status().await?;
    if status.has_fault() {
        return Err(AppError::Invalid(
            "Printer is not ready: check paper supply and cover".into(),
        ));
    }
    if status.paper != oz_hal::PaperStatus::Ok {
        tracing::warn!(paper = ?status.paper, "printer paper is low, continuing");
    }
    let lines: Vec<&str> = args.body.lines().collect();
    let n = lines.len();
    printer.print_receipt(&args.body).await?;
    if let Some(ref app) = state.app {
        let _ = app.emit("receipt:printed", serde_json::json!({ "lines": n }));
    }
    Ok(PrintReceiptResult { printed_lines: n })
}

/// List all registered barcode scanners (scoped).
#[tauri::command]
pub async fn list_scanners_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<ScannerInfo>, AppError> {
    state.resolve_scope(&session_token)?;
    let ids = state.registry.scanner_ids().await;
    Ok(ids.into_iter().map(|id| ScannerInfo { id }).collect())
}

/// Start a barcode scanner (scoped).
#[tauri::command]
pub async fn start_scanner_scoped(
    scanner_id: String,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.resolve_scope(&session_token)?;
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
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!(scanner = %scanner_id, error = %e, "scanner poll error");
                            let _ = app.emit("barcode:error", serde_json::json!({ "error": e.to_string() }));
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    }
                }
            }
        }
    });
    state.scanner_cancel.lock().await.replace(tx);
    Ok(())
}

/// Stop the active barcode scanner (scoped).
#[tauri::command]
pub async fn stop_scanner_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.resolve_scope(&session_token)?;
    let mut cancel = state.scanner_cancel.lock().await;
    if let Some(sender) = cancel.take() {
        let _ = sender.send(());
    }
    Ok(())
}

/// List all registered customer displays (scoped).
#[tauri::command]
pub async fn list_displays_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    state.resolve_scope(&session_token)?;
    Ok(state.registry.display_ids().await)
}

/// Show content on a customer-facing pole display (scoped).
#[tauri::command]
pub async fn display_show_scoped(
    args: DisplayShowArgs,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.resolve_scope(&session_token)?;
    let display = state
        .registry
        .display(&args.display_id)
        .await
        .ok_or_else(|| {
            AppError::Invalid(format!("no display registered as '{}'", args.display_id))
        })?;
    let content = DisplayContent {
        line1: args.line1,
        line2: args.line2,
    };
    display.connect().await?;
    display.show(&content).await?;
    Ok(())
}

/// Discover all connected USB hardware devices (scoped).
#[tauri::command]
pub async fn discover_hardware_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<UsbDeviceInfo>, AppError> {
    state.resolve_scope(&session_token)?;
    match probe_all() {
        Ok(devices) => Ok(devices),
        Err(e) => Err(AppError::Internal(format!(
            "hardware discovery failed: {e}"
        ))),
    }
}

/// Clear a customer-facing pole display (scoped).
#[tauri::command]
pub async fn display_clear_scoped(
    display_id: String,
    session_token: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.resolve_scope(&session_token)?;
    let display = state
        .registry
        .display(&display_id)
        .await
        .ok_or_else(|| AppError::Invalid(format!("no display registered as '{display_id}'")))?;
    display.clear().await?;
    Ok(())
}

#[cfg(test)]
#[path = "hardware_tests.rs"]
mod tests;
