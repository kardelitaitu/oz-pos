//! Reporting queries: revenue summaries, top products, heatmap, low-stock alerts.
/*
last audited 25-07-26 by RSA-Agent (oz-core slice B5 part 2: reports deep read)
crate: oz-core | status: SAFE | lint: CLEAN
findings: COR-21 MEDIUM: ALL date/hour bucketing uses UTC (DATE(created_at), strftime('%H')) with zero localtime adjustment while timestamps are written Utc::now() and the primary market is UTC+7/+8 — daily/weekly/monthly revenue, heatmap, occupancy, trends mis-bucket transactions outside 00:00-07:00 local; HourlyOccupancyRow doc claims "local store time as stored" — drift. Also: top_products limit unclamped (voided_items clamps — inconsistent); deprecated low_stock_alerts reads legacy inventory (see COR-19); COGS uses current product cost by documented reporting-layer semantics
next: add store-timezone setting + bucket via chrono/SQLite offset modifier; clamp top_products limit (COR-21) | perf: correlated COGS subqueries are deliberate anti-multiplication design, documented
*/

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
    /// Cost of goods sold in minor units (Σ current cost × qty over the
    /// date's completed-sale lines; 0 when no costs are recorded).
    pub cogs_minor: i64,
    /// Gross profit in minor units: revenue − COGS.
    pub gross_profit_minor: i64,
    /// Gross margin as a percentage of revenue; 0.0 when revenue is 0.
    pub gross_margin_percent: f64,
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
    /// Cost of goods sold in minor units (Σ current cost × qty over the
    /// week's completed-sale lines; 0 when no costs are recorded).
    pub cogs_minor: i64,
    /// Gross profit in minor units: revenue − COGS.
    pub gross_profit_minor: i64,
    /// Gross margin as a percentage of revenue; 0.0 when revenue is 0.
    pub gross_margin_percent: f64,
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
    /// Cost of goods sold in minor units (Σ current cost × qty over the
    /// month's completed-sale lines; 0 when no costs are recorded).
    pub cogs_minor: i64,
    /// Gross profit in minor units: revenue − COGS.
    pub gross_profit_minor: i64,
    /// Gross margin as a percentage of revenue; 0.0 when revenue is 0.
    pub gross_margin_percent: f64,
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
    /// Cost of goods sold in minor units (snapshotted sale-line cost,
    /// falling back to the product's current cost).
    pub cogs_minor: i64,
    /// Gross profit in minor units: revenue − COGS.
    pub gross_profit_minor: i64,
    /// Gross margin as a percentage of revenue; 0.0 when revenue is 0.
    pub gross_margin_percent: f64,
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
    /// Product currency code.
    pub currency: String,
    /// Product selling price per unit in minor units.
    pub price_minor: i64,
    /// Product cost (HPP) per unit in minor units.
    pub cost_minor: i64,
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

/// Sales revenue split by payment method for a date range.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentMethodRow {
    /// Payment method key (`cash`, `card`, `qris`, `ewallet`, … or `other`).
    pub payment_method: String,
    /// Total revenue in minor units.
    pub total_minor: i64,
    /// Number of completed sales paid this way.
    pub sale_count: i64,
}

/// Voided-sale totals for a date range.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VoidedSummaryRow {
    /// Number of voided sales.
    pub void_count: i64,
    /// Sum of the voided sales' totals in minor units.
    pub void_total_minor: i64,
}

/// One product line found on voided sales.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VoidedItemRow {
    /// Product display name (SKU fallback for deleted products).
    pub name: String,
    /// Total quantity voided.
    pub qty: i64,
}

/// Average basket size for a date range.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BasketSizeRow {
    /// Number of completed sales.
    pub sale_count: i64,
    /// Mean `line_count` across those sales (0.0 when no sales).
    pub avg_line_count: f64,
}

/// Basket size (mean line count) per day within a date range — the raw
/// per-bucket shape behind the analytics basket-size trend card.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BasketTrendRow {
    /// ISO date `YYYY-MM-DD`.
    pub date: String,
    /// Completed sales that day.
    pub sale_count: i64,
    /// Mean `line_count` across that day's completed sales.
    pub avg_line_count: f64,
}

/// New vs returning customers for a date range.
///
/// A customer is "returning" when they have a completed sale before the
/// range start; otherwise the range visit counts them as new.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CustomerSplitRow {
    /// Distinct customers whose first completed sale falls inside the range.
    pub new_count: i64,
    /// Distinct customers with a completed sale inside the range who had
    /// one before it.
    pub returning_count: i64,
}

/// One discount code's redemption count within a date range.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscountCodeRow {
    /// Discount label (empty label → `discount`).
    pub label: String,
    /// Completed sales using this discount.
    pub redeemed_count: i64,
}

/// Discount usage summary for a date range.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscountsSummaryRow {
    /// Completed sales in the range.
    pub sale_count: i64,
    /// Completed sales that applied any discount.
    pub discounted_sale_count: i64,
    /// `discounted_sale_count / sale_count` as a percentage (0.0 when none).
    pub share_percent: f64,
    /// Top discount codes by redemption count.
    pub codes: Vec<DiscountCodeRow>,
}

/// Stock-turnover snapshot for a date range at one location.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InventoryTurnoverRow {
    /// Units sold across completed sales in the range.
    pub units_sold: i64,
    /// Units on hand at the location (from `stock_summary`).
    pub stock_on_hand: i64,
    /// Number of catalog products.
    pub sku_count: i64,
    /// Length of the queried range in days (inclusive).
    pub range_days: i64,
}

/// Units sold per day for a date range (inventory trend line).
#[derive(Debug, Clone, serde::Serialize)]
pub struct InventoryTrendRow {
    /// ISO date `YYYY-MM-DD`.
    pub date: String,
    /// Units sold that day.
    pub units_sold: i64,
}

/// Completed table-bound orders per day (restaurant table-turnover source).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TableTurnoverRow {
    /// ISO date `YYYY-MM-DD`.
    pub date: String,
    /// Completed sales linked to a KDS order carrying a table number.
    pub table_orders: i64,
}

/// Completed table-bound orders per hour of day (0–23) — the real shape
/// behind the restaurant occupancy curve.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HourlyOccupancyRow {
    /// Hour of day (0–23, local store time as stored in `created_at`).
    pub hour: i64,
    /// Completed sales linked to a KDS order carrying a table number.
    pub table_orders: i64,
}

impl Store<'_> {
    /// Map the shared revenue/COGS columns of the aggregation rows.
    fn revenue_profit_fields(row: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, i64, i64, f64)> {
        let total_minor: i64 = row.get("total_minor")?;
        let cogs_minor: i64 = row.get("cogs_minor")?;
        let gross_profit_minor = total_minor - cogs_minor;
        let gross_margin_percent = if total_minor > 0 {
            gross_profit_minor as f64 / total_minor as f64 * 100.0
        } else {
            0.0
        };
        Ok((
            total_minor,
            cogs_minor,
            gross_profit_minor,
            gross_margin_percent,
        ))
    }

    /// Daily revenue for a date range.
    pub fn daily_revenue(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<DailyRevenueRow>, CoreError> {
        // COGS is a correlated subquery: joining sale_lines directly would
        // multiply revenue/count by the line count per sale, so revenue stays
        // on the sales table and only the cost side joins the lines. Costs
        // use the product's current cost_minor (reporting-layer semantics).
        let mut stmt = self.conn.prepare(
            "SELECT DATE(s.created_at) AS date,
                    SUM(s.total_minor) AS total_minor,
                    s.currency AS currency,
                    COUNT(*) AS sale_count,
                    (SELECT COALESCE(SUM(COALESCE(sl2.cost_minor, p2.cost_minor, 0) * sl2.qty), 0)
                     FROM sale_lines sl2
                     JOIN sales s2 ON sl2.sale_id = s2.id
                     LEFT JOIN products p2 ON sl2.sku = p2.sku
                     WHERE s2.status = 'completed'
                       AND s2.currency = s.currency
                       AND DATE(s2.created_at) = DATE(s.created_at)) AS cogs_minor
             FROM sales s
             WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY DATE(s.created_at), s.currency
             ORDER BY date ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            let (total_minor, cogs_minor, gross_profit_minor, gross_margin_percent) =
                Self::revenue_profit_fields(row)?;
            Ok(DailyRevenueRow {
                date: row.get("date")?,
                total_minor,
                currency: row.get("currency")?,
                sale_count: row.get("sale_count")?,
                cogs_minor,
                gross_profit_minor,
                gross_margin_percent,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Weekly revenue (Monday-first weeks) for a date range.
    pub fn weekly_revenue(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<WeeklyRevenueRow>, CoreError> {
        // Monday-first weeks, matching the UI's `weekStartKey` and
        // `rangeForGranularity('weekly')`. `'-6 days', 'weekday 1'` is the
        // correct boundary idiom: `'weekday 1', '-7 days'` would push a
        // Monday sale into the PREVIOUS week. COGS is a correlated subquery
        // keyed on the same week expression, so joining sale_lines never
        // multiplies revenue/count per sale line.
        let mut stmt = self.conn.prepare(
            "SELECT DATE(s.created_at, '-6 days', 'weekday 1') AS week_start,
                    SUM(s.total_minor) AS total_minor, s.currency AS currency,
                    COUNT(*) AS sale_count,
                    (SELECT COALESCE(SUM(COALESCE(sl2.cost_minor, p2.cost_minor, 0) * sl2.qty), 0)
                     FROM sale_lines sl2
                     JOIN sales s2 ON sl2.sale_id = s2.id
                     LEFT JOIN products p2 ON sl2.sku = p2.sku
                     WHERE s2.status = 'completed'
                       AND s2.currency = s.currency
                       AND DATE(s2.created_at, '-6 days', 'weekday 1')
                           = DATE(s.created_at, '-6 days', 'weekday 1')) AS cogs_minor
             FROM sales s
             WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY week_start, s.currency
             ORDER BY week_start ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            let (total_minor, cogs_minor, gross_profit_minor, gross_margin_percent) =
                Self::revenue_profit_fields(row)?;
            Ok(WeeklyRevenueRow {
                week_start: row.get("week_start")?,
                total_minor,
                currency: row.get("currency")?,
                sale_count: row.get("sale_count")?,
                cogs_minor,
                gross_profit_minor,
                gross_margin_percent,
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
        // COGS is a correlated subquery keyed on the same YYYY-MM expression,
        // so joining sale_lines never multiplies revenue/count per line.
        let mut stmt = self.conn.prepare(
            "SELECT SUBSTR(s.created_at, 1, 7) AS month,
                    SUM(s.total_minor) AS total_minor, s.currency AS currency,
                    COUNT(*) AS sale_count,
                    (SELECT COALESCE(SUM(COALESCE(sl2.cost_minor, p2.cost_minor, 0) * sl2.qty), 0)
                     FROM sale_lines sl2
                     JOIN sales s2 ON sl2.sale_id = s2.id
                     LEFT JOIN products p2 ON sl2.sku = p2.sku
                     WHERE s2.status = 'completed'
                       AND s2.currency = s.currency
                       AND SUBSTR(s2.created_at, 1, 7) = SUBSTR(s.created_at, 1, 7)) AS cogs_minor
             FROM sales s
             WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY month, s.currency
             ORDER BY month ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            let (total_minor, cogs_minor, gross_profit_minor, gross_margin_percent) =
                Self::revenue_profit_fields(row)?;
            Ok(MonthlyRevenueRow {
                month: row.get("month")?,
                total_minor,
                currency: row.get("currency")?,
                sale_count: row.get("sale_count")?,
                cogs_minor,
                gross_profit_minor,
                gross_margin_percent,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Top products ranked by `order_by` (`"revenue"` or `"profit"`) for a
    /// date range. Unknown values fall back to revenue ranking.
    pub fn top_products(
        &self,
        start_date: &str,
        end_date: &str,
        limit: i64,
        order_by: &str,
    ) -> Result<Vec<TopProductRow>, CoreError> {
        let order_clause = if order_by == "profit" {
            "gross_profit_minor DESC"
        } else {
            "total_minor DESC"
        };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT p.id AS product_id, p.sku, p.name,
                    SUM(sl.qty) AS total_qty,
                    SUM(sl.line_minor) AS total_minor,
                    SUM(COALESCE(sl.cost_minor, p.cost_minor, 0) * sl.qty) AS cogs_minor,
                    (SUM(sl.line_minor) - SUM(COALESCE(sl.cost_minor, p.cost_minor, 0) * sl.qty)) AS gross_profit_minor
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             JOIN products p ON sl.sku = p.sku
             WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY p.id
             ORDER BY {order_clause}, p.sku
             LIMIT ?3"
        ))?;
        let rows = stmt.query_map(params![start_date, end_date, limit], |row| {
            let total_minor = row.get::<_, i64>("total_minor")?;
            let cogs_minor = row.get::<_, i64>("cogs_minor")?;
            let gross_profit_minor = row.get::<_, i64>("gross_profit_minor")?;
            Ok(TopProductRow {
                product_id: row.get("product_id")?,
                sku: row.get("sku")?,
                name: row.get("name")?,
                total_qty: row.get("total_qty")?,
                total_minor,
                cogs_minor,
                gross_profit_minor,
                gross_margin_percent: if total_minor > 0 {
                    (gross_profit_minor as f64 / total_minor as f64) * 100.0
                } else {
                    0.0
                },
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
    /// **Deprecated in favour of `low_stock_alerts_at_location`
    /// (Self::low_stock_alerts_at_location)**, which respects the
    /// per-location stock from `stock_summary`.
    #[deprecated(note = "use low_stock_alerts_at_location instead")]
    pub fn low_stock_alerts(&self, threshold: i64) -> Result<Vec<LowStockAlert>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id AS product_id, p.sku, p.name, p.currency,
                    p.price_minor, p.cost_minor,
                    COALESCE(i.qty, 0) AS current_qty,
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
                currency: row.get("currency")?,
                price_minor: row.get("price_minor")?,
                cost_minor: row.get("cost_minor")?,
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
            "SELECT p.id AS product_id, p.sku, p.name, p.currency,
                    p.price_minor, p.cost_minor,
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
                currency: row.get("currency")?,
                price_minor: row.get("price_minor")?,
                cost_minor: row.get("cost_minor")?,
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

    /// Acknowledge a stock alert event — records who acknowledged it and
    /// transitions the status from `active` to `acknowledged`.
    ///
    /// Only `active` alerts can be acknowledged; already-`acknowledged` or
    /// `resolved` alerts are left unchanged silently.
    pub fn acknowledge_stock_alert(&self, alert_id: &str, user_id: &str) -> Result<(), CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let affected = self.conn.execute(
            "UPDATE stock_alert_events
             SET status = 'acknowledged', acknowledged_at = ?1, acknowledged_by = ?2
             WHERE id = ?3 AND status = 'active'",
            params![now, user_id, alert_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "active_stock_alert",
                id: alert_id.to_owned(),
            });
        }
        Ok(())
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

    /// Revenue split by payment method for a date range.
    pub fn payment_method_breakdown(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<PaymentMethodRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(payment_method, 'other') AS payment_method,
                    SUM(total_minor) AS total_minor,
                    COUNT(*) AS sale_count
             FROM sales
             WHERE status = 'completed' AND DATE(created_at) BETWEEN ?1 AND ?2
             GROUP BY payment_method
             ORDER BY total_minor DESC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(PaymentMethodRow {
                payment_method: row.get("payment_method")?,
                total_minor: row.get("total_minor")?,
                sale_count: row.get("sale_count")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Voided-sale totals for a date range.
    pub fn voided_sales_summary(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<VoidedSummaryRow, CoreError> {
        let row = self.conn.query_row(
            "SELECT COUNT(*) AS void_count,
                    COALESCE(SUM(total_minor), 0) AS void_total_minor
             FROM sales
             WHERE status = 'voided' AND DATE(created_at) BETWEEN ?1 AND ?2",
            params![start_date, end_date],
            |row| {
                Ok(VoidedSummaryRow {
                    void_count: row.get("void_count")?,
                    void_total_minor: row.get("void_total_minor")?,
                })
            },
        )?;
        Ok(row)
    }

    /// Top product lines found on voided sales for a date range.
    pub fn voided_items(
        &self,
        start_date: &str,
        end_date: &str,
        limit: i64,
    ) -> Result<Vec<VoidedItemRow>, CoreError> {
        let limit = limit.clamp(1, 100);
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(p.name, sl.sku) AS name, SUM(sl.qty) AS qty
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             LEFT JOIN products p ON sl.sku = p.sku
             WHERE s.status = 'voided' AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY sl.sku
             ORDER BY qty DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![start_date, end_date, limit], |row| {
            Ok(VoidedItemRow {
                name: row.get("name")?,
                qty: row.get("qty")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Average basket size (mean line count) for a date range.
    pub fn avg_basket_size(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<BasketSizeRow, CoreError> {
        let row = self.conn.query_row(
            "SELECT COUNT(*) AS sale_count,
                    COALESCE(AVG(line_count), 0) AS avg_line_count
             FROM sales
             WHERE status = 'completed' AND DATE(created_at) BETWEEN ?1 AND ?2",
            params![start_date, end_date],
            |row| {
                Ok(BasketSizeRow {
                    sale_count: row.get("sale_count")?,
                    avg_line_count: row.get("avg_line_count")?,
                })
            },
        )?;
        Ok(row)
    }

    /// Per-day basket size (mean line count) for a date range.
    pub fn basket_size_trend(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<BasketTrendRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DATE(created_at) AS date,
                    COUNT(*) AS sale_count,
                    COALESCE(AVG(line_count), 0) AS avg_line_count
             FROM sales
             WHERE status = 'completed' AND DATE(created_at) BETWEEN ?1 AND ?2
             GROUP BY DATE(created_at)
             ORDER BY date ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(BasketTrendRow {
                date: row.get("date")?,
                sale_count: row.get("sale_count")?,
                avg_line_count: row.get("avg_line_count")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// New vs returning customer counts for a date range.
    pub fn customer_split(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<CustomerSplitRow, CoreError> {
        let row = self.conn.query_row(
            "WITH range_customers AS (
                SELECT DISTINCT customer_id FROM sales
                WHERE status = 'completed' AND customer_id IS NOT NULL
                  AND DATE(created_at) BETWEEN ?1 AND ?2
             )
             SELECT
                (SELECT COUNT(*) FROM range_customers rc
                 WHERE NOT EXISTS (
                   SELECT 1 FROM sales s
                   WHERE s.customer_id = rc.customer_id AND s.status = 'completed'
                     AND DATE(s.created_at) < ?1)) AS new_count,
                (SELECT COUNT(*) FROM range_customers rc
                 WHERE EXISTS (
                   SELECT 1 FROM sales s
                   WHERE s.customer_id = rc.customer_id AND s.status = 'completed'
                     AND DATE(s.created_at) < ?1)) AS returning_count",
            params![start_date, end_date],
            |row| {
                Ok(CustomerSplitRow {
                    new_count: row.get("new_count")?,
                    returning_count: row.get("returning_count")?,
                })
            },
        )?;
        Ok(row)
    }

    /// Discount usage for a date range: share of discounted sales plus the
    /// most-redeemed discount codes.
    pub fn discounts_summary(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<DiscountsSummaryRow, CoreError> {
        let (sale_count, discounted_sale_count): (i64, i64) = self.conn.query_row(
            "SELECT COUNT(*) AS sale_count,
                    COALESCE(SUM(CASE WHEN discount_percent > 0 THEN 1 ELSE 0 END), 0)
                        AS discounted_sale_count
             FROM sales
             WHERE status = 'completed' AND DATE(created_at) BETWEEN ?1 AND ?2",
            params![start_date, end_date],
            |row| Ok((row.get("sale_count")?, row.get("discounted_sale_count")?)),
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(NULLIF(discount_label, ''), 'discount') AS label,
                    COUNT(*) AS redeemed_count
             FROM sales
             WHERE status = 'completed' AND discount_percent > 0
               AND DATE(created_at) BETWEEN ?1 AND ?2
             GROUP BY discount_label
             ORDER BY redeemed_count DESC
             LIMIT 5",
        )?;
        let codes = stmt
            .query_map(params![start_date, end_date], |row| {
                Ok(DiscountCodeRow {
                    label: row.get("label")?,
                    redeemed_count: row.get("redeemed_count")?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let share_percent = if sale_count > 0 {
            discounted_sale_count as f64 / sale_count as f64 * 100.0
        } else {
            0.0
        };
        Ok(DiscountsSummaryRow {
            sale_count,
            discounted_sale_count,
            share_percent,
            codes,
        })
    }

    /// Stock-turnover snapshot for a date range at one location: units sold
    /// over the period vs stock on hand, plus the catalog size.
    pub fn inventory_turnover(
        &self,
        start_date: &str,
        end_date: &str,
        location_id: &str,
    ) -> Result<InventoryTurnoverRow, CoreError> {
        let row = self.conn.query_row(
            "SELECT
                (SELECT COALESCE(SUM(sl.qty), 0) FROM sale_lines sl
                 JOIN sales s ON sl.sale_id = s.id
                 WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2)
                    AS units_sold,
                (SELECT COALESCE(SUM(COALESCE(qty, 0)), 0) FROM stock_summary
                 WHERE location_id = ?3) AS stock_on_hand,
                (SELECT COUNT(*) FROM products) AS sku_count",
            params![start_date, end_date, location_id],
            |row| {
                Ok(InventoryTurnoverRow {
                    units_sold: row.get("units_sold")?,
                    stock_on_hand: row.get("stock_on_hand")?,
                    sku_count: row.get("sku_count")?,
                    range_days: 0,
                })
            },
        )?;
        Ok(InventoryTurnoverRow {
            range_days: Self::inclusive_range_days(start_date, end_date),
            ..row
        })
    }

    /// Units sold per day for a date range (the inventory trend line).
    pub fn inventory_trend(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<InventoryTrendRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DATE(s.created_at) AS date,
                    COALESCE(SUM(sl.qty), 0) AS units_sold
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             WHERE s.status = 'completed' AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY DATE(s.created_at)
             ORDER BY date ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(InventoryTrendRow {
                date: row.get("date")?,
                units_sold: row.get("units_sold")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Completed table-bound orders per day for a date range. Table service
    /// is tracked through KDS orders carrying a table number; each completed
    /// sale with one represents a single table turn (takeaway orders without
    /// a table number are excluded).
    pub fn table_turnover(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<TableTurnoverRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DATE(s.created_at) AS date,
                    COUNT(*) AS table_orders
             FROM kds_orders k
             JOIN sales s ON k.sale_id = s.id
             WHERE s.status = 'completed'
               AND k.table_number IS NOT NULL AND k.table_number != ''
               AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY DATE(s.created_at)
             ORDER BY date ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(TableTurnoverRow {
                date: row.get("date")?,
                table_orders: row.get("table_orders")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Completed table-bound orders grouped by hour of day (0–23) within a
    /// date range — the real signal behind the occupancy-by-hour curve.
    pub fn hourly_table_activity(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<HourlyOccupancyRow>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(strftime('%H', s.created_at) AS INTEGER) AS hour,
                    COUNT(*) AS table_orders
             FROM kds_orders k
             JOIN sales s ON k.sale_id = s.id
             WHERE s.status = 'completed'
               AND k.table_number IS NOT NULL AND k.table_number != ''
               AND DATE(s.created_at) BETWEEN ?1 AND ?2
             GROUP BY hour
             ORDER BY hour ASC",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok(HourlyOccupancyRow {
                hour: row.get("hour")?,
                table_orders: row.get("table_orders")?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Inclusive day count of a `YYYY-MM-DD` range (0 when unparseable).
    fn inclusive_range_days(start_date: &str, end_date: &str) -> i64 {
        let (Ok(start), Ok(end)) = (
            chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d"),
            chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d"),
        ) else {
            return 0;
        };
        (end - start).num_days() + 1
    }
}

#[cfg(test)]
#[path = "reports_tests.rs"]
mod tests;
