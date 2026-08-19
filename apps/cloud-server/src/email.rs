//! Email report delivery — background scheduled send loop + SMTP transport.
//!
//! ## Background task
//!
//! [`start_report_sender_loop`] spawns a tokio task that polls every 60
//! seconds. Scheduling logic (cadence, timezone, dedup) and report-type
//! filtering are delegated to [`oz_core::export::email_sender`].
//!
//! ## Test send
//!
//! [`send_test_report`] is a public function that the Tauri desktop client
//! can invoke to validate SMTP configuration without waiting for the
//! scheduled window.

use std::sync::Arc;

use lettre::message::header::ContentType;
use lettre::{AsyncTransport, Message};
use oz_core::{
    Store,
    export::{
        email_report::{ReportEmail, SmtpConfig},
        email_sender,
    },
};
use tracing::{error, info};

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

    // Validate config early so the caller gets a clear error.
    smtp_config
        .validate()
        .map_err(|e| format!("Invalid SMTP config: {e}"))?;

    let transport = email_sender::build_smtp_transport(smtp_config)?;

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

// ── Background scheduled send loop ──────────────────────────────────

/// Start the background task that polls every 60s and sends scheduled
/// report emails when the configured send time arrives.
///
/// Uses [`email_sender::should_send_scheduled`] for cadence + timezone +
/// dedup logic and [`email_sender::generate_filtered_report_email`] for
/// report-type-aware generation.
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

/// Try to send a scheduled report — checks the schedule config via the
/// shared scheduler, generates a filtered report, and sends.
async fn try_send_scheduled(
    db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
) -> Result<(), String> {
    // Scope 1: Read SMTP + schedule config, check schedule.
    let (smtp_config, schedule, recipients, should_send) = {
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

        // Use shared scheduler for cadence + timezone + dedup
        let should_send = email_sender::should_send_scheduled(&store, &schedule)
            .map_err(|e| format!("Schedule check failed: {e}"))?;

        (
            smtp_config,
            schedule.clone(),
            schedule.recipients.clone(),
            should_send,
        )
    };

    if !should_send {
        return Ok(());
    }

    // Scope 2: Generate filtered report
    let store_name = {
        let conn = db.lock().await;
        let store = Store::new(&conn);
        get_store_name(&store).unwrap_or_else(|_| "OZ-POS Store".to_string())
    };

    let report = {
        let conn = db.lock().await;
        let store = Store::new(&conn);
        email_sender::generate_filtered_report_email(&store, &schedule, &store_name)
            .map_err(|e| format!("Failed to generate report: {e}"))?
    };

    send_email(&smtp_config, &report, &recipients).await?;

    // Record successful send for dedup
    {
        let conn = db.lock().await;
        let store = Store::new(&conn);
        email_sender::record_sent_timestamp(&store)
            .map_err(|e| format!("Failed to record send timestamp: {e}"))?;
    }

    info!(
        "Scheduled report sent to {} recipients (cadence: {}, types: {:?})",
        recipients.len(),
        schedule.cadence,
        schedule.report_types,
    );

    Ok(())
}

/// Send a test report immediately (used by the Tauri desktop client for
/// the "Send Test Report" button in Settings).
///
/// Uses the filtered report generator so the test email reflects the
/// user's report_type selections.
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

    let store_name = get_store_name(&store).unwrap_or_else(|_| "OZ-POS Store".to_string());

    let report = email_sender::generate_filtered_report_email(&store, &schedule, &store_name)
        .map_err(|e| format!("Failed to generate report: {e}"))?;
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
#[path = "email_tests.rs"]
mod tests;
