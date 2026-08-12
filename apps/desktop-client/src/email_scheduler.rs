//! Background email report scheduler for the desktop client.
//!
//! Polls every 60 seconds and sends scheduled report emails using the
//! shared [`oz_core::export::email_sender`] module for cadence, timezone,
//! deduplication, and report-type filtering logic.

use oz_core::Store;
use oz_core::export::email_sender;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Run the email report scheduler loop indefinitely.
///
/// Polls the database every 60 seconds. When the shared scheduler
/// determines it's time to send, generates a filtered report and
/// delivers it via SMTP.
pub async fn run_scheduler_loop(db: Arc<tokio::sync::Mutex<rusqlite::Connection>>) {
    let poll_interval = std::time::Duration::from_secs(60);
    info!("Email report scheduler started (poll interval: 60s)");

    loop {
        tokio::time::sleep(poll_interval).await;

        if let Err(e) = try_send_scheduled(&db).await {
            error!("Email report scheduler error: {e}");
        }
    }
}

async fn try_send_scheduled(
    db: &Arc<tokio::sync::Mutex<rusqlite::Connection>>,
) -> Result<(), String> {
    // Scope 1: Read config and check schedule
    let (smtp_config, schedule, should_send) = {
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

        let should_send = email_sender::should_send_scheduled(&store, &schedule)
            .map_err(|e| format!("Schedule check: {e}"))?;

        (smtp_config, schedule, should_send)
    };

    if !should_send {
        return Ok(());
    }

    // Scope 2: Generate filtered report
    let (report, recipients) = {
        let conn = db.lock().await;
        let store = Store::new(&conn);
        let name = oz_core::Settings::get(store.conn, "store.name")
            .ok()
            .flatten()
            .unwrap_or_else(|| "OZ-POS Store".to_string());
        let report = email_sender::generate_filtered_report_email(&store, &schedule, &name)
            .map_err(|e| format!("Report gen: {e}"))?;
        (report, schedule.recipients.clone())
    };

    if recipients.is_empty() {
        warn!("No recipients configured, skipping scheduled send");
        return Ok(());
    }

    // Scope 3: Build transport and send
    send_email_via_smtp(&smtp_config, &report, &recipients).await?;

    // Record successful send for dedup
    {
        let conn = db.lock().await;
        let store = Store::new(&conn);
        email_sender::record_sent_timestamp(&store)
            .map_err(|e| format!("Record timestamp: {e}"))?;
    }

    info!(
        "Scheduled report sent to {} recipients (cadence: {})",
        recipients.len(),
        schedule.cadence,
    );

    Ok(())
}

/// Send a single email report via SMTP using the shared transport builder.
async fn send_email_via_smtp(
    config: &oz_core::export::email_report::SmtpConfig,
    email: &oz_core::export::email_report::ReportEmail,
    to: &[String],
) -> Result<(), String> {
    use lettre::AsyncTransport;

    let transport = email_sender::build_smtp_transport(config)?;

    for recipient in to {
        let msg = lettre::Message::builder()
            .from(
                config
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
                            .header(lettre::message::header::ContentType::TEXT_PLAIN)
                            .body(email.text_body.clone()),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_HTML)
                            .body(email.html_body.clone()),
                    ),
            )
            .map_err(|e| format!("Failed to build email: {e}"))?;

        transport
            .send(msg)
            .await
            .map_err(|e| format!("SMTP send to {recipient}: {e}"))?;
        info!("Scheduled report sent to {recipient}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::db::Store;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn test_db() -> Arc<tokio::sync::Mutex<Connection>> {
        let conn = migrations::fresh_db();
        Arc::new(tokio::sync::Mutex::new(conn))
    }

    #[tokio::test]
    async fn skips_when_no_smtp_config() {
        let db = test_db();
        let result = try_send_scheduled(&db).await;
        assert!(
            result.is_ok(),
            "should return Ok when SMTP is not configured"
        );
    }

    #[tokio::test]
    async fn skips_when_schedule_disabled() {
        let db = test_db();

        // Seed SMTP config so it doesn't bail at that gate.
        {
            let conn = db.lock().await;
            let store = Store::new(&conn);
            store
                .set_setting("smtp.host", "localhost")
                .expect("set smtp host");
            store
                .set_setting("smtp.port", "587")
                .expect("set smtp port");
            store
                .set_setting("smtp.username", "user")
                .expect("set smtp user");
            store
                .set_setting("smtp.password", "pass")
                .expect("set smtp pass");
            store
                .set_setting("smtp.from", "pos@example.com")
                .expect("set smtp from");
            // No report schedule set — get_report_schedule returns None.
        }

        let result = try_send_scheduled(&db).await;
        assert!(
            result.is_ok(),
            "should return Ok when no report schedule is configured"
        );
    }

    #[tokio::test]
    async fn error_from_db_is_propagated() {
        let db = test_db();

        // Corrupt the settings table so get_smtp_config fails.
        {
            let conn = db.lock().await;
            conn.execute_batch("DROP TABLE settings")
                .expect("drop settings");
        }

        let result = try_send_scheduled(&db).await;
        assert!(result.is_err(), "should propagate DB errors");
    }
}
