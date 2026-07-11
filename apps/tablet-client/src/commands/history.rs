//! Sales history and report commands: list, get, export summaries.
//!
//! These commands provide read-only access to completed sales and
//! aggregate report data for the dashboard, history screens, and
//! end-of-day reporting.

use serde::Serialize;
use tauri::{State, command};

use oz_core::Money;
use oz_core::db::{DailySummaryRow, SalesByHourRow, Store};

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
pub async fn list_sales(state: State<'_, AppState>) -> Result<Vec<SaleListItem>, AppError> {
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
pub async fn export_eod_report(state: State<'_, AppState>) -> Result<EodReport, AppError> {
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::Currency;
    use oz_core::{Money, SaleLine};

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    fn make_sale_line(sale_id: &str, sku: &str, qty: i64, unit: i64) -> SaleLine {
        SaleLine {
            id: uuid::Uuid::now_v7().to_string(),
            sale_id: sale_id.into(),
            sku: sku.into(),
            qty,
            unit_price: price(unit),
            line_total: price(unit * qty),
            line_position: 1,
            tax_amount: Money::zero(usd()),
            tax_rate_id: None,
            serial_number: None,
        }
    }

    // ── SaleListItem ───────────────────────────────────────────────────

    #[test]
    fn sale_list_item_debug() {
        let item = SaleListItem {
            id: "s1".into(),
            total: price(5000),
            line_count: 3,
            status: "completed".into(),
            payment_method: Some("cash".into()),
            user_id: Some("u1".into()),
            created_at: "2025-01-01".into(),
        };
        let d = format!("{item:?}");
        assert!(d.contains("s1"));
        assert!(d.contains("5000"));
        assert!(d.contains("completed"));
        assert!(d.contains("cash"));
    }

    #[test]
    fn sale_list_item_serialize() {
        let item = SaleListItem {
            id: "s2".into(),
            total: price(7500),
            line_count: 1,
            status: "voided".into(),
            payment_method: None,
            user_id: None,
            created_at: "2025-06-01".into(),
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["id"], "s2");
        assert_eq!(json["line_count"], 1);
        assert_eq!(json["status"], "voided");
        assert!(json["payment_method"].is_null());
    }

    // ── SaleDetail ──────────────────────────────────────────────────────

    #[test]
    fn sale_detail_debug() {
        let detail = SaleDetail {
            id: "sd1".into(),
            total: price(10000),
            line_count: 2,
            status: "completed".into(),
            payment_method: Some("card".into()),
            tendered_minor: Some(12000),
            user_id: Some("u2".into()),
            created_at: "2025-03-15".into(),
            lines: vec![make_sale_line("sd1", "SKU-A", 2, 5000)],
        };
        let d = format!("{detail:?}");
        assert!(d.contains("sd1"));
        assert!(d.contains("SKU-A"));
    }

    #[test]
    fn sale_detail_serialize() {
        let detail = SaleDetail {
            id: "sd2".into(),
            total: price(3000),
            line_count: 1,
            status: "completed".into(),
            payment_method: None,
            tendered_minor: None,
            user_id: None,
            created_at: "2025-01-01".into(),
            lines: vec![],
        };
        let json = serde_json::to_value(&detail).unwrap();
        assert_eq!(json["id"], "sd2");
        assert!(json["tendered_minor"].is_null());
        assert_eq!(json["lines"].as_array().unwrap().len(), 0);
    }

    // ── PaymentBreakdown ────────────────────────────────────────────────

    #[test]
    fn payment_breakdown_debug() {
        let pb = PaymentBreakdown {
            method: "cash".into(),
            count: 15,
            total: 50000,
        };
        let d = format!("{pb:?}");
        assert!(d.contains("cash"));
        assert!(d.contains("15"));
        assert!(d.contains("50000"));
    }

    #[test]
    fn payment_breakdown_serialize() {
        let pb = PaymentBreakdown {
            method: "card".into(),
            count: 42,
            total: 120000,
        };
        let json = serde_json::to_value(&pb).unwrap();
        assert_eq!(json["method"], "card");
        assert_eq!(json["count"], 42);
        assert_eq!(json["total"], 120000);
    }

    // ── EodReport ───────────────────────────────────────────────────────

    #[test]
    fn eod_report_debug() {
        let report = EodReport {
            total_sales: 100,
            total_revenue: 500000,
            currency: "IDR".into(),
            payment_breakdown: vec![],
            void_count: 3,
            void_total: 15000,
            discount_count: 10,
            discount_total: 25000,
            hourly_breakdown: vec![],
        };
        let d = format!("{report:?}");
        assert!(d.contains("100"));
        assert!(d.contains("IDR"));
    }

    #[test]
    fn eod_report_serialize() {
        let report = EodReport {
            total_sales: 50,
            total_revenue: 250000,
            currency: "USD".into(),
            payment_breakdown: vec![PaymentBreakdown {
                method: "cash".into(),
                count: 30,
                total: 150000,
            }],
            void_count: 1,
            void_total: 5000,
            discount_count: 5,
            discount_total: 10000,
            hourly_breakdown: vec![],
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["total_sales"], 50);
        assert_eq!(json["currency"], "USD");
        assert_eq!(json["void_count"], 1);
        assert!(!json["payment_breakdown"].as_array().unwrap().is_empty());
    }
}
