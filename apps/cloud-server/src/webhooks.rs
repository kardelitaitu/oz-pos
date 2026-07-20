//! Webhook receiver — accepts payment events from Stripe and Square,
//! verifies their signatures, and writes corresponding actions to the
//! `offline_queue` for the local POS terminal to pick up via sync.
//!
//! # Endpoints
//!
//! - `POST /api/webhooks/stripe` — Stripe payment_intent events
//! - `POST /api/webhooks/square` — Square charge/Payment events
//!
//! # Configuration
//!
//! | Variable | Required | Description |
//! |---|---|---|
//! | `STRIPE_WEBHOOK_SECRET` | For Stripe endpoint | Stripe webhook signing secret |
//! | `SQUARE_WEBHOOK_SIGNATURE_KEY` | For Square endpoint | Square webhook signature key |
//! | `SQUARE_WEBHOOK_URL` | For Square endpoint | Public webhook URL (used in signature verification) |
//!
//! # Flow
//!
//! 1. Gateway sends event → server verifies HMAC signature
//! 2. Parses event to extract `payment_intent_id` or `charge_id`
//! 3. Looks up matching payment record via `gateway_reference`
//! 4. Creates `offline_queue` item with action `finalize_sale`
//!    so the next sync cycle finalizes the pending sale

use axum::{Router, extract::State, http::StatusCode, routing::post};
use hmac::{Hmac, Mac};
use rusqlite::params;
use sha2::Sha256;

use crate::CloudServerState;

/// HMAC-SHA256 type alias for webhook signature verification.
type HmacSha256 = Hmac<Sha256>;

/// Build the webhooks router (unauthenticated — Stripe/Square verify
/// themselves via HMAC signatures, not JWT).
pub fn webhooks_router(state: CloudServerState) -> Router {
    Router::new()
        .route("/api/webhooks/stripe", post(stripe_webhook_handler))
        .route("/api/webhooks/square", post(square_webhook_handler))
        .with_state(state)
}

/// Stripe webhook event payload (minimal — we only need `type` and `id`).
#[derive(serde::Deserialize, Debug)]
struct StripeEvent {
    /// Event type (e.g. `payment_intent.succeeded`, `charge.captured`).
    r#type: String,
    /// Event data payload.
    data: StripeEventData,
}

#[derive(serde::Deserialize, Debug)]
struct StripeEventData {
    /// The object that triggered the event.
    object: serde_json::Value,
}

/// Square webhook event payload (minimal).
#[derive(serde::Deserialize, Debug)]
struct SquareEvent {
    /// Merchant ID
    #[allow(dead_code)]
    merchant_id: String,
    /// Event type (e.g. `payment.updated`, `payment.created`).
    r#type: String,
    /// Event ID
    #[allow(dead_code)]
    event_id: String,
    /// Event data
    data: SquareEventData,
}

#[derive(serde::Deserialize, Debug)]
struct SquareEventData {
    /// The object type that triggered the event.
    #[serde(rename = "type")]
    #[allow(dead_code)]
    object_type: String,
    /// Object ID (the payment/charge ID).
    id: String,
}

/// Extract the payment intent ID from a Stripe event object.
fn extract_stripe_payment_id(object: &serde_json::Value) -> Option<String> {
    // payment_intent.succeeded → object.id = "pi_xxx"
    // charge.captured → object.payment_intent = "pi_xxx"
    if let Some(id) = object
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|id| id.starts_with("pi_"))
    {
        return Some(id.to_owned());
    }
    // Try payment_intent field on charge objects
    if let Some(pi) = object
        .get("payment_intent")
        .and_then(|v| v.as_str())
        .filter(|pi| pi.starts_with("pi_"))
    {
        return Some(pi.to_owned());
    }
    None
}

/// Verify a Stripe webhook signature.
///
/// Stripe sends `Stripe-Signature: t=<timestamp>,v1=<signature>`.
/// The signature is HMAC-SHA256 of `<timestamp>.<payload>`.
/// See: <https://docs.stripe.com/webhooks/signatures>
fn verify_stripe_signature(payload: &[u8], signature_header: &str, secret: &str) -> bool {
    // Parse the signature header: t=...,v1=...
    let mut timestamp = None;
    let mut signature = None;
    for part in signature_header.split(',') {
        if let Some((key, value)) = part.split_once('=') {
            match key.trim() {
                "t" => timestamp = Some(value.trim()),
                "v1" => signature = Some(value.trim()),
                _ => {}
            }
        }
    }

    let (ts, sig) = match (timestamp, signature) {
        (Some(t), Some(s)) => (t, s),
        _ => return false,
    };

    // Build the signed payload: timestamp + "." + raw body
    let mut signed_bytes = Vec::with_capacity(ts.len() + 1 + payload.len());
    signed_bytes.extend_from_slice(ts.as_bytes());
    signed_bytes.push(b'.');
    signed_bytes.extend_from_slice(payload);

    // Compute expected HMAC
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(&signed_bytes);

    // Verify against the provided signature (hex-encoded)
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex::encode(expected);
    expected_hex == sig
}

/// Verify a Square webhook signature.
///
/// Square sends `x-square-hmacsha256-signature: <signature>`.
/// The signature is HMAC-SHA256 of `<webhook_url>.<body>.<timestamp>`.
/// See: <https://developer.squareup.com/docs/webhooks/step-verify>
fn verify_square_signature(
    payload: &[u8],
    signature_header: &str,
    webhook_url: &str,
    secret: &str,
    timestamp: &str,
) -> bool {
    let body_str = std::str::from_utf8(payload).unwrap_or("");
    let signed_payload = format!("{}.{}.{}", webhook_url, body_str, timestamp);

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());

    let expected = mac.finalize().into_bytes();
    let expected_hex = hex::encode(expected);
    expected_hex == signature_header
}

/// `POST /api/webhooks/stripe` — receive Stripe payment events.
async fn stripe_webhook_handler(
    State(state): State<CloudServerState>,
    headers: axum::http::HeaderMap,
    body_bytes: axum::body::Bytes,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    // 1. Extract Stripe-Signature header
    let signature_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "missing Stripe-Signature header".into(),
            )
        })?;

    // 2. Read the webhook secret from state (loaded from env at startup)
    let secret = state.stripe_webhook_secret.as_deref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "STRIPE_WEBHOOK_SECRET not configured".into(),
        )
    })?;

    // 3. Verify signature
    if !verify_stripe_signature(&body_bytes, signature_header, secret) {
        return Err((StatusCode::UNAUTHORIZED, "invalid webhook signature".into()));
    }

    // 4. Parse event
    let event: StripeEvent = serde_json::from_slice(&body_bytes)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid event body: {e}")))?;

    // 5. Extract payment intent ID
    let payment_id = extract_stripe_payment_id(&event.data.object).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "could not extract payment intent ID from event".into(),
        )
    })?;

    // 6. Look up the sale by gateway_reference
    let sale_id = lookup_sale_by_gateway_reference(&state, &payment_id).await?;

    // 7. Queue a finalize_sale action
    enqueue_finalize_sale(&state, &sale_id).await?;

    tracing::info!(payment_id, sale_id, event_type = %event.r#type, "stripe webhook processed");

    Ok(axum::Json(serde_json::json!({
        "status": "accepted",
        "sale_id": sale_id,
        "event_type": event.r#type,
    })))
}

/// `POST /api/webhooks/square` — receive Square payment events.
async fn square_webhook_handler(
    State(state): State<CloudServerState>,
    headers: axum::http::HeaderMap,
    body_bytes: axum::body::Bytes,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    // 1. Extract Square signature header
    let signature_header = headers
        .get("x-square-hmacsha256-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "missing x-square-hmacsha256-signature header".into(),
            )
        })?;

    // 2. Extract timestamp header
    let timestamp = headers
        .get("x-square-timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "missing x-square-timestamp header".into(),
            )
        })?;

    // 3. Read the webhook signature key from state (loaded from env at startup)
    let secret = state
        .square_webhook_signature_key
        .as_deref()
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SQUARE_WEBHOOK_SIGNATURE_KEY not configured".into(),
            )
        })?;

    // 4. Read the webhook URL from state (loaded from env at startup)
    let webhook_url = state.square_webhook_url.as_deref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SQUARE_WEBHOOK_URL not configured".into(),
        )
    })?;

    // 5. Verify signature
    if !verify_square_signature(
        &body_bytes,
        signature_header,
        webhook_url,
        secret,
        timestamp,
    ) {
        return Err((StatusCode::UNAUTHORIZED, "invalid webhook signature".into()));
    }

    // 6. Parse event
    let event: SquareEvent = serde_json::from_slice(&body_bytes)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid event body: {e}")))?;

    // Square uses payment IDs (not pi_xxx prefix). Use the data.id directly.
    let payment_id = event.data.id.clone();

    // 7. Look up the sale by gateway_reference
    let sale_id = lookup_sale_by_gateway_reference(&state, &payment_id).await?;

    // 8. Queue a finalize_sale action
    enqueue_finalize_sale(&state, &sale_id).await?;

    tracing::info!(payment_id, sale_id, event_type = %event.r#type, "square webhook processed");

    Ok(axum::Json(serde_json::json!({
        "status": "accepted",
        "sale_id": sale_id,
        "event_type": event.r#type,
    })))
}

/// Look up a sale by its `gateway_reference` in the payments table.
async fn lookup_sale_by_gateway_reference(
    state: &CloudServerState,
    gateway_ref: &str,
) -> Result<String, (StatusCode, String)> {
    let conn = state.db.lock().await;
    let sale_id: Option<String> = conn
        .query_row(
            "SELECT sale_id FROM payments WHERE gateway_reference = ?1 LIMIT 1",
            params![gateway_ref],
            |row| row.get(0),
        )
        .ok();

    sale_id.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("no sale found for gateway reference: {gateway_ref}"),
        )
    })
}

/// Enqueue a `finalize_sale` action into the offline_queue so the local
/// terminal can complete the pending sale via sync.
async fn enqueue_finalize_sale(
    state: &CloudServerState,
    sale_id: &str,
) -> Result<(), (StatusCode, String)> {
    let conn = state.db.lock().await;
    let id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let payload = serde_json::json!({
        "sale_id": sale_id,
    })
    .to_string();

    conn.execute(
        "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id)
         VALUES (?1, ?2, ?3, 'pending', ?4, 'default')",
        params![id, "finalize_sale", payload, now],
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to enqueue finalize_sale: {e}"),
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    fn fresh_db() -> rusqlite::Connection {
        oz_core::migrations::fresh_db()
    }

    fn test_state() -> CloudServerState {
        CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        }
    }

    fn test_state_with_stripe(secret: &str) -> CloudServerState {
        CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: Some(secret.to_owned()),
            square_webhook_signature_key: None,
            square_webhook_url: None,
        }
    }

    fn test_state_with_square(secret: &str, url: &str) -> CloudServerState {
        CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: Some(secret.to_owned()),
            square_webhook_url: Some(url.to_owned()),
        }
    }

    fn test_router() -> Router {
        let state = test_state();
        webhooks_router(state)
    }

    /// Seed a payment with a specific gateway_reference so the webhook
    /// handler can look up the sale.
    fn seed_payment(conn: &rusqlite::Connection, gateway_ref: &str, sale_id: &str) {
        // First seed a sale
        conn.execute(
            "INSERT OR IGNORE INTO sales (id, total_minor, currency, line_count, status, created_at)
             VALUES (?1, 1000, 'USD', 1, 'pending', '2026-07-01T00:00:00Z')",
            params![sale_id],
        ).ok();

        conn.execute(
            "INSERT OR IGNORE INTO payments (id, sale_id, method, amount_minor, currency,
                                              gateway_reference, gateway_status, created_at)
             VALUES (?1, ?2, 'card', 1000, 'USD', ?3, 'requires_capture', '2026-07-01T00:00:00Z')",
            params![uuid::Uuid::now_v7().to_string(), sale_id, gateway_ref],
        )
        .unwrap();
    }

    /// Build a valid Stripe signature for the given payload and secret.
    fn stripe_signature(payload: &[u8], secret: &str) -> String {
        let timestamp = "1719000000";
        let mut signed_bytes = Vec::with_capacity(timestamp.len() + 1 + payload.len());
        signed_bytes.extend_from_slice(timestamp.as_bytes());
        signed_bytes.push(b'.');
        signed_bytes.extend_from_slice(payload);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(&signed_bytes);
        let expected = hex::encode(mac.finalize().into_bytes());
        format!("t={},v1={}", timestamp, expected)
    }

    // ── Stripe signature verification ─────────────────────────────

    #[test]
    fn verify_stripe_signature_valid() {
        let secret = "whsec_test_secret_key_12345";
        let payload = b"{\"type\":\"payment_intent.succeeded\"}";
        let header = stripe_signature(payload, secret);
        assert!(verify_stripe_signature(payload, &header, secret));
    }

    #[test]
    fn verify_stripe_signature_invalid() {
        let secret = "whsec_test_secret_key_12345";
        let payload = b"{\"type\":\"payment_intent.succeeded\"}";
        let header = "t=1719000000,v1=invalid_signature_hex";
        assert!(!verify_stripe_signature(payload, header, secret));
    }

    #[test]
    fn verify_stripe_signature_wrong_secret() {
        let secret = "whsec_correct_secret";
        let wrong_secret = "whsec_wrong_secret";
        let payload = b"test_payload";
        let header = stripe_signature(payload, secret);
        assert!(!verify_stripe_signature(payload, &header, wrong_secret));
    }

    #[test]
    fn verify_stripe_signature_malformed_header() {
        let secret = "whsec_test";
        assert!(!verify_stripe_signature(
            b"{}",
            "not-a-valid-header",
            secret
        ));
        assert!(!verify_stripe_signature(b"{}", "v1=abc123", secret));
        assert!(!verify_stripe_signature(b"{}", "t=123", secret));
        assert!(!verify_stripe_signature(b"{}", "", secret));
    }

    // ── Square signature verification ─────────────────────────────

    #[test]
    fn verify_square_signature_valid() {
        let secret = "sq0csp-test-signature-key";
        let payload = b"{\"merchant_id\":\"m_001\",\"type\":\"payment.updated\"}";
        let url = "https://example.com/api/webhooks/square";
        let timestamp = "2026-07-01T12:00:00Z";

        let signed = format!(
            "{}.{}.{}",
            url,
            std::str::from_utf8(payload).unwrap(),
            timestamp
        );

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed.as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());

        assert!(verify_square_signature(
            payload, &expected, url, secret, timestamp
        ));
    }

    #[test]
    fn verify_square_signature_invalid() {
        let secret = "sq0csp-test-key";
        assert!(!verify_square_signature(
            b"{}",
            "invalid_signature",
            "https://example.com",
            secret,
            "2026-07-01T12:00:00Z"
        ));
    }

    // ── Stripe payment ID extraction ──────────────────────────────

    #[test]
    fn extract_stripe_payment_id_from_intent() {
        let obj = serde_json::json!({
            "id": "pi_3NcdefghIJklmnOPQRSTUvwx",
            "amount": 1000,
            "status": "succeeded"
        });
        assert_eq!(
            extract_stripe_payment_id(&obj).unwrap(),
            "pi_3NcdefghIJklmnOPQRSTUvwx"
        );
    }

    #[test]
    fn extract_stripe_payment_id_from_charge() {
        let obj = serde_json::json!({
            "id": "ch_3NcdefghIJklmnOPQRSTUvwx",
            "payment_intent": "pi_3NcdefghIJklmnOPQRSTUvwx",
            "amount": 1000,
        });
        assert_eq!(
            extract_stripe_payment_id(&obj).unwrap(),
            "pi_3NcdefghIJklmnOPQRSTUvwx"
        );
    }

    #[test]
    fn extract_stripe_payment_id_non_stripe_id() {
        let obj = serde_json::json!({"id": "evt_001"});
        assert!(extract_stripe_payment_id(&obj).is_none());
    }

    #[test]
    fn extract_stripe_payment_id_no_payment_intent() {
        let obj = serde_json::json!({"id": "ch_001"});
        assert!(extract_stripe_payment_id(&obj).is_none());
    }

    // ── Sale lookup ───────────────────────────────────────────────

    #[tokio::test]
    async fn lookup_sale_by_gateway_ref_found() {
        let state = test_state();
        {
            let conn = state.db.lock().await;
            seed_payment(&conn, "pi_test_123", "sale-001");
        }
        let sale_id = lookup_sale_by_gateway_reference(&state, "pi_test_123")
            .await
            .unwrap();
        assert_eq!(sale_id, "sale-001");
    }

    #[tokio::test]
    async fn lookup_sale_by_gateway_ref_not_found() {
        let state = test_state();
        let result = lookup_sale_by_gateway_reference(&state, "pi_nonexistent").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::NOT_FOUND);
    }

    // ── Webhook endpoint integration ──────────────────────────────

    #[tokio::test]
    async fn stripe_webhook_missing_signature_returns_400() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/webhooks/stripe")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"type":"payment_intent.succeeded","data":{"object":{"id":"pi_test","amount":1000}}}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn stripe_webhook_invalid_signature_returns_401() {
        let state = test_state_with_stripe("whsec_test_secret");
        let app = webhooks_router(state);
        let req = Request::builder()
            .method("POST")
            .uri("/api/webhooks/stripe")
            .header("Content-Type", "application/json")
            .header("Stripe-Signature", "t=1719000000,v1=invalid_sig")
            .body(Body::from(r#"{"type":"payment_intent.succeeded","data":{"object":{"id":"pi_test","amount":1000}}}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn stripe_webhook_valid_signature_happy_path() {
        let secret = "whsec_test_webhook_secret_123";

        let state = test_state_with_stripe(secret);
        let sale_id = "sale-webhook-001";
        {
            let conn = state.db.lock().await;
            seed_payment(&conn, "pi_3NcdefghIJklmnOPQRSTUvwx", sale_id);
        }

        let app = webhooks_router(state.clone());

        let payload = br#"{"type":"payment_intent.succeeded","data":{"object":{"id":"pi_3NcdefghIJklmnOPQRSTUvwx","amount":1000,"status":"succeeded"}}}"#;
        let signature = stripe_signature(payload, secret);

        let req = Request::builder()
            .method("POST")
            .uri("/api/webhooks/stripe")
            .header("Content-Type", "application/json")
            .header("Stripe-Signature", &signature)
            .body(Body::from(payload.to_vec()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["status"], "accepted");
        assert_eq!(json["sale_id"], sale_id);

        // Verify offline_queue item was created
        {
            let conn = state.db.lock().await;
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM offline_queue WHERE action = 'finalize_sale'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "should have enqueued one finalize_sale action");
        }
    }

    #[tokio::test]
    async fn stripe_webhook_sale_not_found_returns_404() {
        let secret = "whsec_test_secret";

        let state = test_state_with_stripe(secret);
        let app = webhooks_router(state);

        let payload = br#"{"type":"payment_intent.succeeded","data":{"object":{"id":"pi_unknown","amount":1000}}}"#;
        let signature = stripe_signature(payload, secret);

        let req = Request::builder()
            .method("POST")
            .uri("/api/webhooks/stripe")
            .header("Content-Type", "application/json")
            .header("Stripe-Signature", &signature)
            .body(Body::from(payload.to_vec()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn square_webhook_missing_signature_returns_400() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/webhooks/square")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"merchant_id":"m_001","type":"payment.updated","event_id":"evt_001","data":{"type":"payment","id":"pmt_001"}}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn square_webhook_valid_signature_happy_path() {
        let secret = "sq0csp-test-webhook-key";
        let url = "https://example.com/api/webhooks/square";

        let state = test_state_with_square(secret, url);
        let sale_id = "sale-square-001";
        {
            let conn = state.db.lock().await;
            seed_payment(&conn, "pmt_square_001", sale_id);
        }

        let app = webhooks_router(state.clone());

        let payload = br#"{"merchant_id":"m_001","type":"payment.updated","event_id":"evt_001","data":{"type":"payment","id":"pmt_square_001"}}"#;
        let timestamp = "2026-07-01T12:00:00Z";

        // Build Square signature
        let body_str = std::str::from_utf8(payload).unwrap();
        let signed = format!("{}.{}.{}", url, body_str, timestamp);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let req = Request::builder()
            .method("POST")
            .uri("/api/webhooks/square")
            .header("Content-Type", "application/json")
            .header("x-square-hmacsha256-signature", &signature)
            .header("x-square-timestamp", timestamp)
            .body(Body::from(payload.to_vec()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["status"], "accepted");
        assert_eq!(json["sale_id"], sale_id);

        // Verify offline_queue item was created
        {
            let conn = state.db.lock().await;
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM offline_queue WHERE action = 'finalize_sale'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "should have enqueued one finalize_sale action");
        }
    }
}
