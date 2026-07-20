//! Unified analytics export — collect all report data into a single JSON-bundle.
//!
//! [`Store::export_analytics_bundle`] runs every report query defined in
//! [`crate::db::reports`] (daily/weekly/monthly revenue, top products, hourly
//! heatmap, category breakdown, low-stock alerts, active stock alerts) and
//! packages them together with export metadata into a serializable
//! [`AnalyticsBundle`].

use serde::{Deserialize, Serialize};

use crate::db::Store;
use crate::db::reports::{
    CategoryBreakdownRow, DailyRevenueRow, HourlyHeatmapRow, LowStockAlert, MonthlyRevenueRow,
    StockAlertEvent, TopProductRow, WeeklyRevenueRow,
};
use crate::error::CoreError;

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
}
