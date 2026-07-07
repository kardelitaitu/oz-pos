//! Reporting queries: revenue summaries, top products, heatmap, low-stock alerts.

use rusqlite::params;

use crate::db::Store;
use crate::error::CoreError;

/// Revenue aggregated by date.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyRevenueRow {
    /// ISO date YYYY-MM-DD
    pub date: String,
    /// Total revenue in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Number of completed sales on this date.
    pub sale_count: i64,
}

/// Weekly revenue aggregation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WeeklyRevenueRow {
    /// ISO date of the week start (Sunday).
    pub week_start: String,
    /// Total revenue in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Number of completed sales in this week.
    pub sale_count: i64,
}

/// Monthly revenue aggregation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonthlyRevenueRow {
    /// YYYY-MM
    pub month: String,
    /// Total revenue in minor units.
    pub total_minor: i64,
    /// ISO-4217 currency code.
    pub currency: String,
    /// Number of completed sales in this month.
    pub sale_count: i64,
}

/// Top product ranking.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TopProductRow {
    /// Product unique identifier.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// Product display name.
    pub name: String,
    /// Total quantity sold.
    pub total_qty: i64,
    /// Total revenue in minor units.
    pub total_minor: i64,
}

/// Hourly sales heatmap entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HourlyHeatmapRow {
    /// Day of week (0=Sunday, 1=Monday, ...).
    pub day_of_week: i64,
    /// Hour of day (0–23).
    pub hour: i64,
    /// Total revenue in minor units.
    pub total_minor: i64,
    /// Number of completed sales in this time slot.
    pub sale_count: i64,
}

/// Low-stock alert.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LowStockAlert {
    /// Product unique identifier.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// Product display name.
    pub name: String,
    /// Current inventory quantity.
    pub current_qty: i64,
    /// Low-stock threshold that triggered the alert.
    pub threshold: i64,
}

/// Category sales breakdown.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CategoryBreakdownRow {
    /// Category id (None for uncategorised products).
    pub category_id: Option<String>,
    /// Category display name.
    pub category_name: String,
    /// Total revenue in minor units.
    pub total_minor: i64,
    /// Number of distinct sales that included this category.
    pub sale_count: i64,
    /// Percentage of grand total revenue (0.0–100.0).
    pub percentage: f64,
}

impl Store<'_> {
    /// Daily revenue for a date range.
    pub fn daily_revenue(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<DailyRevenueRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DATE(created_at) AS date, SUM(total_minor) AS total_minor,
                    currency, COUNT(*) AS sale_count
             FROM sales
             WHERE status = 'completed' AND DATE(created_at) BETWEEN ?1 AND ?2
             GROUP BY DATE(created_at), currency
             ORDER BY date ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(DailyRevenueRow {
                date: row.get("date")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                sale_count: row.get("sale_count")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Weekly revenue (Sunday-based) for a date range.
    pub fn weekly_revenue(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<WeeklyRevenueRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DATE(created_at, 'weekday 0', '-7 days') AS week_start,
                    SUM(total_minor) AS total_minor, currency, COUNT(*) AS sale_count
             FROM sales
             WHERE status = 'completed' AND DATE(created_at) BETWEEN ?1 AND ?2
             GROUP BY week_start, currency
             ORDER BY week_start ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(WeeklyRevenueRow {
                week_start: row.get("week_start")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                sale_count: row.get("sale_count")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Monthly revenue for a date range.
    pub fn monthly_revenue(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<MonthlyRevenueRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT SUBSTR(created_at, 1, 7) AS month,
                    SUM(total_minor) AS total_minor, currency, COUNT(*) AS sale_count
             FROM sales
             WHERE status = 'completed' AND DATE(created_at) BETWEEN ?1 AND ?2
             GROUP BY month, currency
             ORDER BY month ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(MonthlyRevenueRow {
                month: row.get("month")?,
                total_minor: row.get("total_minor")?,
                currency: row.get("currency")?,
                sale_count: row.get("sale_count")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Top products ranked by total revenue for a date range.
    pub fn top_products(
        &self,
        start_date: &str,
        end_date: &str,
        limit: i64,
    ) -> Result<Vec<TopProductRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id AS product_id, p.sku, p.name,
                    SUM(sl.qty) AS total_qty,
                    SUM(sl.line_minor) AS total_minor
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             JOIN products p ON sl.sku = p.sku
             WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY p.id
             ORDER BY total_minor DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![start_date, end_date, limit], |row| {
            Ok(TopProductRow {
                product_id: row.get("product_id")?,
                sku: row.get("sku")?,
                name: row.get("name")?,
                total_qty: row.get("total_qty")?,
                total_minor: row.get("total_minor")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Hourly sales heatmap for a date range.
    pub fn hourly_heatmap(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<HourlyHeatmapRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(strftime('%w', created_at) AS INTEGER) AS day_of_week,
                    CAST(strftime('%H', created_at) AS INTEGER) AS hour,
                    SUM(total_minor) AS total_minor,
                    COUNT(*) AS sale_count
             FROM sales
             WHERE status = 'completed' AND DATE(created_at) BETWEEN ?1 AND ?2
             GROUP BY day_of_week, hour
             ORDER BY day_of_week, hour",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(HourlyHeatmapRow {
                day_of_week: row.get("day_of_week")?,
                hour: row.get("hour")?,
                total_minor: row.get("total_minor")?,
                sale_count: row.get("sale_count")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Products whose current stock is at or below `threshold`.
    pub fn low_stock_alerts(&self, threshold: i64) -> Result<Vec<LowStockAlert>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id AS product_id, p.sku, p.name, COALESCE(i.qty, 0) AS current_qty,
                    ?1 AS threshold
             FROM products p
             LEFT JOIN inventory i ON p.id = i.product_id
             WHERE COALESCE(i.qty, 0) <= ?1
             ORDER BY current_qty ASC",
        )?;
        let rows = stmt.query_map(params![threshold], |row| {
            Ok(LowStockAlert {
                product_id: row.get("product_id")?,
                sku: row.get("sku")?,
                name: row.get("name")?,
                current_qty: row.get("current_qty")?,
                threshold: row.get("threshold")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Revenue breakdown by product category for a date range.
    ///
    /// Each row includes a `percentage` field relative to the grand total
    /// across all categories in the queried period.
    pub fn category_breakdown(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<CategoryBreakdownRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.category_id, COALESCE(c.name, 'Uncategorised') AS category_name,
                    SUM(sl.line_minor) AS total_minor,
                    COUNT(DISTINCT s.id) AS sale_count
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             JOIN products p ON sl.sku = p.sku
             LEFT JOIN categories c ON p.category_id = c.id
             WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY p.category_id
             ORDER BY total_minor DESC",
        )?;
        let mut rows: Vec<CategoryBreakdownRow> = stmt
            .query_map(params![start_date, end_date], |row| {
                Ok(CategoryBreakdownRow {
                    category_id: row.get("category_id")?,
                    category_name: row.get("category_name")?,
                    total_minor: row.get("total_minor")?,
                    sale_count: row.get("sale_count")?,
                    percentage: 0.0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let grand_total: f64 = rows.iter().map(|r| r.total_minor as f64).sum();
        if grand_total > 0.0 {
            for row in &mut rows {
                row.percentage = (row.total_minor as f64 / grand_total) * 100.0;
            }
        }

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Store;
    use crate::money::Currency;
    use crate::{Cart, CartLine, Money, Sale, SaleStatus, Sku, migrations};
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
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

    fn seed_completed_sale(conn: &Connection, sku: &str, qty: i64, unit_minor: i64) -> String {
        let s = store(conn);
        let money = Money {
            minor_units: unit_minor,
            currency: usd(),
        };
        s.create_product(sku, sku, money, None, None, 100).unwrap();

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new(sku), qty, price(unit_minor)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();
        sale.id
    }

    // ── Daily revenue ──────────────────────────────────────────────

    #[test]
    fn daily_revenue_empty() {
        let conn = fresh();
        let rows = store(&conn)
            .daily_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn daily_revenue_with_sales() {
        let conn = fresh();
        seed_completed_sale(&conn, "COFFEE", 2, 350);
        seed_completed_sale(&conn, "BAGEL", 1, 450);
        let rows = store(&conn)
            .daily_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].total_minor, 1150);
        assert_eq!(rows[0].sale_count, 2);
        assert_eq!(rows[0].currency, "USD");
    }

    // ── Weekly revenue ─────────────────────────────────────────────

    #[test]
    fn weekly_revenue_empty() {
        let conn = fresh();
        let rows = store(&conn)
            .weekly_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn weekly_revenue_with_sales() {
        let conn = fresh();
        seed_completed_sale(&conn, "TEA", 3, 200);
        let rows = store(&conn)
            .weekly_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].total_minor, 600);
    }

    // ── Monthly revenue ────────────────────────────────────────────

    #[test]
    fn monthly_revenue_empty() {
        let conn = fresh();
        let rows = store(&conn)
            .monthly_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn monthly_revenue_with_sales() {
        let conn = fresh();
        seed_completed_sale(&conn, "JUICE", 1, 500);
        let rows = store(&conn)
            .monthly_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].total_minor, 500);
    }

    // ── Top products ───────────────────────────────────────────────

    #[test]
    fn top_products_empty() {
        let conn = fresh();
        let rows = store(&conn)
            .top_products("2000-01-01", "2099-12-31", 10)
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn top_products_with_sales() {
        let conn = fresh();
        seed_completed_sale(&conn, "COFFEE", 2, 350);
        seed_completed_sale(&conn, "BAGEL", 1, 450);
        let rows = store(&conn)
            .top_products("2000-01-01", "2099-12-31", 10)
            .unwrap();
        assert!(!rows.is_empty());
        // BAGEL has higher unit price but lower qty → check ordering
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn top_products_respects_limit() {
        let conn = fresh();
        seed_completed_sale(&conn, "A", 1, 100);
        seed_completed_sale(&conn, "B", 1, 200);
        seed_completed_sale(&conn, "C", 1, 300);
        let rows = store(&conn)
            .top_products("2000-01-01", "2099-12-31", 2)
            .unwrap();
        assert_eq!(rows.len(), 2);
        // Highest revenue first
        assert_eq!(rows[0].sku, "C");
        assert_eq!(rows[1].sku, "B");
    }

    // ── Hourly heatmap ─────────────────────────────────────────────

    #[test]
    fn hourly_heatmap_empty() {
        let conn = fresh();
        let rows = store(&conn)
            .hourly_heatmap("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn hourly_heatmap_with_sales() {
        let conn = fresh();
        seed_completed_sale(&conn, "SNACK", 1, 300);
        let rows = store(&conn)
            .hourly_heatmap("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].sale_count, 1);
    }

    // ── Low stock alerts ───────────────────────────────────────────

    #[test]
    fn low_stock_alerts_empty() {
        let conn = fresh();
        let rows = store(&conn).low_stock_alerts(0).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn low_stock_alerts_finds_low_stock() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("LOW", "Low Stock Item", money, None, None, 2)
            .unwrap();
        s.create_product("OK", "OK Stock Item", money, None, None, 100)
            .unwrap();
        let rows = s.low_stock_alerts(5).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sku, "LOW");
        assert_eq!(rows[0].current_qty, 2);
        assert_eq!(rows[0].threshold, 5);
    }

    #[test]
    fn low_stock_alerts_no_inventory_row() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        // Create a product without inventory record — qty defaults to 0.
        s.create_product("NO-INV", "No Inventory", money, None, None, 0)
            .unwrap();
        let rows = s.low_stock_alerts(0).unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].current_qty, 0);
    }

    // ── Category breakdown ─────────────────────────────────────────

    #[test]
    fn category_breakdown_empty() {
        let conn = fresh();
        let rows = store(&conn)
            .category_breakdown("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn category_breakdown_with_sales() {
        let conn = fresh();
        let s = store(&conn);
        s.create_category("cat-1", "Beverages", "#fff", "").unwrap();

        let money = Money {
            minor_units: 350,
            currency: usd(),
        };
        s.create_product("COFFEE", "Coffee", money, Some("cat-1"), None, 100)
            .unwrap();

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        let rows = s.category_breakdown("2000-01-01", "2099-12-31").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].category_name, "Beverages");
        assert_eq!(rows[0].total_minor, 700);
        assert!((rows[0].percentage - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn category_breakdown_no_category() {
        let conn = fresh();
        let s = store(&conn);

        let money = Money {
            minor_units: 200,
            currency: usd(),
        };
        s.create_product("GENERIC", "Generic Item", money, None, None, 100)
            .unwrap();

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("GENERIC"), 1, price(200)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        let rows = s.category_breakdown("2000-01-01", "2099-12-31").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].category_name, "Uncategorised");
        assert_eq!(rows[0].category_id, None);
    }
}
