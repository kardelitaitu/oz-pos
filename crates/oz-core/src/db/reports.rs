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
mod tests {
    use crate::db::Store;
    use crate::db::products::{CreateProductAttributes, UpdateProductAttributes};
    use crate::kds::CreateKdsOrderInput;
    use crate::money::Currency;
    use crate::{Cart, CartLine, Money, Sale, SaleStatus, Sku, migrations};
    use rusqlite::{Connection, params};

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
             VALUES ('role-staff', 'staff', '', '[]', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES (?1, ?1, 'x', ?1, 'role-staff', 1, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')",
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
        // No costs were set on the seeded products → COGS 0, profit = revenue.
        assert_eq!(rows[0].cogs_minor, 0);
        assert_eq!(rows[0].gross_profit_minor, 1150);
        assert_eq!(rows[0].gross_margin_percent, 100.0);
    }

    #[test]
    fn daily_revenue_gross_profit_from_product_costs() {
        let conn = fresh();
        // Seed a sale with a costed product and one with no cost at all.
        seed_completed_sale(&conn, "STEAK", 2, 2500);
        conn.execute(
            "UPDATE products SET cost_minor = 800 WHERE sku = 'STEAK'",
            [],
        )
        .unwrap();
        seed_completed_sale(&conn, "FREE", 1, 500);

        let rows = store(&conn)
            .daily_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert_eq!(rows.len(), 1);
        // Revenue 5500, COGS 2 × 800 = 1600 → profit 3900 (~70.9%).
        assert_eq!(rows[0].total_minor, 5500);
        assert_eq!(rows[0].cogs_minor, 1600);
        assert_eq!(rows[0].gross_profit_minor, 3900);
        let expected = 3900.0 / 5500.0 * 100.0;
        assert!(
            (rows[0].gross_margin_percent - expected).abs() < 1e-9,
            "margin was {}",
            rows[0].gross_margin_percent
        );
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
        // No costs seeded → COGS 0, profit equals revenue.
        assert_eq!(rows[0].cogs_minor, 0);
        assert_eq!(rows[0].gross_profit_minor, 600);
        assert_eq!(rows[0].gross_margin_percent, 100.0);
    }

    #[test]
    fn weekly_revenue_gross_profit_from_product_costs() {
        let conn = fresh();
        seed_completed_sale(&conn, "STEAK", 2, 2500);
        conn.execute(
            "UPDATE products SET cost_minor = 800 WHERE sku = 'STEAK'",
            [],
        )
        .unwrap();
        let rows = store(&conn)
            .weekly_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert_eq!(rows[0].total_minor, 5000);
        assert_eq!(rows[0].cogs_minor, 1600);
        assert_eq!(rows[0].gross_profit_minor, 3400);
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
        // No costs seeded → COGS 0, profit equals revenue.
        assert_eq!(rows[0].cogs_minor, 0);
        assert_eq!(rows[0].gross_profit_minor, 500);
        assert_eq!(rows[0].gross_margin_percent, 100.0);
    }

    #[test]
    fn monthly_revenue_gross_profit_from_product_costs() {
        let conn = fresh();
        seed_completed_sale(&conn, "STEAK", 2, 2500);
        conn.execute(
            "UPDATE products SET cost_minor = 800 WHERE sku = 'STEAK'",
            [],
        )
        .unwrap();
        let rows = store(&conn)
            .monthly_revenue("2000-01-01", "2099-12-31")
            .unwrap();
        assert_eq!(rows[0].total_minor, 5000);
        assert_eq!(rows[0].cogs_minor, 1600);
        assert_eq!(rows[0].gross_profit_minor, 3400);
    }

    // ── Top products ───────────────────────────────────────────────

    #[test]
    fn top_products_empty() {
        let conn = fresh();
        let rows = store(&conn)
            .top_products("2000-01-01", "2099-12-31", 10, "revenue")
            .unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn top_products_with_sales() {
        let conn = fresh();
        seed_completed_sale(&conn, "COFFEE", 2, 350);
        seed_completed_sale(&conn, "BAGEL", 1, 450);
        let rows = store(&conn)
            .top_products("2000-01-01", "2099-12-31", 10, "revenue")
            .unwrap();
        assert!(!rows.is_empty());
        // BAGEL has higher unit price but lower qty → check ordering
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn top_products_ranks_by_gross_profit_when_requested() {
        let conn = fresh();
        let s = store(&conn);
        // A: high revenue, thin margin. B: lower revenue, fat margin.
        s.create_product_with_attributes(
            "A",
            "Thin Margin",
            price(1000),
            None,
            None,
            100,
            None,
            &CreateProductAttributes {
                cost_minor: 900,
                ..Default::default()
            },
        )
        .unwrap();
        s.create_product_with_attributes(
            "B",
            "Fat Margin",
            price(500),
            None,
            None,
            100,
            None,
            &CreateProductAttributes {
                cost_minor: 100,
                ..Default::default()
            },
        )
        .unwrap();
        for sku in ["A", "B"] {
            let mut cart = Cart::new(usd());
            cart.add_line(CartLine::new(
                Sku::new(sku),
                10,
                price(if sku == "A" { 1000 } else { 500 }),
            ))
            .unwrap();
            let mut sale = Sale::from_cart(&cart).unwrap();
            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            sale.created_at = now.clone();
            sale.updated_at = now;
            s.create_sale(&sale).unwrap();
            s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
            s.update_sale_status(&sale.id, SaleStatus::Completed)
                .unwrap();
        }

        let by_revenue = s
            .top_products("2000-01-01", "2099-12-31", 10, "revenue")
            .unwrap();
        assert_eq!(by_revenue[0].sku, "A");
        assert_eq!(by_revenue[0].gross_profit_minor, 1000);

        let by_profit = s
            .top_products("2000-01-01", "2099-12-31", 10, "profit")
            .unwrap();
        // Revenue A=10000 > B=5000, but profit B=4000 > A=1000.
        assert_eq!(by_profit[0].sku, "B");
        assert_eq!(by_profit[0].gross_profit_minor, 4000);
        assert_eq!(by_profit[1].sku, "A");

        // Unknown keys fall back to revenue ranking.
        let by_unknown = s
            .top_products("2000-01-01", "2099-12-31", 10, "quantity")
            .unwrap();
        assert_eq!(by_unknown[0].sku, "A");
    }

    #[test]
    fn top_products_respects_limit() {
        let conn = fresh();
        seed_completed_sale(&conn, "A", 1, 100);
        seed_completed_sale(&conn, "B", 1, 200);
        seed_completed_sale(&conn, "C", 1, 300);
        let rows = store(&conn)
            .top_products("2000-01-01", "2099-12-31", 2, "revenue")
            .unwrap();
        assert_eq!(rows.len(), 2);
        // Highest revenue first
        assert_eq!(rows[0].sku, "C");
        assert_eq!(rows[1].sku, "B");
    }

    #[test]
    fn top_products_with_costs_and_snapshot_fallback() {
        let conn = fresh();
        let s = store(&conn);
        // Product with a cost — the sale-line snapshot freezes it at checkout.
        s.create_product_with_attributes(
            "COFFEE",
            "Coffee",
            price(350),
            None,
            None,
            100,
            None,
            &CreateProductAttributes {
                cost_minor: 150,
                ..Default::default()
            },
        )
        .unwrap();
        // Product with no cost — snapshot stays NULL, falls back to current cost.
        s.create_product("TEA", "Tea", price(200), None, None, 100, None)
            .unwrap();

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("TEA"), 3, price(200)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        // Editing HPP after the sale must NOT restate the frozen snapshot.
        s.update_product_attributes(
            "COFFEE",
            &UpdateProductAttributes {
                cost_minor: Some(999),
                ..Default::default()
            },
        )
        .unwrap();

        let rows = s
            .top_products("2000-01-01", "2099-12-31", 10, "revenue")
            .unwrap();
        let coffee = rows.iter().find(|r| r.sku == "COFFEE").unwrap();
        // COGS = snapshot 150 × 2 = 300 (not 999 × 2); profit = 700 − 300.
        assert_eq!(coffee.total_minor, 700);
        assert_eq!(coffee.cogs_minor, 300);
        assert_eq!(coffee.gross_profit_minor, 400);
        assert!((coffee.gross_margin_percent - 57.14).abs() < 0.01);
        let tea = rows.iter().find(|r| r.sku == "TEA").unwrap();
        // No cost recorded anywhere → COGS 0, profit = revenue, margin 100%.
        assert_eq!(tea.total_minor, 600);
        assert_eq!(tea.cogs_minor, 0);
        assert_eq!(tea.gross_profit_minor, 600);
        assert!((tea.gross_margin_percent - 100.0).abs() < f64::EPSILON);
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
        let rows = s
            .top_products("2000-01-01", "2099-12-31", 10, "revenue")
            .unwrap();
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

    // ── Weekly revenue boundary / invariant tests ──────────────────

    /// Non-completed sales (Draft, Active, Voided) MUST NOT appear
    /// in `weekly_revenue`. Same `status='completed'` filter as the
    /// daily / monthly variants — pins contract parity across.
    #[test]
    fn weekly_revenue_excludes_non_completed() {
        let conn = fresh();
        let s = store(&conn);
        let sku = "SKU1";
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product(sku, sku, money, None, None, 100, None)
            .unwrap();

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new(sku), 1, price(100)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.created_at = "2026-07-20T12:00:00.000Z".into();
        sale.updated_at = sale.created_at.clone();
        s.create_sale(&sale).unwrap();
        // Status stays at Draft — never advanced to Active/Completed.

        let rows = s.weekly_revenue("2026-07-01", "2026-07-31").unwrap();
        assert!(
            rows.is_empty(),
            "weekly_revenue must only include completed sales"
        );
    }

    /// A range narrower than a full calendar week produces exactly
    /// 1 row containing the bucket. Pins that single-day queries
    /// return a single weekly bucket (Sunday-based).
    #[test]
    fn weekly_revenue_partial_week_range() {
        let conn = fresh();
        seed_completed_sale(&conn, "SKU", 1, 100);
        // Override the seeded created_at to a known Monday.
        conn.execute(
            "UPDATE sales SET created_at = '2026-07-20T10:00:00.000Z'",
            [],
        )
        .unwrap();

        let rows = store(&conn)
            .weekly_revenue("2026-07-20", "2026-07-20")
            .unwrap();
        assert_eq!(rows.len(), 1);
        // 2026-07-20 is a Monday → Monday-first week starts the same day.
        assert_eq!(rows[0].week_start, "2026-07-20");
        assert_eq!(rows[0].total_minor, 100);
        assert_eq!(rows[0].sale_count, 1);
    }

    /// Leap days are bucketed to their Monday-first week by SQLite's
    /// `'-6 days', 'weekday 1'` modifier. Pins: Feb 29, 2024 (Thursday)
    /// falls into the week that starts Monday 2024-02-26.
    #[test]
    fn weekly_revenue_leap_day_falls_in_week() {
        let conn = fresh();
        seed_completed_sale(&conn, "SKU", 1, 100);
        conn.execute(
            "UPDATE sales SET created_at = '2024-02-29T10:00:00.000Z'",
            [],
        )
        .unwrap();

        let rows = store(&conn)
            .weekly_revenue("2024-02-29", "2024-02-29")
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].week_start, "2024-02-26",
            "Feb 29 2024 (Thursday) -> Monday 2024-02-26"
        );
        assert_eq!(rows[0].total_minor, 100);
    }

    /// Week start must be Monday-first, matching the UI's `weekStartKey`
    /// (tables/basket/heatmap), `rangeForGranularity('weekly')` and the
    /// dev mock. Pins the two boundaries: a sale on Sunday 2026-08-16
    /// belongs to the week that started Mon 2026-08-10, and a sale on a
    /// Monday is its own week start.
    #[test]
    fn weekly_revenue_monday_first_week_start() {
        let conn = fresh();
        seed_completed_sale(&conn, "SKU", 1, 100);
        let s = store(&conn);

        // Sunday 2026-08-16 → week starting Monday 2026-08-10 (NOT the
        // preceding Sunday 2026-08-09, and not the following Sunday).
        conn.execute(
            "UPDATE sales SET created_at = '2026-08-16T10:00:00.000Z'",
            [],
        )
        .unwrap();
        let rows = s.weekly_revenue("2026-08-16", "2026-08-16").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].week_start, "2026-08-10");
        assert_eq!(rows[0].total_minor, 100);

        // A Monday sale starts its own week.
        conn.execute(
            "UPDATE sales SET created_at = '2026-08-10T10:00:00.000Z'",
            [],
        )
        .unwrap();
        let rows = s.weekly_revenue("2026-08-10", "2026-08-10").unwrap();
        assert_eq!(rows[0].week_start, "2026-08-10");
    }

    /// Currency zero-boundary: a sale with `total_minor = 0` MUST
    /// still increment `sale_count` and contribute 0 to `total_minor`.
    #[test]
    fn weekly_revenue_zero_revenue_sale() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 0,
            currency: usd(),
        };
        s.create_product("ZERO", "Zero Item", money, None, None, 100, None)
            .unwrap();

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("ZERO"), 1, price(0)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        let rows = s.weekly_revenue("2000-01-01", "2099-12-31").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sale_count, 1, "zero-revenue sale still counted");
        assert_eq!(rows[0].total_minor, 0);
    }

    /// Multi-currency week: two sales in different currencies within
    /// the same calendar week produce TWO rows (GROUP BY week, currency).
    #[test]
    fn weekly_revenue_multiple_currencies_separate_rows() {
        let conn = fresh();
        let s = store(&conn);
        let gbp = "GBP".parse().unwrap();

        // USD sale.
        let usd_money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("USD_A", "USD Item", usd_money, None, None, 100, None)
            .unwrap();
        let mut cart_usd = Cart::new(usd());
        cart_usd
            .add_line(CartLine::new(Sku::new("USD_A"), 1, price(100)))
            .unwrap();
        let mut sale_usd = Sale::from_cart(&cart_usd).unwrap();
        sale_usd.created_at = "2026-07-20T10:00:00.000Z".into();
        sale_usd.updated_at = "2026-07-20T10:00:00.000Z".into();
        s.create_sale(&sale_usd).unwrap();
        s.update_sale_status(&sale_usd.id, SaleStatus::Active)
            .unwrap();
        s.update_sale_status(&sale_usd.id, SaleStatus::Completed)
            .unwrap();

        // GBP sale (separate currency). Construct GBP Money manually
        // since the `price()` helper returns USD.
        let gbp_money = Money {
            minor_units: 200,
            currency: gbp,
        };
        s.create_product("GBP_A", "GBP Item", gbp_money, None, None, 100, None)
            .unwrap();
        let mut cart_gbp = Cart::new(gbp);
        cart_gbp
            .add_line(CartLine::new(Sku::new("GBP_A"), 1, gbp_money))
            .unwrap();
        let mut sale_gbp = Sale::from_cart(&cart_gbp).unwrap();
        sale_gbp.created_at = "2026-07-22T10:00:00.000Z".into();
        sale_gbp.updated_at = "2026-07-22T10:00:00.000Z".into();
        s.create_sale(&sale_gbp).unwrap();
        s.update_sale_status(&sale_gbp.id, SaleStatus::Active)
            .unwrap();
        s.update_sale_status(&sale_gbp.id, SaleStatus::Completed)
            .unwrap();

        let rows = s.weekly_revenue("2026-07-01", "2026-07-31").unwrap();
        // Both sales are in the same week (Mon 2026-07-20 -> Sun 2026-07-26).
        assert_eq!(
            rows.len(),
            2,
            "two currencies in same week should produce 2 rows"
        );
        let currencies: Vec<String> = rows.iter().map(|r| r.currency.clone()).collect();
        assert!(currencies.contains(&"USD".to_string()));
        assert!(currencies.contains(&"GBP".to_string()));
        assert!(rows.iter().all(|r| r.week_start == "2026-07-20"));
    }

    // ── Payment method breakdown ───────────────────────────────────

    #[test]
    fn payment_method_breakdown_groups_by_method() {
        let conn = fresh();
        conn.execute_batch(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, created_at) VALUES
                ('p1', 1000, 'USD', 1, 'completed', 'cash', '2026-07-10T09:00:00Z'),
                ('p2', 2000, 'USD', 1, 'completed', 'card', '2026-07-11T09:00:00Z'),
                ('p3', 500,  'USD', 1, 'completed', 'cash', '2026-07-12T09:00:00Z');",
        )
        .unwrap();
        let rows = store(&conn)
            .payment_method_breakdown("2026-07-01", "2026-07-31")
            .unwrap();
        let cash = rows.iter().find(|r| r.payment_method == "cash").unwrap();
        let card = rows.iter().find(|r| r.payment_method == "card").unwrap();
        assert_eq!(cash.total_minor, 1500);
        assert_eq!(cash.sale_count, 2);
        assert_eq!(card.total_minor, 2000);
        assert_eq!(card.sale_count, 1);
        // Highest revenue first
        assert_eq!(rows[0].payment_method, "card");
    }

    #[test]
    fn payment_method_breakdown_empty() {
        let conn = fresh();
        let rows = store(&conn)
            .payment_method_breakdown("2000-01-01", "2099-12-31")
            .unwrap();
        assert!(rows.is_empty());
    }

    // ── Voided sales summary ───────────────────────────────────────

    #[test]
    fn voided_sales_summary_counts_and_totals() {
        let conn = fresh();
        conn.execute_batch(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at) VALUES
                ('v1', 1000, 'USD', 1, 'voided',    '2026-07-10T09:00:00Z'),
                ('v2', 2500, 'USD', 1, 'voided',    '2026-07-11T09:00:00Z'),
                ('c1', 900,  'USD', 1, 'completed', '2026-07-12T09:00:00Z');",
        )
        .unwrap();
        let row = store(&conn)
            .voided_sales_summary("2026-07-01", "2026-07-31")
            .unwrap();
        assert_eq!(row.void_count, 2);
        assert_eq!(row.void_total_minor, 3500);
    }

    #[test]
    fn voided_sales_summary_empty() {
        let conn = fresh();
        let row = store(&conn)
            .voided_sales_summary("2000-01-01", "2099-12-31")
            .unwrap();
        assert_eq!(row.void_count, 0);
        assert_eq!(row.void_total_minor, 0);
    }

    // ── Voided items ───────────────────────────────────────────────

    #[test]
    fn voided_items_ranks_products_by_qty() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("A", "Alpha", money, None, None, 100, None)
            .unwrap();
        s.create_product("B", "Beta", money, None, None, 100, None)
            .unwrap();
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("A"), 3, price(100)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("B"), 1, price(100)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Voided).unwrap();

        let rows = s.voided_items("2000-01-01", "2099-12-31", 5).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Alpha");
        assert_eq!(rows[0].qty, 3);
        assert_eq!(rows[1].name, "Beta");
    }

    // ── Average basket size ────────────────────────────────────────

    #[test]
    fn avg_basket_size_computes_mean_line_count() {
        let conn = fresh();
        conn.execute_batch(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at) VALUES
                ('b1', 1000, 'USD', 2, 'completed', '2026-07-10T09:00:00Z'),
                ('b2', 2000, 'USD', 4, 'completed', '2026-07-11T09:00:00Z'),
                ('b3', 500,  'USD', 1, 'voided',    '2026-07-12T09:00:00Z');",
        )
        .unwrap();
        let row = store(&conn)
            .avg_basket_size("2026-07-01", "2026-07-31")
            .unwrap();
        assert_eq!(row.sale_count, 2);
        assert!((row.avg_line_count - 3.0).abs() < 1e-9);
    }

    #[test]
    fn avg_basket_size_empty() {
        let conn = fresh();
        let row = store(&conn)
            .avg_basket_size("2000-01-01", "2099-12-31")
            .unwrap();
        assert_eq!(row.sale_count, 0);
        assert_eq!(row.avg_line_count, 0.0);
    }

    // ── Customer split ─────────────────────────────────────────────

    #[test]
    fn customer_split_new_vs_returning() {
        let conn = fresh();
        conn.execute_batch(
            "INSERT INTO customers (id, name, created_at, updated_at) VALUES
                ('cust-returning', 'Returning', '2026-06-01T00:00:00.000Z', '2026-06-01T00:00:00.000Z'),
                ('cust-new',       'New',       '2026-06-01T00:00:00.000Z', '2026-06-01T00:00:00.000Z');
             INSERT INTO sales (id, total_minor, currency, line_count, status, customer_id, created_at) VALUES
                ('old1', 100, 'USD', 1, 'completed', 'cust-returning', '2026-06-01T09:00:00Z'),
                ('r1',   200, 'USD', 1, 'completed', 'cust-returning', '2026-07-10T09:00:00Z'),
                ('n1',   300, 'USD', 1, 'completed', 'cust-new',       '2026-07-11T09:00:00Z'),
                ('n2',   400, 'USD', 1, 'completed', 'cust-new',       '2026-07-12T09:00:00Z'),
                ('anon', 500, 'USD', 1, 'completed', NULL,             '2026-07-13T09:00:00Z');",
        )
        .unwrap();
        let row = store(&conn)
            .customer_split("2026-07-01", "2026-07-31")
            .unwrap();
        // cust-new has no sale before the range -> new; cust-returning does -> returning.
        assert_eq!(row.new_count, 1);
        assert_eq!(row.returning_count, 1);
    }

    #[test]
    fn customer_split_empty() {
        let conn = fresh();
        let row = store(&conn)
            .customer_split("2000-01-01", "2099-12-31")
            .unwrap();
        assert_eq!(row.new_count, 0);
        assert_eq!(row.returning_count, 0);
    }

    // ── Discounts summary ──────────────────────────────────────────

    #[test]
    fn discounts_summary_counts_and_lists_codes() {
        let conn = fresh();
        conn.execute_batch(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, discount_percent, discount_label, created_at) VALUES
                ('d1', 1000, 'USD', 1, 'completed', 10, 'WELCOME10', '2026-07-10T09:00:00Z'),
                ('d2', 2000, 'USD', 1, 'completed', 15, 'WELCOME10', '2026-07-11T09:00:00Z'),
                ('d3', 500,  'USD', 1, 'completed', 5,  'PROMO8.8',  '2026-07-12T09:00:00Z'),
                ('d4', 300,  'USD', 1, 'completed', 0,  NULL,        '2026-07-13T09:00:00Z');",
        )
        .unwrap();
        let row = store(&conn)
            .discounts_summary("2026-07-01", "2026-07-31")
            .unwrap();
        assert_eq!(row.sale_count, 4);
        assert_eq!(row.discounted_sale_count, 3);
        assert!((row.share_percent - 75.0).abs() < 1e-9);
        assert_eq!(row.codes.len(), 2);
        assert_eq!(row.codes[0].label, "WELCOME10");
        assert_eq!(row.codes[0].redeemed_count, 2);
    }

    // ── Inventory turnover + trend ─────────────────────────────────

    #[test]
    fn inventory_turnover_uses_stock_summary_and_range() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("A", "Alpha", money, None, None, 10, None)
            .unwrap();
        s.create_product("B", "Beta", money, None, None, 20, None)
            .unwrap();
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("A"), 3, price(100)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();

        let row = s
            .inventory_turnover(
                "2000-01-01",
                "2099-12-31",
                crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            )
            .unwrap();
        assert_eq!(row.units_sold, 3);
        assert_eq!(row.sku_count, 2);
        assert_eq!(row.range_days, 36525); // 2000-01-01 ..= 2099-12-31 (25 leap years)
        // stock_on_hand reflects the seeded inventory rows for the two products.
        assert_eq!(row.stock_on_hand, 30);
    }

    #[test]
    fn inventory_trend_returns_daily_units() {
        let conn = fresh();
        let s = store(&conn);
        let money = Money {
            minor_units: 100,
            currency: usd(),
        };
        s.create_product("A", "Alpha", money, None, None, 100, None)
            .unwrap();
        // Two completed sales for product A on the same day (7 units total).
        for qty in [2, 5] {
            let mut cart = Cart::new(usd());
            cart.add_line(CartLine::new(Sku::new("A"), qty, price(100)))
                .unwrap();
            let mut sale = Sale::from_cart(&cart).unwrap();
            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            sale.created_at = now.clone();
            sale.updated_at = now;
            s.create_sale(&sale).unwrap();
            s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
            s.update_sale_status(&sale.id, SaleStatus::Completed)
                .unwrap();
        }
        let rows = s.inventory_trend("2000-01-01", "2099-12-31").unwrap();
        // Both sales share the same day -> one row with 7 units.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].units_sold, 7);
    }

    // ── Table turnover ────────────────────────────────────────────

    #[test]
    fn table_turnover_counts_completed_table_orders() {
        let conn = fresh();
        let s = store(&conn);
        let t1 = seed_completed_sale(&conn, "STEAK", 1, 12000);
        let t2 = seed_completed_sale(&conn, "PASTA", 2, 9000);
        let takeaway = seed_completed_sale(&conn, "SANDWICH", 1, 5000);
        // Two table-service orders with a table number, one takeaway without.
        for sale_id in [&t1, &t2] {
            s.create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id.clone(),
                store_id: None,
                items_summary: "x".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
                table_number: Some("T5".into()),
                priority: false,
            })
            .unwrap();
        }
        s.create_kds_order(CreateKdsOrderInput {
            sale_id: takeaway.clone(),
            store_id: None,
            items_summary: "x".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

        let rows = s.table_turnover("2000-01-01", "2099-12-31").unwrap();
        // Both table orders share today's date -> one row counting 2 turns;
        // the takeaway order is excluded.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].table_orders, 2);
    }

    #[test]
    fn hourly_table_activity_groups_completed_table_orders_by_hour() {
        let conn = fresh();
        let s = store(&conn);
        let t1 = seed_completed_sale(&conn, "STEAK", 1, 12000);
        let t2 = seed_completed_sale(&conn, "PASTA", 2, 9000);
        let t3 = seed_completed_sale(&conn, "SANDWICH", 1, 5000);
        // Two table-service orders at distinct hours, one takeaway without a
        // table number that must be excluded.
        for (sale_id, at) in [
            (&t1, "2026-01-01T08:30:00.000Z"),
            (&t2, "2026-01-01T12:45:00.000Z"),
        ] {
            s.create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id.clone(),
                store_id: None,
                items_summary: "x".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
                table_number: Some("T5".into()),
                priority: false,
            })
            .unwrap();
            // Pin the completion hour so the grouping is deterministic.
            conn.execute(
                "UPDATE sales SET created_at = ?1 WHERE id = ?2",
                params![at, sale_id],
            )
            .unwrap();
        }
        s.create_kds_order(CreateKdsOrderInput {
            sale_id: t3.clone(),
            store_id: None,
            items_summary: "x".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
            table_number: None,
            priority: false,
        })
        .unwrap();

        let rows = s.hourly_table_activity("2000-01-01", "2099-12-31").unwrap();
        // Two table orders at hours 8 and 12 -> two rows; the takeaway is
        // excluded entirely.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].hour, 8);
        assert_eq!(rows[0].table_orders, 1);
        assert_eq!(rows[1].hour, 12);
        assert_eq!(rows[1].table_orders, 1);
    }

    #[test]
    fn basket_size_trend_groups_daily_averages() {
        let conn = fresh();
        let s = store(&conn);
        let t1 = seed_completed_sale(&conn, "STEAK", 1, 12000);
        let t2 = seed_completed_sale(&conn, "PASTA", 1, 9000);
        // A two-line sale so the average can exceed 1.
        s.create_product("SIDE", "SIDE", price(4000), None, None, 100, None)
            .unwrap();
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("STEAK"), 1, price(12000)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("SIDE"), 1, price(4000)))
            .unwrap();
        let mut sale = Sale::from_cart(&cart).unwrap();
        sale.created_at = "2026-01-02T12:00:00.000Z".into();
        sale.updated_at = sale.created_at.clone();
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Active).unwrap();
        s.update_sale_status(&sale.id, SaleStatus::Completed)
            .unwrap();
        let t3 = sale.id;
        // Pin the first two single-line sales to the day before.
        for (id, at) in [
            (&t1, "2026-01-01T08:00:00.000Z"),
            (&t2, "2026-01-01T10:00:00.000Z"),
        ] {
            conn.execute(
                "UPDATE sales SET created_at = ?1 WHERE id = ?2",
                params![at, id],
            )
            .unwrap();
        }

        let rows = s.basket_size_trend("2000-01-01", "2099-12-31").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2026-01-01");
        assert_eq!(rows[0].sale_count, 2);
        assert_eq!(rows[0].avg_line_count, 1.0);
        assert_eq!(rows[1].date, "2026-01-02");
        assert_eq!(rows[1].sale_count, 1);
        assert_eq!(rows[1].avg_line_count, 2.0);
        let _ = t3;
    }
}
