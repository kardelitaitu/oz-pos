//! Refund commands — process refund against a completed sale.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::db::Store;
use oz_core::permissions;
use oz_core::{Money, Refund, RefundLine, Sale};

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Refundlinearg.
pub struct RefundLineArg {
    /// ID of the associated sale line.
    pub sale_line_id: String,
    /// Stock-keeping unit identifier.
    pub sku: String,
    /// Quantity.
    pub qty: i64,
    /// Unit Price Minor.
    pub unit_price_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Total amount in minor currency units.
    pub line_total_minor: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Processrefundargs.
pub struct ProcessRefundArgs {
    /// ID of the original completed sale.
    pub sale_id: String,
    /// Reason for the refund.
    pub reason: String,
    /// Optional internal note.
    pub note: Option<String>,
    /// User ID of the staff processing the refund.
    pub user_id: String,
    /// Lines being refunded.
    pub lines: Vec<RefundLineArg>,
}

/// Args for `process_refund_scoped` — identical to `ProcessRefundArgs`
/// but without `user_id` (read from the session token instead).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRefundScopedArgs {
    /// ID of the associated sale.
    pub sale_id: String,
    /// Reason.
    pub reason: String,
    /// Note.
    pub note: Option<String>,
    /// Lines.
    pub lines: Vec<RefundLineArg>,
}

#[derive(Debug, Serialize)]
/// Processrefundresult.
pub struct ProcessRefundResult {
    /// ID of the associated refund.
    pub refund_id: String,
    /// Total amount in minor currency units.
    pub total_minor: i64,
}

/// Process a refund within the store resolved from a session token.
///
/// ADR #7: Scoped variant of `process_refund`. The `user_id` for
/// permission checks and the refund record is read from the resolved
/// `SessionContext`.
#[tauri::command]
pub async fn process_refund_scoped(
    session_token: String,
    args: ProcessRefundScopedArgs,
    state: State<'_, AppState>,
) -> Result<ProcessRefundResult, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::SALES_REFUND).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;

    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    run_process_refund_unchecked(
        &db,
        &args.sale_id,
        &args.reason,
        args.note.as_deref(),
        &session.user_id,
        &args.lines,
    )
}

/// Process an already-authorized refund against a store-scoped database.
/// Scoped commands authorize the session against the global identity DB
/// before opening the store connection, then call this business path.
fn run_process_refund_unchecked(
    db: &rusqlite::Connection,
    sale_id: &str,
    reason: &str,
    note: Option<&str>,
    user_id: &str,
    lines: &[RefundLineArg],
) -> Result<ProcessRefundResult, AppError> {
    let store = Store::new(db);

    // Verify the sale exists and is completed.
    let sale = store
        .get_sale(sale_id)?
        .ok_or_else(|| AppError::Invalid(format!("sale {} not found", sale_id)))?;
    if sale.status != oz_core::SaleStatus::Completed {
        return Err(AppError::Invalid(format!(
            "cannot refund a sale with status {:?}; only completed sales can be refunded",
            sale.status
        )));
    }

    // Build refund domain objects.
    // Use collect::<Result> so invalid currency codes surface as errors
    // instead of silently falling back to the sale currency via .unwrap_or().
    let refund_lines: Vec<RefundLine> = lines
        .iter()
        .map(|l| {
            let currency: oz_core::Currency = l
                .currency
                .parse()
                .map_err(|_| AppError::Invalid(format!("invalid currency code: {}", l.currency)))?;
            let unit_price = Money {
                minor_units: l.unit_price_minor,
                currency,
            };
            let line_total = Money {
                minor_units: l.line_total_minor,
                currency,
            };
            Ok(RefundLine::new(
                &l.sale_line_id,
                &l.sku,
                l.qty,
                unit_price,
                line_total,
            ))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let total = refund_lines.iter().try_fold(
        Money::zero(sale.currency),
        |acc, line| {
            acc.checked_add(line.line_total).ok_or_else(|| {
                AppError::Invalid(format!(
                    "refund total overflow or line/sale currency mismatch (line {} in {}, sale in {})",
                    line.sku, line.line_total.currency, sale.currency
                ))
            })
        },
    )?;
    let total_minor = total.minor_units;

    let refund = Refund::new(
        sale_id,
        total,
        reason,
        note.unwrap_or(""),
        user_id,
        refund_lines,
    );

    store.create_refund(&refund)?;

    tracing::info!(
        refund_id = %refund.id,
        sale_id,
        total_minor,
        reason,
        "refund processed"
    );

    Ok(ProcessRefundResult {
        refund_id: refund.id,
        total_minor,
    })
}

/// Look up a sale by receipt barcode from the store resolved from a session token.
///
/// ADR #7: Scoped variant of `lookup_sale_by_receipt_barcode`.
///
/// Requires `SALES_PROCESS` permission.
#[tauri::command]
pub async fn lookup_sale_by_receipt_barcode_scoped(
    session_token: String,
    barcode: String,
    state: State<'_, AppState>,
) -> Result<Option<Sale>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::SALES_PROCESS).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    let sale = store.lookup_sale_by_receipt_barcode(&barcode)?;
    drop(db);
    Ok(sale)
}

/// List all refunds for a sale from the store resolved from a session token.
///
/// ADR #7: Scoped variant of `list_refunds`.
///
/// Requires `SALES_PROCESS` permission.
#[tauri::command]
pub async fn list_refunds_scoped(
    session_token: String,
    sale_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Refund>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, oz_core::permissions::SALES_PROCESS).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    let refunds = store.list_refunds_for_sale(&sale_id)?;
    drop(db);
    Ok(refunds)
}

#[cfg(test)]
#[path = "refunds_tests.rs"]
mod tests;
