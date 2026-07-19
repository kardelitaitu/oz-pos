//! Integration tests for [`StripePaymentProcessor`] using `wiremock` to
//! simulate the Stripe REST API.
//!
//! Each test creates its own `MockServer` to avoid race conditions from
//! shared mutable state when tests run in parallel.

use foundation::{Currency, Money};
use oz_payment::PaymentProcessor;
use oz_payment::drivers::stripe::StripePaymentProcessor;
use oz_payment::types::{PaymentMethod, PaymentRequest};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

mod fixtures;
use fixtures::stripe_processor;

/// Helper: construct a USD currency.
fn usd() -> Currency {
    "USD".parse().unwrap()
}

/// Helper: create a request for the given major-unit amount.
fn request(major_amount: i64) -> PaymentRequest {
    PaymentRequest {
        amount: Money::from_major(major_amount, usd()).unwrap(),
        reference: None,
        description: None,
        idempotency_key: None,
    }
}

// ── Authorize ────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn authorize_happy_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payment_intents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_mock_authorize_001",
            "amount": 1500,
            "amount_received": 1500,
            "currency": "usd",
            "status": "requires_capture"
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let result = proc.authorize(&request(15)).await.unwrap();
    assert!(result.success);
    assert_eq!(result.transaction_id.unwrap(), "pi_mock_authorize_001");
    assert_eq!(result.amount_charged.minor_units, 1500);
}

#[tokio::test(flavor = "multi_thread")]
async fn authorize_declined() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payment_intents"))
        .respond_with(ResponseTemplate::new(402).set_body_json(serde_json::json!({
            "error": {
                "type": "card_error",
                "message": "Your card was declined.",
                "code": "card_declined"
            }
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let err = proc.authorize(&request(10)).await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("declined"),
        "expected declined error, got: {msg}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn authorize_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payment_intents"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let err = proc.authorize(&request(10)).await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("500"), "expected HTTP 500 error, got: {msg}");
}

#[tokio::test(flavor = "multi_thread")]
async fn authorize_non_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payment_intents"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let err = proc.authorize(&request(10)).await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("failed to parse"),
        "expected parse error, got: {msg}"
    );
}

// ── Sale (authorize + capture) ───────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn sale_happy_path() {
    let mock_server = MockServer::start().await;

    // Authorize response
    Mock::given(method("POST"))
        .and(path("/payment_intents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_sale_001",
            "amount": 2000,
            "amount_received": 2000,
            "currency": "usd",
            "status": "requires_capture"
        })))
        .mount(&mock_server)
        .await;

    // Capture response
    Mock::given(method("POST"))
        .and(path("/payment_intents/pi_sale_001/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_sale_001",
            "amount": 2000,
            "amount_received": 2000,
            "currency": "usd",
            "status": "succeeded"
        })))
        .mount(&mock_server)
        .await;

    let proc = StripePaymentProcessor::new_with_endpoint("sk_test_mock", &mock_server.uri(), false);

    let result = proc.sale(&request(20)).await.unwrap();
    assert!(result.success);
    assert_eq!(result.amount_charged.minor_units, 2000);
}

// ── Capture ──────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn capture_happy_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payment_intents/pi_capture_001/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_capture_001",
            "amount": 3000,
            "amount_received": 3000,
            "currency": "usd",
            "status": "succeeded"
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let result = proc.capture("pi_capture_001").await.unwrap();
    assert!(result.success);
    assert_eq!(result.amount_charged.minor_units, 3000);
}

#[tokio::test(flavor = "multi_thread")]
async fn capture_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payment_intents/pi_missing/capture"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {
                "type": "invalid_request_error",
                "message": "No such payment_intent: pi_missing",
                "code": "resource_missing"
            }
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let err = proc.capture("pi_missing").await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("resource_missing") || msg.contains("No such"),
        "unexpected error: {msg}"
    );
}

// ── Refund ───────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn refund_happy_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/refunds"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "re_mock_refund_001",
            "amount": 5000,
            "currency": "usd",
            "status": "succeeded"
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let result = proc.refund("pi_to_refund", None).await.unwrap();
    assert!(result.success);
    assert_eq!(result.transaction_id.unwrap(), "re_mock_refund_001");
    assert_eq!(result.amount_charged.minor_units, 5000);
}

#[tokio::test(flavor = "multi_thread")]
async fn refund_declined() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/refunds"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": {
                "type": "invalid_request_error",
                "message": "This payment_intent has not been captured.",
                "code": "payment_intent_uncaptured"
            }
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let err = proc.refund("pi_uncaptured", None).await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("uncaptured") || msg.contains("not been captured"),
        "unexpected error: {msg}"
    );
}

// ── Receipt ──────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn receipt_happy_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/payment_intents/pi_receipt_001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_receipt_001",
            "amount": 1000,
            "amount_received": 1000,
            "currency": "usd",
            "status": "succeeded"
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let receipt = proc.receipt("pi_receipt_001").await.unwrap();
    assert_eq!(receipt.transaction_id, "pi_receipt_001");
    assert_eq!(receipt.method, PaymentMethod::Card);
    assert_eq!(receipt.amount.minor_units, 1000);
}

#[tokio::test(flavor = "multi_thread")]
async fn receipt_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/payment_intents/pi_nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {
                "type": "invalid_request_error",
                "message": "No such payment_intent: pi_nonexistent"
            }
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let err = proc.receipt("pi_nonexistent").await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("No such"), "unexpected error: {msg}");
}

// ── Request body verification ───────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn authorize_sends_correct_form_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payment_intents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_form_001",
            "amount": 5000,
            "amount_received": 5000,
            "currency": "usd",
            "status": "requires_capture"
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let req = PaymentRequest {
        amount: Money::from_major(50, usd()).unwrap(),
        reference: None,
        description: Some("Test order #42".into()),
        idempotency_key: None,
    };

    let _ = proc.authorize(&req).await.unwrap();

    let received = mock_server.received_requests().await.unwrap_or_default();
    assert_eq!(received.len(), 1, "expected 1 request");

    let body = std::str::from_utf8(&received[0].body).unwrap();
    assert!(body.contains("amount=5000"), "body: {body}");
    assert!(body.contains("currency=usd"), "body: {body}");
    assert!(
        body.contains("payment_method_types%5B%5D=card"),
        "body: {body}"
    );
    assert!(body.contains("capture_method=manual"), "body: {body}");
    assert!(
        body.contains("description=Test+order+%2342"),
        "body: {body}"
    );

    let auth_header = received[0]
        .headers
        .get("Authorization")
        .expect("Authorization header should be present");
    assert_eq!(
        auth_header, "Bearer sk_test_shared_fixture",
        "unexpected auth header: {auth_header:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn refund_sends_correct_form_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/refunds"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "re_form_001",
            "amount": 3000,
            "currency": "usd",
            "status": "succeeded"
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let _ = proc.refund("pi_to_refund", None).await.unwrap();

    let received = mock_server.received_requests().await.unwrap_or_default();
    assert_eq!(received.len(), 1);

    let body = std::str::from_utf8(&received[0].body).unwrap();
    assert!(body.contains("payment_intent=pi_to_refund"), "body: {body}");
}

// ── Partial amount refund ────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn refund_partial_amount() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/refunds"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "re_partial_001",
            "amount": 1000,
            "currency": "usd",
            "status": "succeeded"
        })))
        .mount(&mock_server)
        .await;

    let proc = stripe_processor(&mock_server.uri(), false);

    let partial_amount = Some(Money::from_major(10, usd()).unwrap());
    let result = proc.refund("pi_partial", partial_amount).await.unwrap();
    assert!(result.success);
    assert_eq!(result.amount_charged.minor_units, 1000);
}

// ─── Card-present ────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn authorize_card_present_sends_correct_method_type() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/payment_intents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "pi_terminal_001",
            "amount": 2000,
            "amount_received": 2000,
            "currency": "usd",
            "status": "requires_capture"
        })))
        .mount(&mock_server)
        .await;

    let proc =
        StripePaymentProcessor::new_with_endpoint("sk_test_terminal", &mock_server.uri(), true);

    let _ = proc.authorize(&request(20)).await.unwrap();

    let received = mock_server.received_requests().await.unwrap_or_default();
    assert_eq!(received.len(), 1);

    let body = std::str::from_utf8(&received[0].body).unwrap();
    assert!(
        body.contains("payment_method_types%5B%5D=card_present"),
        "body: {body}"
    );
}
