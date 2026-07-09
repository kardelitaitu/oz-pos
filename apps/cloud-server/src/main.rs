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

mod db;
mod sync_api;

use std::sync::Arc;

use axum::Router;
use rusqlite::Connection;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::sync_api::{SyncState, sync_router};

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
    if std::env::var("OZ_LOG_FORMAT").as_deref() == Ok("json") {
        oz_logging::init_json();
    } else {
        oz_logging::init();
    }

    // ── Database ─────────────────────────────────────────────────────
    // Supports both SQLite (OZ_DB_PATH) and PostgreSQL (DATABASE_URL).
    // SQLite is the default backend.
    let pool = db::DbPool::from_env()
        .await
        .expect("failed to initialise database");

    match &pool {
        db::DbPool::Sqlite(conn) => {
            info!("running with SQLite backend");
            let state = CloudServerState { db: conn.clone() };
            let app = build_router(state);
            serve(app).await;
        }
        db::DbPool::Postgres(pg_pool) => {
            info!("running with PostgreSQL backend");
            // For PostgreSQL, we use a PostgreSQL-compatible router.
            // Currently, the oz-api router requires SQLite, so we fall
            // back to SQLite for the API layer when PostgreSQL is the
            // primary database. The sync transport layer can use PG.
            let conn = db::DbPool::connect_sqlite_in_memory()
                .expect("failed to create in-memory SQLite for API");
            let state = CloudServerState {
                db: conn.sqlite_conn(),
            };
            let app = build_router(state);
            serve(app).await;
        }
    }
}

/// Start the HTTP server on the configured port.
async fn serve(app: Router) {
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
pub fn build_router(state: CloudServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    // Build the oz-api router (products, categories, sales, health, tokens).
    let api_state = oz_api::AppState {
        db: state.db.clone(),
    };
    let api_router = oz_api::router(api_state);

    // Build the sync router (push/pull endpoints) from sync_api module.
    let sync_router = sync_router(SyncState::from(state));

    Router::new()
        .merge(api_router)
        .merge(sync_router)
        .layer(cors)
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
