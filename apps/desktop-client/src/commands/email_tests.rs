use super::*;
use tauri::Manager as _;

// ── Helper ────────────────────────────────────────────────────────

fn test_app() -> tauri::App<tauri::test::MockRuntime> {
    let state = AppState::for_test();
    tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap()
}

// ── get_report_schedule ────────────────────────────────────────────

#[tokio::test]
async fn get_report_schedule_does_not_panic() {
    let app = test_app();
    // The function should either return a default or an error — never panic.
    let _ = get_report_schedule(app.state()).await;
}

// ── send_test_report ──────────────────────────────────────────────

#[tokio::test]
async fn send_test_report_fails_without_smtp_config() {
    let app = test_app();
    let result = send_test_report(app.state()).await;
    // Should fail because SMTP is not configured in test state.
    assert!(result.is_err(), "should fail without SMTP config");
}
