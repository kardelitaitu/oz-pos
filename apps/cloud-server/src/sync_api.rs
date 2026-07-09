//! Sync API — server-side handlers for the offline-sync push/pull protocol.
//!
//! These endpoints mirror the client-side [`platform_sync::transport`] types:
//!
//! - `POST /api/sync/push` — receives items, persists with existing IDs, returns outcomes
//! - `POST /api/sync/pull` — receives a `PullRequest` with `since` timestamp, returns `PullResponse`
//! - `GET  /api/sync/status` — returns server status and pending queue count

use std::sync::Arc;

use axum::{Router, extract::State, routing::{get, post}};
use rusqlite::{Connection, params};
use tokio::sync::Mutex;

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

/// Build the sync router with all three endpoints.
pub fn sync_router(state: SyncState) -> Router {
    Router::new()
        .route("/api/sync/push", post(push_handler))
        .route("/api/sync/pull", post(pull_handler))
        .route("/api/sync/status", get(status_handler))
        .with_state(state)
}

/// `POST /api/sync/push` — receive and persist offline queue items.
///
/// Each item is inserted with its existing client-generated ID. Duplicate
/// IDs (UNIQUE constraint violation) are reported as `Rejected`.
async fn push_handler(
    State(state): State<SyncState>,
    axum::Json(items): axum::Json<Vec<oz_core::offline::OfflineQueueItem>>,
) -> Result<axum::Json<PushResponse>, (axum::http::StatusCode, String)> {
    use oz_core::offline::OfflineQueueStatus;

    let conn = state.db.lock().await;
    let mut results = Vec::with_capacity(items.len());

    for item in &items {
        match conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                item.id, item.action, item.payload,
                OfflineQueueStatus::Pending.as_stored_str(),
                item.retry_count, item.last_error, item.created_at, item.synced_at,
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
    axum::Json(req): axum::Json<PullRequest>,
) -> Result<axum::Json<PullResponse>, (axum::http::StatusCode, String)> {
    let conn = state.db.lock().await;

    let items = if let Some(ref since) = req.since {
        let mut stmt = conn
            .prepare(
                "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at
                 FROM offline_queue WHERE created_at >= ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows = stmt
            .query_map(params![since], row_to_item)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at
                 FROM offline_queue ORDER BY created_at ASC",
            )
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_item)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    Ok(axum::Json(PullResponse { items }))
}

/// `GET /api/sync/status` — return server health, version, and pending queue depth.
async fn status_handler(
    State(state): State<SyncState>,
) -> axum::Json<SyncStatusResponse> {
    let pending_count = {
        let conn = state.db.lock().await;
        conn.query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending'",
            [],
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
    pub status: String,
    pub version: String,
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

    fn test_router() -> Router {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        sync_router(state)
    }

    #[tokio::test]
    async fn push_empty_array_returns_ok() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/push")
            .header("Content-Type", "application/json")
            .body(Body::from("[]"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn push_inserts_items_with_existing_ids() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = sync_router(state.clone());

        let body = r#"[
            {"id":"a1","action":"create","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null},
            {"id":"a2","action":"update","payload":"{\"x\":1}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:01:00Z","synced_at":null}
        ]"#;
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/push")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
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
            .query_row("SELECT COUNT(*) FROM offline_queue WHERE id IN ('a1','a2')", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn push_duplicate_id_returns_rejected() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = sync_router(state.clone());

        // Insert first item directly
        {
            let conn = state.db.lock().await;
            conn.execute(
                "INSERT INTO offline_queue (id, action, payload, status, created_at)
                 VALUES ('dup', 'test', '{}', 'pending', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        // Try to push a duplicate
        let body = r#"[{"id":"dup","action":"create","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}]"#;
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/push")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
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
    async fn pull_returns_all_items_when_no_since() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = sync_router(state.clone());

        // Seed items
        {
            let conn = state.db.lock().await;
            conn.execute_batch(
                "INSERT INTO offline_queue (id, action, payload, status, created_at) VALUES
                 ('b', 'act', '{}', 'pending', '2026-01-02T00:00:00Z'),
                 ('a', 'act', '{}', 'pending', '2026-01-01T00:00:00Z')",
            )
            .unwrap();
        }

        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/pull")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"since":null}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let pull_resp: PullResponse = serde_json::from_slice(&body_bytes).unwrap();
        // Should be ordered by created_at ASC
        assert_eq!(pull_resp.items.len(), 2);
        assert_eq!(pull_resp.items[0].id, "a");
        assert_eq!(pull_resp.items[1].id, "b");
    }

    #[tokio::test]
    async fn pull_filters_by_since_timestamp() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = sync_router(state.clone());

        {
            let conn = state.db.lock().await;
            conn.execute_batch(
                "INSERT INTO offline_queue (id, action, payload, status, created_at) VALUES
                 ('old', 'act', '{}', 'pending', '2026-01-01T00:00:00Z'),
                 ('mid', 'act', '{}', 'pending', '2026-01-15T00:00:00Z'),
                 ('new', 'act', '{}', 'pending', '2026-02-01T00:00:00Z')",
            )
            .unwrap();
        }

        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/pull")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"since":"2026-01-15T00:00:00Z"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let pull_resp: PullResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(pull_resp.items.len(), 2);
        assert_eq!(pull_resp.items[0].id, "mid");
        assert_eq!(pull_resp.items[1].id, "new");
    }

    #[tokio::test]
    async fn status_returns_ok() {
        let app = test_router();
        let req = Request::builder()
            .uri("/api/sync/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn status_returns_json() {
        let app = test_router();
        let req = Request::builder()
            .uri("/api/sync/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["version"].is_string());
        assert_eq!(json["pending_count"], 0);
    }

    #[tokio::test]
    async fn status_reflects_pending_count() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = sync_router(state.clone());

        {
            let conn = state.db.lock().await;
            conn.execute_batch(
                "INSERT INTO offline_queue (id, action, payload, status, created_at) VALUES
                 ('x', 'a', '{}', 'pending', '2026-01-01T00:00:00Z'),
                 ('y', 'a', '{}', 'pending', '2026-01-01T00:00:00Z')",
            )
            .unwrap();
        }

        let req = Request::builder()
            .uri("/api/sync/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["pending_count"], 2);
    }

    #[tokio::test]
    async fn push_response_uses_transport_types() {
        // Verify that PushResponse from the handler matches the transport type
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/push")
            .header("Content-Type", "application/json")
            .body(Body::from("[]"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        // Deserialize as the transport type
        let push_resp: PushResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert!(push_resp.results.is_empty());
    }

    #[tokio::test]
    async fn pull_response_uses_transport_types() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/pull")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"since":null}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let pull_resp: PullResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert!(pull_resp.items.is_empty());
    }

    #[tokio::test]
    async fn push_rejects_malformed_json() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/push")
            .header("Content-Type", "application/json")
            .body(Body::from("not json"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn pull_rejects_malformed_json() {
        let app = test_router();
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/pull")
            .header("Content-Type", "application/json")
            .body(Body::from("not json"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
