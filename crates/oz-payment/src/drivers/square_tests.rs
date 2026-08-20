use super::*;

fn test_api_key() -> String {
    "EAAA_test_square_api_key_123456".to_string()
}

fn test_location_id() -> String {
    "L_ABC123".to_string()
}

#[test]
fn square_processor_construction() {
    let proc = SquarePaymentProcessor::new(&test_api_key(), &test_location_id());
    assert_eq!(proc.api_key, test_api_key());
    assert_eq!(proc.location_id, test_location_id());
}

#[test]
fn square_device_info() {
    let proc = SquarePaymentProcessor::new(&test_api_key(), &test_location_id());
    let info = proc.device_info();
    assert_eq!(info.vendor, "Square");
    assert_eq!(info.model, "Square API");
}

#[test]
fn square_debug_masks_key() {
    let proc = SquarePaymentProcessor::new(&test_api_key(), &test_location_id());
    let debug = format!("{:?}", proc);
    assert!(!debug.contains("EAAA_test"));
    assert!(debug.contains("***"));
}

#[test]
fn square_parse_payment_success() {
    let json = r#"{
        "payment": {
            "id": "sq_payment_123",
            "status": "COMPLETED",
            "amount_money": {"amount": 1000, "currency": "USD"},
            "created_at": "2026-06-30T12:00:00Z"
        }
    }"#;
    let payment = SquarePaymentProcessor::parse_payment(json).unwrap();
    assert_eq!(payment.id, "sq_payment_123");
    assert_eq!(payment.status, "COMPLETED");
    assert_eq!(payment.amount_money.amount, 1000);
    assert_eq!(payment.created_at.unwrap(), "2026-06-30T12:00:00Z");
}

#[test]
fn square_parse_refund_success() {
    let json = r#"{
        "refund": {
            "id": "sq_refund_456",
            "status": "COMPLETED",
            "amount_money": {"amount": 500, "currency": "USD"}
        }
    }"#;
    let refund = SquarePaymentProcessor::parse_refund(json).unwrap();
    assert_eq!(refund.id, "sq_refund_456");
    assert_eq!(refund.status, "COMPLETED");
    assert_eq!(refund.amount_money.amount, 500);
}

#[test]
fn square_parse_error_body() {
    let json = r#"{
        "errors": [
            {"code": "CARD_DECLINED", "detail": "The card was declined."}
        ]
    }"#;
    let body: SquareErrorBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.errors[0].code.as_deref(), Some("CARD_DECLINED"));
    assert_eq!(
        body.errors[0].detail.as_deref(),
        Some("The card was declined.")
    );
}

#[test]
fn square_parse_error_no_detail() {
    let json = r#"{"errors": [{"code": "NOT_FOUND"}]}"#;
    let body: SquareErrorBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.errors[0].detail, None);
}

#[test]
fn square_parse_error_formats() {
    let err = SquarePaymentProcessor::parse_error(
        402,
        r#"{"errors":[{"code":"CARD_DECLINED","detail":"declined"}]}"#,
    );
    let msg = err.to_string();
    assert!(msg.contains("declined"));
}

#[test]
fn square_parse_error_non_json() {
    let err = SquarePaymentProcessor::parse_error(500, "Internal Server Error");
    let msg = err.to_string();
    assert!(msg.contains("500"));
    assert!(msg.contains("Internal Server Error"));
}

#[test]
fn square_payment_result_completed() {
    let data = PaymentData {
        id: "sq_1".into(),
        status: "COMPLETED".into(),
        amount_money: MoneyAmountResponse {
            amount: 1000,
            currency: "USD".into(),
        },
        created_at: None,
    };
    let (success, money) = SquarePaymentProcessor::payment_result(&data).unwrap();
    assert!(success);
    assert_eq!(money.minor_units, 1000);
}

#[test]
fn square_payment_result_approved() {
    let data = PaymentData {
        id: "sq_2".into(),
        status: "APPROVED".into(),
        amount_money: MoneyAmountResponse {
            amount: 2000,
            currency: "USD".into(),
        },
        created_at: None,
    };
    let (success, _) = SquarePaymentProcessor::payment_result(&data).unwrap();
    assert!(success);
}

#[test]
fn square_payment_result_failed() {
    let data = PaymentData {
        id: "sq_3".into(),
        status: "FAILED".into(),
        amount_money: MoneyAmountResponse {
            amount: 500,
            currency: "USD".into(),
        },
        created_at: None,
    };
    let (success, _) = SquarePaymentProcessor::payment_result(&data).unwrap();
    assert!(!success);
}

#[test]
fn square_to_money_constructs() {
    let m = SquarePaymentProcessor::to_money(1000, "USD").unwrap();
    assert_eq!(m.minor_units, 1000);
}

// PA-02: an unknown gateway currency code must be a hard error, never a
// silent USD fallback that mislabels the recorded amount.
#[test]
fn square_to_money_rejects_unknown_currency() {
    assert!(SquarePaymentProcessor::to_money(1000, "XX").is_err());
    assert!(SquarePaymentProcessor::to_money(1000, "US").is_err());
    assert!(SquarePaymentProcessor::to_money(1000, "usd1").is_err());
}

#[test]
fn square_to_square_amount() {
    let usd: Currency = "USD".parse().unwrap();
    let amount = Money::from_major(10, usd).unwrap();
    assert_eq!(SquarePaymentProcessor::to_square_amount(&amount), 1000);
}
