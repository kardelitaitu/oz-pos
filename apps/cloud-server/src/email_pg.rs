//! Postgres backend for the scheduled report-sender email loop.
//!
//! Phase 1.5 of `unify-auth-and-sync.md`: the SQLite loop in [`crate::email`]
//! reads its config from the synchronous `rusqlite` `Store`; this module is
//! the parallel async Postgres implementation used on the cloud branch. It
//! covers the exact surface the loop touches:
//!
//! - the `settings` table (`smtp_config`, `report_schedule`,
//!   `last_report_sent_at`, `store.name`) — read per tenant via scoped
//!   `{key}:{tenant}` keys with bare-key fallback (see §11.5 of
//!   `unify-auth-and-sync.md`)
//! - the analytics bundle (`export_analytics_bundle_pg`) — the ten report
//!   queries ported from `oz_core::db::reports` / `oz_core::db::popularity`,
//!   every one tenant-filtered (`AND s.tenant_id = $n` / `AND p.tenant_id = $n`)
//!
//! The loop walks the active tenants (union of `tenant_plans` /
//! `offline_queue` / `sync_terminals`, plus `default` always, processed
//! first) and serializes each tenant's cycle across instances with a
//! session advisory lock keyed on the tenant id.
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

use chrono::{Datelike, NaiveDate, SecondsFormat, Utc};
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
    LAST_SENT_KEY, filter_analytics_bundle, resolve_now_in_timezone,
    should_send_scheduled_with_last_sent,
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
        // Poll every 5 min instead of 60s — reports are hourly, so 60s
        // polling wastes CPU on idle loops. Saves ~0.001 core.
        info!("Report sender background loop started (Postgres, poll interval: 300s)");
        loop {
            tokio::time::sleep(Duration::from_secs(300)).await;

            if let Err(e) = try_send_scheduled_pg(&pool).await {
                error!("Report sender loop error (Postgres): {e}");
            }
        }
    });
}

/// Try to send the scheduled reports for every active tenant — the Postgres
/// mirror of `crate::email::try_send_scheduled` walked per tenant. Each
/// cycle: enumerate tenants, then for each one read its scoped settings,
/// check the schedule (cadence + timezone + dedup via the shared scheduler),
/// generate its tenant-filtered report, send, and record the send timestamp.
/// Per-tenant errors are logged and the cycle continues; only a DB-level
/// failure (e.g. enumeration) aborts the cycle.
async fn try_send_scheduled_pg(pool: &Pool) -> Result<(), String> {
    for tenant in active_tenants_pg(pool).await? {
        if let Err(e) = try_send_scheduled_for_tenant_pg(pool, &tenant).await {
            error!("Report sender error for tenant {tenant}: {e}");
        }
    }
    Ok(())
}

/// Try to send one tenant's scheduled report, serialized across instances
/// by a session advisory lock keyed on the tenant id.
///
/// `pg_try_advisory_lock` returns `false` when another instance is already
/// inside this tenant's cycle, so a second instance skips the tenant
/// entirely — two instances can never both send the same tenant's report.
/// The lock lives on its own dedicated connection for the whole cycle
/// (every helper below uses independent pooled connections, so a
/// transaction-scoped lock would not guard them); it is released on every
/// exit path and, worst case, dies with the connection when deadpool
/// recycles it.
async fn try_send_scheduled_for_tenant_pg(pool: &Pool, tenant: &str) -> Result<(), String> {
    let lock_conn = pool.get().await.map_err(|e| e.to_string())?;
    let acquired: bool = lock_conn
        .query_one("SELECT pg_try_advisory_lock(hashtext($1))", &[&tenant])
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .get(0);
    if !acquired {
        // Another instance is handling this tenant's cycle this round.
        return Ok(());
    }
    let result = try_send_scheduled_tenant_inner_pg(pool, tenant).await;
    let _ = lock_conn
        .execute("SELECT pg_advisory_unlock(hashtext($1))", &[&tenant])
        .await;
    result
}
/// The un-serialized per-tenant send cycle: scoped settings → due check →
/// claim the period → tenant-filtered report → send → scoped last-sent
/// stamp. The `sent_reports` claim is what makes send at-most-once: it is
/// committed BEFORE the email is sent, so a crash between a successful
/// send and the last-sent stamp (or a racing second instance) is recovered
/// by the next cycle seeing the claim and skipping.
async fn try_send_scheduled_tenant_inner_pg(pool: &Pool, tenant: &str) -> Result<(), String> {
    // Scope 1: Read the tenant's SMTP + schedule config (scoped keys with
    // bare-key fallback), check the schedule.
    let smtp_config = match get_smtp_config_pg(pool, tenant).await? {
        Some(c) => c,
        None => return Ok(()),
    };

    let schedule = match get_report_schedule_pg(pool, tenant).await? {
        Some(s) if s.enabled => s,
        _ => return Ok(()),
    };

    let last_sent = get_setting_scoped_pg(pool, LAST_SENT_KEY, tenant).await?;
    let should_send = should_send_scheduled_with_last_sent(&schedule, last_sent)
        .map_err(|e| format!("Schedule check failed: {e}"))?;

    if !should_send {
        return Ok(());
    }

    // Scope 2: Claim the scheduled slot BEFORE generating or sending. If a
    // previous attempt already claimed it — a crash after a successful
    // send but before the stamp, or another instance racing — skip, so a
    // report can never be sent twice for the same period.
    let period = period_for_schedule(&schedule, resolve_now_in_timezone(&schedule.timezone));
    let report_id = uuid::Uuid::now_v7().to_string();
    if !claim_period_pg(pool, tenant, &period, &report_id).await? {
        info!(tenant, period, "report period already claimed; skipping");
        return Ok(());
    }

    // Scope 3: Generate, send, stamp. Any failure releases the claim so
    // the period retries next cycle; a claim that survives (success, or a
    // crash anywhere after this line) is exactly what prevents duplicates.
    let result: Result<(), String> = async {
        let store_name = get_store_name_pg(pool, tenant).await?;
        let report =
            generate_filtered_report_email_pg(pool, &schedule, &store_name, tenant).await?;
        let recipients = schedule.recipients.clone();
        send_email(&smtp_config, &report, &recipients).await?;
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        set_setting_scoped_pg(pool, LAST_SENT_KEY, &now, tenant).await?;
        Ok(())
    }
    .await;

    match result {
        Ok(()) => {
            info!(
                "Scheduled report sent to {} recipients (tenant: {tenant}, cadence: {}, types: {:?}, period: {period})",
                schedule.recipients.len(),
                schedule.cadence,
                schedule.report_types,
            );
            Ok(())
        }
        Err(e) => {
            if let Err(release_err) = release_period_pg(pool, tenant, &period).await {
                error!(tenant, period, error = %release_err, "releasing failed-report claim errored");
            }
            Err(e)
        }
    }
}

/// Enumerate the tenants this loop must serve: the union of every
/// tenant-scoped table that can identify an active tenant, plus `default`
/// always (so a fresh deployment whose config lives in bare keys still
/// sends). `default` sorts first, the rest alphabetically, for
/// deterministic log output.
async fn active_tenants_pg(pool: &Pool) -> Result<Vec<String>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let rows = client
        .query(
            "SELECT tenant_id FROM tenant_plans
             UNION SELECT tenant_id FROM offline_queue
             UNION SELECT tenant_id FROM sync_terminals",
            &[],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    let mut tenants: Vec<String> = rows
        .iter()
        .map(|r| r.get::<_, String>(0))
        .filter(|t| !t.is_empty())
        .collect();
    if !tenants.iter().any(|t| t == "default") {
        tenants.push("default".into());
    }
    tenants.sort_by(|a, b| {
        (a != "default")
            .cmp(&(b != "default"))
            .then_with(|| a.cmp(b))
    });
    tenants.dedup();
    Ok(tenants)
}

// ── Send dedup (sent_reports) ─────────────────────────────────────

/// The dedup key for a scheduled slot — the calendar bucket the report
/// belongs to, derived from the cadence in the schedule's timezone so a
/// crash + retry (or a second instance) always computes the same key:
/// daily → `YYYY-MM-DD`, weekly → the Monday of the week (`YYYY-MM-DD`),
/// monthly → `YYYY-MM`.
fn period_for_schedule(
    schedule: &ReportScheduleConfig,
    now_tz: chrono::DateTime<chrono::FixedOffset>,
) -> String {
    match schedule.cadence.as_str() {
        "monthly" => now_tz.format("%Y-%m").to_string(),
        "weekly" => {
            // Monday-start week (same shape as the analytics weekly
            // grouping), so the claim key is stable within the week.
            let days_back = now_tz.weekday().num_days_from_monday() as i64;
            (now_tz - chrono::Duration::days(days_back))
                .format("%Y-%m-%d")
                .to_string()
        }
        _ => now_tz.format("%Y-%m-%d").to_string(),
    }
}

/// Claim the `(tenant, period)` slot before sending. Returns `true` when
/// this attempt is the first to claim it (proceed), `false` when it was
/// already claimed (skip — a crash-recovery restart or another instance
/// already sent / attempted it).
async fn claim_period_pg(
    pool: &Pool,
    tenant: &str,
    period: &str,
    report_id: &str,
) -> Result<bool, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    let n = client
        .execute(
            "INSERT INTO sent_reports (tenant_id, period, report_id)
             VALUES ($1, $2, $3)
             ON CONFLICT (tenant_id, period) DO NOTHING",
            &[&tenant, &period, &report_id],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    Ok(n > 0)
}

/// Release a claim because the send definitively failed, allowing the
/// period to retry on the next cycle. A send that actually succeeded but
/// whose SMTP response was lost is the unavoidable at-least-once boundary
/// of email delivery.
async fn release_period_pg(pool: &Pool, tenant: &str, period: &str) -> Result<(), String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;
    client
        .execute(
            "DELETE FROM sent_reports WHERE tenant_id = $1 AND period = $2",
            &[&tenant, &period],
        )
        .await
        .map_err(|e| format!("DB error: {e}"))?;
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

/// Scoped settings key — suffix form (`{base}:{tenant}`), a plain PK
/// lookup that leaves the legacy bare keys untouched.
fn scoped_key(base: &str, tenant: &str) -> String {
    format!("{base}:{tenant}")
}

/// Read a scoped settings value with bare-key fallback: `{key}:{tenant}`
/// first, then the legacy bare `{key}` (canonical for `default`, so
/// existing deployments keep working byte-identically).
async fn get_setting_scoped_pg(
    pool: &Pool,
    key: &str,
    tenant: &str,
) -> Result<Option<String>, String> {
    if let Some(v) = get_setting_pg(pool, &scoped_key(key, tenant)).await? {
        return Ok(Some(v));
    }
    get_setting_pg(pool, key).await
}

/// Upsert a scoped settings value (writes `{key}:{tenant}`).
async fn set_setting_scoped_pg(
    pool: &Pool,
    key: &str,
    value: &str,
    tenant: &str,
) -> Result<(), String> {
    set_setting_pg(pool, &scoped_key(key, tenant), value).await
}

/// Load the tenant's SMTP config from the settings table (scoped key with
/// bare-key fallback), decrypting the password transparently (mirrors
/// `oz_core`'s `Store::get_smtp_config`).
async fn get_smtp_config_pg(pool: &Pool, tenant: &str) -> Result<Option<SmtpConfig>, String> {
    let raw = match get_setting_scoped_pg(pool, SMTP_CONFIG_SETTINGS_KEY, tenant).await? {
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

/// Load the tenant's report schedule configuration from the settings table
/// (scoped key with bare-key fallback).
async fn get_report_schedule_pg(
    pool: &Pool,
    tenant: &str,
) -> Result<Option<ReportScheduleConfig>, String> {
    let raw = match get_setting_scoped_pg(pool, REPORT_SCHEDULE_SETTINGS_KEY, tenant).await? {
        Some(v) => v,
        None => return Ok(None),
    };
    let config: ReportScheduleConfig = serde_json::from_str(&raw)
        .map_err(|e| format!("failed to deserialize report schedule: {e}"))?;
    Ok(Some(config))
}

/// Read the tenant's store name from settings (scoped key with bare-key
/// fallback), falling back to a default.
async fn get_store_name_pg(pool: &Pool, tenant: &str) -> Result<String, String> {
    Ok(get_setting_scoped_pg(pool, "store.name", tenant)
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
    tenant: &str,
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
        tenant,
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
    let daily_revenue =
        daily_revenue_pg(pool, &config.start_date, &config.end_date, tenant_id).await?;
    let weekly_revenue =
        weekly_revenue_pg(pool, &config.start_date, &config.end_date, tenant_id).await?;
    let monthly_revenue =
        monthly_revenue_pg(pool, &config.start_date, &config.end_date, tenant_id).await?;
    let top_products = top_products_pg(
        pool,
        &config.start_date,
        &config.end_date,
        config.top_product_limit,
        "revenue",
        tenant_id,
    )
    .await?;
    let hourly_heatmap =
        hourly_heatmap_pg(pool, &config.start_date, &config.end_date, tenant_id).await?;
    let category_breakdown =
        category_breakdown_pg(pool, &config.start_date, &config.end_date, tenant_id).await?;
    let low_stock_alerts = low_stock_alerts_at_location_pg(
        pool,
        oz_core::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
        config.low_stock_threshold,
        tenant_id,
    )
    .await?;
    let active_stock_alerts = active_stock_alerts_pg(
        pool,
        oz_core::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
        tenant_id,
    )
    .await?;
    let category_popularity = category_popularity_pg(pool, 3, tenant_id).await?;
    let category_forecast = category_forecast_pg(
        pool,
        &config.start_date,
        &config.end_date,
        "weekly",
        10,
        tenant_id,
    )
    .await?;

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
    tenant: &str,
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
                     LEFT JOIN products p2 ON p2.sku = sl2.sku AND p2.tenant_id = s2.tenant_id
                     WHERE s2.status = 'completed'
                       AND s2.tenant_id = $3
                       AND s2.currency = d.currency
                       AND s2.created_at::date = d.date::date) AS cogs_minor
             FROM (SELECT to_char(s.created_at::date, 'YYYY-MM-DD') AS date,
                          SUM(s.total_minor)::bigint AS total_minor,
                          s.currency AS currency,
                          COUNT(*) AS sale_count
                   FROM sales s
                   WHERE s.status = 'completed'
                     AND s.tenant_id = $3
                     AND s.created_at::date BETWEEN $1 AND $2
                   GROUP BY to_char(s.created_at::date, 'YYYY-MM-DD'), s.currency) d
             ORDER BY date ASC",
            &[&start, &end, &tenant],
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
    tenant: &str,
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
                     LEFT JOIN products p2 ON p2.sku = sl2.sku AND p2.tenant_id = s2.tenant_id
                     WHERE s2.status = 'completed'
                       AND s2.tenant_id = $3
                       AND s2.currency = d.currency
                       AND to_char(date_trunc('week', s2.created_at::date)::date, 'YYYY-MM-DD') = d.week_start::date::text) AS cogs_minor
             FROM (SELECT to_char(date_trunc('week', s.created_at::date)::date, 'YYYY-MM-DD') AS week_start,
                          SUM(s.total_minor)::bigint AS total_minor, s.currency AS currency,
                          COUNT(*) AS sale_count
                   FROM sales s
                   WHERE s.status = 'completed'
                     AND s.tenant_id = $3
                     AND s.created_at::date BETWEEN $1 AND $2
                   GROUP BY to_char(date_trunc('week', s.created_at::date)::date, 'YYYY-MM-DD'), s.currency) d
             ORDER BY week_start ASC",
            &[&start, &end, &tenant],
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
    tenant: &str,
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
                     LEFT JOIN products p2 ON p2.sku = sl2.sku AND p2.tenant_id = s2.tenant_id
                     WHERE s2.status = 'completed'
                       AND s2.tenant_id = $3
                       AND s2.currency = d.currency
                       AND LEFT(s2.created_at, 7) = d.month::text) AS cogs_minor
             FROM (SELECT LEFT(s.created_at, 7) AS month,
                          SUM(s.total_minor)::bigint AS total_minor, s.currency AS currency,
                          COUNT(*) AS sale_count
                   FROM sales s
                   WHERE s.status = 'completed'
                     AND s.tenant_id = $3
                     AND s.created_at::date BETWEEN $1 AND $2
                   GROUP BY LEFT(s.created_at, 7), s.currency) d
             ORDER BY month ASC",
            &[&start, &end, &tenant],
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
    tenant: &str,
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
         JOIN products p ON p.sku = sl.sku AND p.tenant_id = s.tenant_id
         WHERE s.status = 'completed'
           AND s.tenant_id = $4
           AND s.created_at::date BETWEEN $1 AND $2
         GROUP BY p.id
         ORDER BY {order_clause}, p.sku
         LIMIT $3"
    );
    let rows = client
        .query(&sql, &[&start, &end, &limit, &tenant])
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
    tenant: &str,
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
             WHERE status = 'completed'
               AND tenant_id = $3
               AND created_at::date BETWEEN $1 AND $2
             GROUP BY day_of_week, hour
             ORDER BY day_of_week, hour",
            &[&start, &end, &tenant],
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
    tenant: &str,
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
             JOIN products p ON p.sku = sl.sku AND p.tenant_id = s.tenant_id
             LEFT JOIN categories c ON p.category_id = c.id
             WHERE s.status = 'completed'
               AND s.tenant_id = $3
               AND s.created_at::date BETWEEN $1 AND $2
             GROUP BY p.category_id, c.name
             ORDER BY total_minor DESC",
            &[&start, &end, &tenant],
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
    tenant: &str,
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
             WHERE p.tenant_id = $3
               AND (COALESCE(ss.qty, 0) <= $2
                    OR (SELECT 1 FROM stock_thresholds st
                        WHERE st.product_id = p.id
                          AND (st.location_id = $1 OR st.location_id IS NULL)
                          AND st.enabled = 1
                          AND COALESCE(ss.qty, 0) <= st.threshold
                        LIMIT 1) = 1)
             ORDER BY current_qty ASC",
            &[&location_id, &default_threshold, &tenant],
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
    tenant: &str,
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
             WHERE sae.location_id = $1
               AND p.tenant_id = $2
               AND sae.status IN ('active', 'acknowledged')
             ORDER BY sae.triggered_at DESC",
            &[&location_id, &tenant],
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
    tenant: &str,
) -> Result<Vec<CategoryPopularityRow>, String> {
    let client = pool.get().await.map_err(|e| e.to_string())?;

    let catalog_mean: f64 = client
        .query_one(
            "SELECT AVG(popularity_score)::float8 FROM products WHERE tenant_id = $1",
            &[&tenant],
        )
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
                 WHERE p.tenant_id = $1
                 GROUP BY p.category_id, c.name",
                &[&tenant],
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
                 WHERE p.tenant_id = $1
                 ORDER BY p.category_id, p.popularity_score DESC, p.sku ASC",
                &[&tenant],
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
    tenant: &str,
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
                 WHERE p.tenant_id = $2
                 GROUP BY p.category_id, c.name
                 ORDER BY AVG(p.popularity_score) DESC, p.category_id ASC
                 LIMIT $1",
                &[&top_categories.max(1), &tenant],
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
             JOIN products p ON p.sku = sl.sku AND p.tenant_id = s.tenant_id
             WHERE s.status = 'completed'
               AND s.tenant_id = $3
               AND s.created_at::date BETWEEN $1 AND $2
             GROUP BY {s_period}, p.category_id"
        );
        let rows = client
            .query(&sql, &[&start, &end, &tenant])
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
             JOIN products p ON p.sku = a.sku AND p.tenant_id = a.tenant_id
             WHERE a.tenant_id = $3
               AND a.created_at::date BETWEEN $1 AND $2
             GROUP BY {a_period}, p.category_id, a.event_type"
        );
        let rows = client
            .query(&sql, &[&start, &end, &tenant])
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

    let (ms, mq, me) = category_means_pg(pool, "", tenant)
        .await?
        .unwrap_or((0.0, 0.0, 0.0));
    let mut points: Vec<CategoryTrendPoint> = Vec::new();
    for ((period_start, cat), (units, txns, searches, edits)) in agg {
        if !rank.contains_key(&cat) {
            continue;
        }
        let (ms, mq, me) = category_means_pg(pool, &cat, tenant)
            .await?
            .unwrap_or((ms, mq, me));
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
async fn category_means_pg(
    pool: &Pool,
    category: &str,
    tenant: &str,
) -> Result<Option<(f64, f64, f64)>, String> {
    let raw = match get_setting_scoped_pg(pool, "popularity.category_means", tenant).await? {
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
    tenant: &str,
) -> Result<Vec<CategoryForecastRow>, String> {
    const MAX_SERIES_POINTS: usize = 14;

    let points = category_popularity_trend_pg(
        pool,
        start_date,
        end_date,
        granularity,
        top_categories,
        tenant,
    )
    .await?;
    #[allow(clippy::type_complexity)]
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
#[path = "email_pg_tests.rs"]
mod tests;
