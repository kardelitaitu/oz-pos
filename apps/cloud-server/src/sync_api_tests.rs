use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::collections::HashMap;
use tower::ServiceExt;

fn fresh_db() -> Connection {
    oz_core::migrations::fresh_db()
}

/// Create a test JWT token scoped to the given tenant.
fn test_token(tenant_id: Option<&str>) -> String {
    oz_api::auth::create_token("test", Some(24), tenant_id, None)
        .unwrap()
        .token
}

/// Helper: build an authorized request builder with a Bearer token.
fn authed(method: axum::http::Method, uri: &str, tenant_id: Option<&str>) -> Request<Body> {
    let token = test_token(tenant_id);
    Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap()
}

/// Helper: build an authorized POST request with a JSON body.
fn authed_post(uri: &str, body: &str, tenant_id: Option<&str>) -> Request<Body> {
    let token = test_token(tenant_id);
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

fn test_router() -> Router {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    sync_router(state, false)
}

fn test_router_with_state(state: SyncState) -> Router {
    sync_router(state, false)
}

/// Build a router with plan enforcement explicitly enabled/disabled,
/// avoiding the `OZ_ENFORCE_PLANS` env var (ADR sync-plan-gating).
fn test_router_with_plan_enforcement(enforce: bool) -> Router {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    sync_router_with_plan_enforcement(state, enforce)
}

/// Seed a tenant plan in the test DB shared by the given router.
/// The router owns its DB, so this writes via the same migrations
/// connection the router was built from is not reachable; instead we
/// build the state first, seed it, then build the router around it.
/// Build a router whose test DB already has a plan row for `tenant`,
/// with enforcement explicitly enabled/disabled. Seeding happens before
/// the router is built so the handler sees the row.
async fn test_router_with_plan(tenant: &str, plan: &str, enforce: bool) -> Router {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    {
        let conn = state.db.lock().await;
        oz_core::Store::new(&conn)
            .set_tenant_plan(
                tenant,
                if plan == "pro" {
                    oz_core::TenantPlan::Pro
                } else {
                    oz_core::TenantPlan::Free
                },
            )
            .unwrap();
    }
    sync_router_with_plan_enforcement(state, enforce)
}

// ── Auth enforcement ─────────────────────────────────────────────

#[tokio::test]
async fn push_rejects_without_auth() {
    let app = test_router();
    let req = Request::builder()
        .method("POST")
        .uri("/api/sync/push")
        .header("Content-Type", "application/json")
        .body(Body::from("[]"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pull_rejects_without_auth() {
    let app = test_router();
    let req = Request::builder()
        .method("POST")
        .uri("/api/sync/pull")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"since":null}"#))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn status_rejects_without_auth() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/sync/status")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn snapshot_rejects_without_auth() {
    let app = test_router();
    let req = Request::builder()
        .uri("/api/sync/snapshot")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn snapshot_returns_data_with_auth() {
    let app = test_router();
    let req = authed(axum::http::Method::GET, "/api/sync/snapshot", None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(json["products"].is_array());
    assert!(json["tax_rates"].is_array());
    assert!(json["users"].is_array());
}

#[tokio::test]
async fn snapshot_omits_pin_hash_entirely() {
    // SYNC-06: the snapshot contract must never export credential
    // verifier material. Even with a user seeded that HAS a pin_hash,
    // the serialised snapshot bytes must not contain the field.
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT INTO roles (id, name, permissions) VALUES ('r-owner', 'Owner', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, tenant_id)
             VALUES ('user-secret', 'alice', 'SENSITIVE-HASH', 'Alice', 'r-owner', 'tenant-a')",
            [],
        )
        .unwrap();
    }

    let req = authed(
        axum::http::Method::GET,
        "/api/sync/snapshot",
        Some("tenant-a"),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    // The raw wire bytes must not contain the sensitive field name NOR
    // the seeded hash value anywhere (users array or otherwise).
    assert!(
        !body_str.contains("pin_hash"),
        "snapshot must not contain pin_hash: {body_str}"
    );
    assert!(
        !body_str.contains("SENSITIVE-HASH"),
        "snapshot must not leak the seeded hash: {body_str}"
    );

    // But the non-secret metadata must still be present.
    let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(json["users"].as_array().unwrap().len(), 1);
    assert_eq!(json["users"][0]["username"], "alice");
    assert_eq!(json["users"][0]["display_name"], "Alice");
}

#[tokio::test]
async fn snapshot_query_failure_returns_500_not_empty_success() {
    // SYNC-09: a server-side snapshot query failure must never be
    // mistaken for a valid empty snapshot. Drop the products table to
    // force a query error and assert the handler returns a non-2xx
    // status with an error envelope.
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    {
        let conn = state.db.lock().await;
        conn.execute("DROP TABLE products", []).unwrap();
    }

    let req = authed(axum::http::Method::GET, "/api/sync/snapshot", None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(
        json["error"].is_string(),
        "error envelope must carry a message: {json}"
    );
}

#[tokio::test]
async fn snapshot_serves_store_id_when_present() {
    // The snapshot contract carries the migration 069/117 soft-scoping
    // column: a product tagged with a store_id must round-trip through
    // the handler so the client can import it scoped instead of into
    // the global catalog.
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT INTO store_profiles (id, name) VALUES ('store-a', 'Store A')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, tenant_id, store_id)
             VALUES ('prod-a', 'SKU-A', 'Product A', 100, 'USD', 'tenant-a', 'store-a')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, tenant_id)
             VALUES ('prod-g', 'SKU-G', 'Global', 200, 'USD', 'tenant-a')",
            [],
        )
        .unwrap();
    }

    let req = authed(
        axum::http::Method::GET,
        "/api/sync/snapshot",
        Some("tenant-a"),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let products = json["products"].as_array().unwrap();
    let scoped = products
        .iter()
        .find(|p| p["sku"] == "SKU-A")
        .expect("scoped product must be present");
    assert_eq!(
        scoped["store_id"], "store-a",
        "store-tagged product must carry its store_id in the snapshot"
    );
    let global = products
        .iter()
        .find(|p| p["sku"] == "SKU-G")
        .expect("global product must be present");
    assert!(
        global["store_id"].is_null(),
        "global product must carry a null store_id"
    );
}

#[tokio::test]
async fn snapshot_cache_hit_returns_200() {
    // After a successful snapshot, a second request within the TTL
    // must still return 200 (the cache path must return Ok), serve
    // application/json, and round-trip as the same JSON body — proving
    // the raw-bytes cache-hit path (SOTA finding C) is wire-compatible
    // with the original axum::Json response.
    let app = test_router();
    let req1 = authed(axum::http::Method::GET, "/api/sync/snapshot", None);
    let resp1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);
    assert_eq!(
        resp1
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/json"
    );
    let body1 = resp1.into_body().collect().await.unwrap().to_bytes();
    let json1: serde_json::Value = serde_json::from_slice(&body1).unwrap();

    // Second request hits the cache: same status, same content-type,
    // same JSON.
    let req2 = authed(axum::http::Method::GET, "/api/sync/snapshot", None);
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    assert_eq!(
        resp2
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/json"
    );
    let body2 = resp2.into_body().collect().await.unwrap().to_bytes();
    let json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    assert_eq!(json1, json2, "cache hit must serve the identical JSON");
}

#[tokio::test]
async fn pull_returns_500_on_malformed_row() {
    // SYNC-10: a row that fails to decode must fail the whole pull
    // (5xx) rather than being silently dropped. Seed a row whose
    // retry_count is non-numeric (SQLite stores it as TEXT despite the
    // INTEGER affinity) so the SQLite row decoder's `get::<_, i64>` fails.
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    {
        let conn = state.db.lock().await;
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, created_at, tenant_id) VALUES
             ('good', 'act', '{}', 'pending', 0, '2026-01-01T00:00:00Z', 'tenant-a'),
             ('bad', 'act', '{}', 'pending', 'not-a-number', '2026-01-02T00:00:00Z', 'tenant-a')",
        )
        .unwrap();
    }

    let req = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-a"));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "malformed row must fail the pull, not be silently dropped"
    );
}

#[tokio::test]
async fn snapshot_tenant_isolation() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    // Seed a product for tenant-a only.
    {
        let conn = state.db.lock().await;
        // Seed a role so the FK on users is satisfied.
        conn.execute(
            "INSERT INTO roles (id, name, permissions) VALUES ('r-owner', 'Owner', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, tenant_id)
             VALUES ('prod-a', 'SKU-A', 'Product A', 100, 'USD', 'tenant-a')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tax_rates (id, name, rate_bps, tenant_id)
             VALUES ('tax-a', 'Tax A', 800, 'tenant-a')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, tenant_id)
             VALUES ('user-a', 'alice', 'hash', 'Alice', 'r-owner', 'tenant-a')",
            [],
        )
        .unwrap();
    }

    // Tenant B's snapshot should be empty (no data seeded for tenant-b).
    let req_b = authed(
        axum::http::Method::GET,
        "/api/sync/snapshot",
        Some("tenant-b"),
    );
    let resp_b = app.clone().oneshot(req_b).await.unwrap();
    assert_eq!(resp_b.status(), StatusCode::OK);
    let body_b = resp_b.into_body().collect().await.unwrap().to_bytes();
    let json_b: serde_json::Value = serde_json::from_slice(&body_b).unwrap();
    assert_eq!(
        json_b["products"].as_array().unwrap().len(),
        0,
        "tenant-b should see no products"
    );
    assert_eq!(
        json_b["tax_rates"].as_array().unwrap().len(),
        0,
        "tenant-b should see no tax rates"
    );
    assert_eq!(
        json_b["users"].as_array().unwrap().len(),
        0,
        "tenant-b should see no users"
    );

    // Tenant A's snapshot should contain the seeded data.
    let req_a = authed(
        axum::http::Method::GET,
        "/api/sync/snapshot",
        Some("tenant-a"),
    );
    let resp_a = app.oneshot(req_a).await.unwrap();
    assert_eq!(resp_a.status(), StatusCode::OK);
    let body_a = resp_a.into_body().collect().await.unwrap().to_bytes();
    let json_a: serde_json::Value = serde_json::from_slice(&body_a).unwrap();
    assert_eq!(
        json_a["products"].as_array().unwrap().len(),
        1,
        "tenant-a should see 1 product"
    );
    assert_eq!(json_a["products"][0]["sku"], "SKU-A");
    assert_eq!(
        json_a["tax_rates"].as_array().unwrap().len(),
        1,
        "tenant-a should see 1 tax rate"
    );
    assert_eq!(
        json_a["users"].as_array().unwrap().len(),
        1,
        "tenant-a should see 1 user"
    );
}

// ── Basic push/pull with auth ────────────────────────────────────

#[tokio::test]
async fn push_empty_array_returns_ok() {
    let app = test_router();
    let req = authed_post("/api/sync/push", "[]", None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn push_inserts_items_with_existing_ids() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    let id1 = uuid::Uuid::now_v7().to_string();
    let id2 = uuid::Uuid::now_v7().to_string();
    let body = format!(
        r#"[
            {{"id":"{id1}","action":"create","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}},
            {{"id":"{id2}","action":"update","payload":"{{\"x\":1}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:01:00Z","synced_at":null}}
        ]"#
    );
    let req = authed_post("/api/sync/push", &body, None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let push_resp: PushResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(push_resp.results.len(), 2);
    assert!(matches!(push_resp.results[0], PushOutcome::Accepted));
    assert!(matches!(push_resp.results[1], PushOutcome::Accepted));

    // Verify both persisted
    let conn = state.db.lock().await;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE id IN (?1, ?2)",
            rusqlite::params![id1, id2],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn push_duplicate_id_returns_rejected() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    let dup_id = uuid::Uuid::now_v7().to_string();

    // Insert first item directly (with explicit tenant_id)
    {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id)
             VALUES (?1, 'test', '{}', 'pending', '2026-01-01T00:00:00Z', 'default')",
            [&dup_id],
        )
        .unwrap();
    }

    // Try to push a duplicate
    let body = format!(
        r#"[{{"id":"{dup_id}","action":"create","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}}]"#
    );
    let req = authed_post("/api/sync/push", &body, None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let push_resp: PushResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(push_resp.results.len(), 1);
    match &push_resp.results[0] {
        PushOutcome::Rejected { reason } => {
            assert!(reason.contains("duplicate"), "got: {reason}");
        }
        other => panic!("expected Rejected, got: {other:?}"),
    }
}

#[tokio::test]
async fn push_rejects_invalid_non_uuid_id() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    // A hostile id (the round-119 injection string) must be rejected at
    // push, never persisted — defense-in-depth so only UUID ids ever
    // reach the prune DELETE path. A well-formed UUIDv7 in the same
    // batch must still be accepted (valid clients are not blocked).
    let hostile = "x'); CREATE TABLE hacked(id TEXT);--";
    let valid = uuid::Uuid::now_v7().to_string();
    let body = format!(
        r#"[
            {{"id":"{hostile}","action":"create","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}},
            {{"id":"{valid}","action":"create","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}}
        ]"#
    );
    let req = authed_post("/api/sync/push", &body, None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let push_resp: PushResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(push_resp.results.len(), 2);
    match &push_resp.results[0] {
        PushOutcome::Rejected { reason } => {
            assert!(
                reason.contains("invalid id"),
                "expected invalid-id rejection, got: {reason}"
            );
        }
        other => panic!("expected Rejected for hostile id, got: {other:?}"),
    }
    assert!(
        matches!(push_resp.results[1], PushOutcome::Accepted),
        "valid UUID must be accepted: {:?}",
        push_resp.results[1]
    );

    // The hostile id must never be persisted; the valid one must be.
    let conn = state.db.lock().await;
    let hostile_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE id = ?1",
            [hostile],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hostile_count, 0, "hostile id was persisted!");
    let valid_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE id = ?1",
            [&valid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(valid_count, 1, "valid UUID should be persisted");
    // The injected CREATE TABLE must never have executed.
    let hacked: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'hacked'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hacked, 0, "injected CREATE TABLE executed!");
}

/// The two-phase handler logic (UUID validation → single batch INSERT)
/// must report the right per-item outcome for a MIXED batch: an invalid
/// id, a fresh item, and a duplicate of an existing row. The duplicate
/// must not abort the valid insert — the batch survives it.
#[tokio::test]
async fn handler_mixed_batch_invalid_uuid_valid_duplicate() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    // Seed an existing row that the batch will duplicate.
    let dup_id = uuid::Uuid::now_v7().to_string();
    {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id)
             VALUES (?1, 'test', '{}', 'pending', '2026-01-01T00:00:00Z', 'default')",
            [&dup_id],
        )
        .unwrap();
    }

    let hostile = "not-a-uuid";
    let fresh = uuid::Uuid::now_v7().to_string();
    let body = format!(
        r#"[
            {{"id":"{hostile}","action":"create","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}},
            {{"id":"{fresh}","action":"create","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}},
            {{"id":"{dup_id}","action":"create","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}}
        ]"#
    );
    let req = authed_post("/api/sync/push", &body, None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let push_resp: PushResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(push_resp.results.len(), 3);
    match &push_resp.results[0] {
        PushOutcome::Rejected { reason } => {
            assert!(reason.contains("invalid id"), "got: {reason}");
        }
        other => panic!("expected invalid-id Rejected, got: {other:?}"),
    }
    assert!(
        matches!(push_resp.results[1], PushOutcome::Accepted),
        "fresh item must be Accepted: {:?}",
        push_resp.results[1]
    );
    match &push_resp.results[2] {
        PushOutcome::Rejected { reason } => {
            assert!(
                reason.contains("duplicate id"),
                "duplicate must be Rejected, got: {reason}"
            );
        }
        other => panic!("expected duplicate Rejected, got: {other:?}"),
    }

    // Exactly the fresh item was persisted; the duplicate and hostile id
    // were not double-inserted.
    let conn = state.db.lock().await;
    let fresh_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE id = ?1",
            [&fresh],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(fresh_count, 1, "fresh item must be persisted once");
    let dup_total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE id = ?1",
            [&dup_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(dup_total, 1, "duplicate id must not be double-inserted");
}

/// With `OZ_SKIP_PUSH_VALIDATION=1` (trusted server-to-server), a
/// non-UUID id must pass through the handler and be inserted — the
/// validation gate is the only thing stopping it.
#[tokio::test]
async fn push_batch_skip_validation_flag_accepts_non_uuid() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: true,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    let non_uuid = "server-generated-key-123";
    let body = format!(
        r#"[{{"id":"{non_uuid}","action":"create","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}}]"#
    );
    let req = authed_post("/api/sync/push", &body, None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let push_resp: PushResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(push_resp.results.len(), 1);
    assert!(
        matches!(push_resp.results[0], PushOutcome::Accepted),
        "skip_push_validation must accept the non-UUID id: {:?}",
        push_resp.results[0]
    );

    let conn = state.db.lock().await;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE id = ?1",
            [non_uuid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 1,
        "non-UUID id must be persisted when validation is skipped"
    );
}

#[tokio::test]
async fn pull_returns_items_for_tenant() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    // Seed items for both tenants
    {
        let conn = state.db.lock().await;
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id) VALUES
             ('t1-a', 'act', '{}', 'pending', '2026-01-02T00:00:00Z', 'tenant-a'),
             ('t1-b', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'tenant-a'),
             ('t2-a', 'act', '{}', 'pending', '2026-01-03T00:00:00Z', 'tenant-b')",
        )
        .unwrap();
    }

    // Pull as tenant-a — should only see tenant-a's items
    let req = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-a"));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let pull_resp: PullResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(pull_resp.items.len(), 2);
    assert_eq!(pull_resp.items[0].id, "t1-b");
    assert_eq!(pull_resp.items[1].id, "t1-a");
}

#[tokio::test]
async fn pull_tenant_isolation() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    // Seed items for both tenants
    {
        let conn = state.db.lock().await;
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id) VALUES
             ('a-only', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'tenant-a'),
             ('b-only', 'act', '{}', 'pending', '2026-01-02T00:00:00Z', 'tenant-b')",
        )
        .unwrap();
    }

    // Tenant B should NOT see tenant A's item
    let req = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-b"));
    let resp = app.clone().oneshot(req).await.unwrap();
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let pull_resp: PullResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(pull_resp.items.len(), 1);
    assert_eq!(pull_resp.items[0].id, "b-only");

    // Tenant A should NOT see tenant B's item
    let req_a = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-a"));
    let resp_a = app.oneshot(req_a).await.unwrap();
    let body_a = resp_a.into_body().collect().await.unwrap().to_bytes();
    let pull_a: PullResponse = serde_json::from_slice(&body_a).unwrap();
    assert_eq!(pull_a.items.len(), 1);
    assert_eq!(pull_a.items[0].id, "a-only");
}

#[tokio::test]
async fn pull_filters_by_since_and_tenant() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    {
        let conn = state.db.lock().await;
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id) VALUES
             ('old', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'default'),
             ('mid', 'act', '{}', 'pending', '2026-01-15T00:00:00Z', 'default'),
             ('new', 'act', '{}', 'pending', '2026-02-01T00:00:00Z', 'default'),
             ('other', 'act', '{}', 'pending', '2026-01-15T00:00:00Z', 'other-tenant')",
        )
        .unwrap();
    }

    // Should return mid and new for default tenant
    let req = authed_post(
        "/api/sync/pull",
        r#"{"since":"2026-01-15T00:00:00Z"}"#,
        None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let pull_resp: PullResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(pull_resp.items.len(), 2);
    assert_eq!(pull_resp.items[0].id, "mid");
    assert_eq!(pull_resp.items[1].id, "new");
}

// ── Status endpoint ─────────────────────────────────────────────

#[tokio::test]
async fn status_returns_ok() {
    let app = test_router();
    let req = authed(axum::http::Method::GET, "/api/sync/status", None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn status_returns_json() {
    let app = test_router();
    let req = authed(axum::http::Method::GET, "/api/sync/status", None);
    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
    assert_eq!(json["pending_count"], 0);
}

#[tokio::test]
async fn status_counts_only_current_tenant() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    {
        let conn = state.db.lock().await;
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id) VALUES
             ('a1', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'tenant-a'),
             ('a2', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'tenant-a'),
             ('b1', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'tenant-b')",
        )
        .unwrap();
    }

    // Tenant A sees 2 pending
    let req_a = authed(
        axum::http::Method::GET,
        "/api/sync/status",
        Some("tenant-a"),
    );
    let resp_a = app.clone().oneshot(req_a).await.unwrap();
    let body_a = resp_a.into_body().collect().await.unwrap().to_bytes();
    let json_a: serde_json::Value = serde_json::from_slice(&body_a).unwrap();
    assert_eq!(json_a["pending_count"], 2);

    // Tenant B sees 1 pending
    let req_b = authed(
        axum::http::Method::GET,
        "/api/sync/status",
        Some("tenant-b"),
    );
    let resp_b = app.clone().oneshot(req_b).await.unwrap();
    let body_b = resp_b.into_body().collect().await.unwrap().to_bytes();
    let json_b: serde_json::Value = serde_json::from_slice(&body_b).unwrap();
    assert_eq!(json_b["pending_count"], 1);
}

#[tokio::test]
async fn status_counts_zero_for_empty_tenant() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    {
        let conn = state.db.lock().await;
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id) VALUES
             ('x', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'tenant-a')",
        )
        .unwrap();
    }

    // Tenant C (no items) sees 0
    let req = authed(
        axum::http::Method::GET,
        "/api/sync/status",
        Some("tenant-c"),
    );
    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["pending_count"], 0);
}

/// The tenant-count cache must serve the TTL'd value instead of re-scanning
/// `offline_queue` on every status poll. First read warms the cache; rows
/// added for new tenants afterwards are NOT visible until the cache
/// expires (the value only feeds tiered-heartbeat sizing).
#[tokio::test]
async fn tenant_count_cache_serves_stale_value_within_ttl() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };

    {
        let conn = state.db.lock().await;
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id) VALUES
             ('t1', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'tenant-a')",
        )
        .unwrap();
    }

    // First read warms the cache: 1 distinct tenant.
    assert_eq!(state.cached_tenant_count().await, 1);

    // A second tenant appears in the DB, but within the 60s TTL the cache
    // must serve the previous count (no rescan of offline_queue).
    {
        let conn = state.db.lock().await;
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id) VALUES
             ('t2', 'act', '{}', 'pending', '2026-01-01T00:00:00Z', 'tenant-b')",
        )
        .unwrap();
    }

    assert_eq!(
        state.cached_tenant_count().await,
        1,
        "cache must serve the stale count within TTL"
    );
}

// ── Plan enforcement (ADR sync-plan-gating) ─────────────────────

#[tokio::test]
async fn free_tenant_push_rejected_when_enforced() {
    let app = test_router_with_plan_enforcement(true);
    // No plan row → fail closed to free.
    let req = authed_post("/api/sync/push", r#"[]"#, Some("tenant-free-no-row"));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        json["error"], "plan_required",
        "a free tenant must get a structured plan_required rejection"
    );
}

#[tokio::test]
async fn explicitly_free_tenant_push_rejected_when_enforced() {
    let app = test_router_with_plan("tenant-free", "free", true).await;
    let req = authed_post("/api/sync/push", r#"[]"#, Some("tenant-free"));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"], "plan_required");
}

#[tokio::test]
async fn pro_tenant_push_accepted_when_enforced() {
    let app = test_router_with_plan("tenant-pro", "pro", true).await;
    let id = uuid::Uuid::now_v7().to_string();
    let body = format!(
        r#"[{{"id":"{id}","action":"complete_sale","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}}]"#
    );
    let req = authed_post("/api/sync/push", &body, Some("tenant-pro"));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "a pro tenant must be able to push when enforcement is on"
    );
}

#[tokio::test]
async fn free_tenant_push_allowed_when_not_enforced() {
    // Dev mode: OZ_ENFORCE_PLANS unset — everything works as before.
    let app = test_router_with_plan("tenant-free", "free", false).await;
    let id = uuid::Uuid::now_v7().to_string();
    let body = format!(
        r#"[{{"id":"{id}","action":"complete_sale","payload":"{{}}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}}]"#
    );
    let req = authed_post("/api/sync/push", &body, Some("tenant-free"));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "dev mode must not gate free tenants"
    );
}

#[tokio::test]
async fn status_endpoint_also_gated_by_plan() {
    let app = test_router_with_plan_enforcement(true);
    let req = authed(
        axum::http::Method::GET,
        "/api/sync/status",
        Some("tenant-gated"),
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"], "plan_required");
}

// ── Transport type compatibility ─────────────────────────────────

#[tokio::test]
async fn push_response_uses_transport_types() {
    let app = test_router();
    let req = authed_post("/api/sync/push", "[]", None);
    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let push_resp: PushResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert!(push_resp.results.is_empty());
}

#[tokio::test]
async fn pull_response_uses_transport_types() {
    let app = test_router();
    let req = authed_post("/api/sync/pull", r#"{"since":null}"#, None);
    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let pull_resp: PullResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert!(pull_resp.items.is_empty());
}

// ── Validation ──────────────────────────────────────────────────

#[tokio::test]
async fn push_rejects_malformed_json() {
    let app = test_router();
    let token = test_token(None);
    let req = Request::builder()
        .method("POST")
        .uri("/api/sync/push")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from("not json"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── Anchor expiry (P-1 retention) ────────────────────────────

#[tokio::test]
async fn pull_returns_410_when_anchor_expired() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    // Seed an item with a known timestamp.
    {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id)
             VALUES ('a1', 'act', '{}', 'pending', '2026-04-15T00:00:00Z', 'default')",
            [],
        )
        .unwrap();
    }

    // Pull with a `since` timestamp older than the oldest row.
    // The anchor (2025-01-01) is before the oldest row (2026-04-15),
    // so the server should return 410 Gone.
    let req = authed_post(
        "/api/sync/pull",
        r#"{"since":"2025-01-01T00:00:00Z"}"#,
        None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::GONE);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["error"], "anchor_expired");
    assert_eq!(json["oldest_available"], "2026-04-15T00:00:00Z");
}

#[tokio::test]
async fn pull_succeeds_when_anchor_is_fresh() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state.clone());

    {
        let conn = state.db.lock().await;
        conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id)
             VALUES ('a1', 'act', '{}', 'pending', '2026-04-15T00:00:00Z', 'default')",
            [],
        )
        .unwrap();
    }

    // Pull with a `since` timestamp newer than the oldest row.
    // The anchor (2026-05-01) is after the oldest row, so normal
    // response is expected.
    let req = authed_post(
        "/api/sync/pull",
        r#"{"since":"2026-05-01T00:00:00Z"}"#,
        None,
    );
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let pull_resp: PullResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert!(pull_resp.items.is_empty()); // since is after the only row
}

#[tokio::test]
async fn pull_null_since_never_expired() {
    let state = SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    };
    let app = test_router_with_state(state);

    // Initial sync (since = null) should always succeed regardless
    // of what's in the DB.
    let req = authed_post("/api/sync/pull", r#"{"since":null}"#, None);
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn pull_rejects_malformed_json() {
    let app = test_router();
    let token = test_token(None);
    let req = Request::builder()
        .method("POST")
        .uri("/api/sync/pull")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from("not json"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── P152: Rate Limiting Integration Tests ────────────────────────

/// Helper: send `count` authed POST requests and return all status codes.
async fn send_n_push_requests(app: &Router, tenant: Option<&str>, n: usize) -> Vec<StatusCode> {
    let mut codes = Vec::with_capacity(n);
    let token = test_token(tenant);
    for _ in 0..n {
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/push")
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .body(Body::from("[]"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        codes.push(resp.status());
    }
    codes
}

/// Helper: create a SyncState with a shared RateLimiterState for
/// cross-request rate limit testing.
fn shared_state() -> SyncState {
    SyncState {
        db: Arc::new(Mutex::new(fresh_db())),
        snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: RateLimiterState::new(),
        pg: None,
        skip_push_validation: false,
        tenant_count_cache: TenantCountCache::default(),
    }
}

#[tokio::test]
async fn rate_limit_429_when_push_limit_exceeded() {
    let state = shared_state();
    let app = test_router_with_state(state);

    // First 100 requests should be OK (push limit = 100/min).
    let codes = send_n_push_requests(&app, Some("tenant-rl"), 101).await;

    assert_eq!(codes[0..100].iter().filter(|c| c.is_success()).count(), 100);
    assert_eq!(codes[100], StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn rate_limit_429_includes_retry_after_header() {
    let state = shared_state();
    let app = test_router_with_state(state);

    // Exhaust the push limit.
    let _ = send_n_push_requests(&app, Some("tenant-hdr"), 100).await;

    // 101st request should return 429 with Retry-After header.
    let token = test_token(Some("tenant-hdr"));
    let req = Request::builder()
        .method("POST")
        .uri("/api/sync/push")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from("[]"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    let retry_after = resp.headers().get("Retry-After");
    assert!(retry_after.is_some(), "429 must include Retry-After header");
    let secs: u64 = retry_after.unwrap().to_str().unwrap().parse().unwrap();
    assert!(secs > 0, "Retry-After must be positive: {secs}");

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["error"], "rate_limit_exceeded");
    assert!(json["retry_after_seconds"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn rate_limit_tenant_isolation() {
    let state = shared_state();
    let app = test_router_with_state(state);

    // Exhaust tenant A's push limit.
    let _ = send_n_push_requests(&app, Some("tenant-a"), 100).await;

    // Tenant A's 101st request should be 429.
    let token_a = test_token(Some("tenant-a"));
    let req_a = Request::builder()
        .method("POST")
        .uri("/api/sync/push")
        .header("Authorization", format!("Bearer {token_a}"))
        .header("Content-Type", "application/json")
        .body(Body::from("[]"))
        .unwrap();
    let resp_a = app.clone().oneshot(req_a).await.unwrap();
    assert_eq!(resp_a.status(), StatusCode::TOO_MANY_REQUESTS);

    // Tenant B should still succeed (independent bucket).
    let token_b = test_token(Some("tenant-b"));
    let req_b = Request::builder()
        .method("POST")
        .uri("/api/sync/push")
        .header("Authorization", format!("Bearer {token_b}"))
        .header("Content-Type", "application/json")
        .body(Body::from("[]"))
        .unwrap();
    let resp_b = app.oneshot(req_b).await.unwrap();
    assert_eq!(
        resp_b.status(),
        StatusCode::OK,
        "tenant B should not be rate-limited when tenant A is exhausted"
    );
}

#[tokio::test]
async fn rate_limit_endpoint_isolation() {
    let state = shared_state();
    let app = test_router_with_state(state);

    // Exhaust push endpoint for a tenant.
    let _ = send_n_push_requests(&app, Some("tenant-ep"), 100).await;

    // Push should now be 429.
    let token = test_token(Some("tenant-ep"));
    let req_push = Request::builder()
        .method("POST")
        .uri("/api/sync/push")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from("[]"))
        .unwrap();
    let resp_push = app.clone().oneshot(req_push).await.unwrap();
    assert_eq!(resp_push.status(), StatusCode::TOO_MANY_REQUESTS);

    // Pull should still work (separate 300/min limit).
    let req_pull = Request::builder()
        .method("POST")
        .uri("/api/sync/pull")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"since":null}"#))
        .unwrap();
    let resp_pull = app.oneshot(req_pull).await.unwrap();
    assert_eq!(
        resp_pull.status(),
        StatusCode::OK,
        "pull should not be rate-limited when push is exhausted"
    );
}

#[tokio::test]
async fn rate_limit_burst_allowance() {
    let state = shared_state();
    let app = test_router_with_state(state);

    // Burst of 50 requests should all succeed (push capacity = 100).
    let codes = send_n_push_requests(&app, Some("tenant-burst"), 50).await;
    assert!(
        codes.iter().all(|c| c.is_success()),
        "first 50 requests should all succeed within burst allowance"
    );

    // Next 50 should also succeed (we're still within 100 burst).
    let codes2 = send_n_push_requests(&app, Some("tenant-burst"), 50).await;
    assert!(codes2.iter().all(|c| c.is_success()));

    // 101st should be 429.
    let codes3 = send_n_push_requests(&app, Some("tenant-burst"), 1).await;
    assert_eq!(codes3[0], StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn rate_limit_status_endpoint_within_burst_limit() {
    let state = shared_state();
    let app = test_router_with_state(state);

    // /api/sync/status has a 300/min limit. 50 rapid GETs should all pass.
    // Health endpoints (/health, /api/health) are in the main router
    // (build_router in main.rs) which does NOT apply rate_limit_middleware —
    // they are exempt by architecture, so no 429 test is needed for them.
    let token = test_token(Some("tenant-status"));
    for _ in 0..50 {
        let req = Request::builder()
            .method("GET")
            .uri("/api/sync/status")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert!(
            resp.status().is_success(),
            "status endpoint should not be rate-limited at 50 requests (300/min limit)"
        );
    }
}
