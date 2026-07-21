//! Email report delivery — SMTP configuration and report email generation.
//!
//! [`SmtpConfig`] holds the SMTP server connection parameters and is
//! persisted in the `settings` table under key `smtp_config` as JSON
//! (same pattern as [`ReportScheduleConfig`](super::ReportScheduleConfig)).
//!
//! [`ReportEmailBuilder`] consumes an [`AnalyticsBundle`](super::AnalyticsBundle)
//! and produces a structured email with HTML and plain-text alternatives
//! suitable for SMTP delivery.

use serde::{Deserialize, Serialize};

use super::AnalyticsBundle;
use crate::db::Store;
use crate::error::CoreError;

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
        if self.port == 0 || self.port > 65535 {
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
    pub fn save_smtp_config(&self, config: &SmtpConfig) -> Result<(), CoreError> {
        let json = serde_json::to_string(config)
            .map_err(|e| CoreError::Internal(format!("failed to serialize SMTP config: {e}")))?;
        self.set_setting(SMTP_CONFIG_SETTINGS_KEY, &json)
    }

    /// Load the SMTP config from the settings table.
    /// Returns `None` if no config has been saved yet.
    pub fn get_smtp_config(&self) -> Result<Option<SmtpConfig>, CoreError> {
        let raw = match self.get_setting(SMTP_CONFIG_SETTINGS_KEY)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let config: SmtpConfig = serde_json::from_str(&raw)
            .map_err(|e| CoreError::Internal(format!("failed to deserialize SMTP config: {e}")))?;
        Ok(Some(config))
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
            sections.push_str(r#"</tr></thead><tbody>"#);
            for row in &bundle.top_products {
                sections.push_str(&format!(
                    r#"<tr><td style="padding:6px 12px;border-bottom:1px solid #e5e7eb;font-size:13px;font-family:monospace;">{}</td><td style="padding:6px 12px;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;">{}</td><td style="padding:6px 12px;text-align:right;border-bottom:1px solid #e5e7eb;font-size:13px;font-variant-numeric:tabular-nums;">{}</td></tr>"#,
                    html_escape(&row.sku),
                    html_escape(&row.name),
                    row.total_qty,
                    format_amount(row.total_minor, ""),
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
                "{:<10} {:<24} {:>6} {:>12}\n",
                "SKU", "Name", "Qty", "Revenue"
            ));
            for row in &bundle.top_products {
                let name = if row.name.len() > 24 {
                    format!("{}…", &row.name[..23])
                } else {
                    row.name.clone()
                };
                text.push_str(&format!(
                    "{:<10} {:<24} {:>6} {:>12}\n",
                    row.sku,
                    name,
                    row.total_qty,
                    format_amount(row.total_minor, ""),
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
fn format_amount(minor: i64, currency: &str) -> String {
    let major = minor as f64 / 100.0;
    if !currency.is_empty() {
        format!("{:.2} {}", major, currency)
    } else {
        format!("{:.2}", major)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Store;
    use crate::export::{AnalyticsBundle, ExportConfig, ExportMetadata};
    use crate::migrations;

    // ── SmtpConfig validation ──────────────────────────────────────

    #[test]
    fn smtp_config_defaults() {
        let cfg = SmtpConfig::default();
        assert!(cfg.host.is_empty());
        assert_eq!(cfg.port, 587);
        assert!(cfg.username.is_none());
        assert!(cfg.password.is_none());
        assert!(cfg.from.is_empty());
        assert!(cfg.use_tls);
    }

    #[test]
    fn smtp_config_valid_passes() {
        let cfg = SmtpConfig {
            host: "smtp.example.com".into(),
            port: 587,
            username: Some("user".into()),
            password: Some("pass".into()),
            from: "reports@store.com".into(),
            use_tls: true,
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn smtp_config_empty_host_fails() {
        let cfg = SmtpConfig {
            host: "".into(),
            ..SmtpConfig::default()
        };
        let err = cfg.validate().unwrap_err();
        assert!(format!("{err}").contains("host"));
    }

    #[test]
    fn smtp_config_zero_port_fails() {
        let cfg = SmtpConfig {
            host: "smtp.example.com".into(),
            port: 0,
            ..SmtpConfig::default()
        };
        let err = cfg.validate().unwrap_err();
        assert!(format!("{err}").contains("port"));
    }

    #[test]
    fn smtp_config_invalid_from_email_fails() {
        let cfg = SmtpConfig {
            host: "smtp.example.com".into(),
            port: 587,
            from: "not-an-email".into(),
            ..SmtpConfig::default()
        };
        let err = cfg.validate().unwrap_err();
        assert!(format!("{err}").contains("valid email"));
    }

    #[test]
    fn smtp_config_from_with_at_only_fails() {
        let cfg = SmtpConfig {
            host: "smtp.example.com".into(),
            port: 587,
            from: "user@".into(),
            ..SmtpConfig::default()
        };
        let err = cfg.validate().unwrap_err();
        assert!(format!("{err}").contains("valid email"));
    }

    #[test]
    fn smtp_config_too_high_port_fails() {
        let cfg = SmtpConfig {
            host: "smtp.example.com".into(),
            port: 99999,
            ..SmtpConfig::default()
        };
        let err = cfg.validate().unwrap_err();
        assert!(format!("{err}").contains("port"));
    }

    // ── SmtpConfig persistence ─────────────────────────────────────

    #[test]
    fn smtp_config_save_and_load() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);

        // Initially no config
        let loaded = s.get_smtp_config().unwrap();
        assert!(loaded.is_none());

        // Save a config
        let cfg = SmtpConfig {
            host: "smtp.example.com".into(),
            port: 587,
            username: Some("apikey".into()),
            password: Some("secret123".into()),
            from: "pos@mystore.com".into(),
            use_tls: true,
        };
        s.save_smtp_config(&cfg).unwrap();

        // Load and verify
        let loaded = s.get_smtp_config().unwrap().unwrap();
        assert_eq!(loaded.host, "smtp.example.com");
        assert_eq!(loaded.port, 587);
        assert_eq!(loaded.username, Some("apikey".into()));
        assert_eq!(loaded.password, Some("secret123".into()));
        assert_eq!(loaded.from, "pos@mystore.com");
        assert!(loaded.use_tls);
    }

    #[test]
    fn smtp_config_roundtrip_none_values() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);

        let cfg = SmtpConfig {
            host: "mail.server.com".into(),
            port: 25,
            username: None,
            password: None,
            from: "noreply@server.com".into(),
            use_tls: false,
        };
        s.save_smtp_config(&cfg).unwrap();
        let loaded = s.get_smtp_config().unwrap().unwrap();
        assert_eq!(loaded.host, "mail.server.com");
        assert!(loaded.username.is_none());
        assert!(loaded.password.is_none());
        assert!(!loaded.use_tls);
    }

    // ── ReportEmailBuilder ─────────────────────────────────────────

    fn sample_bundle() -> AnalyticsBundle {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);

        // Seed some data
        s.create_product(
            "COFFEE",
            "Coffee",
            crate::Money::from_major(3_50, "USD".parse().unwrap()),
            None,
            None,
            100,
            None,
        )
        .unwrap();
        s.create_product(
            "BAGEL",
            "Bagel",
            crate::Money::from_major(4_50, "USD".parse().unwrap()),
            None,
            None,
            50,
            None,
        )
        .unwrap();

        let mut cart = crate::Cart::new("USD".parse().unwrap());
        cart.add_line(crate::CartLine::new(
            crate::Sku::new("COFFEE"),
            2,
            crate::Money::from_major(3_50, "USD".parse().unwrap()),
        ))
        .unwrap();
        cart.add_line(crate::CartLine::new(
            crate::Sku::new("BAGEL"),
            1,
            crate::Money::from_major(4_50, "USD".parse().unwrap()),
        ))
        .unwrap();

        let mut sale = crate::Sale::from_cart(&cart).unwrap();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        sale.created_at = now.clone();
        sale.updated_at = now;
        s.create_sale(&sale).unwrap();
        s.update_sale_status(&sale.id, crate::SaleStatus::Active)
            .unwrap();
        s.update_sale_status(&sale.id, crate::SaleStatus::Completed)
            .unwrap();

        s.export_analytics_bundle(ExportConfig::default(), "t1", "Test Store")
            .unwrap()
    }

    #[test]
    fn report_email_subject_contains_store_and_date() {
        let bundle = sample_bundle();
        let email = ReportEmailBuilder::build(&bundle, "My Store", "2026-07-20");
        assert!(email.subject.contains("My Store"));
        assert!(email.subject.contains("2026-07-20"));
        assert!(email.subject.contains("OZ-POS Report"));
    }

    #[test]
    fn report_email_html_contains_tables() {
        let bundle = sample_bundle();
        let email = ReportEmailBuilder::build(&bundle, "Store", "today");
        assert!(email.html_body.contains("<table"));
        assert!(email.html_body.contains("Daily Revenue"));
        assert!(email.html_body.contains("Top Products"));
        assert!(email.html_body.contains("OZ-POS Report"));
        assert!(email.html_body.contains("</html>"));
    }

    #[test]
    fn report_email_text_contains_sections() {
        let bundle = sample_bundle();
        let email = ReportEmailBuilder::build(&bundle, "Store", "today");
        assert!(email.text_body.contains("DAILY REVENUE"));
        assert!(email.text_body.contains("TOP PRODUCTS"));
        assert!(email.text_body.contains("OZ-POS Report"));
    }

    #[test]
    fn report_email_empty_bundle_generates_minimal() {
        let conn = migrations::fresh_db();
        let s = Store::new(&conn);
        let bundle = s
            .export_analytics_bundle(ExportConfig::default(), "", "")
            .unwrap();
        let email = ReportEmailBuilder::build(&bundle, "Empty Store", "period");
        assert!(email.html_body.contains("OZ-POS Report"));
        assert!(!email.html_body.contains("Daily Revenue"));
        assert!(email.text_body.contains("OZ-POS Report"));
    }

    #[test]
    fn html_escape_handles_special_chars() {
        assert_eq!(html_escape("a&b<c>d\"e"), "a&amp;b&lt;c&gt;d&quot;e");
    }
}
