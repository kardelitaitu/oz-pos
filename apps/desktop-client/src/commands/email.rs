//! Email report commands — send test reports and manage SMTP config.
//!
//! These commands allow the settings UI to validate SMTP connectivity
//! by sending a test report email immediately.

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

/// Send a test report email using the currently configured SMTP
/// settings and report schedule.
///
/// This is a Tauri IPC command, invoked from the Settings → Reports
/// screen's "Send Test Report" button.
///
/// # Returns
///
/// A success message string on completion, or an [`AppError`] on
/// failure (invalid config, SMTP connection refused, etc.).
#[tauri::command]
pub async fn send_test_report(state: State<'_, AppState>) -> Result<String, AppError> {
    let db = state.db.clone();

    // Scope DB operations to drop MutexGuard<Connection> before any .await
    // (rusqlite::Connection is !Send, so the guard makes the future !Send
    // if held across an await point).
    let (smtp_config, recipients, report_email) = {
        let conn = db.lock().await;
        let store = oz_core::Store::new(&conn);

        // Load SMTP config
        let smtp_config = store
            .get_smtp_config()
            .map_err(|e| AppError::Internal(format!("Failed to load SMTP config: {e}")))?
            .ok_or_else(|| {
                AppError::Internal("SMTP not configured. Please save SMTP settings first.".into())
            })?;

        // Load schedule config (or use defaults)
        let schedule = store
            .get_report_schedule()
            .map_err(|e| AppError::Internal(format!("Failed to load report schedule: {e}")))?
            .unwrap_or_default();

        let recipients = if schedule.recipients.is_empty() {
            vec![smtp_config.from.clone()]
        } else {
            schedule.recipients.clone()
        };

        // Generate report email
        let lookback_days = schedule.lookback_days.max(1);
        let lookback_start = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(lookback_days as i64))
            .ok_or_else(|| AppError::Internal("Failed to compute lookback date".into()))?
            .format("%Y-%m-%d")
            .to_string();
        let end = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let store_name = oz_core::Settings::get(store.conn, "store.name")
            .ok()
            .flatten()
            .unwrap_or_else(|| "OZ-POS Store".to_string());

        let bundle = store
            .export_analytics_bundle(
                oz_core::export::ExportConfig {
                    start_date: lookback_start.clone(),
                    end_date: end.clone(),
                    ..oz_core::export::ExportConfig::default()
                },
                "",
                &store_name,
            )
            .map_err(|e| AppError::Internal(format!("Failed to export analytics: {e}")))?;

        let date_label = format!("{} to {}", lookback_start, end);
        let report_email = oz_core::export::email_report::ReportEmailBuilder::build(
            &bundle,
            &store_name,
            &date_label,
        );

        (smtp_config, recipients, report_email)
    }; // conn + store dropped here — now safe to .await

    // Build SMTP transport and send
    let transport = build_smtp_transport(&smtp_config)
        .map_err(|e| AppError::Internal(format!("SMTP transport failed: {e}")))?;

    for recipient in &recipients {
        use lettre::AsyncTransport;

        let msg = lettre::Message::builder()
            .from(
                smtp_config
                    .from
                    .parse()
                    .map_err(|e| AppError::Internal(format!("Invalid from address: {e}")))?,
            )
            .to(recipient
                .parse()
                .map_err(|e| AppError::Internal(format!("Invalid recipient '{recipient}': {e}")))?)
            .subject(&report_email.subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_PLAIN)
                            .body(report_email.text_body.clone()),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_HTML)
                            .body(report_email.html_body.clone()),
                    ),
            )
            .map_err(|e| AppError::Internal(format!("Failed to build email: {e}")))?;

        transport
            .send(msg)
            .await
            .map_err(|e| AppError::Internal(format!("SMTP send failed: {e}")))?;
    }

    Ok(format!(
        "Test report sent to {} recipient(s)",
        recipients.len()
    ))
}

/// Get the current report schedule configuration.
///
/// Returns the saved [`ReportScheduleConfig`] or a default if none
/// has been persisted yet.
#[tauri::command]
pub async fn get_report_schedule(
    state: State<'_, AppState>,
) -> Result<oz_core::export::ReportScheduleConfig, AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    store
        .get_report_schedule()
        .map_err(|e| AppError::Internal(format!("Failed to load report schedule: {e}")))
        .map(|opt| opt.unwrap_or_default())
}

/// Save the report schedule configuration.
#[tauri::command]
pub async fn save_report_schedule(
    state: State<'_, AppState>,
    config: oz_core::export::ReportScheduleConfig,
) -> Result<(), AppError> {
    let conn = state.db.lock().await;
    let store = oz_core::Store::new(&conn);
    store
        .save_report_schedule(&config)
        .map_err(|e| AppError::Internal(format!("Failed to save report schedule: {e}")))
}

/// Build an async SMTP transport from the config.
/// Logs errors internally so the caller can provide a generic message.
fn build_smtp_transport(
    config: &oz_core::export::email_report::SmtpConfig,
) -> Result<lettre::AsyncSmtpTransport<lettre::Tokio1Executor>, String> {
    use lettre::transport::smtp::authentication::Credentials;

    let creds = match (&config.username, &config.password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => {
            Some(Credentials::new(u.clone(), p.clone()))
        }
        _ => None,
    };

    if config.use_tls || config.port == 465 {
        match lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&config.host) {
            Ok(relay) => {
                let relay = if let Some(c) = creds {
                    relay.credentials(c)
                } else {
                    relay
                };
                Ok(relay.port(config.port).build())
            }
            Err(e) => Err(format!(
                "Failed to build TLS SMTP transport to {}: {e}",
                config.host
            )),
        }
    } else {
        let transport =
            lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::builder_dangerous(&config.host)
                .port(config.port);
        let transport = if let Some(c) = creds {
            transport.credentials(c)
        } else {
            transport
        };
        Ok(transport.build())
    }
}
