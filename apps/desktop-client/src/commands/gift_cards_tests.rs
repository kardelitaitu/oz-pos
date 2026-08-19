use super::*;
use tauri::Manager as _;

// ── Existing tests (preserved) ────────────────────────────────────

#[test]
fn balance_result_debug() {
    let result = BalanceResult {
        balance_minor: 1500,
        currency: "USD".into(),
        status: "active".into(),
    };
    let d = format!("{result:?}");
    assert!(d.contains("1500"));
}

#[test]
fn balance_result_serialize() {
    let result = BalanceResult {
        balance_minor: 2500,
        currency: "IDR".into(),
        status: "active".into(),
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["balance_minor"], 2500);
    assert_eq!(json["currency"], "IDR");
}

#[test]
fn balance_result_zero_and_empty() {
    let result = BalanceResult {
        balance_minor: 0,
        currency: "".into(),
        status: "".into(),
    };
    assert_eq!(result.balance_minor, 0);
}

// ── Helper ────────────────────────────────────────────────────────

fn test_app() -> tauri::App<tauri::test::MockRuntime> {
    let state = AppState::for_test();
    tauri::test::mock_builder()
        .manage(state)
        .build(tauri::generate_context!())
        .unwrap()
}

// ── CRUD tests ────────────────────────────────────────────────────

#[tokio::test]
async fn issue_gift_card_does_not_panic() {
    let app = test_app();
    let input = IssueGiftCardInput {
        card_number: "GC-001".into(),
        pin: None,
        initial_amount_minor: 5000,
        currency: "USD".into(),
        issued_to: None,
        created_by: "user-owner".into(),
        expiry_date: None,
    };

    // Should either succeed or fail gracefully (e.g. table missing).
    let _ = issue_gift_card(input, app.state()).await;
}

#[tokio::test]
async fn get_gift_card_balance_does_not_panic() {
    let app = test_app();
    let _ = get_gift_card_balance("nonexistent-card".into(), app.state()).await;
}

#[tokio::test]
async fn list_gift_cards_does_not_panic() {
    let app = test_app();
    let filter = GiftCardFilter::default();
    let _ = list_gift_cards(filter, app.state()).await;
}

#[tokio::test]
async fn redeem_does_not_panic() {
    let app = test_app();
    let _ = redeem_gift_card("nonexistent".into(), 100, "sale-1".into(), app.state()).await;
}

#[tokio::test]
async fn freeze_does_not_panic() {
    let app = test_app();
    let _ = freeze_gift_card("nonexistent".into(), app.state()).await;
}

#[tokio::test]
async fn unfreeze_does_not_panic() {
    let app = test_app();
    let _ = unfreeze_gift_card("nonexistent".into(), app.state()).await;
}

#[tokio::test]
async fn top_up_does_not_panic() {
    let app = test_app();
    let _ = top_up_gift_card("nonexistent".into(), 100, app.state()).await;
}

#[tokio::test]
async fn get_gift_card_does_not_panic() {
    let app = test_app();
    let _ = get_gift_card("nonexistent".into(), app.state()).await;
}
