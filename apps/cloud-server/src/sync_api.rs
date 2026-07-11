//! Sync API — server-side handlers for the offline-sync push/pull protocol.
//!
//! These endpoints mirror the client-side [`platform_sync::transport`] types:
//!
//! - `POST /api/sync/push` — receives items, persists with existing IDs, returns outcomes
//! - `POST /api/sync/pull` — receives a `PullRequest` with `since` timestamp, returns `PullResponse`
//! - `GET  /api/sync/status` — returns server status and pending queue count

use std::sync::Arc;

use axum::{
    Router,
    extract::{Extension, State},
    middleware,
    routing::{get, post},
};
use rusqlite::{Connection, params};
use tokio::sync::Mutex;

use oz_api::auth::{ApiTokenClaims, auth_middleware};
use platform_sync::transport::{PullRequest, PullResponse, PushOutcome, PushResponse};

/// Shared state for sync handlers — a database connection behind `Arc<Mutex<>>`.
#[derive(Clone)]
pub struct SyncState {
    pub db: Arc<Mutex<Connection>>,
}

impl From<super::CloudServerState> for SyncState {
    fn from(state: super::CloudServerState) -> Self {
        Self { db: state.db }
    }
}

/// Build the sync router with all three endpoints, protected by JWT auth.
pub fn sync_router(state: SyncState) -> Router {
    Router::new()
        .route("/api/sync/push", post(push_handler))
        .route("/api/sync/pull", post(pull_handler))
        .route("/api/sync/status", get(status_handler))
        .with_state(state)
        .layer(middleware::from_fn(auth_middleware))
}

/// `POST /api/sync/push` — receive and persist offline queue items.
///
/// Each item is inserted with its existing client-generated ID. Duplicate
/// IDs (UNIQUE constraint violation) are reported as `Rejected`.
async fn push_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
    axum::Json(items): axum::Json<Vec<oz_core::offline::OfflineQueueItem>>,
) -> Result<axum::Json<PushResponse>, (axum::http::StatusCode, String)> {
    use oz_core::offline::OfflineQueueStatus;

    // Tenant isolation: use the tenant_id from the JWT claims, not the
    // incoming JSON body, to prevent tenant spoofing.
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");

    let conn = state.db.lock().await;
    let mut results = Vec::with_capacity(items.len());

    for item in &items {
        match conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                item.id, item.action, item.payload,
                OfflineQueueStatus::Pending.as_stored_str(),
                item.retry_count, item.last_error, item.created_at, item.synced_at,
                tenant_id,
            ],
        ) {
            Ok(_) => results.push(PushOutcome::Accepted),
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    results.push(PushOutcome::Rejected {
                        reason: format!("duplicate id: {}", item.id),
                    });
                } else {
                    results.push(PushOutcome::Rejected {
                        reason: format!("database error: {e}"),
                    });
                }
            }
        }
    }

    Ok(axum::Json(PushResponse { results }))
}

/// `POST /api/sync/pull` — return items changed since the given timestamp.
///
/// Uses a SQL-level `WHERE created_at >= ?` filter for efficiency with
/// large datasets.
async fn pull_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
    axum::Json(req): axum::Json<PullRequest>,
) -> Result<axum::Json<PullResponse>, (axum::http::StatusCode, String)> {
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    let conn = state.db.lock().await;

    let items = if let Some(ref since) = req.since {
        let mut stmt = conn
            .prepare(
                "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id
                 FROM offline_queue WHERE created_at >= ?1 AND tenant_id = ?2 ORDER BY created_at ASC",
            )
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows = stmt
            .query_map(params![since, tenant_id], row_to_item)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id
                 FROM offline_queue WHERE tenant_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows = stmt
            .query_map(params![tenant_id], row_to_item)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    Ok(axum::Json(PullResponse { items }))
}

/// `GET /api/sync/status` — return server health, version, and pending queue depth.
async fn status_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
) -> axum::Json<SyncStatusResponse> {
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    let pending_count = {
        let conn = state.db.lock().await;
        conn.query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending' AND tenant_id = ?1",
            params![tenant_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
    };

    axum::Json(SyncStatusResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        pending_count,
    })
}

/// Response from the status endpoint.
#[derive(Debug, serde::Serialize)]
pub struct SyncStatusResponse {
    /// Server health status (e.g. `"ok"`).
    pub status: String,
    /// Server package version.
    pub version: String,
    /// Number of items in the queue with status `pending`.
    pub pending_count: i64,
}

/// Convert a SQLite row to an `OfflineQueueItem`.
fn row_to_item(row: &rusqlite::Row) -> rusqlite::Result<oz_core::offline::OfflineQueueItem> {
    let status_str: String = row.get("status")?;
    Ok(oz_core::offline::OfflineQueueItem {
        id: row.get("id")?,
        action: row.get("action")?,
        payload: row.get("payload")?,
        status: oz_core::offline::OfflineQueueStatus::from_stored_str(&status_str)
            .unwrap_or(oz_core::offline::OfflineQueueStatus::Pending),
        retry_count: row.get("retry_count")?,
        last_error: row.get("last_error")?,
        created_at: row.get("created_at")?,
        synced_at: row.get("synced_at")?,
        tenant_id: row.get("tenant_id")?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn fresh_db() -> Connection {
        oz_core::migrations::fresh_db()
    }

    /// Create a test JWT token scoped to the given tenant.
    fn test_token(tenant_id: Option<&str>) -> String {
        oz_api::auth::create_token("test", Some(24), tenant_id).token
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
        };
        sync_router(state)
    }

    fn test_router_with_state(state: SyncState) -> Router {
        sync_router(state)
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
        };
        let app = test_router_with_state(state.clone());

        let body = r#"[
            {"id":"a1","action":"create","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null},
            {"id":"a2","action":"update","payload":"{\"x\":1}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:01:00Z","synced_at":null}
        ]"#;
        let req = authed_post("/api/sync/push", body, None);
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
                "SELECT COUNT(*) FROM offline_queue WHERE id IN ('a1','a2')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn push_duplicate_id_returns_rejected() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = test_router_with_state(state.clone());

        // Insert first item directly (with explicit tenant_id)
        {
            let conn = state.db.lock().await;
            conn.execute(
                "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id)
                 VALUES ('dup', 'test', '{}', 'pending', '2026-01-01T00:00:00Z', 'default')",
                [],
            )
            .unwrap();
        }

        // Try to push a duplicate
        let body = r#"[{"id":"dup","action":"create","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}]"#;
        let req = authed_post("/api/sync/push", body, None);
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
    async fn pull_returns_items_for_tenant() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
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
}
