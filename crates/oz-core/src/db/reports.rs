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

/// A row from the `stock_alert_events` table (ADR-18 §9e).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockAlertEvent {
    /// Unique event ID.
    pub id: String,
    /// FK to `stock_thresholds.id`.
    pub threshold_id: String,
    /// The affected product ID.
    pub product_id: String,
    /// The affected location ID.
    pub location_id: String,
    /// Current stock at time of event.
    pub current_qty: i64,
    /// Threshold that was breached.
    pub threshold: i64,
    /// One of 'active', 'acknowledged', 'resolved'.
    pub status: String,
    /// ISO-8601 timestamp when the alert was triggered.
    pub triggered_at: String,
    /// ISO-8601 timestamp when the alert was acknowledged (nullable).
    pub acknowledged_at: Option<String>,
    /// ISO-8601 timestamp when the alert was resolved (nullable).
    pub resolved_at: Option<String>,
    /// User ID who acknowledged the alert (nullable).
    pub acknowledged_by: Option<String>,
    /// Product SKU (empty string if product deleted).
    pub product_sku: String,
    /// Product display name (empty string if product deleted).
    pub product_name: String,
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
    ///
    /// **Deprecated in favour of [`low_stock_alerts_at_location`]
    /// (Self::low_stock_alerts_at_location)**, which respects the
    /// per-location stock from `stock_summary`.
    #[deprecated(note = "use low_stock_alerts_at_location instead")]
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

    /// Per-location low-stock alerts using `stock_summary`.
    ///
    /// For each product at the given location, if the current qty from
    /// `stock_summary` is ≤ `default_threshold` AND no custom threshold
    /// (product+location or product+global) is configured, the row appears
    /// with the `default_threshold` value. If a custom threshold is
    /// configured, that threshold is used instead.
    pub fn low_stock_alerts_at_location(
        &self,
        location_id: &str,
        default_threshold: i64,
    ) -> Result<Vec<LowStockAlert>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id AS product_id, p.sku, p.name,
                    COALESCE(ss.qty, 0) AS current_qty,
                    COALESCE(
                        (SELECT st.threshold FROM stock_thresholds st
                         WHERE st.product_id = p.id
                           AND st.location_id = ?1 AND st.enabled = 1
                         LIMIT 1),
                        (SELECT st.threshold FROM stock_thresholds st
                         WHERE st.product_id = p.id
                           AND st.location_id IS NULL AND st.enabled = 1
                         LIMIT 1),
                        ?2
                    ) AS threshold
             FROM products p
             LEFT JOIN stock_summary ss
                ON ss.item_id = p.id AND ss.location_id = ?1
             WHERE COALESCE(ss.qty, 0) <= ?2
                OR (SELECT 1 FROM stock_thresholds st
                    WHERE st.product_id = p.id
                      AND (st.location_id = ?1 OR st.location_id IS NULL)
                      AND st.enabled = 1
                      AND COALESCE(ss.qty, 0) <= st.threshold
                    LIMIT 1) = 1
             ORDER BY current_qty ASC",
        )?;
        let rows = stmt.query_map(params![location_id, default_threshold], |row| {
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

    /// Active (non-resolved) stock alert events for a location, enriched
    /// with product SKU and name.
    ///
    /// Returns rows from `stock_alert_events` LEFT JOINed with `products`,
    /// where `status` is 'active' or 'acknowledged', filtered by
    /// `location_id`, ordered by `triggered_at DESC`.
    pub fn active_stock_alerts(
        &self,
        location_id: &str,
    ) -> Result<Vec<StockAlertEvent>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT sae.id, sae.threshold_id, sae.product_id, sae.location_id,
                    sae.current_qty, sae.threshold, sae.status,
                    sae.triggered_at, sae.acknowledged_at, sae.resolved_at,
                    sae.acknowledged_by,
                    COALESCE(p.sku, '') AS product_sku,
                    COALESCE(p.name, '') AS product_name
             FROM stock_alert_events sae
             LEFT JOIN products p ON sae.product_id = p.id
             WHERE sae.location_id = ?1 AND sae.status IN ('active', 'acknowledged')
             ORDER BY sae.triggered_at DESC",
        )?;
        let rows = stmt.query_map(params![location_id], |row| {
            Ok(StockAlertEvent {
                id: row.get("id")?,
                threshold_id: row.get("threshold_id")?,
                product_id: row.get("product_id")?,
                location_id: row.get("location_id")?,
                current_qty: row.get("current_qty")?,
                threshold: row.get("threshold")?,
                status: row.get("status")?,
                triggered_at: row.get("triggered_at")?,
                acknowledged_at: row.get("acknowledged_at")?,
                resolved_at: row.get("resolved_at")?,
                acknowledged_by: row.get("acknowledged_by")?,
                product_sku: row.get("product_sku")?,
                product_name: row.get("product_name")?,
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

    fn insert_user(conn: &Connection, id: &str) {
        // Ensure a role exists for the FK reference.
        conn.execute(
            "INSERT OR IGNORE INTO roles (id, name, description, permissions, created_at, updated_at)
             VALUES ('role-cashier', 'cashier', '', '[]', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES (?1, ?1, 'x', ?1, 'role-cashier', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
            rusqlite::params![id],
        )
        .unwrap();
    }

    fn seed_completed_sale(conn: &Connection, sku: &str, qty: i64, unit_minor: i64) -> String {
        let s = store(conn);
        let money = Money {
            minor_units: unit_minor,
            currency: usd(),
        };
        s.create_product(sku, sku, money, None, None, 100, None)
            .unwrap();

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

    #[allow(deprecated)]
    #[test]
    fn low_stock_alerts_empty() {
        let conn = fresh();
        let rows = store(&conn).low_stock_alerts(0).unwrap();
        assert!(rows.is_empty());
    }

    #[allow(deprecated)]
    #[test]
    fn low_stock_alerts_finds_low_stock() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("LOW", "Low Stock Item", money, None, None, 2, None)
            .unwrap();
        s.create_product("OK", "OK Stock Item", money, None, None, 100, None)
            .unwrap();
        let rows = s.low_stock_alerts(5).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sku, "LOW");
        assert_eq!(rows[0].current_qty, 2);
        assert_eq!(rows[0].threshold, 5);
    }

    #[allow(deprecated)]
    #[test]
    fn low_stock_alerts_no_inventory_row() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        // Create a product without inventory record — qty defaults to 0.
        s.create_product("NO-INV", "No Inventory", money, None, None, 0, None)
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
        s.create_product("COFFEE", "Coffee", money, Some("cat-1"), None, 100, None)
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
        s.create_product("GENERIC", "Generic Item", money, None, None, 100, None)
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

    // ── Low stock alerts at location ───────────────────────────────

    #[test]
    fn low_stock_alerts_at_location_empty() {
        let conn = fresh();
        let s = store(&conn);
        let rows = s
            .low_stock_alerts_at_location(crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID, 0)
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn low_stock_alerts_at_location_finds_low_stock() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("LOW", "Low Stock Item", money, None, None, 2, None)
            .unwrap();
        s.create_product("OK", "OK Stock Item", money, None, None, 100, None)
            .unwrap();
        let rows = s
            .low_stock_alerts_at_location(crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID, 5)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sku, "LOW");
        assert_eq!(rows[0].current_qty, 2);
        assert_eq!(rows[0].threshold, 5);
    }

    #[test]
    fn low_stock_alerts_at_location_respects_custom_threshold() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        let prod = s
            .create_product("CUSTOM", "Custom Threshold", money, None, None, 5, None)
            .unwrap();
        // Set a custom threshold of 10.
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute(
            "INSERT INTO stock_thresholds (id, product_id, location_id, threshold, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, 10, 1, ?4, ?4)",
            rusqlite::params![
                uuid::Uuid::now_v7().to_string(),
                prod.id,
                crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
                now
            ],
        )
        .unwrap();

        // Default threshold is 3, but custom threshold (10) overrides it.
        let rows = s
            .low_stock_alerts_at_location(crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID, 3)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sku, "CUSTOM");
        assert_eq!(rows[0].current_qty, 5);
        assert_eq!(
            rows[0].threshold, 10,
            "custom threshold should override default"
        );
    }

    // ── Active stock alerts ────────────────────────────────────────

    #[test]
    fn active_stock_alerts_empty() {
        let conn = fresh();
        let s = store(&conn);
        let rows = s
            .active_stock_alerts(crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID)
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn active_stock_alerts_returns_active_only() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        let prod = s
            .create_product("ALERT", "Alert Product", money, None, None, 2, None)
            .unwrap();
        let loc_id = crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Create a threshold.
        let tid = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO stock_thresholds (id, product_id, location_id, threshold, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, 5, 1, ?4, ?4)",
            rusqlite::params![tid, prod.id, loc_id, now],
        )
        .unwrap();

        // Create an active alert.
        conn.execute(
            "INSERT INTO stock_alert_events (id, threshold_id, product_id, location_id, current_qty, threshold, status, triggered_at)
             VALUES (?1, ?2, ?3, ?4, 2, 5, 'active', ?5)",
            rusqlite::params![uuid::Uuid::now_v7().to_string(), tid, prod.id, loc_id, now],
        )
        .unwrap();

        let rows = s.active_stock_alerts(loc_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, "active");
        assert_eq!(rows[0].current_qty, 2);
        assert_eq!(rows[0].threshold, 5);
        assert_eq!(rows[0].product_sku, "ALERT");
        assert_eq!(rows[0].product_name, "Alert Product");
    }

    #[test]
    fn active_stock_alerts_excludes_resolved() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        let prod = s
            .create_product("RESOLVED", "Resolved Product", money, None, None, 2, None)
            .unwrap();
        let loc_id = crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let tid = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO stock_thresholds (id, product_id, location_id, threshold, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, 5, 1, ?4, ?4)",
            rusqlite::params![tid, prod.id, loc_id, now],
        )
        .unwrap();

        // Create a resolved alert.
        conn.execute(
            "INSERT INTO stock_alert_events (id, threshold_id, product_id, location_id, current_qty, threshold, status, triggered_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, 2, 5, 'resolved', ?5, ?6)",
            rusqlite::params![
                uuid::Uuid::now_v7().to_string(),
                tid,
                prod.id,
                loc_id,
                now,
                now
            ],
        )
        .unwrap();

        let rows = s.active_stock_alerts(loc_id).unwrap();
        assert!(rows.is_empty(), "resolved alerts should be excluded");
    }

    #[test]
    fn active_stock_alerts_includes_acknowledged() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        let prod = s
            .create_product("ACK", "Acknowledged Product", money, None, None, 2, None)
            .unwrap();
        let loc_id = crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Create a user for the acknowledged_by FK reference.
        insert_user(&conn, "user-1");

        let tid = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO stock_thresholds (id, product_id, location_id, threshold, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, 5, 1, ?4, ?4)",
            rusqlite::params![tid, prod.id, loc_id, now],
        )
        .unwrap();

        // Create an acknowledged alert.
        conn.execute(
            "INSERT INTO stock_alert_events (id, threshold_id, product_id, location_id, current_qty, threshold, status, triggered_at, acknowledged_at, acknowledged_by)
             VALUES (?1, ?2, ?3, ?4, 2, 5, 'acknowledged', ?5, ?5, 'user-1')",
            rusqlite::params![uuid::Uuid::now_v7().to_string(), tid, prod.id, loc_id, now],
        )
        .unwrap();

        let rows = s.active_stock_alerts(loc_id).unwrap();
        assert_eq!(rows.len(), 1, "acknowledged alerts should be included");
        assert_eq!(rows[0].status, "acknowledged");
        assert_eq!(
            rows[0].acknowledged_by.as_deref(),
            Some("user-1"),
            "acknowledged_by should be populated"
        );
    }

    // ── Extended edge cases ───────────────────────────────────────

    #[test]
    fn daily_revenue_ignores_sales_outside_range() {
        let conn = fresh();
        // Create a sale with the seeded timestamp (today).
        seed_completed_sale(&conn, "TODAY", 1, 500);
        // Query a date range that doesn't include today.
        let rows = store(&conn)
            .daily_revenue("2000-01-01", "2000-01-31")
            .unwrap();
        assert!(
            rows.is_empty(),
            "sales outside the date range should be excluded"
        );
    }

    #[test]
    fn weekly_revenue_multiple_weeks() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("W1", "Week 1", money, None, None, 100, None)
            .unwrap();
        s.create_product("W2", "Week 2", money, None, None, 100, None)
            .unwrap();

        // Create a sale with an old date (simulate week 1).
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("W1"), 1, price(100)))
            .unwrap();
        let mut sale1 = Sale::from_cart(&cart).unwrap();
        sale1.created_at = "2026-01-05T12:00:00.000Z".to_string();
        sale1.updated_at = "2026-01-05T12:00:00.000Z".to_string();
        s.create_sale(&sale1).unwrap();
        s.update_sale_status(&sale1.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale1.id, SaleStatus::Completed)
            .unwrap();

        // Create a sale with a later date (simulate week 2).
        let mut cart2 = Cart::new(usd());
        cart2
            .add_line(CartLine::new(Sku::new("W2"), 1, price(100)))
            .unwrap();
        let mut sale2 = Sale::from_cart(&cart2).unwrap();
        sale2.created_at = "2026-01-12T12:00:00.000Z".to_string();
        sale2.updated_at = "2026-01-12T12:00:00.000Z".to_string();
        s.create_sale(&sale2).unwrap();
        s.update_sale_status(&sale2.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale2.id, SaleStatus::Completed)
            .unwrap();

        let rows = s.weekly_revenue("2026-01-01", "2026-01-31").unwrap();
        assert_eq!(rows.len(), 2, "should have two weekly rows");
        // Both rows should have 100 each.
        assert_eq!(rows[0].total_minor, 100);
        assert_eq!(rows[1].total_minor, 100);
    }

    #[test]
    fn monthly_revenue_multiple_months() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("M1", "Month 1", money, None, None, 100, None)
            .unwrap();
        s.create_product("M2", "Month 2", money, None, None, 100, None)
            .unwrap();

        // January sale.
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("M1"), 1, price(100)))
            .unwrap();
        let mut sale1 = Sale::from_cart(&cart).unwrap();
        sale1.created_at = "2026-01-15T12:00:00.000Z".to_string();
        sale1.updated_at = "2026-01-15T12:00:00.000Z".to_string();
        s.create_sale(&sale1).unwrap();
        s.update_sale_status(&sale1.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale1.id, SaleStatus::Completed)
            .unwrap();

        // February sale.
        let mut cart2 = Cart::new(usd());
        cart2
            .add_line(CartLine::new(Sku::new("M2"), 1, price(100)))
            .unwrap();
        let mut sale2 = Sale::from_cart(&cart2).unwrap();
        sale2.created_at = "2026-02-10T12:00:00.000Z".to_string();
        sale2.updated_at = "2026-02-10T12:00:00.000Z".to_string();
        s.create_sale(&sale2).unwrap();
        s.update_sale_status(&sale2.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale2.id, SaleStatus::Completed)
            .unwrap();

        let rows = s.monthly_revenue("2026-01-01", "2026-02-28").unwrap();
        assert_eq!(rows.len(), 2, "should have two monthly rows");
        assert_eq!(rows[0].month, "2026-01");
        assert_eq!(rows[0].total_minor, 100);
        assert_eq!(rows[1].month, "2026-02");
        assert_eq!(rows[1].total_minor, 100);
    }

    #[test]
    fn top_products_product_deleted_after_sale() {
        let conn = fresh();
        let s = store(&conn);
        seed_completed_sale(&conn, "DELETED", 2, 500);

        // Delete the product (simulate what happens when a product is removed).
        conn.execute("DELETE FROM products WHERE sku = 'DELETED'", [])
            .unwrap();

        // top_products JOINs with products, so the deleted product won't appear.
        let rows = s.top_products("2000-01-01", "2099-12-31", 10).unwrap();
        assert!(
            rows.is_empty(),
            "deleted products should not appear in top products"
        );
    }

    #[test]
    fn hourly_heatmap_multiple_hours() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("H1", "Hour 1", money, None, None, 100, None)
            .unwrap();
        s.create_product("H2", "Hour 2", money, None, None, 100, None)
            .unwrap();

        // Morning sale.
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("H1"), 1, price(100)))
            .unwrap();
        let mut sale1 = Sale::from_cart(&cart).unwrap();
        sale1.created_at = "2026-06-01T08:30:00.000Z".to_string();
        sale1.updated_at = "2026-06-01T08:30:00.000Z".to_string();
        s.create_sale(&sale1).unwrap();
        s.update_sale_status(&sale1.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale1.id, SaleStatus::Completed)
            .unwrap();

        // Afternoon sale.
        let mut cart2 = Cart::new(usd());
        cart2
            .add_line(CartLine::new(Sku::new("H2"), 1, price(100)))
            .unwrap();
        let mut sale2 = Sale::from_cart(&cart2).unwrap();
        sale2.created_at = "2026-06-01T14:00:00.000Z".to_string();
        sale2.updated_at = "2026-06-01T14:00:00.000Z".to_string();
        s.create_sale(&sale2).unwrap();
        s.update_sale_status(&sale2.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale2.id, SaleStatus::Completed)
            .unwrap();

        let rows = s.hourly_heatmap("2026-06-01", "2026-06-01").unwrap();
        assert_eq!(rows.len(), 2, "should have two hourly entries");
        // Use hour comparison instead of position-dependent assert
        let hours: Vec<i64> = rows.iter().map(|r| r.hour).collect();
        assert!(hours.contains(&8), "should include hour 8");
        assert!(hours.contains(&14), "should include hour 14");
    }

    #[test]
    fn category_breakdown_percentage_multiple_categories() {
        let conn = fresh();
        let s = store(&conn);

        s.create_category("cat-drinks", "Drinks", "#00f", "")
            .unwrap();
        s.create_category("cat-food", "Food", "#f00", "").unwrap();

        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("COLA", "Cola", money, Some("cat-drinks"), None, 100, None)
            .unwrap();
        s.create_product("BURGER", "Burger", money, Some("cat-food"), None, 100, None)
            .unwrap();

        // Drinks: 2 colas × 100 = 200
        let mut cart1 = Cart::new(usd());
        cart1
            .add_line(CartLine::new(Sku::new("COLA"), 2, price(100)))
            .unwrap();
        let mut sale1 = Sale::from_cart(&cart1).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale1.created_at = now.clone();
        sale1.updated_at = now.clone();
        s.create_sale(&sale1).unwrap();
        s.update_sale_status(&sale1.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale1.id, SaleStatus::Completed)
            .unwrap();

        // Food: 1 burger × 100 = 100; grand_total = 300
        let mut cart2 = Cart::new(usd());
        cart2
            .add_line(CartLine::new(Sku::new("BURGER"), 1, price(100)))
            .unwrap();
        let mut sale2 = Sale::from_cart(&cart2).unwrap();
        sale2.created_at = now.clone();
        sale2.updated_at = now.clone();
        s.create_sale(&sale2).unwrap();
        s.update_sale_status(&sale2.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale2.id, SaleStatus::Completed)
            .unwrap();

        let rows = s.category_breakdown("2000-01-01", "2099-12-31").unwrap();
        assert_eq!(rows.len(), 2);
        // Drinks should be first (higher total_minor = 200), Food second (100).
        assert_eq!(rows[0].category_name, "Drinks");
        assert_eq!(rows[0].total_minor, 200);
        // Drinks: 200/300 = 66.666...%
        assert!((rows[0].percentage - 200.0 / 3.0).abs() < 0.01);
        assert_eq!(rows[1].category_name, "Food");
        assert_eq!(rows[1].total_minor, 100);
        // Food: 100/300 = 33.333...%
        assert!((rows[1].percentage - 100.0 / 3.0).abs() < 0.01);
        // Percentages should sum to ~100.
        let total_pct: f64 = rows.iter().map(|r| r.percentage).sum();
        assert!(
            (total_pct - 100.0).abs() < 0.01,
            "percentages should sum to 100"
        );
    }
}
