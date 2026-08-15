//! Webhook receiver — accepts payment events from Stripe and Square,
//! verifies their signatures, and routes them:
//!
//! - **Stripe subscription lifecycle events** (`customer.subscription.*`,
//!   `checkout.session.completed`, `invoice.paid`) update the tenant's
//!   sync plan via `set_tenant_plan` (ADR sync-plan-gating) — a paid
//!   subscription upgrades the tenant to `pro`, cancellation downgrades
//!   to `free`.
//! - **Payment events** (`payment_intent.*`, `charge.*`, Square
//!   payments) write a `finalize_sale` action to the `offline_queue` for
//!   the local POS terminal to pick up via sync.
//!
//! # Endpoints
//!
//! - `POST /api/webhooks/stripe` — Stripe events (subscriptions + payments)
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
//! 2. Routes by event type (subscription → plan update, else payment)
//! 3. Subscription: resolves the tenant (metadata or `stripe_customers`
//!    mapping) and calls `Store::set_tenant_plan`
//! 4. Payment: looks up matching payment record via `gateway_reference`
//!    and creates an `offline_queue` `finalize_sale` action so the next
//!    sync cycle finalizes the pending sale

use axum::{
    Router, extract::State, http::StatusCode, middleware, response::Response, routing::post,
};
use hmac::{Hmac, Mac};
use rusqlite::params;
use sha2::Sha256;

use crate::CloudServerState;

/// HMAC-SHA256 type alias for webhook signature verification.
type HmacSha256 = Hmac<Sha256>;

/// Build the webhooks router (unauthenticated — Stripe/Square verify
/// themselves via HMAC signatures, not JWT).
///
/// A response-status middleware counts every 5xx into
/// `webhook_5xx_total` — webhooks are the payment-authenticity boundary,
/// so a server-side failure (misconfigured secret, DB error, bad event
/// shape) is an operator-visible signal that payment/plan state may be
/// stale.
pub fn webhooks_router(state: CloudServerState) -> Router {
    Router::new()
        .route("/api/webhooks/stripe", post(stripe_webhook_handler))
        .route("/api/webhooks/square", post(square_webhook_handler))
        .layer(middleware::from_fn(count_webhook_5xx))
        .with_state(state)
}

/// Axum middleware that counts 5xx responses from the webhook handlers
/// into the `webhook_5xx_total` Prometheus counter.
async fn count_webhook_5xx(request: axum::extract::Request, next: middleware::Next) -> Response {
    let response = next.run(request).await;
    if response.status().is_server_error() {
        crate::metrics::WEBHOOK_5XX_TOTAL.inc();
    }
    response
}

/// Stripe webhook event payload (minimal — we only need `type` and `id`).
#[derive(serde::Deserialize, Debug)]
struct StripeEvent {
    /// Unique event identifier for idempotency (Stripe redelivers webhooks).
    id: String,
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

/// Stripe event types that carry subscription lifecycle state.
///
/// These update the tenant's sync plan (ADR sync-plan-gating); all other
/// events (payment_intent.*, charge.*, …) finalise a sale.
fn is_subscription_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "customer.subscription.created"
            | "customer.subscription.updated"
            | "customer.subscription.deleted"
            | "checkout.session.completed"
            | "invoice.paid"
    )
}

/// The sync plan a subscription event implies, from the subscription status.
///
/// - `active` / `trialing` / `past_due` → `Pro` (paid or in grace)
/// - `canceled` / `unpaid` / `incomplete_expired` → `Free` (no access)
/// - anything else (e.g. `incomplete`) → `None`, meaning "leave the plan
///   unchanged" — the tenant keeps its current plan until a clearer state.
fn plan_for_subscription_status(status: Option<&str>) -> Option<oz_core::TenantPlan> {
    match status {
        Some("active" | "trialing" | "past_due") => Some(oz_core::TenantPlan::Pro),
        Some("canceled" | "unpaid" | "incomplete_expired") => Some(oz_core::TenantPlan::Free),
        _ => None,
    }
}

/// Resolve the OZ-POS tenant for a subscription event.
///
/// Prefers the `tenant_id` metadata set on the Checkout Session / subscription
/// (Stripe forwards object metadata onto the subscription). Falls back to the
/// `stripe_customers` mapping for events that carry only a customer id
/// (`invoice.paid`, `customer.subscription.deleted`, …). Returns `None` when
/// the tenant cannot be determined.
async fn resolve_subscription_tenant(
    state: &CloudServerState,
    object: &serde_json::Value,
) -> Result<Option<String>, (StatusCode, String)> {
    // 1. Metadata: data.object.metadata.tenant_id
    if let Some(tenant) = object
        .get("metadata")
        .and_then(|m| m.get("tenant_id"))
        .and_then(|v| v.as_str())
    {
        // Also (re)record the customer mapping while we have it — later
        // events (invoice.paid, deleted) carry only the customer id.
        if let Some(customer) = object.get("customer").and_then(|v| v.as_str()) {
            if let Some(pool) = &state.pg {
                let client = pool.get().await.map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("failed to record stripe customer mapping: {e}"),
                    )
                })?;
                client
                    .execute(
                        "INSERT INTO stripe_customers (stripe_customer_id, tenant_id, updated_at)
                         VALUES ($1, $2, $3)
                         ON CONFLICT (stripe_customer_id) DO UPDATE SET
                            tenant_id = excluded.tenant_id,
                            updated_at = excluded.updated_at",
                        &[
                            &customer,
                            &tenant,
                            &chrono::Utc::now()
                                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        ],
                    )
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("failed to record stripe customer mapping: {e}"),
                        )
                    })?;
            } else {
                let conn = state.db.lock().await;
                oz_core::Store::new(&conn)
                    .set_stripe_customer(customer, tenant)
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("failed to record stripe customer mapping: {e}"),
                        )
                    })?;
            }
        }
        return Ok(Some(tenant.to_owned()));
    }

    // 2. Mapping table via the customer id.
    if let Some(customer) = object.get("customer").and_then(|v| v.as_str()) {
        let tenant = if let Some(pool) = &state.pg {
            let client = pool.get().await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to look up stripe customer mapping: {e}"),
                )
            })?;
            client
                .query_opt(
                    "SELECT tenant_id FROM stripe_customers WHERE stripe_customer_id = $1",
                    &[&customer],
                )
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("failed to look up stripe customer mapping: {e}"),
                    )
                })?
                .map(|r| r.get::<_, String>(0))
        } else {
            let conn = state.db.lock().await;
            oz_core::Store::new(&conn)
                .get_tenant_for_stripe_customer(customer)
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("failed to look up stripe customer mapping: {e}"),
                    )
                })?
        };
        return Ok(tenant);
    }

    Ok(None)
}

/// Handle a subscription lifecycle event by setting the tenant's sync plan.
///
/// Returns `{"status":"ignored"}` (200) when the tenant cannot be resolved
/// so Stripe stops retrying, and `{"status":"accepted","plan":…}` on
/// success.
async fn handle_subscription_event(
    state: &CloudServerState,
    event: &StripeEvent,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    let tenant_id = match resolve_subscription_tenant(state, &event.data.object).await? {
        Some(t) => t,
        None => {
            tracing::warn!(event_type = %event.r#type, "subscription event with unresolvable tenant — ignoring");
            record_event_processed(state, &event.id, "stripe", Some(&event.r#type)).await;
            return Ok(axum::Json(serde_json::json!({
                "status": "ignored",
                "event_type": event.r#type,
            })));
        }
    };

    // checkout.session.completed and invoice.paid imply an active
    // subscription even though their object carries no status field.
    let plan = match event.r#type.as_str() {
        "checkout.session.completed" | "invoice.paid" => Some(oz_core::TenantPlan::Pro),
        "customer.subscription.deleted" => Some(oz_core::TenantPlan::Free),
        _ => plan_for_subscription_status(event.data.object.get("status").and_then(|v| v.as_str())),
    };

    let Some(plan) = plan else {
        tracing::debug!(event_type = %event.r#type, tenant_id, "subscription status leaves plan unchanged");
        record_event_processed(state, &event.id, "stripe", Some(&event.r#type)).await;
        return Ok(axum::Json(serde_json::json!({
            "status": "accepted",
            "tenant_id": tenant_id,
            "plan": "unchanged",
            "event_type": event.r#type,
        })));
    };

    if let Some(pool) = &state.pg {
        oz_api::pg::set_tenant_plan(pool, &tenant_id, plan)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to set tenant plan: {e}"),
                )
            })?;
    } else {
        let conn = state.db.lock().await;
        oz_core::Store::new(&conn)
            .set_tenant_plan(&tenant_id, plan)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to set tenant plan: {e}"),
                )
            })?;
        drop(conn);
    }

    tracing::info!(tenant_id, plan = plan.as_db_str(), event_type = %event.r#type, "stripe subscription updated tenant plan");
    record_event_processed(state, &event.id, "stripe", Some(&event.r#type)).await;

    Ok(axum::Json(serde_json::json!({
        "status": "accepted",
        "tenant_id": tenant_id,
        "plan": plan.as_db_str(),
        "event_type": event.r#type,
    })))
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

    // 4b. Idempotency: skip already-processed events.
    //     Stripe guarantees at-least-once delivery; redelivered events
    //     must not double-count a subscription upgrade or payment capture.
    if event_already_processed(&state, &event.id).await {
        tracing::debug!(event_id = %event.id, event_type = %event.r#type, "webhook already processed — skipping");
        return Ok(axum::Json(serde_json::json!({
            "status": "already_processed",
            "event_id": event.id,
            "event_type": event.r#type,
        })));
    }

    // 5. Subscription lifecycle events update the tenant's sync plan
    //    (ADR sync-plan-gating) instead of finalising a sale.
    if is_subscription_event(&event.r#type) {
        return handle_subscription_event(&state, &event).await;
    }

    // 6. Extract payment intent ID
    let payment_id = extract_stripe_payment_id(&event.data.object).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "could not extract payment intent ID from event".into(),
        )
    })?;

    // 7. Look up the sale (and its owner tenant) by gateway_reference
    let (sale_id, tenant_id) = lookup_sale_by_gateway_reference(&state, &payment_id).await?;

    // 8. Queue a finalize_sale action under the sale's tenant
    enqueue_finalize_sale(&state, &sale_id, &tenant_id).await?;

    tracing::info!(payment_id, sale_id, tenant_id, event_type = %event.r#type, "stripe webhook processed");
    record_event_processed(&state, &event.id, "stripe", Some(&event.r#type)).await;

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

    // 6b. Idempotency: skip already-processed events
    if square_event_already_processed(&state, &event.event_id).await {
        tracing::debug!(event_id = %event.event_id, event_type = %event.r#type, "square webhook already processed — skipping");
        return Ok(axum::Json(serde_json::json!({
            "status": "already_processed",
            "event_id": event.event_id,
            "event_type": event.r#type,
        })));
    }

    // Square uses payment IDs (not pi_xxx prefix). Use the data.id directly.
    let payment_id = event.data.id.clone();

    // 7. Look up the sale (and its owner tenant) by gateway_reference
    let (sale_id, tenant_id) = lookup_sale_by_gateway_reference(&state, &payment_id).await?;

    // 8. Queue a finalize_sale action under the sale's tenant
    enqueue_finalize_sale(&state, &sale_id, &tenant_id).await?;

    tracing::info!(payment_id, sale_id, tenant_id, event_type = %event.r#type, "square webhook processed");
    record_event_processed(&state, &event.event_id, "square", Some(&event.r#type)).await;

    Ok(axum::Json(serde_json::json!({
        "status": "accepted",
        "sale_id": sale_id,
        "event_type": event.r#type,
    })))
}

// ── Webhook idempotency helpers ─────────────────────────────────────
/// Check whether a webhook event has already been processed.
///
/// Pure check -- does not insert. The recording happens in
/// [`record_event_processed`] after all side effects succeed.
/// Read `processed_webhooks` dedup state, backend-aware.
///
/// Returns `true` when the event id already has a row. Backend failures
/// degrade to `false` ("not processed") exactly like the historical SQLite
/// path — a dedup miss is at-least-once, which the webhook contract already
/// tolerates; the subsequent `record_event_processed` upsert is a no-op on
/// conflict.
async fn event_already_processed(state: &CloudServerState, event_id: &str) -> bool {
    if let Some(pool) = &state.pg {
        let Ok(client) = pool.get().await else {
            return false;
        };
        return client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM processed_webhooks WHERE event_id = $1)",
                &[&event_id],
            )
            .await
            .map(|r| r.get::<_, bool>(0))
            .unwrap_or(false);
    }
    let conn = state.db.lock().await;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM processed_webhooks WHERE event_id = ?1",
            rusqlite::params![event_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    count > 0
}

/// Like [`event_already_processed`] but for Square events.
async fn square_event_already_processed(state: &CloudServerState, event_id: &str) -> bool {
    event_already_processed(state, event_id).await
}

/// Record a webhook event as successfully processed.
///
/// Called after all side effects (plan update, sale finalization) have
/// succeeded. If the row was already inserted by [`event_already_processed`]
/// this is a no-op.
async fn record_event_processed(
    state: &CloudServerState,
    event_id: &str,
    provider: &str,
    event_type: Option<&str>,
) {
    if let Some(pool) = &state.pg {
        if let Ok(client) = pool.get().await {
            let _ = client
                .execute(
                    "INSERT INTO processed_webhooks (event_id, provider, event_type) VALUES ($1, $2, $3)
                     ON CONFLICT (event_id) DO NOTHING",
                    &[&event_id, &provider, &event_type],
                )
                .await;
        }
        return;
    }
    let conn = state.db.lock().await;
    let _ = conn.execute(
        "INSERT OR IGNORE INTO processed_webhooks (event_id, provider, event_type) VALUES (?1, ?2, ?3)",
        rusqlite::params![event_id, provider, event_type],
    );
}

/// Look up a sale by its `gateway_reference` in the payments table.
async fn lookup_sale_by_gateway_reference(
    state: &CloudServerState,
    gateway_ref: &str,
) -> Result<(String, String), (StatusCode, String)> {
    // Returns `(sale_id, tenant_id)` so the caller can enqueue the
    // finalize action under the sale's owner.
    let row: Option<(String, String)> = if let Some(pool) = &state.pg {
        let Ok(client) = pool.get().await else {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to acquire database connection".into(),
            ));
        };
        client
            .query_opt(
                "SELECT p.sale_id, s.tenant_id FROM payments p\n                 JOIN sales s ON p.sale_id = s.id\n                 WHERE p.gateway_reference = $1 LIMIT 1",
                &[&gateway_ref],
            )
            .await
            .ok()
            .flatten()
            .map(|r| (r.get::<_, String>(0), r.get::<_, String>(1)))
    } else {
        let conn = state.db.lock().await;
        conn.query_row(
            "SELECT p.sale_id, s.tenant_id FROM payments p\n             JOIN sales s ON p.sale_id = s.id\n             WHERE p.gateway_reference = ?1 LIMIT 1",
            params![gateway_ref],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .ok()
    };

    row.ok_or_else(|| {
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
    tenant_id: &str,
) -> Result<(), (StatusCode, String)> {
    let id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let payload = serde_json::json!({
        "sale_id": sale_id,
    })
    .to_string();

    if let Some(pool) = &state.pg {
        let client = pool.get().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to enqueue finalize_sale: {e}"),
            )
        })?;
        client
            .execute(
                "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id)
                 VALUES ($1, $2, $3, 'pending', $4, $5)",
                &[&id, &"finalize_sale", &payload, &now, &tenant_id],
            )
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to enqueue finalize_sale: {e}"),
                )
            })?;
        return Ok(());
    }

    let conn = state.db.lock().await;
    conn.execute(
        "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id)
         VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
        params![id, "finalize_sale", payload, now, tenant_id],
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
            pg: None,
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        }
    }

    fn test_state_with_stripe(secret: &str) -> CloudServerState {
        CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            pg: None,
            started_at: Instant::now(),
            stripe_webhook_secret: Some(secret.to_owned()),
            square_webhook_signature_key: None,
            square_webhook_url: None,
        }
    }

    fn test_state_with_square(secret: &str, url: &str) -> CloudServerState {
        CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            pg: None,
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
        let (sale_id, tenant_id) = lookup_sale_by_gateway_reference(&state, "pi_test_123")
            .await
            .unwrap();
        assert_eq!(sale_id, "sale-001");
        assert_eq!(
            tenant_id, "default",
            "unscoped seeded sale belongs to default"
        );
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

        let payload = br#"{"id":"evt_test_happy","type":"payment_intent.succeeded","data":{"object":{"id":"pi_3NcdefghIJklmnOPQRSTUvwx","amount":1000,"status":"succeeded"}}}"#;
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

    // ── Subscription lifecycle → tenant plan (ADR sync-plan-gating) ──

    /// Send a signed Stripe subscription event through the router.
    async fn post_stripe_subscription(
        state: &CloudServerState,
        event_type: &str,
        object: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let app = webhooks_router(state.clone());
        let secret = state.stripe_webhook_secret.clone().unwrap();
        let payload = serde_json::json!({
            "id": format!("evt_test_{}", uuid::Uuid::now_v7()),
            "type": event_type,
            "data": { "object": object },
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
        let signature = stripe_signature(&bytes, &secret);

        let req = Request::builder()
            .method("POST")
            .uri("/api/webhooks/stripe")
            .header("Content-Type", "application/json")
            .header("Stripe-Signature", &signature)
            .body(Body::from(bytes))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value =
            serde_json::from_slice(&body_bytes).unwrap_or(serde_json::json!({}));
        (status, json)
    }

    fn tenant_plan(state: &CloudServerState, tenant_id: &str) -> Option<oz_core::TenantPlan> {
        let conn = state.db.try_lock().unwrap();
        oz_core::Store::new(&conn)
            .get_tenant_plan(tenant_id)
            .unwrap()
    }

    #[tokio::test]
    async fn subscription_created_upgrades_tenant_to_pro() {
        let state = test_state_with_stripe("whsec_sub_test");
        let (status, json) = post_stripe_subscription(
            &state,
            "customer.subscription.created",
            serde_json::json!({
                "id": "sub_abc",
                "customer": "cus_123",
                "status": "active",
                "metadata": { "tenant_id": "tenant-a" },
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "accepted");
        assert_eq!(json["plan"], "pro");
        assert_eq!(
            tenant_plan(&state, "tenant-a"),
            Some(oz_core::TenantPlan::Pro),
            "paid subscription must upgrade the tenant's sync plan"
        );
    }

    #[tokio::test]
    async fn subscription_created_records_customer_mapping() {
        let state = test_state_with_stripe("whsec_sub_test");
        post_stripe_subscription(
            &state,
            "customer.subscription.created",
            serde_json::json!({
                "id": "sub_abc",
                "customer": "cus_123",
                "status": "active",
                "metadata": { "tenant_id": "tenant-a" },
            }),
        )
        .await;
        let conn = state.db.try_lock().unwrap();
        let tenant = oz_core::Store::new(&conn)
            .get_tenant_for_stripe_customer("cus_123")
            .unwrap();
        assert_eq!(tenant, Some("tenant-a".to_string()));
    }

    #[tokio::test]
    async fn subscription_deleted_downgrades_tenant_to_free_via_mapping() {
        let state = test_state_with_stripe("whsec_sub_test");
        // Seed: tenant-a is pro, and we know the customer mapping from the
        // original checkout (deleted events carry only the customer id).
        {
            let conn = state.db.try_lock().unwrap();
            let store = oz_core::Store::new(&conn);
            store
                .set_tenant_plan("tenant-a", oz_core::TenantPlan::Pro)
                .unwrap();
            store.set_stripe_customer("cus_123", "tenant-a").unwrap();
        }
        let (status, json) = post_stripe_subscription(
            &state,
            "customer.subscription.deleted",
            serde_json::json!({ "id": "sub_abc", "customer": "cus_123" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "accepted");
        assert_eq!(json["plan"], "free");
        assert_eq!(
            tenant_plan(&state, "tenant-a"),
            Some(oz_core::TenantPlan::Free),
            "cancelled subscription must downgrade the tenant's sync plan"
        );
    }

    #[tokio::test]
    async fn checkout_completed_upgrades_tenant_to_pro() {
        let state = test_state_with_stripe("whsec_sub_test");
        let (status, json) = post_stripe_subscription(
            &state,
            "checkout.session.completed",
            serde_json::json!({
                "id": "cs_abc",
                "customer": "cus_456",
                "subscription": "sub_abc",
                "metadata": { "tenant_id": "tenant-b" },
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["plan"], "pro");
        assert_eq!(
            tenant_plan(&state, "tenant-b"),
            Some(oz_core::TenantPlan::Pro)
        );
    }

    #[tokio::test]
    async fn invoice_paid_renews_pro_via_customer_mapping() {
        let state = test_state_with_stripe("whsec_sub_test");
        {
            let conn = state.db.try_lock().unwrap();
            oz_core::Store::new(&conn)
                .set_stripe_customer("cus_789", "tenant-c")
                .unwrap();
        }
        // invoice.paid carries only the customer id — resolve via mapping.
        let (status, json) = post_stripe_subscription(
            &state,
            "invoice.paid",
            serde_json::json!({ "id": "in_abc", "customer": "cus_789", "subscription": "sub_abc" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["plan"], "pro");
        assert_eq!(
            tenant_plan(&state, "tenant-c"),
            Some(oz_core::TenantPlan::Pro)
        );
    }

    #[tokio::test]
    async fn subscription_updated_canceled_downgrades_tenant() {
        let state = test_state_with_stripe("whsec_sub_test");
        {
            let conn = state.db.try_lock().unwrap();
            oz_core::Store::new(&conn)
                .set_tenant_plan("tenant-a", oz_core::TenantPlan::Pro)
                .unwrap();
        }
        let (status, _) = post_stripe_subscription(
            &state,
            "customer.subscription.updated",
            serde_json::json!({
                "id": "sub_abc",
                "customer": "cus_123",
                "status": "canceled",
                "metadata": { "tenant_id": "tenant-a" },
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            tenant_plan(&state, "tenant-a"),
            Some(oz_core::TenantPlan::Free),
            "a canceled subscription must downgrade the plan"
        );
    }

    #[tokio::test]
    async fn subscription_event_unknown_tenant_is_ignored_with_200() {
        let state = test_state_with_stripe("whsec_sub_test");
        let (status, json) = post_stripe_subscription(
            &state,
            "customer.subscription.created",
            serde_json::json!({
                "id": "sub_abc",
                "customer": "cus_unknown",
                "status": "active",
                "metadata": {},
            }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "unresolvable events must 200 so Stripe stops retrying"
        );
        assert_eq!(json["status"], "ignored");
    }

    #[tokio::test]
    async fn stripe_webhook_sale_not_found_returns_404() {
        let secret = "whsec_test_secret";

        let state = test_state_with_stripe(secret);
        let app = webhooks_router(state);

        let payload = br#"{"id":"evt_test_404","type":"payment_intent.succeeded","data":{"object":{"id":"pi_unknown","amount":1000}}}"#;
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

    /// Integration test against a live Postgres (the same Docker service
    /// `db.rs` uses, port 15432). Skips when unreachable, so the suite stays
    /// green on machines without a running Postgres.
    #[tokio::test]
    async fn pg_integration_webhooks_read_write_postgres() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
        let pool = match crate::db::DbPool::connect_postgres(&url, false, 20).await {
            Ok(crate::db::DbPool::Postgres(pool)) => pool,
            Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
            Err(e) => {
                eprintln!("PG webhooks integration test skipped: {e}");
                return;
            }
        };
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            pg: Some(pool.clone()),
            started_at: Instant::now(),
            stripe_webhook_secret: Some("whsec_test".into()),
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        let tenant = format!("pg-webhook-{}", uuid::Uuid::now_v7());
        let sale_id = format!("sale-{}", uuid::Uuid::now_v7());
        let gateway_ref = format!("pi_pg_{}", uuid::Uuid::now_v7());
        let event_id = format!("evt_pg_{}", uuid::Uuid::now_v7());

        // ── Dedup helpers ──
        assert!(!event_already_processed(&state, &event_id).await);
        assert!(!square_event_already_processed(&state, &event_id).await);
        record_event_processed(
            &state,
            &event_id,
            "stripe",
            Some("payment_intent.succeeded"),
        )
        .await;
        assert!(event_already_processed(&state, &event_id).await);
        assert!(square_event_already_processed(&state, &event_id).await);

        // ── Payment → sale lookup + finalize_sale enqueue ──
        {
            let client = pool.get().await.unwrap();
            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            client
                .execute(
                    "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at, tenant_id)
                     VALUES ($1, 700, 'USD', 1, 'pending', $2, $2, $3)",
                    &[&sale_id, &now, &tenant],
                )
                .await
                .unwrap();
            client
                .execute(
                    "INSERT INTO payments (id, sale_id, method, amount_minor, currency, created_at, gateway_reference)
                     VALUES ($1, $2, 'card', 700, 'USD', $3, $4)",
                    &[
                        &format!("pay-{tenant}"),
                        &sale_id,
                        &now,
                        &gateway_ref,
                    ],
                )
                .await
                .unwrap();
        }
        let (found, found_tenant) = lookup_sale_by_gateway_reference(&state, &gateway_ref)
            .await
            .expect("sale must resolve via gateway reference");
        assert_eq!(found, sale_id);
        assert_eq!(found_tenant, tenant, "lookup must return the sale's tenant");
        assert!(matches!(
            lookup_sale_by_gateway_reference(&state, "pi_missing").await,
            Err((StatusCode::NOT_FOUND, _))
        ));

        enqueue_finalize_sale(&state, &sale_id, &tenant)
            .await
            .expect("enqueue_finalize_sale");
        {
            let client = pool.get().await.unwrap();
            let row = client
                .query_opt(
                    "SELECT tenant_id FROM offline_queue WHERE action = 'finalize_sale' AND payload LIKE $1",
                    &[&format!("%{sale_id}%")],
                )
                .await
                .unwrap();
            let row = row.expect("one finalize_sale must be enqueued on Postgres");
            let queued_tenant: String = row.get(0);
            assert_eq!(
                queued_tenant, tenant,
                "finalize_sale must be enqueued under the sale's tenant"
            );
        }

        // ── Tenant resolution (metadata → mapping) ──
        let with_metadata = serde_json::json!({
            "id": "sub_pg",
            "customer": format!("cus_pg_{}", uuid::Uuid::now_v7()),
            "status": "active",
            "metadata": { "tenant_id": tenant },
        });
        let resolved = resolve_subscription_tenant(&state, &with_metadata)
            .await
            .expect("metadata resolve");
        assert_eq!(resolved.as_deref(), Some(tenant.as_str()));

        let customer_only = serde_json::json!({ "customer": with_metadata["customer"] });
        let resolved = resolve_subscription_tenant(&state, &customer_only)
            .await
            .expect("mapping resolve");
        assert_eq!(resolved.as_deref(), Some(tenant.as_str()));

        // ── Full subscription event → tenant plan upgraded ──
        let event = StripeEvent {
            id: format!("evt_upgrade_{}", uuid::Uuid::now_v7()),
            r#type: "customer.subscription.created".into(),
            data: StripeEventData {
                object: serde_json::json!({
                    "id": "sub_pg",
                    "customer": "cus_upgrade",
                    "status": "active",
                    "metadata": { "tenant_id": tenant },
                }),
            },
        };
        let resp = handle_subscription_event(&state, &event)
            .await
            .expect("handle_subscription_event");
        assert_eq!(resp.0["plan"], "pro");
        assert_eq!(
            oz_api::pg::get_tenant_plan(&pool, &tenant)
                .await
                .expect("get_tenant_plan"),
            Some(oz_core::TenantPlan::Pro)
        );

        // ── Clean up so a shared dev DB stays tidy ──
        {
            let client = pool.get().await.unwrap();
            client
                .execute(
                    "DELETE FROM offline_queue WHERE payload LIKE $1",
                    &[&format!("%{sale_id}%")],
                )
                .await
                .unwrap();
            client
                .execute("DELETE FROM payments WHERE sale_id = $1", &[&sale_id])
                .await
                .unwrap();
            client
                .execute("DELETE FROM sales WHERE id = $1", &[&sale_id])
                .await
                .unwrap();
            client
                .execute(
                    "DELETE FROM stripe_customers WHERE tenant_id = $1",
                    &[&tenant],
                )
                .await
                .unwrap();
            client
                .execute("DELETE FROM tenant_plans WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
            client
                .execute(
                    "DELETE FROM processed_webhooks WHERE event_id = $1",
                    &[&event_id],
                )
                .await
                .unwrap();
        }
    }
}
