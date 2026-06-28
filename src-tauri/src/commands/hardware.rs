//! Hardware-facing Tauri commands: cash drawer, receipt printer, and the
//! barcode subscription event. All commands reach into the HAL via
//! `state.registry` — they never construct a concrete driver.

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, command};

use oz_core::{Currency, Money};
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
    drawer
        .open()
        .await
        .map_err(|e| AppError::Hardware(e.to_string()))?;
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

// ── Structured sales receipt ────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PrintSalesReceiptArgs {
    pub store_name: String,
    pub store_address: String,
    pub store_tax_id: Option<String>,
    pub date: String,
    pub receipt_number: String,
    pub items: Vec<LineItemDto>,
    pub subtotal: MoneyDto,
    pub tax: Option<MoneyDto>,
    pub total: MoneyDto,
    pub payments: Vec<PaymentDto>,
    pub footer: Option<String>,
    #[serde(default)]
    pub paper_width: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LineItemDto {
    pub name: String,
    pub quantity: u32,
    pub unit_price: MoneyDto,
    pub total_price: MoneyDto,
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

    let receipt = receipt::SalesReceipt {
        store: receipt::StoreInfo {
            name: args.store_name,
            address: args.store_address,
            tax_id: args.store_tax_id,
        },
        date: args.date,
        receipt_number: args.receipt_number,
        items: args
            .items
            .into_iter()
            .map(|i| {
                Ok::<_, AppError>(receipt::LineItem {
                    name: i.name,
                    quantity: i.quantity,
                    unit_price: i.unit_price.to_money()?,
                    total_price: i.total_price.to_money()?,
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
        footer: args.footer,
        paper_width: match args.paper_width.as_deref() {
            Some("narrow") => receipt::PaperWidth::Narrow,
            _ => receipt::PaperWidth::Standard,
        },
    };

    let data = receipt::format_sales_receipt(&receipt);
    let line_count = receipt.items.len() + 6; // rough line count for the event

    printer
        .print_raw(&data)
        .await
        .map_err(|e| AppError::Hardware(e.to_string()))?;

    let _ = state
        .app
        .emit("receipt:printed", serde_json::json!({ "lines": line_count }));

    Ok(PrintSalesReceiptResult { printed: true })
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
            "store_name": "OZ Mart",
            "store_address": "123 Main St",
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
        assert_eq!(args.store_name, "OZ Mart");
        assert_eq!(args.items.len(), 1);
        assert_eq!(args.payments.len(), 1);
        assert!(args.footer.is_none());
    }
}
