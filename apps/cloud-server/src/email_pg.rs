//! Postgres backend for the scheduled report-sender email loop.
//!
//! Phase 1.5 of `unify-auth-and-sync.md`: the SQLite loop in [`crate::email`]
//! reads its config from the synchronous `rusqlite` `Store`; this module is
//! the parallel async Postgres implementation used on the cloud branch. It
//! covers the exact surface the loop touches:
//!
//! - the `settings` table (`smtp_config`, `report_schedule`,
//!   `last_report_sent_at`, `store.name`)
//! - the analytics bundle (`export_analytics_bundle_pg`) — the ten report
//!   queries ported from `oz_core::db::reports` / `oz_core::db::popularity`
//!
//! The pure-Rust scheduling / filtering / formatting logic is reused from
//! `oz_core` (`should_send_scheduled_with_last_sent`,
//! `filter_analytics_bundle`, `ReportEmailBuilder::build`) so the SQLite and
//! Postgres loops can never drift apart in cadence, dedup, or layout.
//!
//! # Date handling
//!
//! Both schemas store `created_at` as ISO-8601 UTC text. Postgres casts that
//! text directly (`created_at::date`, `created_at::timestamp`) and reuses the
//! same `YYYY-MM-DD` / `YYYY-MM` string shapes the SQLite queries produced.

use std::time::Duration;

use chrono::{NaiveDate, SecondsFormat, Utc};
use deadpool_postgres::Pool;
use tracing::{error, info};

use oz_core::db::popularity::{
    CategoryForecastRow, CategoryPopularityRow, CategoryTopProduct, CategoryTrendPoint,
};
use oz_core::db::reports::{
    CategoryBreakdownRow, DailyRevenueRow, HourlyHeatmapRow, LowStockAlert, MonthlyRevenueRow,
    StockAlertEvent, TopProductRow, WeeklyRevenueRow,
};
use oz_core::export::email_report::{ReportEmailBuilder, SMTP_CONFIG_SETTINGS_KEY, SmtpConfig};
use oz_core::export::email_sender::{
    LAST_SENT_KEY, filter_analytics_bundle, should_send_scheduled_with_last_sent,
};
use oz_core::export::{
    AnalyticsBundle, ExportConfig, ExportMetadata, REPORT_SCHEDULE_SETTINGS_KEY,
    ReportScheduleConfig,
};
use oz_core::popularity::{linear_forecast, score_from_raw, seasonal_daily_forecast};

use crate::email::send_email;

// ── Background scheduled send loop (Postgres) ──────────────────────

/// Start the background task that polls every 60s and sends scheduled
/// report emails on the Postgres branch.
pub fn start_report_sender_loop_pg(pool: Pool) {
    tokio::spawn(async move {
        info!("Report sender background loop started (Postgres, poll interval: 60s)");

        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;

            if let Err(e) = try_send_scheduled_pg(&pool).await {
                error!("Report sender loop error (Postgres): {e}");
            }
        }
    });
}

/// Try to send a scheduled report — the Postgres mirror of
/// [`crate::email::try_send_scheduled`]: read config, check the schedule
/// (cadence + timezone + dedup via the shared scheduler), generate the
/// filtered report, send, and record the send timestamp.
async fn try_send_scheduled_pg(pool: &Pool) -> Result<(), String> {
    // Scope 1: Read SMTP + schedule config, check schedule.
    let smtp_config = match get_smtp_config_pg(pool).await? {
        Some(c) => c,
        None => return Ok(()),
    };

    let schedule = match get_report_schedule_pg(pool).await? {
        Some(s) if s.enabled => s,
        _ => return Ok(()),
    };

    let last_sent = get_setting_pg(pool, LAST_SENT_KEY).await?;
    let should_send = should_send_scheduled_with_last_sent(&schedule, last_sent)
        .map_err(|e| format!("Schedule check failed: {e}"))?;

    if !should_send {
        return Ok(());
    }

    // Scope 2: Generate the filtered report from Postgres.
    let store_name = get_store_name_pg(pool).await?;
    let report = generate_filtered_report_email_pg(pool, &schedule, &store_name).await?;

    let recipients = schedule.recipients.clone();
    send_email(&smtp_config, &report, &recipients).await?;

    // Record the successful send for dedup.
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    set_setting_pg(pool, LAST_SENT_KEY, &now).await?;

    info!(
        "Scheduled report sent to {} recipients (cadence: {}, types: {:?})",
        recipients.len(),
        schedule.cadence,
        schedule.report_types,
    );

    Ok(())
}

// ── Settings helpers ───────────────────────────────────────────────

/// Read a raw `settings` value (None when absent).
async fn get_setting_pg(pool: &Pool, key: &str) -> Result<Option<String>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let row = client
        .query_opt("SELECT value FROM settings WHERE key = $1", &[&key])
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    Ok(row.map(|r| r.get(0)))
}

/// Upsert a `settings` value.
async fn set_setting_pg(pool: &Pool, key: &str, value: &str) -> Result<(), String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    client
        .execute(
            "INSERT INTO settings (key, value, updated_at)
             VALUES ($1, $2, to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"'))
             ON CONFLICT (key) DO UPDATE
               SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at",
            &[&key, &value],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    Ok(())
}

/// Load the SMTP config from the settings table, decrypting the password
/// transparently (mirrors `oz_core`'s `Store::get_smtp_config`).
async fn get_smtp_config_pg(pool: &Pool) -> Result<Option<SmtpConfig>, String> {
    let raw = match get_setting_pg(pool, SMTP_CONFIG_SETTINGS_KEY).await? {
        Some(v) => v,
        None => return Ok(None),
    };
    let mut config: SmtpConfig = serde_json::from_str(&raw)
        .map_err(|e| format!("failed to deserialize SMTP config: {e}"))?;
    if let Some(ref pwd) = config.password
        && !pwd.is_empty()
    {
        config.password = Some(oz_core::crypto::decrypt_smtp_at_rest(pwd));
    }
    Ok(Some(config))
}

/// Load the report schedule configuration from the settings table.
async fn get_report_schedule_pg(pool: &Pool) -> Result<Option<ReportScheduleConfig>, String> {
    let raw = match get_setting_pg(pool, REPORT_SCHEDULE_SETTINGS_KEY).await? {
        Some(v) => v,
        None => return Ok(None),
    };
    let config: ReportScheduleConfig = serde_json::from_str(&raw)
        .map_err(|e| format!("failed to deserialize report schedule: {e}"))?;
    Ok(Some(config))
}

/// Read the store name from settings, falling back to a default.
async fn get_store_name_pg(pool: &Pool) -> Result<String, String> {
    Ok(get_setting_pg(pool, "store.name")
        .await?
        .unwrap_or_else(|| "OZ-POS Store".to_string()))
}

// ── Report email generation ────────────────────────────────────────

/// Generate a filtered report email from Postgres — the mirror of
/// `oz_core::export::email_sender::generate_filtered_report_email`.
async fn generate_filtered_report_email_pg(
    pool: &Pool,
    schedule: &ReportScheduleConfig,
    store_name: &str,
) -> Result<oz_core::export::email_report::ReportEmail, String> {
    let lookback_start = Utc::now()
        .checked_sub_signed(chrono::Duration::days(schedule.lookback_days as i64))
        .unwrap_or(Utc::now())
        .format("%Y-%m-%d")
        .to_string();
    let end = Utc::now().format("%Y-%m-%d").to_string();

    let mut bundle = export_analytics_bundle_pg(
        pool,
        ExportConfig {
            start_date: lookback_start.clone(),
            end_date: end.clone(),
            ..ExportConfig::default()
        },
        "",
        store_name,
    )
    .await?;

    filter_analytics_bundle(&mut bundle, &schedule.report_types);

    let date_label = format!("{lookback_start} to {end}");
    Ok(ReportEmailBuilder::build(&bundle, store_name, &date_label))
}

/// Parse a `YYYY-MM-DD` range bound into a `NaiveDate` for parameter
/// binding (Postgres `date` columns compare against `date`-typed params).
fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| format!("invalid date '{s}': {e}"))
}

// ── Analytics bundle (Postgres) ────────────────────────────────────

/// Export a complete analytics bundle from Postgres — the mirror of
/// `oz_core::Store::export_analytics_bundle`.
pub async fn export_analytics_bundle_pg(
    pool: &Pool,
    config: ExportConfig,
    tenant_id: &str,
    store_name: &str,
) -> Result<AnalyticsBundle, String> {
    let daily_revenue = daily_revenue_pg(pool, &config.start_date, &config.end_date).await?;
    let weekly_revenue = weekly_revenue_pg(pool, &config.start_date, &config.end_date).await?;
    let monthly_revenue = monthly_revenue_pg(pool, &config.start_date, &config.end_date).await?;
    let top_products = top_products_pg(
        pool,
        &config.start_date,
        &config.end_date,
        config.top_product_limit,
        "revenue",
    )
    .await?;
    let hourly_heatmap = hourly_heatmap_pg(pool, &config.start_date, &config.end_date).await?;
    let category_breakdown =
        category_breakdown_pg(pool, &config.start_date, &config.end_date).await?;
    let low_stock_alerts = low_stock_alerts_at_location_pg(
        pool,
        oz_core::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
        config.low_stock_threshold,
    )
    .await?;
    let active_stock_alerts =
        active_stock_alerts_pg(pool, oz_core::inventory::CANONICAL_DEFAULT_LOCATION_UUID).await?;
    let category_popularity = category_popularity_pg(pool, 3).await?;
    let category_forecast =
        category_forecast_pg(pool, &config.start_date, &config.end_date, "weekly", 10).await?;

    let exported_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

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
        category_popularity,
        category_forecast,
    })
}

/// Compute revenue profit fields from a row (shared by the daily/weekly/
/// monthly queries — same arithmetic as `oz_core::db::reports`).
fn revenue_profit_fields(total_minor: i64, cogs_minor: i64) -> (i64, i64, i64, f64) {
    let gross_profit_minor = total_minor - cogs_minor;
    let gross_margin_percent = if total_minor > 0 {
        gross_profit_minor as f64 / total_minor as f64 * 100.0
    } else {
        0.0
    };
    (
        total_minor,
        cogs_minor,
        gross_profit_minor,
        gross_margin_percent,
    )
}

/// Daily revenue for a date range (Postgres mirror of
/// `oz_core::db::reports::Store::daily_revenue`).
async fn daily_revenue_pg(
    pool: &Pool,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<DailyRevenueRow>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let start = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    let rows = client
        .query(
            "SELECT d.date, d.total_minor, d.currency, d.sale_count,
                    (SELECT COALESCE(SUM(COALESCE(sl2.cost_minor, p2.cost_minor, 0) * sl2.qty)::bigint, 0)
                     FROM sale_lines sl2
                     JOIN sales s2 ON sl2.sale_id = s2.id
                     LEFT JOIN products p2 ON sl2.sku = p2.sku
                     WHERE s2.status = 'completed'
                       AND s2.currency = d.currency
                       AND s2.created_at::date = d.date::date) AS cogs_minor
             FROM (SELECT to_char(s.created_at::date, 'YYYY-MM-DD') AS date,
                          SUM(s.total_minor)::bigint AS total_minor,
                          s.currency AS currency,
                          COUNT(*) AS sale_count
                   FROM sales s
                   WHERE s.status = 'completed'
                     AND s.created_at::date BETWEEN $1 AND $2
                   GROUP BY to_char(s.created_at::date, 'YYYY-MM-DD'), s.currency) d
             ORDER BY date ASC",
            &[&start, &end],
        )
        .await
        .map_err(|e| format!("DB error: {e:?}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let total_minor: i64 = row.get(1);
        let cogs_minor: i64 = row.get(4);
        let (total_minor, cogs_minor, gross_profit_minor, gross_margin_percent) =
            revenue_profit_fields(total_minor, cogs_minor);
        out.push(DailyRevenueRow {
            date: row.get(0),
            total_minor,
            currency: row.get(2),
            sale_count: row.get(3),
            cogs_minor,
            gross_profit_minor,
            gross_margin_percent,
        });
    }
    Ok(out)
}

/// Weekly revenue (Monday-first weeks) for a date range.
async fn weekly_revenue_pg(
    pool: &Pool,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<WeeklyRevenueRow>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let start = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    let rows = client
        .query(
            "SELECT d.week_start, d.total_minor, d.currency, d.sale_count,
                    (SELECT COALESCE(SUM(COALESCE(sl2.cost_minor, p2.cost_minor, 0) * sl2.qty)::bigint, 0)
                     FROM sale_lines sl2
                     JOIN sales s2 ON sl2.sale_id = s2.id
                     LEFT JOIN products p2 ON sl2.sku = p2.sku
                     WHERE s2.status = 'completed'
                       AND s2.currency = d.currency
                       AND to_char(date_trunc('week', s2.created_at::date)::date, 'YYYY-MM-DD') = d.week_start::date::text) AS cogs_minor
             FROM (SELECT to_char(date_trunc('week', s.created_at::date)::date, 'YYYY-MM-DD') AS week_start,
                          SUM(s.total_minor)::bigint AS total_minor, s.currency AS currency,
                          COUNT(*) AS sale_count
                   FROM sales s
                   WHERE s.status = 'completed'
                     AND s.created_at::date BETWEEN $1 AND $2
                   GROUP BY to_char(date_trunc('week', s.created_at::date)::date, 'YYYY-MM-DD'), s.currency) d
             ORDER BY week_start ASC",
            &[&start, &end],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let total_minor: i64 = row.get(1);
        let cogs_minor: i64 = row.get(4);
        let (total_minor, cogs_minor, gross_profit_minor, gross_margin_percent) =
            revenue_profit_fields(total_minor, cogs_minor);
        out.push(WeeklyRevenueRow {
            week_start: row.get(0),
            total_minor,
            currency: row.get(2),
            sale_count: row.get(3),
            cogs_minor,
            gross_profit_minor,
            gross_margin_percent,
        });
    }
    Ok(out)
}

/// Monthly revenue for a date range.
async fn monthly_revenue_pg(
    pool: &Pool,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<MonthlyRevenueRow>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let start = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    let rows = client
        .query(
            "SELECT d.month, d.total_minor, d.currency, d.sale_count,
                    (SELECT COALESCE(SUM(COALESCE(sl2.cost_minor, p2.cost_minor, 0) * sl2.qty)::bigint, 0)
                     FROM sale_lines sl2
                     JOIN sales s2 ON sl2.sale_id = s2.id
                     LEFT JOIN products p2 ON sl2.sku = p2.sku
                     WHERE s2.status = 'completed'
                       AND s2.currency = d.currency
                       AND LEFT(s2.created_at, 7) = d.month::text) AS cogs_minor
             FROM (SELECT LEFT(s.created_at, 7) AS month,
                          SUM(s.total_minor)::bigint AS total_minor, s.currency AS currency,
                          COUNT(*) AS sale_count
                   FROM sales s
                   WHERE s.status = 'completed'
                     AND s.created_at::date BETWEEN $1 AND $2
                   GROUP BY LEFT(s.created_at, 7), s.currency) d
             ORDER BY month ASC",
            &[&start, &end],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let total_minor: i64 = row.get(1);
        let cogs_minor: i64 = row.get(4);
        let (total_minor, cogs_minor, gross_profit_minor, gross_margin_percent) =
            revenue_profit_fields(total_minor, cogs_minor);
        out.push(MonthlyRevenueRow {
            month: row.get(0),
            total_minor,
            currency: row.get(2),
            sale_count: row.get(3),
            cogs_minor,
            gross_profit_minor,
            gross_margin_percent,
        });
    }
    Ok(out)
}

/// Top products ranked by revenue (or profit) for a date range.
async fn top_products_pg(
    pool: &Pool,
    start_date: &str,
    end_date: &str,
    limit: i64,
    order_by: &str,
) -> Result<Vec<TopProductRow>, String> {
    let order_clause = if order_by == "profit" {
        "gross_profit_minor DESC"
    } else {
        "total_minor DESC"
    };
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let start = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    let sql = format!(
        "SELECT p.id AS product_id, p.sku, p.name,
                SUM(sl.qty)::bigint AS total_qty,
                SUM(sl.line_minor)::bigint AS total_minor,
                SUM(COALESCE(sl.cost_minor, p.cost_minor, 0) * sl.qty)::bigint AS cogs_minor,
                (SUM(sl.line_minor) - SUM(COALESCE(sl.cost_minor, p.cost_minor, 0) * sl.qty))::bigint AS gross_profit_minor
         FROM sale_lines sl
         JOIN sales s ON sl.sale_id = s.id
         JOIN products p ON sl.sku = p.sku
         WHERE s.status = 'completed' AND s.created_at::date BETWEEN $1 AND $2
         GROUP BY p.id
         ORDER BY {order_clause}, p.sku
         LIMIT $3"
    );
    let rows = client
        .query(&sql, &[&start, &end, &limit])
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let total_minor: i64 = row.get(4);
        let cogs_minor: i64 = row.get(5);
        let gross_profit_minor: i64 = row.get(6);
        out.push(TopProductRow {
            product_id: row.get(0),
            sku: row.get(1),
            name: row.get(2),
            total_qty: row.get(3),
            total_minor,
            cogs_minor,
            gross_profit_minor,
            gross_margin_percent: if total_minor > 0 {
                (gross_profit_minor as f64 / total_minor as f64) * 100.0
            } else {
                0.0
            },
        });
    }
    Ok(out)
}

/// Hourly sales heatmap for a date range.
async fn hourly_heatmap_pg(
    pool: &Pool,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HourlyHeatmapRow>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let start = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    let rows = client
        .query(
            "SELECT EXTRACT(DOW FROM created_at::timestamp)::bigint AS day_of_week,
                    EXTRACT(HOUR FROM created_at::timestamp)::bigint AS hour,
                    SUM(total_minor)::bigint AS total_minor,
                    COUNT(*) AS sale_count
             FROM sales
             WHERE status = 'completed' AND created_at::date BETWEEN $1 AND $2
             GROUP BY day_of_week, hour
             ORDER BY day_of_week, hour",
            &[&start, &end],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(HourlyHeatmapRow {
            day_of_week: row.get(0),
            hour: row.get(1),
            total_minor: row.get(2),
            sale_count: row.get(3),
        });
    }
    Ok(out)
}

/// Revenue breakdown by product category for a date range.
async fn category_breakdown_pg(
    pool: &Pool,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<CategoryBreakdownRow>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let start = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    let rows = client
        .query(
            "SELECT p.category_id, COALESCE(c.name, 'Uncategorised') AS category_name,
                    SUM(sl.line_minor)::bigint AS total_minor,
                    COUNT(DISTINCT s.id) AS sale_count
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             JOIN products p ON sl.sku = p.sku
             LEFT JOIN categories c ON p.category_id = c.id
             WHERE s.status = 'completed' AND s.created_at::date BETWEEN $1 AND $2
             GROUP BY p.category_id, c.name
             ORDER BY total_minor DESC",
            &[&start, &end],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let mut out: Vec<CategoryBreakdownRow> = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(CategoryBreakdownRow {
            category_id: row.get(0),
            category_name: row.get(1),
            total_minor: row.get(2),
            sale_count: row.get(3),
            percentage: 0.0,
        });
    }

    let grand_total: f64 = out.iter().map(|r| r.total_minor as f64).sum();
    if grand_total > 0.0 {
        for row in &mut out {
            row.percentage = (row.total_minor as f64 / grand_total) * 100.0;
        }
    }

    Ok(out)
}

/// Per-location low-stock alerts using `stock_summary`.
async fn low_stock_alerts_at_location_pg(
    pool: &Pool,
    location_id: &str,
    default_threshold: i64,
) -> Result<Vec<LowStockAlert>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client
        .query(
            "SELECT p.id AS product_id, p.sku, p.name, p.currency,
                    p.price_minor, p.cost_minor,
                    COALESCE(ss.qty, 0) AS current_qty,
                    COALESCE(
                        (SELECT st.threshold FROM stock_thresholds st
                         WHERE st.product_id = p.id
                           AND st.location_id = $1 AND st.enabled = 1
                         LIMIT 1),
                        (SELECT st.threshold FROM stock_thresholds st
                         WHERE st.product_id = p.id
                           AND st.location_id IS NULL AND st.enabled = 1
                         LIMIT 1),
                        $2
                    ) AS threshold
             FROM products p
             LEFT JOIN stock_summary ss
                ON ss.item_id = p.id AND ss.location_id = $1
             WHERE COALESCE(ss.qty, 0) <= $2
                OR (SELECT 1 FROM stock_thresholds st
                    WHERE st.product_id = p.id
                      AND (st.location_id = $1 OR st.location_id IS NULL)
                      AND st.enabled = 1
                      AND COALESCE(ss.qty, 0) <= st.threshold
                    LIMIT 1) = 1
             ORDER BY current_qty ASC",
            &[&location_id, &default_threshold],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(LowStockAlert {
            product_id: row.get(0),
            sku: row.get(1),
            name: row.get(2),
            currency: row.get(3),
            price_minor: row.get(4),
            cost_minor: row.get(5),
            current_qty: row.get(6),
            threshold: row.get(7),
        });
    }
    Ok(out)
}

/// Active (non-resolved) stock alert events for a location.
async fn active_stock_alerts_pg(
    pool: &Pool,
    location_id: &str,
) -> Result<Vec<StockAlertEvent>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client
        .query(
            "SELECT sae.id, sae.threshold_id, sae.product_id, sae.location_id,
                    sae.current_qty, sae.threshold, sae.status,
                    sae.triggered_at, sae.acknowledged_at, sae.resolved_at,
                    sae.acknowledged_by,
                    COALESCE(p.sku, '') AS product_sku,
                    COALESCE(p.name, '') AS product_name
             FROM stock_alert_events sae
             LEFT JOIN products p ON sae.product_id = p.id
             WHERE sae.location_id = $1 AND sae.status IN ('active', 'acknowledged')
             ORDER BY sae.triggered_at DESC",
            &[&location_id],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(StockAlertEvent {
            id: row.get(0),
            threshold_id: row.get(1),
            product_id: row.get(2),
            location_id: row.get::<_, Option<String>>(3).unwrap_or_default(),
            current_qty: row.get(4),
            threshold: row.get(5),
            status: row.get(6),
            triggered_at: row.get(7),
            acknowledged_at: row.get(8),
            resolved_at: row.get(9),
            acknowledged_by: row.get(10),
            product_sku: row.get(11),
            product_name: row.get(12),
        });
    }
    Ok(out)
}

/// Per-category popularity standings (Postgres mirror of
/// `oz_core::db::popularity::Store::category_popularity`).
async fn category_popularity_pg(
    pool: &Pool,
    top_per_category: i64,
) -> Result<Vec<CategoryPopularityRow>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;

    let catalog_mean: f64 = client
        .query_one("SELECT AVG(popularity_score)::float8 FROM products", &[])
        .await
        .map(|r| r.get::<_, Option<f64>>(0).unwrap_or(0.0))
        .unwrap_or(0.0);

    // Per-category aggregates: count + mean score.
    let mut cats: std::collections::HashMap<String, CategoryPopularityRow> =
        std::collections::HashMap::new();
    {
        let rows = client
            .query(
                "SELECT p.category_id, c.name, COUNT(*) AS cnt, AVG(p.popularity_score)::float8 AS mean
                 FROM products p
                 LEFT JOIN categories c ON p.category_id = c.id
                 GROUP BY p.category_id, c.name",
                &[],
            )
            .await
            .map_err(|e| format!("DB error: {e}"))?;
        for row in rows {
            let category: Option<String> = row.get(0);
            let name: Option<String> = row.get(1);
            let cnt: i64 = row.get(2);
            let mean: f64 = row.get(3);
            let key = category.unwrap_or_default();
            cats.insert(
                key.clone(),
                CategoryPopularityRow {
                    category_id: key,
                    category_name: name,
                    product_count: cnt,
                    mean_score: mean,
                    catalog_ratio: if catalog_mean > 0.0 {
                        mean / catalog_mean
                    } else {
                        0.0
                    },
                    top_products: Vec::new(),
                },
            );
        }
    }

    // Ranked products per category (score desc, SKU tiebreak).
    let mut per_cat: std::collections::HashMap<String, Vec<(String, String, f64)>> =
        std::collections::HashMap::new();
    {
        let rows = client
            .query(
                "SELECT p.category_id, p.sku, p.name, p.popularity_score
                 FROM products p
                 ORDER BY p.category_id, p.popularity_score DESC, p.sku ASC",
                &[],
            )
            .await
            .map_err(|e| format!("DB error: {e}"))?;
        for row in rows {
            let category: Option<String> = row.get(0);
            let sku: String = row.get(1);
            let name: String = row.get(2);
            let score: f64 = row.get(3);
            per_cat
                .entry(category.unwrap_or_default())
                .or_default()
                .push((sku, name, score));
        }
    }

    for (key, rows) in per_cat {
        let count = rows.len() as f64;
        let top: Vec<CategoryTopProduct> = rows
            .into_iter()
            .take(top_per_category.max(0) as usize)
            .enumerate()
            .map(|(i, (sku, name, score))| CategoryTopProduct {
                sku,
                name,
                popularity_score: score,
                rank: i as i64 + 1,
                percentile: if count > 1.0 {
                    (count - 1.0 - i as f64) / (count - 1.0)
                } else {
                    1.0
                },
            })
            .collect();
        if let Some(cat) = cats.get_mut(&key) {
            cat.top_products = top;
        }
    }

    let mut out: Vec<CategoryPopularityRow> = cats.into_values().collect();
    out.sort_by(|a, b| {
        b.mean_score
            .partial_cmp(&a.mean_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.category_id.cmp(&b.category_id))
    });
    Ok(out)
}

/// Per-period popularity trend for the top categories — the raw-signal
/// queries of `oz_core::db::popularity::Store::category_popularity_trend`
/// against Postgres, with the ADR #37 blend evaluated by the shared
/// `score_from_raw` helper.
async fn category_popularity_trend_pg(
    pool: &Pool,
    start_date: &str,
    end_date: &str,
    granularity: &str,
    top_categories: i64,
) -> Result<Vec<CategoryTrendPoint>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let start = parse_date(start_date)?;
    let end = parse_date(end_date)?;

    // Period expressions per granularity (same shapes the SQLite version
    // produced: YYYY-MM-DD daily/weekly, YYYY-MM monthly; weekly is
    // Sunday-start via the date_trunc('week') Monday minus one).
    let (s_period, a_period) = match granularity {
        "weekly" => (
            "to_char(date_trunc('week', s.created_at::date)::date - 1, 'YYYY-MM-DD')",
            "to_char(date_trunc('week', a.created_at::date)::date - 1, 'YYYY-MM-DD')",
        ),
        "monthly" => ("LEFT(s.created_at, 7)", "LEFT(a.created_at, 7)"),
        _ => (
            "to_char(s.created_at::date, 'YYYY-MM-DD')",
            "to_char(a.created_at::date, 'YYYY-MM-DD')",
        ),
    };

    // The most popular categories by current mean score.
    let top: Vec<(String, Option<String>)> = {
        let rows = client
            .query(
                "SELECT p.category_id, c.name
                 FROM products p
                 LEFT JOIN categories c ON p.category_id = c.id
                 GROUP BY p.category_id, c.name
                 ORDER BY AVG(p.popularity_score) DESC, p.category_id ASC
                 LIMIT $1",
                &[&top_categories.max(1)],
            )
            .await
            .map_err(|e| format!("DB error: {e}"))?;
        rows.iter()
            .map(|r| {
                (
                    r.get::<_, Option<String>>(0).unwrap_or_default(),
                    r.get::<_, Option<String>>(1),
                )
            })
            .collect()
    };
    if top.is_empty() {
        return Ok(Vec::new());
    }
    let rank: std::collections::HashMap<String, usize> = top
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.clone(), i))
        .collect();

    // (period, category) → raw signals.
    let mut agg: std::collections::HashMap<(String, String), (i64, i64, i64, i64)> =
        std::collections::HashMap::new();
    {
        // Sales: units + distinct transactions per (period, category).
        let sql = format!(
            "SELECT {s_period} AS period_start, p.category_id,
                    SUM(sl.qty)::bigint AS units, COUNT(DISTINCT sl.sale_id) AS txns
             FROM sale_lines sl
             JOIN sales s ON sl.sale_id = s.id
             JOIN products p ON sl.sku = p.sku
             WHERE s.status = 'completed' AND s.created_at::date BETWEEN $1 AND $2
             GROUP BY {s_period}, p.category_id"
        );
        let rows = client
            .query(&sql, &[&start, &end])
            .await
            .map_err(|e| format!("DB error: {e}"))?;
        for row in rows {
            let period_start: String = row.get(0);
            let cat: String = row.get::<_, Option<String>>(1).unwrap_or_default();
            let units: i64 = row.get(2);
            let txns: i64 = row.get(3);
            let e = agg.entry((period_start, cat)).or_insert((0, 0, 0, 0));
            e.0 += units;
            e.1 += txns;
        }
    }
    {
        // Search + edit events per (period, category).
        let sql = format!(
            "SELECT {a_period} AS period_start, p.category_id, a.event_type, COUNT(*) AS cnt
             FROM product_activity a
             JOIN products p ON a.sku = p.sku
             WHERE a.created_at::date BETWEEN $1 AND $2
             GROUP BY {a_period}, p.category_id, a.event_type"
        );
        let rows = client
            .query(&sql, &[&start, &end])
            .await
            .map_err(|e| format!("DB error: {e}"))?;
        for row in rows {
            let period_start: String = row.get(0);
            let cat: String = row.get::<_, Option<String>>(1).unwrap_or_default();
            let etype: String = row.get(2);
            let cnt: i64 = row.get(3);
            let e = agg.entry((period_start, cat)).or_insert((0, 0, 0, 0));
            if etype == "search" {
                e.2 += cnt;
            } else {
                e.3 += cnt;
            }
        }
    }

    let (ms, mq, me) = category_means_pg(pool, "")
        .await?
        .unwrap_or((0.0, 0.0, 0.0));
    let mut points: Vec<CategoryTrendPoint> = Vec::new();
    for ((period_start, cat), (units, txns, searches, edits)) in agg {
        if !rank.contains_key(&cat) {
            continue;
        }
        let (ms, mq, me) = category_means_pg(pool, &cat).await?.unwrap_or((ms, mq, me));
        let score = score_from_raw(
            units as f64,
            units as f64,
            txns as f64,
            searches as f64,
            searches as f64,
            edits as f64,
            edits as f64,
            ms,
            mq,
            me,
        );
        let name = top
            .iter()
            .find(|(id, _)| *id == cat)
            .and_then(|(_, n)| n.clone());
        points.push(CategoryTrendPoint {
            period_start,
            category_id: cat,
            category_name: name,
            score,
            units_sold: units,
            distinct_transactions: txns,
            searches,
            edits,
        });
    }
    points.sort_by(|a, b| {
        a.period_start
            .cmp(&b.period_start)
            .then_with(|| rank[&a.category_id].cmp(&rank[&b.category_id]))
    });
    Ok(points)
}

/// Read the cached smoothing means for a category from the settings table
/// (falls back to `None` — the SQLite path defaults to `(0,0,0)`).
async fn category_means_pg(pool: &Pool, category: &str) -> Result<Option<(f64, f64, f64)>, String> {
    let raw = match get_setting_pg(pool, "popularity.category_means").await? {
        Some(v) => v,
        None => return Ok(None),
    };
    let map: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };
    let entry = map.get(category).or_else(|| map.get(""));
    let Some(entry) = entry else {
        return Ok(None);
    };
    Ok(Some((
        entry.get("sales").and_then(|v| v.as_f64()).unwrap_or(0.0),
        entry.get("search").and_then(|v| v.as_f64()).unwrap_or(0.0),
        entry.get("edits").and_then(|v| v.as_f64()).unwrap_or(0.0),
    )))
}

/// Next-period demand forecast per category — the Postgres mirror of
/// `oz_core::db::popularity::Store::category_forecast`, reusing the shared
/// `linear_forecast` / `seasonal_daily_forecast` fits.
async fn category_forecast_pg(
    pool: &Pool,
    start_date: &str,
    end_date: &str,
    granularity: &str,
    top_categories: i64,
) -> Result<Vec<CategoryForecastRow>, String> {
    const MAX_SERIES_POINTS: usize = 14;

    let points =
        category_popularity_trend_pg(pool, start_date, end_date, granularity, top_categories)
            .await?;
    let mut groups: std::collections::HashMap<
        String,
        (Option<String>, Vec<(chrono::NaiveDate, f64)>),
    > = std::collections::HashMap::new();
    for p in points {
        let date = chrono::NaiveDate::parse_from_str(&p.period_start, "%Y-%m-%d").ok();
        let entry = groups
            .entry(p.category_id.clone())
            .or_insert((p.category_name, Vec::new()));
        if let Some(d) = date {
            entry.1.push((d, p.units_sold as f64));
        }
    }

    let mut out: Vec<CategoryForecastRow> = Vec::new();
    for (category_id, (name, series)) in groups {
        let tail = series
            .iter()
            .rev()
            .take(MAX_SERIES_POINTS)
            .copied()
            .collect::<Vec<(chrono::NaiveDate, f64)>>();
        let tail = tail
            .into_iter()
            .rev()
            .collect::<Vec<(chrono::NaiveDate, f64)>>();
        let f = if granularity == "daily" && tail.len() >= 7 {
            let next = tail
                .last()
                .map(|(d, _)| *d + chrono::Duration::days(1))
                .unwrap_or_else(|| chrono::Utc::now().date_naive());
            seasonal_daily_forecast(&tail, next)
        } else {
            let units: Vec<f64> = tail.iter().map(|(_, u)| *u).collect();
            linear_forecast(&units)
        };
        out.push(CategoryForecastRow {
            category_id,
            category_name: name,
            forecast_units: f.forecast_units,
            trend_per_period: f.trend_per_period,
            recent_avg_units: f.recent_avg_units,
        });
    }
    out.sort_by(|a, b| {
        b.forecast_units
            .cmp(&a.forecast_units)
            .then_with(|| a.category_id.cmp(&b.category_id))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::export::email_report::SMTP_CONFIG_SETTINGS_KEY;

    /// Integration test against a live Postgres instance — seeds a product,
    /// a completed sale with lines, stock, and the settings the loop reads,
    /// then exercises the whole analytics bundle + settings helpers on the
    /// real database. Skips when Postgres is unreachable.
    #[tokio::test]
    async fn pg_integration_email_loop_reads_postgres() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());

        let pool = match crate::db::DbPool::connect_postgres(&url, false, 20).await {
            Ok(crate::db::DbPool::Postgres(pool)) => pool,
            Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
            Err(e) => {
                eprintln!("PG email-loop integration test skipped: {e}");
                return;
            }
        };

        let ns = format!("pg-email-test-{}", uuid::Uuid::now_v7());
        let product_id = format!("{ns}-product");
        let sku = format!("{ns}-sku");
        let category_id = format!("{ns}-cat");
        let sale_id = format!("{ns}-sale");
        let sale_line_id = format!("{ns}-line");
        // Use a fixed January date so parallel PG tests writing "today"
        // rows (webhooks, REST roundtrip) can never land inside this
        // test's analytics window.
        let now = "2026-01-15T09:00:00.000Z";

        // Clean up any leftovers from previous (failed) runs so assertions
        // count only this run's seeded rows.
        {
            let client = pool.get().await.unwrap();
            for sql in [
                "DELETE FROM sale_lines WHERE id LIKE 'pg-email-test-%'",
                "DELETE FROM sales WHERE id LIKE 'pg-email-test-%'",
                "DELETE FROM stock_summary WHERE item_id LIKE 'pg-email-test-%'",
                "DELETE FROM products WHERE id LIKE 'pg-email-test-%'",
                "DELETE FROM categories WHERE name LIKE 'pg-email-test-%'",
                "DELETE FROM settings WHERE key IN ('store.name', 'smtp_config', 'report_schedule', 'last_report_sent_at') AND value LIKE 'pg-email-test-%'",
            ] {
                client.execute(sql, &[]).await.unwrap();
            }
        }

        // ── Seed ──────────────────────────────────────────────────────
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let category_name = format!("{ns} Cat");
        tx.execute(
            "INSERT INTO categories (id, name, colour, icon, created_at, updated_at) VALUES ($1, $2, '#fff', '', $3, $3)",
            &[&category_id, &category_name, &now],
        )
        .await
        .unwrap();
        let barcode = format!("{ns}-barcode");
        tx.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, \
             created_at, updated_at, price_updated_at, track_serial, product_type, version, \
             cost_minor, brand, rack_location, notes, unit, is_active, default_supplier_id, tenant_id)
             VALUES ($1, $2, 'Cold Brew', 5000, 'USD', $3, $4, $5, $5, $5, 0, 'retail', 1, 2000, NULL, NULL, NULL, NULL, 1, NULL, 'default')",
            &[&product_id, &sku, &category_id, &barcode, &now],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty, updated_at) VALUES ($1, $2, 4, $3)",
            &[&product_id, &oz_core::inventory::CANONICAL_DEFAULT_LOCATION_UUID, &now],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, tendered_minor, \
             discount_percent, discount_label, user_id, created_at, updated_at, subtotal_minor, \
             tax_total_minor, customer_id, version)
             VALUES ($1, 5000, 'USD', 1, 'completed', 'cash', 5000, 0, NULL, NULL, $2, $2, 5000, 0, NULL, 1)",
            &[&sale_id, &now],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position, \
             tax_minor, tax_rate_id, serial_number, store_id, course, modifiers_json, tax_breakdown_json, cost_minor)
             VALUES ($1, $2, $3, 1, 5000, 5000, 'USD', 1, 0, NULL, NULL, NULL, NULL, NULL, NULL, 2000)",
            &[&sale_line_id, &sale_id, &sku],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ($1, $2, $3)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at",
            &[&"store.name", &format!("{ns} Store"), &now],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // ── Exercise the analytics bundle ─────────────────────────────
        let config = ExportConfig {
            start_date: "2026-01-01".into(),
            end_date: "2026-01-31".into(),
            top_product_limit: 25,
            low_stock_threshold: 10,
        };
        let bundle = export_analytics_bundle_pg(&pool, config, "default", &format!("{ns} Store"))
            .await
            .unwrap();

        // Daily revenue: one completed sale of 5000 minor units.
        assert_eq!(bundle.daily_revenue.len(), 1);
        assert_eq!(bundle.daily_revenue[0].date, "2026-01-15");
        assert_eq!(bundle.daily_revenue[0].total_minor, 5000);
        assert_eq!(bundle.daily_revenue[0].sale_count, 1);
        assert_eq!(bundle.daily_revenue[0].cogs_minor, 2000);
        assert_eq!(bundle.daily_revenue[0].gross_profit_minor, 3000);

        // Weekly revenue: same sale in the week of 2026-01-12 (Monday).
        assert_eq!(bundle.weekly_revenue.len(), 1);
        assert_eq!(bundle.weekly_revenue[0].week_start, "2026-01-12");
        assert_eq!(bundle.weekly_revenue[0].total_minor, 5000);

        // Monthly revenue: 2026-01.
        assert_eq!(bundle.monthly_revenue.len(), 1);
        assert_eq!(bundle.monthly_revenue[0].month, "2026-01");
        assert_eq!(bundle.monthly_revenue[0].total_minor, 5000);

        // Top products: the seeded SKU with qty 1, revenue 5000, COGS 2000.
        assert_eq!(bundle.top_products.len(), 1);
        assert_eq!(bundle.top_products[0].sku, sku);
        assert_eq!(bundle.top_products[0].total_qty, 1);
        assert_eq!(bundle.top_products[0].total_minor, 5000);
        assert_eq!(bundle.top_products[0].cogs_minor, 2000);
        assert_eq!(bundle.top_products[0].gross_profit_minor, 3000);

        // Hourly heatmap: Sunday=0, the sale is at 09:00 UTC on a Thursday.
        assert!(
            bundle
                .hourly_heatmap
                .iter()
                .any(|h| h.day_of_week == 4 && h.hour == 9)
        );

        // Category breakdown: the seeded category, 5000 minor units.
        assert_eq!(bundle.category_breakdown.len(), 1);
        assert_eq!(
            bundle.category_breakdown[0].category_name,
            format!("{ns} Cat")
        );
        assert_eq!(bundle.category_breakdown[0].total_minor, 5000);
        assert_eq!(bundle.category_breakdown[0].percentage, 100.0);

        // Low stock: qty 4 ≤ threshold 10.
        assert!(
            bundle
                .low_stock_alerts
                .iter()
                .any(|a| a.sku == sku && a.current_qty == 4 && a.threshold == 10)
        );

        // Popularity + forecast are computed (may be empty without activity).
        assert!(bundle.category_popularity.is_empty() || bundle.category_popularity.len() >= 1);

        // ── Settings round-trip (SMTP config + schedule + dedup key) ──
        let smtp = SmtpConfig {
            host: "smtp.test.com".into(),
            port: 587,
            username: Some("u".into()),
            password: Some("pw".into()),
            from: "reports@test.com".into(),
            use_tls: true,
        };
        set_setting_pg(
            &pool,
            SMTP_CONFIG_SETTINGS_KEY,
            &serde_json::to_string(&smtp).unwrap(),
        )
        .await
        .unwrap();
        let loaded = get_smtp_config_pg(&pool).await.unwrap().unwrap();
        assert_eq!(loaded.host, "smtp.test.com");
        assert_eq!(loaded.password, Some("pw".into()));

        let schedule = ReportScheduleConfig {
            enabled: true,
            cadence: "daily".into(),
            report_types: vec!["daily_revenue".into()],
            recipients: vec!["a@b.c".into()],
            send_at_time: "08:00".into(),
            timezone: "UTC".into(),
            lookback_days: 7,
        };
        set_setting_pg(
            &pool,
            REPORT_SCHEDULE_SETTINGS_KEY,
            &serde_json::to_string(&schedule).unwrap(),
        )
        .await
        .unwrap();
        let loaded = get_report_schedule_pg(&pool).await.unwrap().unwrap();
        assert_eq!(loaded.cadence, "daily");
        assert_eq!(loaded.recipients, vec!["a@b.c".to_string()]);

        assert_eq!(
            get_store_name_pg(&pool).await.unwrap(),
            format!("{ns} Store")
        );
        assert_eq!(get_setting_pg(&pool, LAST_SENT_KEY).await.unwrap(), None);

        // ── Cleanup (keys are namespaced; delete the seeded rows) ─────
        let client = pool.get().await.unwrap();
        for (sql, id) in [
            ("DELETE FROM sale_lines WHERE id = $1", &sale_line_id),
            ("DELETE FROM sales WHERE id = $1", &sale_id),
            ("DELETE FROM products WHERE id = $1", &product_id),
            ("DELETE FROM categories WHERE id = $1", &category_id),
        ] {
            client.execute(sql, &[&id]).await.unwrap();
        }
        client
            .execute(
                "DELETE FROM settings WHERE key IN ('store.name', 'smtp_config', 'report_schedule', 'last_report_sent_at')",
                &[],
            )
            .await
            .unwrap();
    }
}
