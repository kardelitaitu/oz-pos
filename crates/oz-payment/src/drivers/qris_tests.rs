use super::*;

fn test_key() -> String {
    "MID-server_test_key_123456".to_string()
}

#[test]
fn qris_constructs() {
    let proc = QrisPaymentProcessor::new(&test_key(), false);
    let info = proc.device_info();
    assert_eq!(info.vendor, "Midtrans");
    assert_eq!(info.model, "QRIS");
}

#[test]
fn qris_constructs_sandbox() {
    let proc = QrisPaymentProcessor::new(&test_key(), true);
    assert!(proc.sandbox);
}

#[test]
fn qris_base_url_production() {
    let proc = QrisPaymentProcessor::new(&test_key(), false);
    assert_eq!(proc.base_url(), "https://api.midtrans.com/v2");
}

#[test]
fn qris_base_url_sandbox() {
    let proc = QrisPaymentProcessor::new(&test_key(), true);
    assert_eq!(proc.base_url(), "https://api.sandbox.midtrans.com/v2");
}

#[test]
fn qris_base_url_custom_endpoint() {
    let proc = QrisPaymentProcessor::new_with_endpoint("sk_test", "http://localhost:9999", false);
    assert_eq!(proc.base_url(), "http://localhost:9999");
}

#[test]
fn qris_default_constructs() {
    let proc = QrisPaymentProcessor::default();
    assert!(!proc.sandbox);
}

#[test]
fn qris_from_env_missing_key() {
    let result = QrisPaymentProcessor::from_env();
    match std::env::var("MIDTRANS_SERVER_KEY") {
        Ok(_) => assert!(result.is_ok()),
        Err(_) => {
            assert!(result.is_err());
            let msg = result.unwrap_err().to_string();
            assert!(msg.contains("not set"), "error: {}", msg);
        }
    }
}

#[test]
fn qris_generate_order_id_format() {
    let id = QrisPaymentProcessor::generate_order_id();
    assert!(
        id.starts_with("QRIS-"),
        "order id should start with QRIS-: {}",
        id
    );
    assert!(id.len() > 10, "order id should have reasonable length");
}

#[test]
fn qris_to_amount_string() {
    let idr = Currency(*b"IDR");
    let money = Money {
        minor_units: 50000,
        currency: idr,
    };
    assert_eq!(QrisPaymentProcessor::to_amount_string(&money), "50000");
}

#[test]
fn qris_parse_amount() {
    assert_eq!(QrisPaymentProcessor::parse_amount("75000"), 75000);
    assert_eq!(QrisPaymentProcessor::parse_amount("0"), 0);
    assert_eq!(QrisPaymentProcessor::parse_amount("abc"), 0);
}

#[test]
fn qris_parse_charge_response() {
    let json = r#"{
        "status_code": "201",
        "status_message": "QRIS transaction is created",
        "transaction_id": "txn_qris_001",
        "order_id": "QRIS-1234567890-abc",
        "gross_amount": "25000",
        "transaction_status": "pending",
        "qr_code_url": "https://api.midtrans.com/qris/qr-code-abc"
    }"#;
    let resp: QrisChargeResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status_code, "201");
    assert_eq!(resp.transaction_id, "txn_qris_001");
    assert_eq!(resp.order_id, "QRIS-1234567890-abc");
    assert_eq!(resp.gross_amount, "25000");
    assert_eq!(resp.transaction_status, "pending");
    assert_eq!(
        resp.qr_code_url.unwrap(),
        "https://api.midtrans.com/qris/qr-code-abc"
    );
}

#[test]
fn qris_parse_charge_response_minimal() {
    let json = r#"{
        "status_code": "201",
        "transaction_id": "txn_002",
        "order_id": "QRIS-xxx",
        "gross_amount": "10000",
        "transaction_status": "pending",
        "status_message": "OK"
    }"#;
    let resp: QrisChargeResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.transaction_id, "txn_002");
    assert!(resp.qr_code_url.is_none());
}

#[test]
fn qris_parse_transaction_status() {
    let json = r#"{
        "transaction_id": "txn_003",
        "order_id": "QRIS-abc",
        "gross_amount": "50000",
        "transaction_status": "settlement",
        "status_code": "200",
        "status_message": "Success",
        "currency": "IDR",
        "payment_type": "qris"
    }"#;
    let tx: TransactionStatusResponse = serde_json::from_str(json).unwrap();
    assert_eq!(tx.transaction_status, "settlement");
    assert_eq!(tx.gross_amount, "50000");
    assert_eq!(tx.currency, "IDR");
}

#[test]
fn qris_parse_error_response() {
    let json = r#"{"status_code": "402", "status_message": "Transaction amount exceeds limit"}"#;
    let err: MidtransErrorResponse = serde_json::from_str(json).unwrap();
    assert_eq!(err.status_code, "402");
    assert_eq!(err.status_message, "Transaction amount exceeds limit");
}

#[test]
fn qris_parse_error_empty() {
    let json = r#"{"status_code": "500", "status_message": ""}"#;
    let err: MidtransErrorResponse = serde_json::from_str(json).unwrap();
    assert_eq!(err.status_code, "500");
    assert!(err.status_message.is_empty());
}

#[test]
fn qris_debug_masks_key() {
    let proc = QrisPaymentProcessor::new(&test_key(), false);
    let debug = format!("{:?}", proc);
    assert!(!debug.contains("test_key"));
    assert!(!debug.contains("MID-server"));
    assert!(debug.contains("***"));
}

#[test]
fn qris_clone_preserves_config() {
    let proc = QrisPaymentProcessor::new(&test_key(), true);
    let cloned = proc.clone();
    assert!(cloned.sandbox);
    let info = cloned.device_info();
    assert_eq!(info.vendor, "Midtrans");
}

#[test]
fn qris_base64_encoding() {
    let encoded = base64_standard("test:key");
    assert!(!encoded.is_empty());
    assert!(!encoded.contains('\n'));
}
