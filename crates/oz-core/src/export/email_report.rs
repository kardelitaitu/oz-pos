//! Email report delivery — SMTP configuration and report email generation.
/*
last audited 25-07-26 by RSA-Agent (oz-core slice D2: email_report deep read)
crate: oz-core | status: SAFE | lint: CLEAN
findings: COR-36 LOW-MED: render_text truncates with byte slicing (&row.name[..21] guarded by byte-len check) — panics when byte 21 is mid-UTF-8 (multi-byte product names); use char-boundary truncation; HTML path escapes all user-controlled cells properly; SMTP password encrypted at rest via crate::crypto with transparent decrypt and documented legacy-plaintext fallback (test-pinned); inline #[cfg(test)] mod store_tests in production file (COR-33 pattern)
next: char-safe truncation (COR-36) | perf: N/A
*/
//!
//! [`SmtpConfig`] holds the SMTP server connection parameters and is
//! persisted in the `settings` table under key `smtp_config` as JSON
//! (same pattern as [`ReportScheduleConfig`](super::ReportScheduleConfig)).
//!
//! [`ReportEmailBuilder`] consumes an [`AnalyticsBundle`]
//! and produces a structured email with HTML and plain-text alternatives
//! suitable for SMTP delivery.

use serde::{Deserialize, Serialize};

use super::AnalyticsBundle;
use crate::db::Store;
use crate::error::CoreError;
use crate::{Currency, format_minor};

// ── SMTP Configuration ─────────────────────────────────────────────

/// SMTP server connection parameters for sending report emails.
///
/// Persisted in the `settings` table under key `smtp_config` as JSON.
///
/// # Example
///
/// ```json
/// {
///   "host": "smtp.example.com",
///   "port": 587,
///   "username": null,
///   "password": null,
///   "from": "reports@store.com",
///   "use_tls": true
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// SMTP server hostname.
    pub host: String,
    /// SMTP server port (25, 465, 587, etc.).
    pub port: u16,
    /// Optional SMTP username (for authenticated relays).
    pub username: Option<String>,
    /// Optional SMTP password (for authenticated relays).
    /// Stored as plaintext in the local settings database — encrypted
    /// at rest is planned for a future security sprint.
    pub password: Option<String>,
    /// From-address for outgoing emails.
    pub from: String,
    /// Whether to use STARTTLS (true) or plaintext (false).
    /// Port 465 typically uses implicit TLS via lettre's TlsParameters.
    pub use_tls: bool,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 587,
            username: None,
            password: None,
            from: String::new(),
            use_tls: true,
        }
    }
}

impl SmtpConfig {
    /// Validate the configuration — returns an error message for the
    /// first field that fails validation.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.host.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "smtp_host",
                message: "SMTP host must not be empty".into(),
            });
        }
        if self.port == 0 {
            return Err(CoreError::Validation {
                field: "smtp_port",
                message: "SMTP port must be between 1 and 65535".into(),
            });
        }
        if self.from.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "smtp_from",
                message: "From-address must not be empty".into(),
            });
        }
        // Basic email format check
        if !self.from.contains('@') || !self.from.contains('.') {
            return Err(CoreError::Validation {
                field: "smtp_from",
                message: "From-address must be a valid email".into(),
            });
        }
        Ok(())
    }
}

/// Settings key used to persist SMTP configuration.
pub const SMTP_CONFIG_SETTINGS_KEY: &str = "smtp_config";

impl Store<'_> {
    /// Save the SMTP config to the settings table.
    ///
    /// The password field is encrypted at rest before serialization
    /// so that casual database inspection does not reveal it.
    /// Decryption happens transparently in [`Self::get_smtp_config`].
    ///
    /// F-029: encryption fails closed — an encrypt failure returns an
    /// error instead of storing the plaintext password.
    pub fn save_smtp_config(&self, config: &SmtpConfig) -> Result<(), CoreError> {
        let mut config = config.clone();
        if let Some(ref pwd) = config.password
            && !pwd.is_empty()
        {
            config.password = Some(crate::crypto::encrypt_smtp_at_rest(pwd).map_err(|e| {
                CoreError::Internal(format!("failed to encrypt SMTP password: {e}"))
            })?);
        }
        let json = serde_json::to_string(&config)
            .map_err(|e| CoreError::Internal(format!("failed to serialize SMTP config: {e}")))?;
        self.set_setting(SMTP_CONFIG_SETTINGS_KEY, &json)
    }

    /// Load the SMTP config from the settings table.
    ///
    /// The password field is transparently decrypted if it was
    /// encrypted at rest. Legacy plaintext passwords are returned
    /// as-is (backward compatible); tampered ciphertext fails closed
    /// with an error (F-029).
    /// Returns `None` if no config has been saved yet.
    pub fn get_smtp_config(&self) -> Result<Option<SmtpConfig>, CoreError> {
        let raw = match self.get_setting(SMTP_CONFIG_SETTINGS_KEY)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let mut config: SmtpConfig = serde_json::from_str(&raw)
            .map_err(|e| CoreError::Internal(format!("failed to deserialize SMTP config: {e}")))?;
        if let Some(ref pwd) = config.password
            && !pwd.is_empty()
        {
            config.password = Some(crate::crypto::decrypt_smtp_at_rest(pwd).map_err(|e| {
                CoreError::Internal(format!("stored SMTP password failed authentication: {e}"))
            })?);
        }
        Ok(Some(config))
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;
    use crate::db::Store;
    use crate::migrations;

    #[test]
    fn smtp_password_encrypted_at_rest() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);

        let cfg = SmtpConfig {
            host: "smtp.test.com".into(),
            port: 587,
            username: Some("user".into()),
            password: Some("my-secret-password".into()),
            from: "test@test.com".into(),
            use_tls: true,
        };
        s.save_smtp_config(&cfg).unwrap();

        // Read the raw JSON from settings — password should be encrypted
        let raw = s.get_setting(SMTP_CONFIG_SETTINGS_KEY).unwrap().unwrap();
        assert!(
            !raw.contains("my-secret-password"),
            "password should be encrypted at rest, got: {raw}"
        );

        // But get_smtp_config should decrypt transparently
        let loaded = s.get_smtp_config().unwrap().unwrap();
        assert_eq!(loaded.password, Some("my-secret-password".into()));
    }

    #[test]
    fn smtp_legacy_plaintext_password_still_readable() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);

        // Simulate legacy plaintext storage by writing directly
        let legacy_json = r#"{"host":"old.smtp.com","port":25,"username":null,"password":"legacy-pass","from":"old@old.com","use_tls":false}"#;
        s.set_setting(SMTP_CONFIG_SETTINGS_KEY, legacy_json)
            .unwrap();

        let loaded = s.get_smtp_config().unwrap().unwrap();
        assert_eq!(
            loaded.password,
            Some("legacy-pass".into()),
            "legacy plaintext passwords should be readable"
        );
    }

    #[test]
    fn smtp_null_password_unchanged() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);

        let cfg = SmtpConfig {
            host: "smtp.test.com".into(),
            port: 587,
            username: None,
            password: None,
            from: "test@test.com".into(),
            use_tls: false,
        };
        s.save_smtp_config(&cfg).unwrap();

        let loaded = s.get_smtp_config().unwrap().unwrap();
        assert!(loaded.password.is_none());
    }
}

// ── Report Email Builder ────────────────────────────────────────────

/// Built email with HTML and plain-text alternatives.
#[derive(Debug, Clone)]
pub struct ReportEmail {
    /// Subject line for the email.
    pub subject: String,
    /// HTML body (rich tables, styling).
    pub html_body: String,
    /// Plain-text fallback body.
    pub text_body: String,
}

/// Generates structured report emails from analytics bundles.
pub struct ReportEmailBuilder;

impl ReportEmailBuilder {
    /// Build a report email from the analytics bundle.
    ///
    /// The subject includes the date range and store name. The body
    /// contains summary tables for all populated report types, rendered
    /// as both HTML and plain-text.
    pub fn build(bundle: &AnalyticsBundle, store_name: &str, date_label: &str) -> ReportEmail {
        let subject = format!("OZ-POS Report — {} ({})", store_name, date_label,);

        let html_body = Self::render_html(bundle, store_name, date_label);
        let text_body = Self::render_text(bundle, store_name, date_label);

        ReportEmail {
            subject,
            html_body,
            text_body,
        }
    }

    /// Render the analytics bundle as an HTML email body.
    fn render_html(bundle: &AnalyticsBundle, store_name: &str, date_label: &str) -> String {
        let mut sections = String::new();

        // Daily Revenue
        if !bundle.daily_revenue.is_empty() {
            sections.push_str(r#"<h3 style="margin-top:24px;color:#1a1a2e;">Daily Revenue</h3>"#);
            sections.push_str(
                r#"<table style="width:100%;border-collapse:collapse;margin-bottom:16px;">"#,
            );
            sections.push_str(r#"<thead><tr style="background:#f0f4f8;">"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:left;border-bottom:2px solid #d1d5db;font-size:13px;">Date</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #d1d5db;font-size:13px;">Total</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #d1d5db;font-size:13px;">Sales</th>"#);
            sections.push_str(r#"</tr></thead><tbody>"#);
            for row in &bundle.daily_revenue {
                sections.push_str(&format!(
                    r#"<tr><td style="padding:6px 12px;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;font-variant-numeric:tabular-nums;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td></tr>"#,
                    html_escape(&row.date),
                    format_amount(row.total_minor, &row.currency),
                    row.sale_count,
                ));
            }
            sections.push_str(r#"</tbody></table>"#);
        }

        // Top Products
        if !bundle.top_products.is_empty() {
            sections.push_str(r#"<h3 style="margin-top:24px;color:#1a1a2e;">Top Products</h3>"#);
            sections.push_str(
                r#"<table style="width:100%;border-collapse:collapse;margin-bottom:16px;">"#,
            );
            sections.push_str(r#"<thead><tr style="background:#f0f4f8;">"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:left;border-bottom:2px solid #d1d5db;font-size:13px;">SKU</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:left;border-bottom:2px solid #d1d5db;font-size:13px;">Name</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #d1d5db;font-size:13px;">Qty</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #d1d5db;font-size:13px;">Revenue</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #d1d5db;font-size:13px;">Gross Profit</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #d1d5db;font-size:13px;">Margin</th>"#);
            sections.push_str(r#"</tr></thead><tbody>"#);
            for row in &bundle.top_products {
                let margin = format!("{:.1}%", row.gross_margin_percent);
                sections.push_str(&format!(
                    r#"<tr><td style="padding:6px 12px;border-bottom:1px solid #e5e7eb;font-size:13px;font-family:monospace;">{}</td><td style="padding:6px 12px;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;font-variant-numeric:tabular-nums;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;font-variant-numeric:tabular-nums;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td></tr>"#,
                    html_escape(&row.sku),
                    html_escape(&row.name),
                    row.total_qty,
                    format_amount(row.total_minor, ""),
                    format_amount(row.gross_profit_minor, ""),
                    margin,
                ));
            }
            sections.push_str(r#"</tbody></table>"#);
        }

        // Category Breakdown
        if !bundle.category_breakdown.is_empty() {
            sections
                .push_str(r#"<h3 style="margin-top:24px;color:#1a1a2e;">Category Breakdown</h3>"#);
            sections.push_str(
                r#"<table style="width:100%;border-collapse:collapse;margin-bottom:16px;">"#,
            );
            sections.push_str(r#"<thead><tr style="background:#f0f4f8;">"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:left;border-bottom:2px solid #d1d5db;font-size:13px;">Category</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #d1d5db;font-size:13px;">Revenue</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #d1d5db;font-size:13px;">%</th>"#);
            sections.push_str(r#"</tr></thead><tbody>"#);
            for row in &bundle.category_breakdown {
                sections.push_str(&format!(
                    r#"<tr><td style="padding:6px 12px;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;">{:.1}%</td></tr>"#,
                    html_escape(&row.category_name),
                    format_amount(row.total_minor, ""),
                    row.percentage,
                ));
            }
            sections.push_str(r#"</tbody></table>"#);
        }

        // Low Stock Alerts
        if !bundle.low_stock_alerts.is_empty() {
            sections
                .push_str(r#"<h3 style="margin-top:24px;color:#991b1b;">⚠️ Low Stock Alerts</h3>"#);
            sections.push_str(
                r#"<table style="width:100%;border-collapse:collapse;margin-bottom:16px;">"#,
            );
            sections.push_str(r#"<thead><tr style="background:#fef2f2;">"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:left;border-bottom:2px solid #fecaca;font-size:13px;">Product</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #fecaca;font-size:13px;">Stock</th>"#);
            sections.push_str(r#"<th style="padding:8px 12px;text-align:right;border-bottom:2px solid #fecaca;font-size:13px;">Threshold</th>"#);
            sections.push_str(r#"</tr></thead><tbody>"#);
            for row in &bundle.low_stock_alerts {
                sections.push_str(&format!(
                    r#"<tr><td style="padding:6px 12px;border-bottom:1px solid #fecaca;font-size:13px;">{} — {}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #fecaca;font-size:13px;font-weight:600;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #fecaca;font-size:13px;">{}</td></tr>"#,
                    html_escape(&row.sku),
                    html_escape(&row.name),
                    row.current_qty,
                    row.threshold,
                ));
            }
            sections.push_str(r#"</tbody></table>"#);
        }

        // Hourly Heatmap (compact summary)
        if !bundle.hourly_heatmap.is_empty() {
            sections.push_str(r#"<h3 style="margin-top:24px;color:#1a1a2e;">Hourly Activity</h3>"#);
            sections.push_str(r#"<p style="font-size:13px;color:#6b7280;">"#);
            let peak = bundle.hourly_heatmap.iter().max_by_key(|h| h.sale_count);
            if let Some(p) = peak {
                sections.push_str(&format!(
                    "Peak hour: Day {} at {:02}:00 — {} sales",
                    p.day_of_week, p.hour, p.sale_count,
                ));
            }
            sections.push_str(r#"</p>"#);
        }

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head>
<body style="margin:0;padding:0;background:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;">
<table width="100%" cellpadding="0" cellspacing="0"><tr><td align="center" style="padding:24px 16px;">
<table width="640" cellpadding="0" cellspacing="0" style="background:#ffffff;border-radius:8px;box-shadow:0 1px 3px rgba(0,0,0,0.1);">
<tr><td style="padding:32px 32px 0 32px;">
<h1 style="margin:0;font-size:22px;font-weight:700;color:#1a1a2e;">OZ-POS Report</h1>
<p style="margin:4px 0 0 0;font-size:14px;color:#6b7280;">{} &mdash; {}</p>
<hr style="border:none;border-top:1px solid #e5e7eb;margin:20px 0;">
</td></tr>
<tr><td style="padding:0 32px;">
{}
</td></tr>
<tr><td style="padding:20px 32px 32px 32px;">
<hr style="border:none;border-top:1px solid #e5e7eb;margin:0 0 16px 0;">
<p style="margin:0;font-size:12px;color:#9ca3af;text-align:center;">
Generated by OZ-POS v{}
</p>
</td></tr>
</table>
</td></tr></table>
</body>
</html>"#,
            html_escape(store_name),
            html_escape(date_label),
            sections,
            html_escape(env!("CARGO_PKG_VERSION")),
        );

        html
    }

    /// Render the analytics bundle as a plain-text email body.
    fn render_text(bundle: &AnalyticsBundle, store_name: &str, date_label: &str) -> String {
        let mut text = String::new();
        text.push_str(&format!(
            "OZ-POS Report — {} ({})\n",
            store_name, date_label
        ));
        text.push_str(&"=".repeat(60));
        text.push('\n');

        // Daily Revenue
        if !bundle.daily_revenue.is_empty() {
            text.push_str("\nDAILY REVENUE\n");
            text.push_str("-------------\n");
            text.push_str(&format!("{:<14} {:>12} {:>6}\n", "Date", "Amount", "Sales"));
            for row in &bundle.daily_revenue {
                text.push_str(&format!(
                    "{:<14} {:>12} {:>6}\n",
                    row.date,
                    format_amount(row.total_minor, &row.currency),
                    row.sale_count,
                ));
            }
        }

        // Top Products
        if !bundle.top_products.is_empty() {
            text.push_str("\nTOP PRODUCTS\n");
            text.push_str("------------\n");
            text.push_str(&format!(
                "{:<10} {:<22} {:>5} {:>12} {:>12} {:>7}\n",
                "SKU", "Name", "Qty", "Revenue", "Gross Profit", "Margin"
            ));
            for row in &bundle.top_products {
                let name = if row.name.len() > 22 {
                    format!("{}…", &row.name[..21])
                } else {
                    row.name.clone()
                };
                let margin = format!("{:.1}%", row.gross_margin_percent);
                text.push_str(&format!(
                    "{:<10} {:<22} {:>5} {:>12} {:>12} {:>7}\n",
                    row.sku,
                    name,
                    row.total_qty,
                    format_amount(row.total_minor, ""),
                    format_amount(row.gross_profit_minor, ""),
                    margin,
                ));
            }
        }

        // Category Breakdown
        if !bundle.category_breakdown.is_empty() {
            text.push_str("\nCATEGORY BREAKDOWN\n");
            text.push_str("------------------\n");
            text.push_str(&format!(
                "{:<24} {:>12} {:>8}\n",
                "Category", "Revenue", "%"
            ));
            for row in &bundle.category_breakdown {
                text.push_str(&format!(
                    "{:<24} {:>12} {:>7.1}%\n",
                    row.category_name,
                    format_amount(row.total_minor, ""),
                    row.percentage,
                ));
            }
        }

        // Low Stock Alerts
        if !bundle.low_stock_alerts.is_empty() {
            text.push_str("\n⚠️ LOW STOCK ALERTS\n");
            text.push_str("-------------------\n");
            for row in &bundle.low_stock_alerts {
                text.push_str(&format!(
                    "  {} — {}: {} in stock (threshold: {})\n",
                    row.sku, row.name, row.current_qty, row.threshold,
                ));
            }
        }

        // Hourly Heatmap summary
        if !bundle.hourly_heatmap.is_empty() {
            text.push_str("\nHOURLY ACTIVITY\n");
            text.push_str("---------------\n");
            let peak = bundle.hourly_heatmap.iter().max_by_key(|h| h.sale_count);
            if let Some(p) = peak {
                text.push_str(&format!(
                    "Peak: Day {} at {:02}:00 — {} sales\n",
                    p.day_of_week, p.hour, p.sale_count,
                ));
            }
        }

        text.push_str(&format!(
            "\n\n---\nGenerated by OZ-POS v{}\n",
            env!("CARGO_PKG_VERSION"),
        ));

        text
    }
}

/// Minimal HTML-escape for a string.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Format a minor-unit amount into a human-readable string.
/// Uses the currency's ISO-4217 minor-unit exponent (e.g. IDR renders
/// whole Rupiah, USD renders cents) instead of a hardcoded /100.
fn format_amount(minor: i64, currency: &str) -> String {
    // Fall back to USD's exponent (2) if the code doesn't parse.
    let cur = currency.parse::<Currency>().unwrap_or(Currency(*b"USD"));
    if !currency.is_empty() {
        format!("{} {}", format_minor(minor, cur), currency)
    } else {
        format_minor(minor, cur)
    }
}

#[cfg(test)]
#[path = "email_report_tests.rs"]
mod tests;
