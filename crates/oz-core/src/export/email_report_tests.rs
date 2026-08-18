use super::*;
use crate::db::Store;
use crate::export::{AnalyticsBundle, ExportConfig};
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
        crate::Money::from_major(3_50, "USD".parse().unwrap()).unwrap(),
        None,
        None,
        100,
        None,
    )
    .unwrap();
    s.create_product(
        "BAGEL",
        "Bagel",
        crate::Money::from_major(4_50, "USD".parse().unwrap()).unwrap(),
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
        crate::Money::from_major(3_50, "USD".parse().unwrap()).unwrap(),
    ))
    .unwrap();
    cart.add_line(crate::CartLine::new(
        crate::Sku::new("BAGEL"),
        1,
        crate::Money::from_major(4_50, "USD".parse().unwrap()).unwrap(),
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
    assert!(email.html_body.contains("Gross Profit"));
    assert!(email.html_body.contains("Margin"));
    assert!(email.html_body.contains("OZ-POS Report"));
    assert!(email.html_body.contains("</html>"));
}

#[test]
fn report_email_text_contains_sections() {
    let bundle = sample_bundle();
    let email = ReportEmailBuilder::build(&bundle, "Store", "today");
    assert!(email.text_body.contains("DAILY REVENUE"));
    assert!(email.text_body.contains("TOP PRODUCTS"));
    assert!(email.text_body.contains("Gross Profit"));
    assert!(email.text_body.contains("Margin"));
    assert!(email.text_body.contains("OZ-POS Report"));
}

#[test]
fn report_email_top_products_show_gross_profit_and_margin() {
    let conn = migrations::fresh_db();
    let s = Store::new(&conn);
    // Product with a cost → the checkout snapshot freezes it.
    s.create_product_with_attributes(
        "COFFEE",
        "Coffee",
        crate::Money::from_major(3_50, "USD".parse().unwrap()).unwrap(),
        None,
        None,
        100,
        None,
        &crate::db::products::CreateProductAttributes {
            // Price is $350.00 (35000 minor); cost $200.00 (20000 minor).
            cost_minor: 20000,
            ..Default::default()
        },
    )
    .unwrap();
    let mut cart = crate::Cart::new("USD".parse().unwrap());
    cart.add_line(crate::CartLine::new(
        crate::Sku::new("COFFEE"),
        2,
        crate::Money::from_major(3_50, "USD".parse().unwrap()).unwrap(),
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

    let bundle = s
        .export_analytics_bundle(ExportConfig::default(), "t1", "Store")
        .unwrap();
    let email = ReportEmailBuilder::build(&bundle, "Store", "today");
    // Revenue $700.00, COGS $400.00, gross profit $300.00, margin 42.9%.
    assert!(email.html_body.contains(">300.00</td>"));
    assert!(email.html_body.contains("42.9%"));
    assert!(email.text_body.contains("300.00"));
    assert!(email.text_body.contains("42.9%"));
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
