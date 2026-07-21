//! Unified analytics export — collect all report data into a single JSON-bundle.
//!
//! [`Store::export_analytics_bundle`] runs every report query defined in
//! [`crate::db::reports`] (daily/weekly/monthly revenue, top products, hourly
//! heatmap, category breakdown, low-stock alerts, active stock alerts) and
//! packages them together with export metadata into a serializable
//! [`AnalyticsBundle`].

pub mod email_report;

use serde::{Deserialize, Serialize};

use crate::db::Store;
use crate::db::reports::{
    CategoryBreakdownRow, DailyRevenueRow, HourlyHeatmapRow, LowStockAlert, MonthlyRevenueRow,
    StockAlertEvent, TopProductRow, WeeklyRevenueRow,
};
use crate::error::CoreError;

/// Column whitelist entries for custom report datasets.
type ColumnWhitelist = &'static [(&'static str, &'static str)];

/// Dataset definition for the custom report builder.
struct DatasetDef {
    table: &'static str,
    columns: ColumnWhitelist,
    has_date_filter: bool,
}

/// Request payload for the custom report builder.
///
/// The backend validates `columns` against a per-dataset whitelist to prevent
/// SQL injection — only columns listed in the whitelist are included in the query.
#[derive(Debug, Clone, Deserialize)]
pub struct CustomReportRequest {
    /// Dataset key ("sales" or "inventory").
    pub dataset: String,
    /// Column names the user wants to see (whitelist-filtered).
    pub columns: Vec<String>,
    /// Optional ISO-8601 start date for date-filterable datasets.
    pub start_date: Option<String>,
    /// Optional ISO-8601 end date for date-filterable datasets.
    pub end_date: Option<String>,
}

/// Response from the custom report builder — a generic grid suitable for
/// table rendering and CSV export.
#[derive(Debug, Clone, Serialize)]
pub struct CustomReportResponse {
    /// Column headers in display order.
    pub columns: Vec<String>,
    /// Row data — each inner vec matches the length of `columns`.
    pub rows: Vec<Vec<String>>,
}

/// Convert a rusqlite Value to its string representation.
fn value_to_string(val: rusqlite::types::Value) -> String {
    match val {
        rusqlite::types::Value::Null => String::new(),
        rusqlite::types::Value::Integer(i) => i.to_string(),
        rusqlite::types::Value::Real(f) => f.to_string(),
        rusqlite::types::Value::Text(s) => s,
        rusqlite::types::Value::Blob(b) => format!("<{} bytes>", b.len()),
    }
}

/// Metadata stamped onto every analytics export.
#[derive(Debug, Clone, Serialize)]
pub struct ExportMetadata {
    /// ISO-8601 timestamp of the export generation.
    pub exported_at: String,
    /// Tenant ID (empty string for single-tenant deployments).
    pub tenant_id: String,
    /// Store profile name.
    pub store_name: String,
    /// Version of OZ-POS that generated this export.
    pub version: String,
}

/// A complete analytics bundle containing every report type plus metadata.
///
/// This is the output of [`Store::export_analytics_bundle`]. Serialize to
/// JSON for consumption by external analytics platforms (BigQuery,
/// Snowflake, custom BI tools) or to NDJSON for streaming ingestion.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsBundle {
    /// Export metadata (timestamp, tenant, store, version).
    pub metadata: ExportMetadata,
    /// Daily revenue rows for the requested date range.
    pub daily_revenue: Vec<DailyRevenueRow>,
    /// Weekly revenue rows for the requested date range.
    pub weekly_revenue: Vec<WeeklyRevenueRow>,
    /// Monthly revenue rows for the requested date range.
    pub monthly_revenue: Vec<MonthlyRevenueRow>,
    /// Top products ranked by revenue (default limit: 25).
    pub top_products: Vec<TopProductRow>,
    /// Hourly heatmap — day-of-week × hour cross-tab of revenue and sale count.
    pub hourly_heatmap: Vec<HourlyHeatmapRow>,
    /// Revenue breakdown by product category with percentage.
    pub category_breakdown: Vec<CategoryBreakdownRow>,
    /// Products at or below threshold at the default location.
    pub low_stock_alerts: Vec<LowStockAlert>,
    /// Active (non-resolved) stock alert events at the default location.
    pub active_stock_alerts: Vec<StockAlertEvent>,
}

/// Configuration knobs for the analytics export.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Start of the date range (inclusive, ISO-8601 YYYY-MM-DD).
    pub start_date: String,
    /// End of the date range (inclusive, ISO-8601 YYYY-MM-DD).
    pub end_date: String,
    /// Maximum number of top products to include.
    pub top_product_limit: i64,
    /// Low-stock threshold for the default location.
    pub low_stock_threshold: i64,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            start_date: "2000-01-01".to_string(),
            end_date: "2099-12-31".to_string(),
            top_product_limit: 25,
            low_stock_threshold: 10,
        }
    }
}

impl Store<'_> {
    /// Export a complete analytics bundle across all report types.
    ///
    /// # Arguments
    ///
    /// * `config` — date range, limit, and threshold knobs.
    /// * `tenant_id` — tenant identifier (empty string for standalone).
    /// * `store_name` — human-readable store name for the metadata header.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] if any of the underlying report queries fail.
    pub fn export_analytics_bundle(
        &self,
        config: ExportConfig,
        tenant_id: &str,
        store_name: &str,
    ) -> Result<AnalyticsBundle, CoreError> {
        let daily_revenue = self.daily_revenue(&config.start_date, &config.end_date)?;
        let weekly_revenue = self.weekly_revenue(&config.start_date, &config.end_date)?;
        let monthly_revenue = self.monthly_revenue(&config.start_date, &config.end_date)?;
        let top_products = self.top_products(
            &config.start_date,
            &config.end_date,
            config.top_product_limit,
        )?;
        let hourly_heatmap = self.hourly_heatmap(&config.start_date, &config.end_date)?;
        let category_breakdown = self.category_breakdown(&config.start_date, &config.end_date)?;
        let low_stock_alerts = self.low_stock_alerts_at_location(
            crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            config.low_stock_threshold,
        )?;
        let active_stock_alerts =
            self.active_stock_alerts(crate::inventory::CANONICAL_DEFAULT_LOCATION_UUID)?;

        let exported_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        Ok(AnalyticsBundle {
            metadata: ExportMetadata {
                exported_at,
                tenant_id: tenant_id.to_string(),
                store_name: store_name.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            daily_revenue,
            weekly_revenue,
            monthly_revenue,
            top_products,
            hourly_heatmap,
            category_breakdown,
            low_stock_alerts,
            active_stock_alerts,
        })
    }
}

/// Scheduled report delivery configuration.
///
/// Persisted in the `settings` table under key `report_schedule` as JSON.
/// When email/SMTP infrastructure is wired in, a background task reads
/// this config and sends analytics bundles on the configured cadence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportScheduleConfig {
    /// Whether scheduled delivery is enabled.
    pub enabled: bool,
    /// Cron-style cadence: "daily", "weekly", "monthly", or a cron expression.
    pub cadence: String,
    /// Report types to include in the delivery.
    pub report_types: Vec<String>,
    /// Recipient email addresses.
    pub recipients: Vec<String>,
    /// ISO-8601 time of day to send (e.g. "08:00" for 8 AM).
    pub send_at_time: String,
    /// Timezone for scheduling (e.g. "Asia/Jakarta").
    pub timezone: String,
    /// Date range window in days (e.g. 7 for last week's data).
    pub lookback_days: u32,
}

impl Default for ReportScheduleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cadence: "daily".to_string(),
            report_types: vec!["daily_revenue".to_string(), "top_products".to_string()],
            recipients: Vec::new(),
            send_at_time: "08:00".to_string(),
            timezone: "UTC".to_string(),
            lookback_days: 1,
        }
    }
}

/// Settings key used to persist the report schedule.
pub const REPORT_SCHEDULE_SETTINGS_KEY: &str = "report_schedule";

/// CSV escape — wraps a cell in quotes and escapes internal quotes.
fn csv_cell(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Write a CSV row from an iterator of cells.
fn csv_row(cells: impl Iterator<Item = String>) -> String {
    cells.map(|c| csv_cell(&c)).collect::<Vec<_>>().join(",")
}

/// Write analytics data as CSV files and a metadata.json to the given directory.
///
/// Creates one `.csv` file per report type plus a `metadata.json` file.
/// Returns the list of file paths written. Existing files are overwritten.
///
/// # Example output
///
/// ```text
/// exports/2026-07-20/
///   metadata.json
///   daily_revenue.csv
///   weekly_revenue.csv
///   monthly_revenue.csv
///   top_products.csv
///   hourly_heatmap.csv
///   category_breakdown.csv
///   low_stock_alerts.csv
///   active_stock_alerts.csv
/// ```
pub fn write_analytics_bundle_csv(
    bundle: &AnalyticsBundle,
    dir: &str,
) -> Result<Vec<String>, CoreError> {
    use std::fs;
    use std::path::Path;

    let root = Path::new(dir);
    fs::create_dir_all(root).map_err(|e| {
        CoreError::Internal(format!("failed to create export directory {dir}: {e}"))
    })?;

    let mut files: Vec<String> = Vec::new();

    let write = |name: &str, content: &str| -> Result<String, CoreError> {
        let path = root.join(name);
        fs::write(&path, content)
            .map_err(|e| CoreError::Internal(format!("failed to write {name}: {e}")))?;
        Ok(path.to_string_lossy().to_string())
    };

    // metadata.json
    let meta_json = serde_json::to_string_pretty(&bundle.metadata)
        .map_err(|e| CoreError::Internal(format!("failed to serialize metadata: {e}")))?;
    files.push(write("metadata.json", &meta_json)?);

    // Daily revenue
    if !bundle.daily_revenue.is_empty() {
        let mut csv = String::from("date,total_minor,currency,sale_count\n");
        for r in &bundle.daily_revenue {
            csv.push_str(&csv_row(
                [
                    r.date.clone(),
                    r.total_minor.to_string(),
                    r.currency.clone(),
                    r.sale_count.to_string(),
                ]
                .into_iter(),
            ));
            csv.push('\n');
        }
        files.push(write("daily_revenue.csv", &csv)?);
    }

    // Weekly revenue
    if !bundle.weekly_revenue.is_empty() {
        let mut csv = String::from("week_start,total_minor,currency,sale_count\n");
        for r in &bundle.weekly_revenue {
            csv.push_str(&csv_row(
                [
                    r.week_start.clone(),
                    r.total_minor.to_string(),
                    r.currency.clone(),
                    r.sale_count.to_string(),
                ]
                .into_iter(),
            ));
            csv.push('\n');
        }
        files.push(write("weekly_revenue.csv", &csv)?);
    }

    // Monthly revenue
    if !bundle.monthly_revenue.is_empty() {
        let mut csv = String::from("month,total_minor,currency,sale_count\n");
        for r in &bundle.monthly_revenue {
            csv.push_str(&csv_row(
                [
                    r.month.clone(),
                    r.total_minor.to_string(),
                    r.currency.clone(),
                    r.sale_count.to_string(),
                ]
                .into_iter(),
            ));
            csv.push('\n');
        }
        files.push(write("monthly_revenue.csv", &csv)?);
    }

    // Top products
    if !bundle.top_products.is_empty() {
        let mut csv = String::from("sku,name,total_qty,total_minor\n");
        for r in &bundle.top_products {
            csv.push_str(&csv_row(
                [
                    r.sku.clone(),
                    r.name.clone(),
                    r.total_qty.to_string(),
                    r.total_minor.to_string(),
                ]
                .into_iter(),
            ));
            csv.push('\n');
        }
        files.push(write("top_products.csv", &csv)?);
    }

    // Hourly heatmap
    if !bundle.hourly_heatmap.is_empty() {
        let mut csv = String::from("day_of_week,hour,total_minor,sale_count\n");
        for r in &bundle.hourly_heatmap {
            csv.push_str(&csv_row(
                [
                    r.day_of_week.to_string(),
                    r.hour.to_string(),
                    r.total_minor.to_string(),
                    r.sale_count.to_string(),
                ]
                .into_iter(),
            ));
            csv.push('\n');
        }
        files.push(write("hourly_heatmap.csv", &csv)?);
    }

    // Category breakdown
    if !bundle.category_breakdown.is_empty() {
        let mut csv = String::from("category_name,total_minor,sale_count,percentage\n");
        for r in &bundle.category_breakdown {
            csv.push_str(&csv_row(
                [
                    r.category_name.clone(),
                    r.total_minor.to_string(),
                    r.sale_count.to_string(),
                    format!("{:.1}", r.percentage),
                ]
                .into_iter(),
            ));
            csv.push('\n');
        }
        files.push(write("category_breakdown.csv", &csv)?);
    }

    // Low stock alerts
    if !bundle.low_stock_alerts.is_empty() {
        let mut csv = String::from("product_id,sku,name,current_qty,threshold\n");
        for r in &bundle.low_stock_alerts {
            csv.push_str(&csv_row(
                [
                    r.product_id.clone(),
                    r.sku.clone(),
                    r.name.clone(),
                    r.current_qty.to_string(),
                    r.threshold.to_string(),
                ]
                .into_iter(),
            ));
            csv.push('\n');
        }
        files.push(write("low_stock_alerts.csv", &csv)?);
    }

    // Active stock alerts
    if !bundle.active_stock_alerts.is_empty() {
        let mut csv = String::from(
            "id,threshold_id,product_id,location_id,current_qty,threshold,status,triggered_at,product_sku,product_name\n",
        );
        for r in &bundle.active_stock_alerts {
            csv.push_str(&csv_row(
                [
                    r.id.clone(),
                    r.threshold_id.clone(),
                    r.product_id.clone(),
                    r.location_id.clone(),
                    r.current_qty.to_string(),
                    r.threshold.to_string(),
                    r.status.clone(),
                    r.triggered_at.clone(),
                    r.product_sku.clone(),
                    r.product_name.clone(),
                ]
                .into_iter(),
            ));
            csv.push('\n');
        }
        files.push(write("active_stock_alerts.csv", &csv)?);
    }

    Ok(files)
}

impl Store<'_> {
    /// Save the report schedule configuration to the settings table.
    pub fn save_report_schedule(&self, config: &ReportScheduleConfig) -> Result<(), CoreError> {
        let json = serde_json::to_string(config).map_err(|e| {
            CoreError::Internal(format!("failed to serialize report schedule: {e}"))
        })?;
        self.set_setting(REPORT_SCHEDULE_SETTINGS_KEY, &json)
    }

    /// Load the report schedule configuration from the settings table.
    /// Returns `None` if no schedule has been saved yet.
    pub fn get_report_schedule(&self) -> Result<Option<ReportScheduleConfig>, CoreError> {
        let raw = match self.get_setting(REPORT_SCHEDULE_SETTINGS_KEY)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let config: ReportScheduleConfig = serde_json::from_str(&raw).map_err(|e| {
            CoreError::Internal(format!("failed to deserialize report schedule: {e}"))
        })?;
        Ok(Some(config))
    }

    /// Build a custom report from user-selected columns and filters.
    ///
    /// Column names are validated against a per-dataset whitelist — unrecognized
    /// columns are silently dropped. This prevents SQL injection while allowing
    /// flexible column selection from predefined options.
    ///
    /// # Supported datasets
    ///
    /// | Key | Table | Date filter |
    /// |-----|-------|-------------|
    /// | `sales` | `sales` | `created_at` |
    /// | `inventory` | `products` | none |
    pub fn build_custom_report(
        &self,
        req: CustomReportRequest,
    ) -> Result<CustomReportResponse, CoreError> {
        let dataset = Self::get_dataset_def(&req.dataset)?;

        // Filter requested columns through the whitelist
        let safe_cols: Vec<&str> = req
            .columns
            .iter()
            .filter_map(|c| {
                dataset
                    .columns
                    .iter()
                    .find(|(col_name, _)| col_name == c)
                    .map(|(col_name, _)| *col_name)
            })
            .collect();

        if safe_cols.is_empty() {
            return Ok(CustomReportResponse {
                columns: Vec::new(),
                rows: Vec::new(),
            });
        }

        // Build safe SQL — column names come from our whitelist, table name
        // from our dataset definitions, both hardcoded and validated above.
        // Date values are parameterized to prevent SQL injection.
        let cols_sql = safe_cols.join(", ");
        let mut sql = format!("SELECT {} FROM {}", cols_sql, dataset.table);
        let mut params: Vec<String> = Vec::new();

        if dataset.has_date_filter {
            if req.start_date.is_some() {
                sql.push_str(" WHERE created_at >= ?1");
                params.push(req.start_date.clone().unwrap());
            }
            if req.end_date.is_some() {
                let param_idx = params.len() + 1;
                let where_clause = if req.start_date.is_some() {
                    " AND"
                } else {
                    " WHERE"
                };
                sql.push_str(&format!("{} created_at <= ?{}", where_clause, param_idx));
                params.push(format!("{} 23:59:59", req.end_date.clone().unwrap()));
            }
        }

        let mut stmt = self.conn.prepare(&sql).map_err(|e| {
            CoreError::Internal(format!("failed to prepare custom report query: {e}"))
        })?;

        let col_count = stmt.column_count();

        // Convert params to rusqlite-compatible references
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let mut row_data = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let val: rusqlite::types::Value = row.get(i)?;
                    row_data.push(value_to_string(val));
                }
                Ok(row_data)
            })
            .map_err(|e| CoreError::Internal(format!("failed to query custom report: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                CoreError::Internal(format!("failed to collect custom report rows: {e}"))
            })?;

        Ok(CustomReportResponse {
            columns: safe_cols.iter().map(|&s| s.to_string()).collect(),
            rows,
        })
    }

    /// Look up a dataset definition by key.
    fn get_dataset_def(key: &str) -> Result<DatasetDef, CoreError> {
        match key {
            "sales" => Ok(DatasetDef {
                table: "sales",
                columns: &[
                    ("id", "Sale ID"),
                    ("total_minor", "Total (minor)"),
                    ("created_at", "Created"),
                    ("status", "Status"),
                    ("customer_id", "Customer ID"),
                ],
                has_date_filter: true,
            }),
            "inventory" => Ok(DatasetDef {
                table: "products",
                columns: &[
                    ("sku", "SKU"),
                    ("name", "Name"),
                    ("price_minor", "Price (minor)"),
                    ("category_id", "Category ID"),
                    ("barcode", "Barcode"),
                ],
                has_date_filter: false,
            }),
            _ => Err(CoreError::Validation {
                field: "dataset",
                message: format!("unknown dataset '{key}'. Supported: sales, inventory"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Store;
    use crate::migrations;
    use crate::{Cart, CartLine, Money, Sale, SaleStatus, Sku};

    fn usd() -> crate::Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    fn seed_sale(conn: &rusqlite::Connection, sku: &str, qty: i64, unit_minor: i64) {
        let s = Store::new(conn);
        s.create_product(sku, sku, price(unit_minor), None, None, 100, None)
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
    }

    #[test]
    fn analytics_bundle_empty_db() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);
        let bundle = s
            .export_analytics_bundle(ExportConfig::default(), "test-tenant", "Test Store")
            .unwrap();
        assert_eq!(bundle.metadata.tenant_id, "test-tenant");
        assert_eq!(bundle.metadata.store_name, "Test Store");
        assert!(!bundle.metadata.exported_at.is_empty());
        assert!(!bundle.metadata.version.is_empty());
        assert!(bundle.daily_revenue.is_empty());
        assert!(bundle.weekly_revenue.is_empty());
        assert!(bundle.monthly_revenue.is_empty());
        assert!(bundle.top_products.is_empty());
        assert!(bundle.hourly_heatmap.is_empty());
        assert!(bundle.category_breakdown.is_empty());
        assert!(bundle.low_stock_alerts.is_empty());
        assert!(bundle.active_stock_alerts.is_empty());
    }

    #[test]
    fn analytics_bundle_with_data() {
        let conn = migrations::fresh_db();
        seed_sale(&conn, "COFFEE", 2, 350);
        seed_sale(&conn, "BAGEL", 1, 450);

        let s = Store::new(&conn);
        let bundle = s
            .export_analytics_bundle(ExportConfig::default(), "", "My Store")
            .unwrap();

        assert_eq!(bundle.metadata.store_name, "My Store");
        assert_eq!(bundle.daily_revenue.len(), 1);
        assert_eq!(bundle.daily_revenue[0].total_minor, 1150);
        assert_eq!(bundle.daily_revenue[0].sale_count, 2);
        assert!(!bundle.weekly_revenue.is_empty());
        assert!(!bundle.monthly_revenue.is_empty());
        assert_eq!(bundle.top_products.len(), 2);
        assert!(!bundle.hourly_heatmap.is_empty());
        assert!(!bundle.category_breakdown.is_empty());
    }

    #[test]
    fn analytics_bundle_serializable() {
        let conn = migrations::fresh_db();
        seed_sale(&conn, "TEA", 1, 200);

        let s = Store::new(&conn);
        let bundle = s
            .export_analytics_bundle(ExportConfig::default(), "t1", "S1")
            .unwrap();

        let json = serde_json::to_string_pretty(&bundle).unwrap();
        assert!(json.contains("\"tenant_id\": \"t1\""));
        assert!(json.contains("\"store_name\": \"S1\""));
        assert!(json.contains("\"daily_revenue\""));
        assert!(json.contains("\"top_products\""));
        assert!(json.contains("\"hourly_heatmap\""));
        assert!(json.contains("\"category_breakdown\""));
        assert!(json.contains("\"low_stock_alerts\""));
        assert!(json.contains("\"active_stock_alerts\""));
        assert!(json.contains("\"exported_at\""));
        assert!(json.contains("\"version\""));
    }

    #[test]
    fn analytics_bundle_respects_date_range() {
        let conn = migrations::fresh_db();
        seed_sale(&conn, "LATTE", 1, 400);

        let s = Store::new(&conn);
        let bundle = s
            .export_analytics_bundle(
                ExportConfig {
                    start_date: "2000-01-01".into(),
                    end_date: "2000-01-31".into(),
                    ..ExportConfig::default()
                },
                "",
                "",
            )
            .unwrap();

        // The sale was created today, which is outside the 2000 date range.
        assert!(bundle.daily_revenue.is_empty());
    }

    #[test]
    fn analytics_bundle_respects_top_product_limit() {
        let conn = migrations::fresh_db();
        seed_sale(&conn, "A", 1, 100);
        seed_sale(&conn, "B", 1, 200);
        seed_sale(&conn, "C", 1, 300);

        let s = Store::new(&conn);
        let config = ExportConfig {
            top_product_limit: 2,
            ..ExportConfig::default()
        };
        let bundle = s.export_analytics_bundle(config, "", "").unwrap();

        assert_eq!(bundle.top_products.len(), 2);
        assert_eq!(bundle.top_products[0].sku, "C");
        assert_eq!(bundle.top_products[1].sku, "B");
    }

    #[test]
    fn export_config_defaults() {
        let cfg = ExportConfig::default();
        assert_eq!(cfg.start_date, "2000-01-01");
        assert_eq!(cfg.end_date, "2099-12-31");
        assert_eq!(cfg.top_product_limit, 25);
        assert_eq!(cfg.low_stock_threshold, 10);
    }

    // ── Report schedule ────────────────────────────────────────────

    #[test]
    fn schedule_config_defaults() {
        let cfg = ReportScheduleConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.cadence, "daily");
        assert_eq!(cfg.report_types.len(), 2);
        assert!(cfg.recipients.is_empty());
        assert_eq!(cfg.send_at_time, "08:00");
        assert_eq!(cfg.timezone, "UTC");
        assert_eq!(cfg.lookback_days, 1);
    }

    #[test]
    fn schedule_save_and_load() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);

        // Initially no schedule
        let loaded = s.get_report_schedule().unwrap();
        assert!(loaded.is_none());

        // Save a schedule
        let cfg = ReportScheduleConfig {
            enabled: true,
            cadence: "weekly".to_string(),
            report_types: vec![
                "daily_revenue".to_string(),
                "top_products".to_string(),
                "hourly_heatmap".to_string(),
            ],
            recipients: vec!["owner@store.com".to_string()],
            send_at_time: "06:00".to_string(),
            timezone: "Asia/Jakarta".to_string(),
            lookback_days: 7,
        };
        s.save_report_schedule(&cfg).unwrap();

        // Load and verify
        let loaded = s.get_report_schedule().unwrap().unwrap();
        assert!(loaded.enabled);
        assert_eq!(loaded.cadence, "weekly");
        assert_eq!(loaded.report_types.len(), 3);
        assert_eq!(loaded.recipients, vec!["owner@store.com"]);
        assert_eq!(loaded.send_at_time, "06:00");
        assert_eq!(loaded.timezone, "Asia/Jakarta");
        assert_eq!(loaded.lookback_days, 7);
    }

    #[test]
    fn schedule_serde_roundtrip() {
        let cfg = ReportScheduleConfig {
            enabled: true,
            cadence: "monthly".to_string(),
            report_types: vec!["daily_revenue".to_string()],
            recipients: vec!["a@b.com".to_string(), "c@d.com".to_string()],
            send_at_time: "09:00".to_string(),
            timezone: "America/New_York".to_string(),
            lookback_days: 30,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: ReportScheduleConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cadence, cfg.cadence);
        assert_eq!(back.recipients, cfg.recipients);
        assert_eq!(back.lookback_days, cfg.lookback_days);
    }

    // ── Custom report builder ──────────────────────────────────────

    #[test]
    fn custom_report_unknown_dataset() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);
        let req = CustomReportRequest {
            dataset: "nonexistent".to_string(),
            columns: vec!["id".to_string()],
            start_date: None,
            end_date: None,
        };
        let err = s.build_custom_report(req).unwrap_err();
        assert!(
            format!("{err}").contains("unknown dataset")
                || format!("{err}").contains("validation error"),
            "got: {err}"
        );
    }

    #[test]
    fn custom_report_invalid_columns_filtered() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);
        // Request includes a column that's not in the whitelist — it gets silently dropped.
        let req = CustomReportRequest {
            dataset: "sales".to_string(),
            columns: vec!["id".to_string(), "password_hash".to_string()],
            start_date: None,
            end_date: None,
        };
        let resp = s.build_custom_report(req).unwrap();
        // Only "id" is in the whitelist
        assert_eq!(resp.columns, vec!["id"]);
    }

    #[test]
    fn custom_report_sales_basic() {
        let conn = migrations::fresh_db();
        seed_sale(&conn, "A", 1, 100);
        seed_sale(&conn, "B", 2, 200);

        let s = Store::new(&conn);
        let req = CustomReportRequest {
            dataset: "sales".to_string(),
            columns: vec![
                "id".to_string(),
                "total_minor".to_string(),
                "status".to_string(),
            ],
            start_date: None,
            end_date: None,
        };
        let resp = s.build_custom_report(req).unwrap();
        assert_eq!(resp.columns.len(), 3);
        assert_eq!(resp.rows.len(), 2);
        // Each row has 3 columns
        assert!(resp.rows.iter().all(|r| r.len() == 3));
    }

    #[test]
    fn custom_report_inventory_columns() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);
        let req = CustomReportRequest {
            dataset: "inventory".to_string(),
            columns: vec![
                "sku".to_string(),
                "name".to_string(),
                "price_minor".to_string(),
            ],
            start_date: None,
            end_date: None,
        };
        let resp = s.build_custom_report(req).unwrap();
        assert_eq!(resp.columns.len(), 3);
        // All three columns must be present header order
        assert_eq!(resp.columns[0], "sku");
        assert_eq!(resp.columns[1], "name");
        assert_eq!(resp.columns[2], "price_minor");
    }

    #[test]
    fn custom_report_empty_columns_returns_empty() {
        let conn = migrations::fresh_db();
        seed_sale(&conn, "X", 1, 50);
        let s = Store::new(&conn);
        let req = CustomReportRequest {
            dataset: "sales".to_string(),
            columns: vec![],
            start_date: None,
            end_date: None,
        };
        let resp = s.build_custom_report(req).unwrap();
        assert!(resp.columns.is_empty());
        assert!(resp.rows.is_empty());
    }

    // ── CSV export ─────────────────────────────────────────────────

    #[test]
    fn csv_export_creates_files() {
        let conn = migrations::fresh_db();
        seed_sale(&conn, "LATTE", 1, 400);

        let s = Store::new(&conn);
        let bundle = s
            .export_analytics_bundle(ExportConfig::default(), "t1", "S1")
            .unwrap();

        let tmp = std::env::temp_dir().join("oz-pos-test-csv");
        let files = write_analytics_bundle_csv(&bundle, tmp.to_str().unwrap()).unwrap();

        // Should have created at least metadata.json + daily_revenue.csv + top_products.csv + heatmap + categories
        assert!(files.iter().any(|f| f.ends_with("metadata.json")));
        assert!(files.iter().any(|f| f.ends_with("daily_revenue.csv")));
        assert!(files.iter().any(|f| f.ends_with("top_products.csv")));
    }

    #[test]
    fn csv_export_empty_bundle_writes_metadata_only() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);
        let bundle = s
            .export_analytics_bundle(ExportConfig::default(), "", "")
            .unwrap();

        let tmp = std::env::temp_dir().join("oz-pos-test-csv-empty");
        let files = write_analytics_bundle_csv(&bundle, tmp.to_str().unwrap()).unwrap();

        // Only metadata.json should be written (bundle is empty)
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("metadata.json"));
    }

    #[test]
    fn csv_cell_escaping() {
        assert_eq!(csv_cell("hello"), "hello");
        assert_eq!(csv_cell("hello, world"), "\"hello, world\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
