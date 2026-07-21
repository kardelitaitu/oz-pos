//! Email report delivery — background scheduled send loop + SMTP transport.
//!
//! ## Background task
//!
//! [`start_report_sender_loop`] spawns a tokio task that polls every 60
//! seconds. When the current time matches the configured `send_at_time`
//! for the active cadence, it generates an analytics bundle and sends it
//! to all configured recipients.
//!
//! ## Test send
//!
//! [`send_test_report`] is a public function that the Tauri desktop client
//! can invoke to validate SMTP configuration without waiting for the
//! scheduled window.

use std::sync::Arc;

use lettre::message::header::ContentType;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use oz_core::{
    Store,
    export::{
        AnalyticsBundle, ExportConfig, ReportScheduleConfig,
        email_report::{ReportEmail, ReportEmailBuilder, SmtpConfig},
    },
};
use tracing::{error, info, warn};

// ── SMTP transport builder ───────────────────────────────────────────

/// Build an async SMTP transport from the configuration.
///
/// Returns `None` when the config is missing or invalid (logged as a
/// warning so the server continues running without crashing).
fn build_transport(config: &SmtpConfig) -> Option<AsyncSmtpTransport<Tokio1Executor>> {
    if let Err(e) = config.validate() {
        warn!("SMTP config invalid, cannot build transport: {e}");
        return None;
    }

    let creds = match (&config.username, &config.password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => {
            Some(Credentials::new(u.clone(), p.clone()))
        }
        _ => None,
    };

    let transport = if config.use_tls || config.port == 465 {
        // Try STARTTLS on standard ports; port 465 often uses implicit TLS
        match AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host) {
            Ok(relay) => {
                let relay = if let Some(c) = creds {
                    relay.credentials(c)
                } else {
                    relay
                };
                relay.port(config.port).build()
            }
            Err(e) => {
                warn!("Failed to build TLS SMTP transport to {}: {e}", config.host);
                return None;
            }
        }
    } else {
        // Plain SMTP without encryption
        let transport =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host).port(config.port);
        let transport = if let Some(c) = creds {
            transport.credentials(c)
        } else {
            transport
        };
        transport.build()
    };

    Some(transport)
}

// ── Send a single email ──────────────────────────────────────────────

/// Send an email report via SMTP.
///
/// # Errors
///
/// Returns a human-readable error string on any SMTP failure
/// (connection refused, auth failure, timeout, etc.).
pub async fn send_email(
    smtp_config: &SmtpConfig,
    email: &ReportEmail,
    to: &[String],
) -> Result<(), String> {
    if to.is_empty() {
        return Err("No recipients configured".into());
    }

    let transport =
        build_transport(smtp_config).ok_or_else(|| "Invalid SMTP config".to_string())?;

    for recipient in to {
        let msg = Message::builder()
            .from(
                smtp_config
                    .from
                    .parse()
                    .map_err(|e| format!("Invalid from address: {e}"))?,
            )
            .to(recipient
                .parse()
                .map_err(|e| format!("Invalid recipient '{recipient}': {e}"))?)
            .subject(&email.subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(email.text_body.clone()),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(email.html_body.clone()),
                    ),
            )
            .map_err(|e| format!("Failed to build email message: {e}"))?;

        transport
            .send(msg)
            .await
            .map_err(|e| format!("SMTP send failed: {e}"))?;
        info!("Report email sent to {recipient}");
    }

    Ok(())
}

// ── Generate a report email from the DB ─────────────────────────────

/// Generate a report email for the current period.
///
/// Loads the report schedule config to determine the lookback window,
/// exports an analytics bundle, and builds the email.
pub fn generate_report_email(
    store: &Store<'_>,
    schedule: &ReportScheduleConfig,
) -> Result<ReportEmail, String> {
    let lookback_start = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(schedule.lookback_days as i64))
        .ok_or_else(|| "Failed to compute lookback date".to_string())?
        .format("%Y-%m-%d")
        .to_string();
    let end = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let store_name = get_store_name(store).unwrap_or_else(|_| "OZ-POS Store".to_string());

    let bundle: AnalyticsBundle = store
        .export_analytics_bundle(
            ExportConfig {
                start_date: lookback_start.clone(),
                end_date: end.clone(),
                ..ExportConfig::default()
            },
            "",
            &store_name,
        )
        .map_err(|e| format!("Failed to export analytics: {e}"))?;

    let date_label = format!("{} to {}", lookback_start, end);
    Ok(ReportEmailBuilder::build(&bundle, &store_name, &date_label))
}

// ── Background scheduled send loop ──────────────────────────────────

/// Start the background task that polls every 60s and sends scheduled
/// report emails when the configured send time arrives.
///
/// The task holds an `Arc<Mutex<Connection>>` — a lightweight clone of
/// the same database handle used by the HTTP handlers. The lock is
/// acquired only during checks and sends (typically < 100ms).
pub fn start_report_sender_loop(db: Arc<tokio::sync::Mutex<rusqlite::Connection>>) {
    tokio::spawn(async move {
        info!("Report sender background loop started (poll interval: 60s)");

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;

            if let Err(e) = try_send_scheduled(db.clone()).await {
                error!("Report sender loop error: {e}");
            }
        }
    });
}

/// Try to send a scheduled report — checks the schedule config and
/// sends if the time matches.
async fn try_send_scheduled(
    db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
) -> Result<(), String> {
    // Scope 1: Read SMTP + schedule config, drop non-Send Store/Connection
    // before any .await to satisfy tokio::spawn Send bounds.
    let (smtp_config, schedule, recipients) = {
        let conn = db.lock().await;
        let store = Store::new(&conn);

        let smtp_config = match store
            .get_smtp_config()
            .map_err(|e| format!("DB error: {e}"))?
        {
            Some(c) => c,
            None => return Ok(()),
        };

        let schedule = match store
            .get_report_schedule()
            .map_err(|e| format!("DB error: {e}"))?
        {
            Some(s) if s.enabled => s,
            _ => return Ok(()),
        };

        (smtp_config, schedule.clone(), schedule.recipients.clone())
    };

    // Check if it's time to send
    let now = chrono::Utc::now();
    let current_time = now.format("%H:%M").to_string();

    if current_time != schedule.send_at_time {
        return Ok(());
    }

    // Scope 2: Generate report
    let report = {
        let conn = db.lock().await;
        let store = Store::new(&conn);
        generate_report_email(&store, &schedule)?
    };

    send_email(&smtp_config, &report, &recipients).await?;

    info!(
        "Scheduled report sent to {} recipients (cadence: {})",
        recipients.len(),
        schedule.cadence,
    );

    Ok(())
}

/// Send a test report immediately (used by the Tauri desktop client for
/// the "Send Test Report" button in Settings).
///
/// Loads SMTP config and the report schedule config, generates a report
/// for the lookback window, and sends it to all configured recipients.
/// Returns a success message or an error string.
#[allow(dead_code)]
pub async fn send_test_report(
    db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
) -> Result<String, String> {
    let conn = db.lock().await;
    let store = Store::new(&conn);

    let smtp_config = store
        .get_smtp_config()
        .map_err(|e| format!("Failed to load SMTP config: {e}"))?
        .ok_or_else(|| "SMTP not configured. Please save SMTP settings first.".to_string())?;

    let schedule = store
        .get_report_schedule()
        .map_err(|e| format!("Failed to load report schedule: {e}"))?
        .unwrap_or_default();

    let recipients = if schedule.recipients.is_empty() {
        vec![smtp_config.from.clone()]
    } else {
        schedule.recipients.clone()
    };

    let report = generate_report_email(&store, &schedule)?;
    drop(store);
    drop(conn);

    send_email(&smtp_config, &report, &recipients).await?;

    Ok(format!(
        "Test report sent to {} recipient(s)",
        recipients.len()
    ))
}

// ── Helper to read store name ───────────────────────────────────────

/// Read the store name from settings, falling back to a default.
fn get_store_name(store: &Store<'_>) -> Result<String, String> {
    use oz_core::settings::Settings;
    let name = Settings::get(store.conn, "store.name").map_err(|e| format!("DB error: {e}"))?;
    Ok(name.unwrap_or_else(|| "OZ-POS Store".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use std::sync::Arc;

    #[tokio::test]
    async fn generate_report_email_smoke() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        let schedule = ReportScheduleConfig {
            enabled: true,
            lookback_days: 7,
            ..ReportScheduleConfig::default()
        };

        let result = generate_report_email(&store, &schedule);
        assert!(result.is_ok(), "should generate email: {:?}", result.err());
        let email = result.unwrap();
        assert!(email.subject.contains("OZ-POS Report"));
        assert!(email.html_body.contains("<table") || email.html_body.contains("<p>"));
        assert!(email.text_body.contains("OZ-POS Report"));
    }

    #[tokio::test]
    async fn generate_report_email_empty_db() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        let schedule = ReportScheduleConfig::default();
        let result = generate_report_email(&store, &schedule);
        assert!(
            result.is_ok(),
            "empty DB should still generate: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn send_test_report_fails_without_smtp_config() {
        let conn = migrations::fresh_db();
        let db = Arc::new(tokio::sync::Mutex::new(conn));
        let result = send_test_report(db).await;
        assert!(result.is_err(), "should fail without SMTP config");
        assert!(result.unwrap_err().contains("SMTP not configured"));
    }
}
