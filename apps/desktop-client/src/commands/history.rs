//! Sales history and report commands: list, get, export summaries.
//!
//! These commands provide read-only access to completed sales and
//! aggregate report data for the dashboard, history screens, and
//! end-of-day reporting.

use serde::Serialize;
use tauri::State;

use oz_core::Money;
use oz_core::db::{DailySummaryRow, SalesByHourRow, Store};
use oz_core::subscription::TenantSubscription;

use crate::error::AppError;
use crate::state::AppState;

// ── Sale list / detail ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
/// Salelistitem.
pub struct SaleListItem {
    /// Unique identifier.
    pub id: String,
    /// Total amount in minor currency units.
    pub total: Money,
    /// Line Count.
    pub line_count: i64,
    /// Current status.
    pub status: String,
    /// Payment Method.
    pub payment_method: Option<String>,
    /// ID of the associated user.
    pub user_id: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Response for the sale-list commands (C1.2).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// The sales plus whether the tier's history window was applied.
pub struct SaleListResponse {
    /// The sales — already capped to the tier's history window.
    pub sales: Vec<SaleListItem>,
    /// C1.2: true when the tier's history window (Free = 30 days) was
    /// applied, so the UI can show the upgrade teaser.
    pub sales_history_capped: bool,
}

/// C1.2: load the tenant subscription and list sales capped to its history
/// window (`sales_history_days()` — Free = 30 days, paid tiers = unlimited).
/// The subscription lives in the global identity DB that also holds sales for
/// the legacy global variant; the store-scoped variant reads sales from the
/// store DB but the subscription from the same global DB.
fn load_capped_sales(db: &rusqlite::Connection) -> Result<SaleListResponse, AppError> {
    let sub = TenantSubscription::load(db, "default")?
        .ok_or_else(|| AppError::Internal("default tenant subscription not found".into()))?;
    sub.verify_signature()?;
    let days = sub.effective_tier().sales_history_days();
    let store = Store::new(db);
    let (sales, capped) = store.list_sales_with_history_cap(days)?;
    Ok(SaleListResponse {
        sales: sales.into_iter().map(map_sale_to_item).collect(),
        sales_history_capped: capped,
    })
}

/// List all sales from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_sales_scoped`
/// with a `session_token` to list sales from the store-scoped database.
#[tauri::command]
pub async fn list_sales(state: State<'_, AppState>) -> Result<SaleListResponse, AppError> {
    let db = state.db.lock().await;
    let response = load_capped_sales(&db);
    drop(db);
    response
}

/// List all sales for the store resolved from a session token.
///
/// ADR #7: Scoped variant of `list_sales`. The backend resolves the
/// opaque `session_token` to a `SessionContext`, opens the store-scoped
/// database, and returns only that store's completed sales.
#[tauri::command]
pub async fn list_sales_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<SaleListResponse, AppError> {
    // C1.2: the tier's history window lives on the tenant subscription in the
    // global identity DB; the sales themselves come from the store DB.
    let days = {
        let db = state.db.lock().await;
        let sub = TenantSubscription::load(&db, "default")?
            .ok_or_else(|| AppError::Internal("default tenant subscription not found".into()))?;
        sub.verify_signature()?;
        sub.effective_tier().sales_history_days()
    };
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let (sales, capped) = store.list_sales_with_history_cap(days)?;
    drop(db);
    Ok(SaleListResponse {
        sales: sales.into_iter().map(map_sale_to_item).collect(),
        sales_history_capped: capped,
    })
}

/// Shared mapping from `oz_core::Sale` to `SaleListItem`.
fn map_sale_to_item(s: oz_core::Sale) -> SaleListItem {
    SaleListItem {
        id: s.id,
        total: s.total,
        line_count: s.line_count,
        status: format!("{:?}", s.status),
        payment_method: s.payment_method,
        user_id: s.user_id,
        created_at: s.created_at,
    }
}

#[derive(Debug, Serialize)]
/// Saledetail.
pub struct SaleDetail {
    /// Unique identifier.
    pub id: String,
    /// Total amount in minor currency units.
    pub total: Money,
    /// Subtotal.
    pub subtotal: Money,
    /// Tax Total.
    pub tax_total: Money,
    /// Line Count.
    pub line_count: i64,
    /// Current status.
    pub status: String,
    /// Payment Method.
    pub payment_method: Option<String>,
    /// Tendered Minor.
    pub tendered_minor: Option<i64>,
    /// ID of the associated user.
    pub user_id: Option<String>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Lines.
    pub lines: Vec<oz_core::SaleLine>,
}

/// Fetch a single sale by ID from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_sale_scoped`
/// with a `session_token` to look up the sale in the store-scoped database.
#[tauri::command]
pub async fn get_sale(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<SaleDetail>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sale = store.get_sale(&id)?;
    drop(db);
    Ok(sale.map(map_sale_to_detail))
}

/// Fetch a single sale by ID from the store resolved from a session token.
///
/// ADR #7: Scoped variant of `get_sale`. The backend resolves the
/// session token to open the store-scoped database and looks up the
/// sale within that store only.
#[tauri::command]
pub async fn get_sale_scoped(
    session_token: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<SaleDetail>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let sale = store.get_sale(&id)?;
    drop(db);
    Ok(sale.map(map_sale_to_detail))
}

/// Shared mapping from `oz_core::Sale` to `SaleDetail`.
fn map_sale_to_detail(s: oz_core::Sale) -> SaleDetail {
    SaleDetail {
        id: s.id,
        total: s.total,
        subtotal: s.subtotal,
        tax_total: s.tax_total,
        line_count: s.line_count,
        status: format!("{:?}", s.status),
        payment_method: s.payment_method,
        tendered_minor: s.tendered_minor,
        user_id: s.user_id,
        created_at: s.created_at,
        lines: s.lines,
    }
}

// ── Dashboard / Export ───────────────────────────────────────────────

/// Fetch the daily sales summary from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `export_daily_summary_scoped`
/// with a `session_token` for store-scoped reports.
#[tauri::command]
pub async fn export_daily_summary(
    state: State<'_, AppState>,
) -> Result<Vec<DailySummaryRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.export_daily_summary()?;
    drop(db);
    Ok(rows)
}

/// Fetch the daily sales summary for the store resolved from a session token.
///
/// ADR #7: Scoped variant of `export_daily_summary`.
#[tauri::command]
pub async fn export_daily_summary_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<DailySummaryRow>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.export_daily_summary()?;
    drop(db);
    Ok(rows)
}

/// Fetch sales-by-hour breakdown from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `export_sales_by_hour_scoped`.
#[tauri::command]
pub async fn export_sales_by_hour(
    state: State<'_, AppState>,
) -> Result<Vec<SalesByHourRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.export_sales_by_hour()?;
    drop(db);
    Ok(rows)
}

/// Fetch sales-by-hour breakdown for the store resolved from a session token.
///
/// ADR #7: Scoped variant of `export_sales_by_hour`.
#[tauri::command]
pub async fn export_sales_by_hour_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<SalesByHourRow>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.export_sales_by_hour()?;
    drop(db);
    Ok(rows)
}

// ── EOD (End-of-Day) Report ──────────────────────────────────────

#[derive(Debug, Serialize)]
/// Eodreport.
pub struct EodReport {
    /// Total Sales.
    pub total_sales: i64,
    /// Total Revenue.
    pub total_revenue: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Payment Breakdown.
    pub payment_breakdown: Vec<PaymentBreakdown>,
    /// Void Count.
    pub void_count: i64,
    /// Void Total.
    pub void_total: i64,
    /// Discount Count.
    pub discount_count: i64,
    /// Discount Total.
    pub discount_total: i64,
    /// Hourly Breakdown.
    pub hourly_breakdown: Vec<SalesByHourRow>,
}

#[derive(Debug, Serialize)]
/// Paymentbreakdown.
pub struct PaymentBreakdown {
    /// Method.
    pub method: String,
    /// Count.
    pub count: i64,
    /// Total amount in minor currency units.
    pub total: i64,
}

/// Fetch the full EOD (End-of-Day) report from the global database.
///
/// **Deprecated for multi-store (ADR #7):** Use `export_eod_report_scoped`.
#[tauri::command]
pub async fn export_eod_report(state: State<'_, AppState>) -> Result<EodReport, AppError> {
    let db = state.db.lock().await;
    build_eod_report(&db)
}

/// Fetch the full EOD report for the store resolved from a session token.
///
/// ADR #7: Scoped variant of `export_eod_report`. Opens the store-scoped
/// database and builds the report from that store's data only.
#[tauri::command]
pub async fn export_eod_report_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<EodReport, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    build_eod_report(&db)
}

/// Shared business logic for building an EOD report from a connection.
fn build_eod_report(db: &rusqlite::Connection) -> Result<EodReport, AppError> {
    let store = Store::new(db);

    let daily = store.export_daily_summary()?;
    let hourly = store.export_sales_by_hour()?;

    // Payment breakdown.
    let mut stmt = db.prepare(
        "SELECT payment_method, COUNT(*) AS cnt, SUM(total_minor) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'completed'
         GROUP BY payment_method
         ORDER BY tot DESC",
    )?;
    let payment_rows: Vec<PaymentBreakdown> = stmt
        .query_map([], |row| {
            Ok(PaymentBreakdown {
                method: row
                    .get::<_, Option<String>>("payment_method")?
                    .unwrap_or_else(|| "Unknown".into()),
                count: row.get("cnt")?,
                total: row.get("tot")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    // Void stats.
    let mut void_stmt = db.prepare(
        "SELECT COUNT(*) AS cnt, COALESCE(SUM(total_minor), 0) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'voided'",
    )?;
    let void_row: (i64, i64) = void_stmt.query_row([], |row| {
        Ok((row.get::<_, i64>("cnt")?, row.get::<_, i64>("tot")?))
    })?;
    drop(void_stmt);

    // Discount stats.
    let mut discount_stmt = db.prepare(
        "SELECT COUNT(*) AS cnt, COALESCE(SUM(total_minor), 0) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'completed' AND discount_percent > 0",
    )?;
    let discount_row: (i64, i64) = discount_stmt.query_row([], |row| {
        Ok((row.get::<_, i64>("cnt")?, row.get::<_, i64>("tot")?))
    })?;
    drop(discount_stmt);

    let total_sales = daily.len() as i64;
    let total_revenue: i64 = daily.iter().map(|r| r.total_minor).sum();
    let currency = daily
        .first()
        .map(|r| r.currency.clone())
        .unwrap_or_else(|| "USD".into());

    Ok(EodReport {
        total_sales,
        total_revenue,
        currency,
        payment_breakdown: payment_rows,
        void_count: void_row.0,
        void_total: void_row.1,
        discount_count: discount_row.0,
        discount_total: discount_row.1,
        hourly_breakdown: hourly,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
