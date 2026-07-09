//! OZ-POS Cloud Sync Server — headless binary (no Tauri, no WebView).
//!
//! Serves both the REST API (`oz-api` routes) and sync-push/pull endpoints
//! on the same HTTP port. Run in production behind a reverse proxy.
//!
//! # Usage
//!
//! ```bash
//! OZ_DB_PATH=/data/oz-pos.db OZ_API_PORT=3099 oz-cloud-server
//! ```
//!
//! # Environment variables
//!
//! | Variable | Default | Description |
//! |---|---|---|
//! | `OZ_DB_PATH` | `oz-pos.db` | Path to the SQLite database file |
//! | `OZ_API_PORT` | `3099` | HTTP server listen port |
//! | `RUST_LOG` | `info` | Log level filter (e.g. `debug`, `oz_cloud_server=debug`) |

use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    routing::{get, post},
};
use rusqlite::{Connection, params};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Shared application state for the cloud server.
///
/// Provides the database connection and any additional server-wide state.
#[derive(Clone)]
pub struct CloudServerState {
    /// Database connection wrapped for axum's `State` extractor.
    pub db: Arc<Mutex<Connection>>,
}

#[tokio::main]
async fn main() {
    // ── Logging ──────────────────────────────────────────────────────
    // Use JSON format in production (set `OZ_LOG_FORMAT=json`), plain
    // text by default for local dev.
    if std::env::var("OZ_LOG_FORMAT").as_deref() == Ok("json") {
        oz_logging::init_json();
    } else {
        oz_logging::init();
    }

    // ── Database ─────────────────────────────────────────────────────
    let db_path = std::env::var("OZ_DB_PATH").unwrap_or_else(|_| "oz-pos.db".into());
    let mut conn = Connection::open(&db_path).expect("failed to open database");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("enabling foreign_keys");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("enabling WAL");
    oz_core::migrations::run(&mut conn).expect("running migrations");
    info!(db = %db_path, "database opened and migrations applied");

    let state = CloudServerState {
        db: Arc::new(Mutex::new(conn)),
    };

    // ── Routes ───────────────────────────────────────────────────────
    let app = build_router(state);

    // ── Server ───────────────────────────────────────────────────────
    let port: u16 = std::env::var("OZ_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3099);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind port");
    info!(port, "OZ-POS cloud server listening");
    axum::serve(listener, app)
        .await
        .expect("server exited with error");
}

/// Build the combined router: REST API + sync endpoints.
fn build_router(state: CloudServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    // Build the oz-api router (products, categories, sales, health, tokens).
    let api_state = oz_api::AppState {
        db: state.db.clone(),
    };
    let api_router = oz_api::router(api_state);

    // Build the sync router (push/pull endpoints).
    let sync_router = Router::new()
        .route("/api/sync/push", post(sync_push_handler))
        .route("/api/sync/pull", post(sync_pull_handler))
        .route("/api/sync/status", get(sync_status_handler))
        .with_state(state);

    Router::new()
        .merge(api_router)
        .merge(sync_router)
        .layer(cors)
}

// ── Sync handlers ─────────────────────────────────────────────────────────

/// `POST /api/sync/push` — receive and process offline queue items.
///
/// Items are inserted with their existing IDs (not re-generated).
/// Duplicate IDs are rejected with a Conflict outcome.
async fn sync_push_handler(
    State(state): State<CloudServerState>,
    axum::Json(items): axum::Json<Vec<oz_core::offline::OfflineQueueItem>>,
) -> Result<axum::Json<platform_sync::transport::PushResponse>, (axum::http::StatusCode, String)> {
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
            Ok(_) => results.push(platform_sync::transport::PushOutcome::Accepted),
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    results.push(platform_sync::transport::PushOutcome::Rejected {
                        reason: format!("duplicate id: {}", item.id),
                    });
                } else {
                    results.push(platform_sync::transport::PushOutcome::Rejected {
                        reason: format!("database error: {e}"),
                    });
                }
            }
        }
    }

    let resp = platform_sync::transport::PushResponse { results };
    Ok(axum::Json(resp))
}

/// Query parameters for the pull endpoint.
#[derive(Debug, serde::Deserialize)]
struct SyncPullQuery {
    /// ISO-8601 timestamp of the last successful sync.
    since: Option<String>,
}

/// `POST /api/sync/pull` — return items changed since the given timestamp.
///
/// Uses `list_all_offline()` and filters by `created_at >= since` on the Rust side.
async fn sync_pull_handler(
    State(state): State<CloudServerState>,
    axum::Json(query): axum::Json<SyncPullQuery>,
) -> Result<axum::Json<platform_sync::transport::PullResponse>, (axum::http::StatusCode, String)> {
    let conn = state.db.lock().await;
    let store = oz_core::db::Store::new(&conn);

    let all_items = store.list_all_offline().unwrap_or_default();
    let items: Vec<_> = if let Some(ref since) = query.since {
        all_items
            .into_iter()
            .filter(|i| i.created_at >= *since)
            .collect()
    } else {
        all_items
    };

    let resp = platform_sync::transport::PullResponse { items };
    Ok(axum::Json(resp))
}

/// Response from the status endpoint.
#[derive(Debug, serde::Serialize)]
struct SyncStatusResponse {
    status: String,
    version: String,
    pending_count: i64,
}

/// `GET /api/sync/status` — return server status and pending queue count.
async fn sync_status_handler(
    State(state): State<CloudServerState>,
) -> axum::Json<SyncStatusResponse> {
    let pending_count = {
        let conn = state.db.lock().await;
        let store = oz_core::db::Store::new(&conn);
        store.pending_offline_count().unwrap_or(0)
    };

    axum::Json(SyncStatusResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        pending_count,
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

    /// Helper: build an in-memory database with migrations applied.
    fn fresh_db() -> Connection {
        oz_core::migrations::fresh_db()
    }

    /// Helper: create a test router backed by an in-memory database.
    fn test_app() -> Router {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        build_router(state)
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn sync_status_returns_ok() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/sync/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn sync_status_returns_json() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/sync/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["version"].is_string());
        assert_eq!(json["pending_count"], 0);
    }

    #[tokio::test]
    async fn sync_push_empty_array() {
        let app = test_app();
        let body = r#"[]"#;
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/push")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn sync_push_and_pull_roundtrip() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = build_router(state.clone());

        // Seed an item directly
        {
            let conn = state.db.lock().await;
            conn.execute(
                "INSERT INTO offline_queue (id, action, payload, status, created_at) 
                 VALUES (?1, ?2, ?3, 'pending', datetime('now'))",
                rusqlite::params!["test-id", "complete_sale", r#"{"total":100}"#],
            )
            .unwrap();
        }

        // Pull should return the seeded item
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/pull")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"since": null}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let items = json["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], "test-id");
        assert_eq!(items[0]["action"], "complete_sale");
    }

    #[tokio::test]
    async fn sync_push_receives_items() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = build_router(state.clone());

        // Push an item
        let body = r#"[{"id":"push-1","action":"create_product","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-01-01T00:00:00Z","synced_at":null}]"#;
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/push")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify it was stored
        let conn = state.db.lock().await;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM offline_queue WHERE id = 'push-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn sync_status_reflects_pending_count() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
        };
        let app = build_router(state.clone());

        // Seed two pending items
        {
            let conn = state.db.lock().await;
            conn.execute(
                "INSERT INTO offline_queue (id, action, payload, status, created_at)
                 VALUES ('a', 'action', '{}', 'pending', datetime('now')),
                        ('b', 'action', '{}', 'pending', datetime('now'))",
                [],
            )
            .unwrap();
        }

        let req = Request::builder()
            .uri("/api/sync/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["pending_count"], 2);
    }

    #[tokio::test]
    async fn cors_headers_present() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/sync/status")
            .header("Origin", "http://example.com")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let allow_origin = resp
            .headers()
            .get("access-control-allow-origin")
            .map(|v| v.to_str().unwrap());
        assert_eq!(allow_origin, Some("*"));
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/unknown")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
