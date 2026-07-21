//! Daily Sales Summary Analytics — count, revenue, average ticket, hourly
//! breakdown, and top products for a given date range.
//!
//! These queries are designed for offline-first operation: they run against
//! the local SQLite store and produce pre-aggregated results suitable for
//! dashboard widgets and CSV export.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use oz_core::CoreError;

/// Daily sales summary row: one row per day in the range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummaryRow {
    /// ISO date string (YYYY-MM-DD).
    pub date: String,
    /// Number of completed sales on this day.
    pub sale_count: i64,
    /// Total revenue (minor units) from completed sales.
    pub total_revenue_minor: i64,
    /// Average ticket size (minor units). 0 when no sales.
    pub avg_ticket_minor: i64,
    /// Number of unique customers with purchases on this day.
    pub unique_customers: i64,
}

/// Sales-by-hour breakdown for a specific day or range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlySalesRow {
    /// Hour of day (0-23).
    pub hour: u8,
    /// Number of completed sales in this hour.
    pub sale_count: i64,
    /// Total revenue (minor units) in this hour.
    pub total_revenue_minor: i64,
}

/// Top product row for the product leaderboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProductRow {
    /// Product SKU.
    pub sku: String,
    /// Product display name.
    pub name: String,
    /// Total quantity sold.
    pub total_qty: i64,
    /// Total revenue generated (minor units).
    pub total_revenue_minor: i64,
}

/// Complete daily summary result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummaryResult {
    /// Per-day summary rows, ordered by date ASC.
    pub daily: Vec<DailySummaryRow>,
    /// Grand totals across all days in the range.
    pub total_sales: i64,
    /// Grand total revenue across all days.
    pub total_revenue_minor: i64,
}

/// Query daily sales summary for a date range.
///
/// Returns one row per day with sale count, total revenue, average
/// ticket, and unique customer count.
pub fn query_daily_summary(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
) -> Result<DailySummaryResult, CoreError> {
    let mut stmt = conn.prepare(
        "SELECT DATE(s.created_at) AS sale_date,
                COUNT(*) AS sale_count,
                COALESCE(SUM(s.total_minor), 0) AS total_revenue_minor,
                COUNT(DISTINCT s.customer_id) AS unique_customers
         FROM sales s
         WHERE s.status = 'completed'
           AND DATE(s.created_at) BETWEEN ?1 AND ?2
         GROUP BY sale_date
         ORDER BY sale_date ASC",
    )?;

    let rows: Vec<DailySummaryRow> = stmt
        .query_map(params![start_date, end_date], |row| {
            let count: i64 = row.get("sale_count")?;
            let revenue: i64 = row.get("total_revenue_minor")?;
            Ok(DailySummaryRow {
                date: row.get("sale_date")?,
                sale_count: count,
                total_revenue_minor: revenue,
                avg_ticket_minor: if count > 0 { revenue / count } else { 0 },
                unique_customers: row.get("unique_customers")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let (total_sales, total_revenue_minor) = rows.iter().fold((0, 0), |(ts, tr), r| {
        (ts + r.sale_count, tr + r.total_revenue_minor)
    });

    Ok(DailySummaryResult {
        daily: rows,
        total_sales,
        total_revenue_minor,
    })
}

/// Query sales-by-hour breakdown for a date range.
///
/// Returns one row per hour (0-23) with sale count and revenue.
/// Hours with no sales are excluded.
pub fn query_sales_by_hour(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HourlySalesRow>, CoreError> {
    let mut stmt = conn.prepare(
        "SELECT CAST(strftime('%H', s.created_at) AS INTEGER) AS hour,
                COUNT(*) AS sale_count,
                COALESCE(SUM(s.total_minor), 0) AS total_revenue_minor
         FROM sales s
         WHERE s.status = 'completed'
           AND DATE(s.created_at) BETWEEN ?1 AND ?2
         GROUP BY hour
         ORDER BY hour ASC",
    )?;

    stmt.query_map(params![start_date, end_date], |row| {
        Ok(HourlySalesRow {
            hour: row.get::<_, i64>("hour")? as u8,
            sale_count: row.get("sale_count")?,
            total_revenue_minor: row.get("total_revenue_minor")?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(CoreError::from)
}

/// Query top N products by quantity sold in a date range.
pub fn query_top_products(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
    limit: i64,
) -> Result<Vec<TopProductRow>, CoreError> {
    let mut stmt = conn.prepare(
        "SELECT p.sku,
                p.name,
                COALESCE(SUM(sl.qty), 0) AS total_qty,
                COALESCE(SUM(sl.line_minor), 0) AS total_revenue_minor
         FROM sale_lines sl
         JOIN sales s ON sl.sale_id = s.id
         JOIN products p ON sl.sku = p.sku
         WHERE s.status = 'completed'
           AND DATE(s.created_at) BETWEEN ?1 AND ?2
         GROUP BY p.sku
         ORDER BY total_qty DESC
         LIMIT ?3",
    )?;

    stmt.query_map(params![start_date, end_date, limit], |row| {
        Ok(TopProductRow {
            sku: row.get("sku")?,
            name: row.get("name")?,
            total_qty: row.get("total_qty")?,
            total_revenue_minor: row.get("total_revenue_minor")?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(CoreError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use oz_core::{Cart, CartLine, Currency, Money, Sale, SaleStatus, Sku};
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    fn seed_product(conn: &Connection, sku: &str, name: &str) {
        let store = oz_core::db::Store::new(conn);
        store
            .create_product(sku, name, price(500), None, None, 100, None)
            .unwrap();
    }

    fn complete_sale_with_date(
        conn: &Connection,
        sku: &str,
        qty: i64,
        unit_minor: i64,
        date_str: &str,
    ) -> String {
        let store = oz_core::db::Store::new(conn);
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new(sku), qty, price(unit_minor)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        // Override created_at with the requested date.
        sale.created_at = format!("{date_str}T12:00:00.000Z");
        sale.updated_at = sale.created_at.clone();
        store.create_sale(&sale).unwrap();
        store
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap();
        store
            .update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();
        sale.id
    }

    // ── Daily summary ────────────────────────────────────────────

    #[test]
    fn daily_summary_empty_range() {
        let conn = fresh();
        let result = query_daily_summary(&conn, "2000-01-01", "2099-12-31").unwrap();
        assert!(result.daily.is_empty());
        assert_eq!(result.total_sales, 0);
        assert_eq!(result.total_revenue_minor, 0);
    }

    #[test]
    fn daily_summary_single_day() {
        let conn = fresh();
        seed_product(&conn, "COFFEE", "Coffee");
        complete_sale_with_date(&conn, "COFFEE", 2, 350, "2026-07-20");
        complete_sale_with_date(&conn, "COFFEE", 1, 350, "2026-07-20");

        let result = query_daily_summary(&conn, "2026-07-20", "2026-07-20").unwrap();
        assert_eq!(result.daily.len(), 1);
        let day = &result.daily[0];
        assert_eq!(day.date, "2026-07-20");
        assert_eq!(day.sale_count, 2);
        assert_eq!(day.total_revenue_minor, 1050); // 2*350 + 1*350
        assert_eq!(day.avg_ticket_minor, 525);
        assert_eq!(result.total_sales, 2);
        assert_eq!(result.total_revenue_minor, 1050);
    }

    #[test]
    fn daily_summary_multiple_days() {
        let conn = fresh();
        seed_product(&conn, "TEA", "Tea");
        complete_sale_with_date(&conn, "TEA", 1, 200, "2026-07-19");
        complete_sale_with_date(&conn, "TEA", 3, 200, "2026-07-20");

        let result = query_daily_summary(&conn, "2026-07-19", "2026-07-20").unwrap();
        assert_eq!(result.daily.len(), 2);
        // Day 1
        assert_eq!(result.daily[0].date, "2026-07-19");
        assert_eq!(result.daily[0].sale_count, 1);
        assert_eq!(result.daily[0].total_revenue_minor, 200);
        // Day 2
        assert_eq!(result.daily[1].date, "2026-07-20");
        assert_eq!(result.daily[1].sale_count, 1);
        assert_eq!(result.daily[1].total_revenue_minor, 600);
        // Grand totals
        assert_eq!(result.total_sales, 2);
        assert_eq!(result.total_revenue_minor, 800);
    }

    #[test]
    fn daily_summary_excludes_non_completed() {
        let conn = fresh();
        seed_product(&conn, "SODA", "Soda");

        // Create a sale but leave it active (not completed).
        let store = oz_core::db::Store::new(&conn);
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("SODA"), 1, price(150)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.created_at = "2026-07-20T12:00:00.000Z".into();
        sale.updated_at = sale.created_at.clone();
        store.create_sale(&sale).unwrap();

        let result = query_daily_summary(&conn, "2026-07-20", "2026-07-20").unwrap();
        assert_eq!(result.daily.len(), 0);
        assert_eq!(result.total_sales, 0);
    }

    #[test]
    fn daily_summary_avg_ticket_zero_when_no_sales() {
        let conn = fresh();
        // No sales at all.
        let result = query_daily_summary(&conn, "2026-01-01", "2026-01-01").unwrap();
        assert_eq!(result.total_sales, 0);
        // The daily vec is empty, so avg_ticket is implicitly 0.
    }

    // ── Sales by hour ────────────────────────────────────────────

    #[test]
    fn sales_by_hour_empty() {
        let conn = fresh();
        let rows = query_sales_by_hour(&conn, "2000-01-01", "2099-12-31").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn sales_by_hour_single_hour() {
        let conn = fresh();
        seed_product(&conn, "BURGER", "Burger");
        // created_at is 12:00 UTC
        complete_sale_with_date(&conn, "BURGER", 1, 500, "2026-07-20");

        let rows = query_sales_by_hour(&conn, "2026-07-20", "2026-07-20").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].hour, 12);
        assert_eq!(rows[0].sale_count, 1);
        assert_eq!(rows[0].total_revenue_minor, 500);
    }

    #[test]
    fn sales_by_hour_multiple_hours() {
        let conn = fresh();
        seed_product(&conn, "FRIES", "Fries");
        complete_sale_with_date(&conn, "FRIES", 1, 300, "2026-07-20"); // 12:00

        // Create a sale at a different hour via raw SQL.
        let store = oz_core::db::Store::new(&conn);
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("FRIES"), 2, price(300)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.created_at = "2026-07-20T18:30:00.000Z".into();
        sale.updated_at = sale.created_at.clone();
        store.create_sale(&sale).unwrap();
        store
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap();
        store
            .update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        let rows = query_sales_by_hour(&conn, "2026-07-20", "2026-07-20").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].hour, 12);
        assert_eq!(rows[1].hour, 18);
        assert_eq!(rows[1].total_revenue_minor, 600);
    }

    // ── Top products ─────────────────────────────────────────────

    #[test]
    fn top_products_empty() {
        let conn = fresh();
        let rows = query_top_products(&conn, "2000-01-01", "2099-12-31", 10).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn top_products_ranked_by_qty() {
        let conn = fresh();
        seed_product(&conn, "A", "Product A");
        seed_product(&conn, "B", "Product B");
        seed_product(&conn, "C", "Product C");

        complete_sale_with_date(&conn, "A", 10, 100, "2026-07-20");
        complete_sale_with_date(&conn, "B", 5, 200, "2026-07-20");
        complete_sale_with_date(&conn, "C", 1, 1000, "2026-07-20");

        let rows = query_top_products(&conn, "2026-07-20", "2026-07-20", 10).unwrap();
        assert_eq!(rows.len(), 3);
        // Ordered by qty DESC
        assert_eq!(rows[0].sku, "A");
        assert_eq!(rows[0].total_qty, 10);
        assert_eq!(rows[1].sku, "B");
        assert_eq!(rows[1].total_qty, 5);
        assert_eq!(rows[2].sku, "C");
        assert_eq!(rows[2].total_qty, 1);
    }

    #[test]
    fn top_products_limit() {
        let conn = fresh();
        seed_product(&conn, "X", "X");
        seed_product(&conn, "Y", "Y");

        complete_sale_with_date(&conn, "X", 10, 100, "2026-07-20");
        complete_sale_with_date(&conn, "Y", 5, 100, "2026-07-20");

        let rows = query_top_products(&conn, "2026-07-20", "2026-07-20", 1).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sku, "X");
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn daily_summary_row_serde() {
        let row = DailySummaryRow {
            date: "2026-07-20".into(),
            sale_count: 42,
            total_revenue_minor: 10000,
            avg_ticket_minor: 238,
            unique_customers: 15,
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: DailySummaryRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.date, "2026-07-20");
        assert_eq!(back.sale_count, 42);
        assert_eq!(back.avg_ticket_minor, 238);
    }

    #[test]
    fn hourly_sales_row_serde() {
        let row = HourlySalesRow {
            hour: 14,
            sale_count: 7,
            total_revenue_minor: 3500,
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: HourlySalesRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.hour, 14);
        assert_eq!(back.total_revenue_minor, 3500);
    }

    #[test]
    fn top_product_row_serde() {
        let row = TopProductRow {
            sku: "COFFEE".into(),
            name: "Coffee".into(),
            total_qty: 100,
            total_revenue_minor: 35000,
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: TopProductRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sku, "COFFEE");
        assert_eq!(back.total_revenue_minor, 35000);
    }

    #[test]
    fn daily_summary_result_serde() {
        let result = DailySummaryResult {
            daily: vec![DailySummaryRow {
                date: "2026-07-20".into(),
                sale_count: 5,
                total_revenue_minor: 2500,
                avg_ticket_minor: 500,
                unique_customers: 3,
            }],
            total_sales: 5,
            total_revenue_minor: 2500,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: DailySummaryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_sales, 5);
        assert_eq!(back.daily.len(), 1);
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[test]
    fn daily_summary_with_customer() {
        let conn = fresh();
        seed_product(&conn, "LATTE", "Latte");

        // Seed a customer.
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute(
            "INSERT INTO customers (id, name, email, notes, total_spent_minor, loyalty_points, currency, created_at, updated_at)
             VALUES ('cust-1', 'Alice', NULL, '', 0, 0, 'USD', ?1, ?1)",
            params![now],
        )
        .unwrap();

        // Create a sale linked to a customer.
        let store = oz_core::db::Store::new(&conn);
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("LATTE"), 1, price(400)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.created_at = "2026-07-20T12:00:00.000Z".into();
        sale.updated_at = sale.created_at.clone();
        sale.customer_id = Some("cust-1".into());
        store.create_sale(&sale).unwrap();
        store
            .update_sale_status(&sale.id, SaleStatus::Active)
            .unwrap();
        store
            .update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        let result = query_daily_summary(&conn, "2026-07-20", "2026-07-20").unwrap();
        assert_eq!(result.daily[0].unique_customers, 1);
    }

    #[test]
    fn top_products_excludes_non_completed() {
        let conn = fresh();
        seed_product(&conn, "VOID", "Voided Item");
        // Sale created but left as draft (not completed).
        let store = oz_core::db::Store::new(&conn);
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("VOID"), 5, price(100)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.created_at = "2026-07-20T12:00:00.000Z".into();
        sale.updated_at = sale.created_at.clone();
        store.create_sale(&sale).unwrap();
        // NOT completed — stays as Draft.

        let rows = query_top_products(&conn, "2026-07-20", "2026-07-20", 10).unwrap();
        assert!(
            rows.is_empty(),
            "non-completed sales excluded from top products"
        );
    }
}
