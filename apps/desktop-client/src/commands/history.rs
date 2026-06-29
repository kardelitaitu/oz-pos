//! Sales history and report commands: list, get, export summaries.
//!
//! These commands provide read-only access to completed sales and
//! aggregate report data for the dashboard, history screens, and
//! end-of-day reporting.

use serde::Serialize;
use tauri::{State, command};

use oz_core::Money;
use oz_core::db::{Store, DailySummaryRow, SalesByHourRow};

use crate::error::AppError;
use crate::state::AppState;

// ── Sale list / detail ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SaleListItem {
    pub id: String,
    pub total: Money,
    pub line_count: i64,
    pub status: String,
    pub payment_method: Option<String>,
    pub user_id: Option<String>,
    pub created_at: String,
}

#[command]
pub async fn list_sales(
    state: State<'_, AppState>,
) -> Result<Vec<SaleListItem>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sales = store.list_sales()?;
    drop(db);
    Ok(sales
        .into_iter()
        .map(|s| SaleListItem {
            id: s.id,
            total: s.total,
            line_count: s.line_count,
            status: format!("{:?}", s.status),
            payment_method: s.payment_method,
            user_id: s.user_id,
            created_at: s.created_at,
        })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct SaleDetail {
    pub id: String,
    pub total: Money,
    pub subtotal: Money,
    pub tax_total: Money,
    pub line_count: i64,
    pub status: String,
    pub payment_method: Option<String>,
    pub tendered_minor: Option<i64>,
    pub user_id: Option<String>,
    pub created_at: String,
    pub lines: Vec<oz_core::SaleLine>,
}

#[command]
pub async fn get_sale(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<SaleDetail>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let sale = store.get_sale(&id)?;
    drop(db);
    Ok(sale.map(|s| SaleDetail {
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
    }))
}

// ── Dashboard / Export ───────────────────────────────────────────────

#[command]
pub async fn export_daily_summary(
    state: State<'_, AppState>,
) -> Result<Vec<DailySummaryRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.export_daily_summary()?;
    drop(db);
    Ok(rows)
}

#[command]
pub async fn export_sales_by_hour(
    state: State<'_, AppState>,
) -> Result<Vec<SalesByHourRow>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.export_sales_by_hour()?;
    drop(db);
    Ok(rows)
}

// ── EOD (End-of-Day) Report ──────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EodReport {
    pub total_sales: i64,
    pub total_revenue: i64,
    pub currency: String,
    pub payment_breakdown: Vec<PaymentBreakdown>,
    pub void_count: i64,
    pub void_total: i64,
    pub discount_count: i64,
    pub discount_total: i64,
    pub hourly_breakdown: Vec<SalesByHourRow>,
}

#[derive(Debug, Serialize)]
pub struct PaymentBreakdown {
    pub method: String,
    pub count: i64,
    pub total: i64,
}

/// Fetch the full EOD (End-of-Day) report for today.
#[command]
pub async fn export_eod_report(
    state: State<'_, AppState>,
) -> Result<EodReport, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    let daily = store.export_daily_summary()?;
    let hourly = store.export_sales_by_hour()?;

    // Payment breakdown.
    let mut stmt = db.prepare(
        "SELECT payment_method, COUNT(*) AS cnt, SUM(total_minor) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'completed'
         GROUP BY payment_method
         ORDER BY tot DESC"
    )?;
    let payment_rows: Vec<PaymentBreakdown> = stmt.query_map([], |row| {
        Ok(PaymentBreakdown {
            method: row.get::<_, Option<String>>("payment_method")?.unwrap_or_else(|| "Unknown".into()),
            count: row.get("cnt")?,
            total: row.get("tot")?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    // Void stats.
    let mut void_stmt = db.prepare(
        "SELECT COUNT(*) AS cnt, COALESCE(SUM(total_minor), 0) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'voided'"
    )?;
    let void_row: (i64, i64) = void_stmt.query_row([], |row| {
        Ok((row.get::<_, i64>("cnt")?, row.get::<_, i64>("tot")?))
    })?;
    drop(void_stmt);

    // Discount stats.
    let mut discount_stmt = db.prepare(
        "SELECT COUNT(*) AS cnt, COALESCE(SUM(total_minor), 0) AS tot
         FROM sales
         WHERE date(created_at) = date('now') AND status = 'completed' AND discount_percent > 0"
    )?;
    let discount_row: (i64, i64) = discount_stmt.query_row([], |row| {
        Ok((row.get::<_, i64>("cnt")?, row.get::<_, i64>("tot")?))
    })?;
    drop(discount_stmt);

    let total_sales = daily.len() as i64;
    let total_revenue: i64 = daily.iter().map(|r| r.total_minor).sum();
    let currency = daily.first().map(|r| r.currency.clone()).unwrap_or_else(|| "USD".into());

    drop(db);

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
