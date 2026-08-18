
use super::*;
use oz_core::export::ReportScheduleConfig;
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

    let store_name = "Test Store";
    let result = email_sender::generate_filtered_report_email(&store, &schedule, store_name);
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
    let result = email_sender::generate_filtered_report_email(&store, &schedule, "Store");
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
