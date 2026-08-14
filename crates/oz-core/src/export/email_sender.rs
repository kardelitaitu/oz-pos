//! Shared email scheduling & sending logic — used by both desktop-client
//! and cloud-server.
//!
//! ## Responsibilities
//!
//! * **Cadence + timezone-aware scheduling** — `should_send_scheduled()`
//!   respects the configured cadence ("daily"/"weekly"/"monthly"),
//!   timezone, and deduplicates via a `last_report_sent_at` setting.
//! * **Report-type filtering** — `filter_analytics_bundle()` zeros out
//!   sections that aren't in the user's checked report_types list.
//! * **Shared SMTP transport** — `build_smtp_transport()` is the single
//!   implementation used by cloud-server, desktop-client commands, and
//!   the desktop background scheduler.
//! * **Send pipeline** — `generate_filtered_report_email()` loads config,
//!   checks schedule, generates + filters + sends in one call.
//!
//! ## Settings keys used
//!
//! | Key | Purpose |
//! |-----|---------|
//! | `last_report_sent_at` | ISO-8601 timestamp of last successful send |
//! | `smtp_config` | SMTP server parameters (JSON) |
//! | `report_schedule` | Schedule config (JSON) |

use chrono::{Datelike, NaiveTime, Timelike, Utc};
use tracing::{info, warn};

use super::email_report::{ReportEmailBuilder, SmtpConfig};
use super::{AnalyticsBundle, ExportConfig, ReportScheduleConfig};
use crate::db::Store;
use crate::error::CoreError;

/// Settings key for deduplication tracking.
pub const LAST_SENT_KEY: &str = "last_report_sent_at";

// ── Shared SMTP transport builder ────────────────────────────────────

/// Build an async SMTP transport from the configuration.
///
/// Uses STARTTLS via [`lettre::AsyncSmtpTransport::relay`] when
/// `use_tls` is enabled or port is 465. Falls back to plaintext
/// via `builder_dangerous` otherwise.
///
/// # Errors
///
/// Returns a human-readable error string if the relay hostname is
/// invalid or the TLS handshake setup fails.
pub fn build_smtp_transport(
    config: &SmtpConfig,
) -> Result<lettre::AsyncSmtpTransport<lettre::Tokio1Executor>, String> {
    use lettre::transport::smtp::authentication::Credentials;

    let creds = match (&config.username, &config.password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => {
            Some(Credentials::new(u.clone(), p.clone()))
        }
        _ => None,
    };

    if config.use_tls || config.port == 465 {
        let relay = lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&config.host)
            .map_err(|e| format!("Failed to build TLS SMTP transport to {}: {e}", config.host))?;
        let relay = if let Some(c) = creds {
            relay.credentials(c)
        } else {
            relay
        };
        Ok(relay.port(config.port).build())
    } else {
        let builder =
            lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::builder_dangerous(&config.host)
                .port(config.port);
        let builder = if let Some(c) = creds {
            builder.credentials(c)
        } else {
            builder
        };
        Ok(builder.build())
    }
}

/// Check whether a scheduled report should be sent now.
///
/// Reads `last_report_sent_at` from settings to prevent duplicate sends
/// within the same period. Respects the configured cadence and timezone.
///
/// # Cadence logic
///
/// | Cadence | Rule |
/// |---------|------|
/// | `daily` | Send every day at `send_at_time` |
/// | `weekly` | Send on Mondays at `send_at_time` |
/// | `monthly` | Send on the 1st at `send_at_time` |
///
/// # Deduplication
///
/// After a successful send the caller should call
/// [`record_sent_timestamp`] to persist the current time under
/// `last_report_sent_at`. This function compares the current date
/// against the last-sent date — same-date means already sent.
pub fn should_send_scheduled(
    store: &Store<'_>,
    schedule: &ReportScheduleConfig,
) -> Result<bool, CoreError> {
    let last_sent = store.get_setting(LAST_SENT_KEY)?;
    should_send_scheduled_with_last_sent(schedule, last_sent)
}

/// Store-free variant of [`should_send_scheduled`] — same cadence +
/// timezone + dedup logic, with the last-sent timestamp supplied by the
/// caller. Lets the cloud server's Postgres report loop reuse this logic
/// without a synchronous rusqlite `Store`.
pub fn should_send_scheduled_with_last_sent(
    schedule: &ReportScheduleConfig,
    last_sent: Option<String>,
) -> Result<bool, CoreError> {
    // Parse send time (HH:MM)
    let send_time =
        NaiveTime::parse_from_str(&schedule.send_at_time, "%H:%M").unwrap_or_else(|_| {
            // If the stored time is malformed, default to 08:00
            // SAFETY: 08:00 is a compile-time constant, always a valid NaiveTime.
            NaiveTime::from_hms_opt(8, 0, 0).unwrap()
        });

    // Resolve current time in the configured timezone.
    // Fall back to UTC if the timezone name is unknown.
    let now_tz = resolve_now_in_timezone(&schedule.timezone);
    // SAFETY: chrono guarantees hour() in 0..24 and minute() in 0..60, so
    // from_hms_opt cannot fail here.
    let current_time = NaiveTime::from_hms_opt(now_tz.hour(), now_tz.minute(), 0).unwrap();

    // Check if it's the right time of day (within a 2-minute window, since
    // the scheduler polls every 60s).
    let diff_seconds = (current_time.num_seconds_from_midnight() as i64
        - send_time.num_seconds_from_midnight() as i64)
        .abs();
    if diff_seconds > 120 {
        return Ok(false);
    }

    // Check cadence
    let weekday = now_tz.weekday();
    let day_of_month = now_tz.day();

    // Check cadence: weekly → Mondays only, monthly → 1st only
    match schedule.cadence.as_str() {
        "weekly" if weekday != chrono::Weekday::Mon => return Ok(false),
        "monthly" if day_of_month != 1 => return Ok(false),
        _ => { /* daily — send every day */ }
    }

    // Deduplication: check last_sent_at
    let today = now_tz.format("%Y-%m-%d").to_string();
    let last_sent = last_sent.unwrap_or_default();

    // Extract date portion of last_sent ISO-8601 timestamp
    let last_date = last_sent.chars().take(10).collect::<String>();
    if last_date == today {
        info!("Report already sent today ({today}), skipping");
        return Ok(false);
    }

    Ok(true)
}

/// Record a successful send timestamp to prevent duplicate sends.
pub fn record_sent_timestamp(store: &Store<'_>) -> Result<(), CoreError> {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    store.set_setting(LAST_SENT_KEY, &now)
}

/// Filter an analytics bundle to only include the report types listed
/// in `report_types`. Unchecked types are replaced with empty vectors.
///
/// The email builder checks `is_empty()` before rendering each section,
/// so zeroing out unchecked sections is equivalent to filtering.
pub fn filter_analytics_bundle(bundle: &mut AnalyticsBundle, report_types: &[String]) {
    let has = |key: &str| report_types.iter().any(|r| r == key);

    if !has("daily_revenue") {
        bundle.daily_revenue.clear();
    }
    if !has("weekly_revenue") {
        bundle.weekly_revenue.clear();
    }
    if !has("monthly_revenue") {
        bundle.monthly_revenue.clear();
    }
    if !has("top_products") {
        bundle.top_products.clear();
    }
    if !has("hourly_heatmap") {
        bundle.hourly_heatmap.clear();
    }
    if !has("category_breakdown") {
        bundle.category_breakdown.clear();
    }
    if !has("low_stock_alerts") {
        bundle.low_stock_alerts.clear();
        bundle.active_stock_alerts.clear();
    }
}

/// Generate a filtered report email for the scheduled period.
///
/// Loads the schedule's lookback window, exports analytics, filters by
/// report_types, and builds the email.
pub fn generate_filtered_report_email(
    store: &Store<'_>,
    schedule: &ReportScheduleConfig,
    store_name: &str,
) -> Result<super::email_report::ReportEmail, CoreError> {
    let lookback_start = Utc::now()
        .checked_sub_signed(chrono::Duration::days(schedule.lookback_days as i64))
        .unwrap_or(Utc::now())
        .format("%Y-%m-%d")
        .to_string();
    let end = Utc::now().format("%Y-%m-%d").to_string();

    let mut bundle: AnalyticsBundle = store
        .export_analytics_bundle(
            ExportConfig {
                start_date: lookback_start.clone(),
                end_date: end.clone(),
                ..ExportConfig::default()
            },
            "",
            store_name,
        )
        .map_err(|e| CoreError::Internal(format!("Failed to export analytics: {e}")))?;

    // Filter out unchecked report types
    filter_analytics_bundle(&mut bundle, &schedule.report_types);

    let date_label = format!("{} to {}", lookback_start, end);
    Ok(ReportEmailBuilder::build(&bundle, store_name, &date_label))
}

/// Resolve the current date-time in the given IANA timezone name.
///
/// Falls back to UTC if the timezone name is unrecognised or parsing fails.
/// Resolve the current instant in a named IANA timezone (falling back to
/// UTC when the name is unknown).
pub fn resolve_now_in_timezone(tz_name: &str) -> chrono::DateTime<chrono::FixedOffset> {
    // Try well-known timezone abbreviations and IANA names.
    // For a full implementation, use the `chrono-tz` crate.
    // This function handles the most common cases.
    match tz_name.to_lowercase().as_str() {
        "utc" | "gmt" | "" => Utc::now().into(),
        // Common fixed offsets
        s if s.starts_with("utc+")
            || s.starts_with("utc-")
            || s.starts_with("gmt+")
            || s.starts_with("gmt-") =>
        {
            let offset_str = &s[3..];
            if let Ok(hours) = offset_str.parse::<i32>()
                && let Some(offset) = chrono::FixedOffset::east_opt(hours * 3600)
            {
                return Utc::now().with_timezone(&offset);
            }
            Utc::now().into()
        }
        // Common IANA timezones
        "asia/jakarta" | "asia/pontianak" => {
            // SAFETY: +07:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(7 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "asia/makassar" | "asia/singapore" | "asia/kuala_lumpur" | "asia/manila"
        | "asia/brunei" => {
            // SAFETY: +08:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "asia/tokyo" | "asia/seoul" => {
            // SAFETY: +09:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "asia/shanghai" | "asia/taipei" | "asia/hong_kong" => {
            // SAFETY: +08:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "australia/sydney" => {
            // SAFETY: +10:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(10 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "europe/london" | "europe/lisbon" => {
            // GMT/BST handled approximately as UTC (DST not tracked without chrono-tz)
            Utc::now().into()
        }
        "europe/berlin" | "europe/paris" | "europe/rome" | "europe/madrid" | "europe/amsterdam" => {
            // SAFETY: +01:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "europe/moscow" => {
            // SAFETY: +03:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(3 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "america/new_york" | "america/toronto" => {
            // SAFETY: -05:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(-5 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "america/chicago" => {
            // SAFETY: -06:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(-6 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "america/denver" => {
            // SAFETY: -07:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(-7 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        "america/los_angeles" | "america/vancouver" => {
            // SAFETY: -08:00 fixed offset is a compile-time constant within chrono's ±23:59:59 offset range.
            let offset = chrono::FixedOffset::east_opt(-8 * 3600).unwrap();
            Utc::now().with_timezone(&offset)
        }
        _ => {
            warn!("Unrecognised timezone '{tz_name}', falling back to UTC");
            Utc::now().into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    #[test]
    fn should_send_daily_within_window() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        // send_at_time to current UTC time ± 1 min (within 2-min window)
        let now = Utc::now();
        let schedule = ReportScheduleConfig {
            enabled: true,
            cadence: "daily".to_string(),
            send_at_time: now.format("%H:%M").to_string(),
            timezone: "UTC".to_string(),
            ..Default::default()
        };

        // Should return true (no last_sent_at record yet)
        let result = should_send_scheduled(&store, &schedule).unwrap();
        assert!(
            result,
            "should send when within time window and no prior send"
        );
    }

    #[test]
    fn should_send_daily_outside_window() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        // Set a time far from now (3 hours before current)
        let schedule = ReportScheduleConfig {
            enabled: true,
            cadence: "daily".to_string(),
            send_at_time: Utc::now()
                .checked_sub_signed(chrono::Duration::hours(3))
                .unwrap()
                .format("%H:%M")
                .to_string(),
            timezone: "UTC".to_string(),
            ..Default::default()
        };

        let result = should_send_scheduled(&store, &schedule).unwrap();
        assert!(!result, "should NOT send when outside time window");
    }

    #[test]
    fn dedup_blocks_same_day() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        // Record a send "today"
        record_sent_timestamp(&store).unwrap();

        let now = Utc::now();
        let schedule = ReportScheduleConfig {
            enabled: true,
            cadence: "daily".to_string(),
            send_at_time: now.format("%H:%M").to_string(),
            timezone: "UTC".to_string(),
            ..Default::default()
        };

        let result = should_send_scheduled(&store, &schedule).unwrap();
        assert!(!result, "dedup should block same-day resend");
    }

    #[test]
    fn weekly_blocks_non_monday() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        // Set send time to current time so time check passes
        let now = Utc::now();
        let schedule = ReportScheduleConfig {
            enabled: true,
            cadence: "weekly".to_string(),
            send_at_time: now.format("%H:%M").to_string(),
            timezone: "UTC".to_string(),
            ..Default::default()
        };

        let result = should_send_scheduled(&store, &schedule).unwrap();

        // Only true if today is Monday AND time window matches
        let is_monday = now.weekday() == chrono::Weekday::Mon;
        assert_eq!(result, is_monday, "weekly should only send on Mondays");
    }

    #[test]
    fn monthly_blocks_non_first() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        let now = Utc::now();
        let schedule = ReportScheduleConfig {
            enabled: true,
            cadence: "monthly".to_string(),
            send_at_time: now.format("%H:%M").to_string(),
            timezone: "UTC".to_string(),
            ..Default::default()
        };

        let result = should_send_scheduled(&store, &schedule).unwrap();
        let is_first = now.day() == 1;
        assert_eq!(result, is_first, "monthly should only send on the 1st");
    }

    #[test]
    fn filter_removes_unchecked_types() {
        use crate::export::ExportMetadata;

        let mut bundle = AnalyticsBundle {
            metadata: ExportMetadata {
                exported_at: "".into(),
                tenant_id: "".into(),
                store_name: "".into(),
                version: "".into(),
            },
            daily_revenue: vec![crate::db::reports::DailyRevenueRow {
                date: "2026-01-01".into(),
                total_minor: 1000,
                currency: "USD".into(),
                sale_count: 1,
                cogs_minor: 0,
                gross_profit_minor: 1000,
                gross_margin_percent: 100.0,
            }],
            weekly_revenue: vec![crate::db::reports::WeeklyRevenueRow {
                week_start: "2026-01-01".into(),
                total_minor: 1000,
                currency: "USD".into(),
                sale_count: 1,
                cogs_minor: 0,
                gross_profit_minor: 1000,
                gross_margin_percent: 100.0,
            }],
            monthly_revenue: vec![],
            top_products: vec![],
            hourly_heatmap: vec![],
            category_breakdown: vec![],
            low_stock_alerts: vec![],
            active_stock_alerts: vec![],
            category_popularity: vec![],
            category_forecast: vec![],
        };

        // Only include weekly_revenue
        let types = vec!["weekly_revenue".to_string()];
        filter_analytics_bundle(&mut bundle, &types);

        assert!(
            bundle.daily_revenue.is_empty(),
            "daily_revenue should be filtered out"
        );
        assert!(
            !bundle.weekly_revenue.is_empty(),
            "weekly_revenue should be kept"
        );
    }

    #[test]
    fn record_and_read_timestamp() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        record_sent_timestamp(&store).unwrap();
        let val = store.get_setting(LAST_SENT_KEY).unwrap();
        assert!(val.is_some());
        assert!(val.unwrap().starts_with("20")); // ISO-8601 year
    }
}
