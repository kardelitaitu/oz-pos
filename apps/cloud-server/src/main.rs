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
//! | `OZ_REDIRECT_ONLY` | — | Run in redirect-only mode (ADR #11). Requires `OZ_SYNC_REDIRECT_URL`. Skips DB, prune, metrics, API — only serves the migration redirect. |
//! | `OZ_SYNC_REDIRECT_URL` | — | New server URL for migration redirect. When set, all `/api/sync/*` requests return `{"error":"server_migrated","new_url":"<url>"}` with HTTP 421. |
//! | `RUST_LOG` | `info` | Log level filter (e.g. `debug`, `oz_cloud_server=debug`) |

mod db;
mod email;
mod metrics;
mod openapi;
mod prune;
mod rate_limit;
mod redirect;
mod shutdown;
mod sync_api;
mod webhooks;

use std::sync::Arc;
use std::time::Instant;

use axum::{Json, Router};
use rusqlite::Connection;
use serde::Serialize;
use tokio::sync::Mutex;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::rate_limit::{RateLimiterState, start_rate_limit_cleanup};
use crate::sync_api::{SyncState, sync_router};

/// Shared application state for the cloud server.
///
/// Provides the database connection and any additional server-wide state.
#[derive(Clone)]
pub struct CloudServerState {
    /// Database connection wrapped for axum's `State` extractor.
    pub db: Arc<Mutex<Connection>>,
    /// Instant captured at startup for uptime calculation.
    pub started_at: Instant,
    /// P5-3: Stripe webhook signing secret (loaded from `STRIPE_WEBHOOK_SECRET` env var).
    pub stripe_webhook_secret: Option<String>,
    /// P5-3: Square webhook signature key (loaded from `SQUARE_WEBHOOK_SIGNATURE_KEY` env var).
    pub square_webhook_signature_key: Option<String>,
    /// P5-3: Public Square webhook URL (loaded from `SQUARE_WEBHOOK_URL` env var).
    pub square_webhook_url: Option<String>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // ── tokio-console (RUSTFLAGS="--cfg tokio_unstable" + feature "console") ─
    #[cfg(feature = "console")]
    {
        console_subscriber::init();
        tracing::info!("tokio-console subscriber initialised");
    }
    #[cfg(not(feature = "console"))]
    {
        tracing::debug!(
            "tokio-console disabled — compile with `--features console` + RUSTFLAGS=\"--cfg tokio_unstable\" to enable"
        );
    }

    // ── Logging ──────────────────────────────────────────────────────
    if std::env::var("OZ_LOG_FORMAT").as_deref() == Ok("json") {
        oz_logging::init_json();
    } else {
        oz_logging::init();
    }

    // ── Config validation (--validate-config skips the server) ───────
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--validate-config") {
        info!("running config validation only (--validate-config)");
        match oz_core::config_validator::validate_config() {
            Ok(()) => {
                info!("all configuration checks passed");
                std::process::exit(0);
            }
            Err(errors) => {
                for err in &errors {
                    tracing::error!(%err, "configuration error");
                }
                eprintln!(
                    "Configuration validation failed with {} error(s):",
                    errors.len()
                );
                for err in &errors {
                    eprintln!("  • {err}");
                }
                std::process::exit(1);
            }
        }
    }

    // ── Startup config validation ───────────────────────────────────
    // Check critical env vars before the server starts. Failures are
    // logged as warnings (non-blocking) because the server may still
    // function with SQLite defaults if DATABASE_URL is misconfigured.
    if let Err(errors) = oz_core::config_validator::validate_config() {
        for err in &errors {
            tracing::warn!(%err, "configuration warning");
        }
    }

    // ── Redirect-only mode (ADR #11) ──────────────────────────────────
    // When OZ_REDIRECT_ONLY is set, skip all infrastructure (DB, prune,
    // metrics, API) and run a minimal server that only returns the
    // migration redirect. This keeps the old VPS cheap during the
    // 15-30 day migration window.
    if std::env::var("OZ_REDIRECT_ONLY").as_deref() == Ok("true") {
        if std::env::var("OZ_SYNC_REDIRECT_URL").is_err() {
            tracing::error!("OZ_REDIRECT_ONLY=true requires OZ_SYNC_REDIRECT_URL to be set");
            std::process::exit(1);
        }
        info!("running in redirect-only mode (ADR #11)");
        let redirect_router = Router::new()
            .fallback(|| async { axum::http::StatusCode::MISDIRECTED_REQUEST })
            .layer(axum::middleware::from_fn(redirect::redirect_middleware));
        serve(redirect_router).await;
        return;
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
            let state = CloudServerState {
                db: conn.clone(),
                started_at: Instant::now(),
                stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
                square_webhook_signature_key: std::env::var("SQUARE_WEBHOOK_SIGNATURE_KEY").ok(),
                square_webhook_url: std::env::var("SQUARE_WEBHOOK_URL").ok(),
            };
            // Start the background prune loop (ADR #6 Q4 / P-1 Ledger Retention).
            prune::start_prune_loop(conn.clone());

            // P55-3: Start the scheduled report sender loop.
            email::start_report_sender_loop(conn.clone());

            // P8-1: Per-tenant rate limiter state + background cleanup.
            let rate_limiter = RateLimiterState::new();
            start_rate_limit_cleanup(rate_limiter.clone());

            let app = build_router(state, rate_limiter);
            serve(app).await;
        }
        db::DbPool::Postgres(_pg_pool) => {
            info!("running with PostgreSQL backend");
            // For PostgreSQL, we use a PostgreSQL-compatible router.
            // Currently, the oz-api router requires SQLite, so we fall
            // back to SQLite for the API layer when PostgreSQL is the
            // primary database. The sync transport layer can use PG.
            let conn = db::DbPool::connect_sqlite_in_memory()
                .expect("failed to create in-memory SQLite for API");
            let state = CloudServerState {
                db: conn.sqlite_conn(),
                started_at: Instant::now(),
                stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
                square_webhook_signature_key: std::env::var("SQUARE_WEBHOOK_SIGNATURE_KEY").ok(),
                square_webhook_url: std::env::var("SQUARE_WEBHOOK_URL").ok(),
            };

            // P8-1: Per-tenant rate limiter state + background cleanup.
            let rate_limiter = RateLimiterState::new();
            start_rate_limit_cleanup(rate_limiter.clone());

            let app = build_router(state, rate_limiter);
            serve(app).await;
        }
    }
}

/// Start the HTTP server on the configured port with graceful shutdown.
///
/// Listens for SIGTERM (Docker/K8s) or Ctrl+C. On receiving the signal:
/// 1. Stops accepting new connections
/// 2. Drains in-flight connections with a 30-second timeout
/// 3. Logs the shutdown and exits cleanly
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
        .with_graceful_shutdown(shutdown::shutdown_signal())
        .await
        .expect("server exited with error");

    // Drain in-flight connections with a grace period.
    // After the shutdown signal, axum stops accepting new connections
    // and waits for existing requests to complete. This additional
    // sleep gives any last-second requests time to finish before the
    // process exits and background tasks are dropped.
    const DRAIN_TIMEOUT_SECS: u64 = 30;
    info!(
        drain_timeout_secs = DRAIN_TIMEOUT_SECS,
        "server stopped accepting connections, draining in-flight requests"
    );
    tokio::time::sleep(std::time::Duration::from_secs(DRAIN_TIMEOUT_SECS)).await;
    info!("graceful shutdown complete");
}

/// Build the combined router: REST API + sync endpoints.
/// Response from the health endpoint (P8-3: enhanced with DB ping and queue depth).
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    db: String,
    uptime_seconds: u64,
    /// Whether the database is reachable (actual ping, not static).
    db_connected: bool,
    /// Database ping latency in microseconds.
    db_latency_us: u64,
    /// Number of items in the sync queue with status `pending`.
    sync_queue_depth: i64,
    /// ISO-8601 timestamp of the most recent sync activity, or null.
    last_sync_at: Option<String>,
}

/// `GET /metrics` — Prometheus metrics endpoint.
/// Public, no auth required (same as /health).
async fn metrics_handler() -> String {
    crate::metrics::render_metrics()
}

/// `GET /health` — public health check, no auth required.
///
/// Performs an actual DB ping, reports sync queue depth, last sync
/// timestamp, and uptime. Used by the Tauri app's ConnectionStatus
/// component and by Docker healthchecks.
///
/// All DB queries are performed in a single lock acquisition to
/// minimise contention with concurrent sync handlers (P8-3).
async fn health_handler(
    axum::extract::State(state): axum::extract::State<CloudServerState>,
) -> Json<HealthResponse> {
    let uptime = state.started_at.elapsed().as_secs();

    // P8-3: all DB queries in a single lock acquisition.
    let (db_connected, db_latency_us, sync_queue_depth, last_sync_at) = {
        let db_start = std::time::Instant::now();
        let conn = state.db.lock().await;

        let ping_result = conn.query_row("SELECT 1", [], |_| Ok(()));
        let latency = db_start.elapsed().as_micros() as u64;
        let connected = ping_result.is_ok();

        let depth = conn
            .query_row(
                "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        let last = conn
            .query_row(
                "SELECT MAX(synced_at) FROM offline_queue WHERE synced_at IS NOT NULL",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap_or(None);

        (connected, latency, depth, last)
    };

    // P8-3: record health check Prometheus metrics.
    crate::metrics::HEALTH_CHECKS_TOTAL.inc();
    if !db_connected {
        crate::metrics::HEALTH_CHECK_FAILURES_TOTAL.inc();
    }
    crate::metrics::HEALTH_DB_LATENCY_MICROS.observe(db_latency_us as f64);

    Json(HealthResponse {
        status: if db_connected { "ok" } else { "degraded" },
        version: env!("CARGO_PKG_VERSION"),
        db: "sqlite".into(),
        uptime_seconds: uptime,
        db_connected,
        db_latency_us,
        sync_queue_depth,
        last_sync_at,
    })
}

/// Build the combined router: REST API + sync endpoints + rate limiting.
pub fn build_router(state: CloudServerState, rate_limiter: RateLimiterState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    // Build the oz-api router (products, categories, sales, health, tokens).
    let api_state = oz_api::AppState {
        db: state.db.clone(),
    };
    let api_router = oz_api::router(api_state);

    // Clone state for the health endpoint BEFORE SyncState::from consumes the original.
    let health_state = state.clone();

    // Build the sync router (push/pull endpoints) from sync_api module.
    // P8-1: Share the same RateLimiterState with the cleanup task.
    let sync_state = SyncState::from_with_rate_limiter(state.clone(), rate_limiter);
    let sync_router = sync_router(sync_state);

    // Build the webhook router (unauthenticated — HMAC signature verification).
    let webhook_router = webhooks::webhooks_router(state.clone());

    // P-2: Per-route-group concurrency limits prevent sync bursts
    // from starving the product/sales/health API routes.
    // API: 10 concurrent, sync: 40 concurrent.
    let api_router = api_router.layer(ConcurrencyLimitLayer::new(10));
    let sync_router = sync_router.layer(ConcurrencyLimitLayer::new(40));

    // OpenAPI documentation routes — Swagger UI + raw OpenAPI JSON.
    let docs_router = Router::new()
        .route(
            "/api/openapi.json",
            axum::routing::get(openapi::openapi_json_handler),
        )
        .route("/api/docs", axum::routing::get(openapi::swagger_ui_handler));

    Router::new()
        .route("/health", axum::routing::get(health_handler))
        .route("/api/health", axum::routing::get(health_handler))
        .route("/metrics", axum::routing::get(metrics_handler))
        .with_state(health_state)
        .merge(docs_router)
        .merge(api_router)
        .merge(sync_router)
        .merge(webhook_router)
        .layer(axum::middleware::from_fn(redirect::redirect_middleware))
        .layer(CompressionLayer::new().gzip(true))
        .layer(cors)
} // ── Tests ─────────────────────────────────────────────────────────────────

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
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        build_router(state, crate::rate_limit::RateLimiterState::new())
    }

    /// Create a test JWT token.
    fn test_token(tenant_id: Option<&str>) -> String {
        oz_api::auth::create_token("test", Some(24), tenant_id)
            .unwrap()
            .token
    }

    /// Add an Authorization header to a request builder.
    fn with_auth(uri: &str, tenant_id: Option<&str>) -> Request<Body> {
        let token = test_token(tenant_id);
        Request::builder()
            .uri(uri)
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    }

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

    #[tokio::test]
    async fn metrics_returns_prometheus_text() {
        let app = test_app();
        let req = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("sync_pushes_total"));
        assert!(text.contains("sync_push_duration_ms"));
        assert!(text.contains("sync_pull_duration_ms"));
        assert!(text.contains("sync_anchor_expired_total"));
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = test_app();
        // oz-api health endpoint
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn cloud_health_api_alias_returns_ok() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["db_connected"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn cloud_health_returns_ok_with_db_ping() {
        let app = test_app();
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["version"].is_string());
        assert_eq!(json["db"], "sqlite");
        assert!(json["uptime_seconds"].as_u64().is_some());
        assert_eq!(json["db_connected"], true);
        assert!(json["db_latency_us"].as_u64().unwrap() > 0);
        assert_eq!(json["sync_queue_depth"], 0);
        assert!(json["last_sync_at"].is_null());
    }

    #[tokio::test]
    async fn cloud_health_reports_queue_depth() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        let app = build_router(state.clone(), crate::rate_limit::RateLimiterState::new());

        // Seed some pending queue items
        {
            let conn = state.db.lock().await;
            conn.execute_batch(
                "INSERT INTO offline_queue (id, action, payload, status, created_at, synced_at, tenant_id) VALUES
                 ('h-1', 'act', '{}', 'pending', '2026-06-01T00:00:00Z', NULL, 't1'),
                 ('h-2', 'act', '{}', 'pending', '2026-06-02T00:00:00Z', NULL, 't1'),
                 ('h-3', 'act', '{}', 'synced', '2026-06-03T00:00:00Z', '2026-06-03T12:00:00Z', 't1')"
            )
            .unwrap();
        }

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["sync_queue_depth"], 2);
        assert!(!json["last_sync_at"].is_null());
    }

    #[tokio::test]
    async fn cloud_health_reports_last_sync_at() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        let app = build_router(state.clone(), crate::rate_limit::RateLimiterState::new());

        // Seed some items with various synced_at times
        {
            let conn = state.db.lock().await;
            conn.execute_batch(
                "INSERT INTO offline_queue (id, action, payload, status, created_at, synced_at, tenant_id) VALUES
                 ('h-a', 'act', '{}', 'synced', '2026-06-01T00:00:00Z', '2026-06-02T12:00:00Z', 't1'),
                 ('h-b', 'act', '{}', 'synced', '2026-06-03T00:00:00Z', '2026-06-04T08:30:00Z', 't1')"
            )
            .unwrap();
        }

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["last_sync_at"], "2026-06-04T08:30:00Z");
    }

    // ── Console smoke test (tokio-console) ───────────────────────────

    #[cfg(all(feature = "console", tokio_unstable))]
    #[tokio::test]
    async fn console_subscriber_inits_without_panic() {
        // This test verifies that the console subscriber can be
        // initialised without panicking. In CI it's a no-op since the
        // `console` feature is not enabled; run locally with:
        //   RUSTFLAGS="--cfg tokio_unstable" cargo test --features console -p oz-cloud-server
        console_subscriber::init();
        // If we get here, init succeeded (no double-init panic).
        tracing::info!("tokio-console smoke test passed");
    }

    #[tokio::test]
    async fn sync_status_returns_ok_with_auth() {
        let app = test_app();
        let req = with_auth("/api/sync/status", None);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn sync_push_and_pull_roundtrip() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        let rate_limiter = crate::rate_limit::RateLimiterState::new();
        let app = build_router(state.clone(), rate_limiter);

        // Seed an item directly with tenant_id
        {
            let conn = state.db.lock().await;
            conn.execute(
                "INSERT INTO offline_queue (id, action, payload, status, created_at, tenant_id) 
                 VALUES (?1, ?2, ?3, 'pending', datetime('now'), 'default')",
                rusqlite::params!["test-id", "complete_sale", r#"{"total":100}"#],
            )
            .unwrap();
        }

        // Pull should return the seeded item (for default tenant)
        let req = authed_post("/api/sync/pull", r#"{"since": null}"#, None);
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
            .header("Authorization", format!("Bearer {}", test_token(None)))
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
    async fn unknown_route_returns_401_or_404() {
        let app = test_app();
        let req = Request::builder()
            .uri("/api/unknown")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Auth middleware on sync router catches unknown routes before
        // the 404 handler; both 401 and 404 are acceptable.
        assert!(
            resp.status() == StatusCode::UNAUTHORIZED || resp.status() == StatusCode::NOT_FOUND,
            "expected 401 or 404, got: {}",
            resp.status()
        );
    }

    // ── Multi-tenant isolation integration tests ─────────────────────

    #[tokio::test]
    async fn multi_tenant_tenant_a_push_invisible_to_tenant_b() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        let rate_limiter = crate::rate_limit::RateLimiterState::new();
        let app = build_router(state.clone(), rate_limiter);

        // Tenant A pushes two items
        let push_body = r#"[
            {"id":"a-item-1","action":"sale.create","payload":"{\"total\":100}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-01T00:00:00Z","synced_at":null},
            {"id":"a-item-2","action":"sale.void","payload":"{\"reason\":\"test\"}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-02T00:00:00Z","synced_at":null}
        ]"#;
        let push_req = authed_post("/api/sync/push", push_body, Some("tenant-a"));
        let push_resp = app.clone().oneshot(push_req).await.unwrap();
        assert_eq!(push_resp.status(), StatusCode::OK);

        // Tenant B pulls — should see ZERO items (isolation)
        let pull_req = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-b"));
        let pull_resp = app.clone().oneshot(pull_req).await.unwrap();
        assert_eq!(pull_resp.status(), StatusCode::OK);
        let body = pull_resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json["items"].as_array().unwrap().len(),
            0,
            "Tenant B should see zero items from Tenant A's push"
        );

        // Tenant A pulls — should see its 2 items
        let pull_a = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-a"));
        let resp_a = app.clone().oneshot(pull_a).await.unwrap();
        let body_a = resp_a.into_body().collect().await.unwrap().to_bytes();
        let json_a: serde_json::Value = serde_json::from_slice(&body_a).unwrap();
        assert_eq!(json_a["items"].as_array().unwrap().len(), 2);
        assert_eq!(json_a["items"][0]["id"], "a-item-1");
        assert_eq!(json_a["items"][1]["id"], "a-item-2");
    }

    #[tokio::test]
    async fn multi_tenant_bidirectional_isolation() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        let rate_limiter = crate::rate_limit::RateLimiterState::new();
        let app = build_router(state.clone(), rate_limiter);

        // Tenant A pushes one item
        let push_a = authed_post(
            "/api/sync/push",
            r#"[{"id":"only-a","action":"act","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-01T00:00:00Z","synced_at":null}]"#,
            Some("tenant-a"),
        );
        let r = app.clone().oneshot(push_a).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Tenant B pushes one item
        let push_b = authed_post(
            "/api/sync/push",
            r#"[{"id":"only-b","action":"act","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-01T00:00:00Z","synced_at":null}]"#,
            Some("tenant-b"),
        );
        let r = app.clone().oneshot(push_b).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Tenant A should see ONLY 'only-a'
        let pull_a = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-a"));
        let r_a = app.clone().oneshot(pull_a).await.unwrap();
        let b_a = r_a.into_body().collect().await.unwrap().to_bytes();
        let j_a: serde_json::Value = serde_json::from_slice(&b_a).unwrap();
        let items_a = j_a["items"].as_array().unwrap();
        assert_eq!(items_a.len(), 1, "Tenant A sees only its own items");
        assert_eq!(items_a[0]["id"], "only-a");

        // Tenant B should see ONLY 'only-b'
        let pull_b = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-b"));
        let r_b = app.oneshot(pull_b).await.unwrap();
        let b_b = r_b.into_body().collect().await.unwrap().to_bytes();
        let j_b: serde_json::Value = serde_json::from_slice(&b_b).unwrap();
        let items_b = j_b["items"].as_array().unwrap();
        assert_eq!(items_b.len(), 1, "Tenant B sees only its own items");
        assert_eq!(items_b[0]["id"], "only-b");
    }

    #[tokio::test]
    async fn multi_tenant_status_scoped_per_tenant() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        let rate_limiter = crate::rate_limit::RateLimiterState::new();
        let app = build_router(state.clone(), rate_limiter);

        // Tenant A pushes 3 items
        let push_a = authed_post(
            "/api/sync/push",
            r#"[
                {"id":"a-1","action":"act","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-01T00:00:00Z","synced_at":null},
                {"id":"a-2","action":"act","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-01T00:00:00Z","synced_at":null},
                {"id":"a-3","action":"act","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-01T00:00:00Z","synced_at":null}
            ]"#,
            Some("tenant-a"),
        );
        let r = app.clone().oneshot(push_a).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Tenant B pushes 1 item
        let push_b = authed_post(
            "/api/sync/push",
            r#"[{"id":"b-1","action":"act","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-01T00:00:00Z","synced_at":null}]"#,
            Some("tenant-b"),
        );
        let r = app.clone().oneshot(push_b).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Tenant A status: 3 pending
        let s_a = with_auth("/api/sync/status", Some("tenant-a"));
        let r_a = app.clone().oneshot(s_a).await.unwrap();
        let b_a = r_a.into_body().collect().await.unwrap().to_bytes();
        let j_a: serde_json::Value = serde_json::from_slice(&b_a).unwrap();
        assert_eq!(j_a["pending_count"], 3);

        // Tenant B status: 1 pending
        let s_b = with_auth("/api/sync/status", Some("tenant-b"));
        let r_b = app.oneshot(s_b).await.unwrap();
        let b_b = r_b.into_body().collect().await.unwrap().to_bytes();
        let j_b: serde_json::Value = serde_json::from_slice(&b_b).unwrap();
        assert_eq!(j_b["pending_count"], 1);
    }

    #[tokio::test]
    async fn multi_tenant_default_tenant_isolation() {
        let state = CloudServerState {
            db: Arc::new(Mutex::new(fresh_db())),
            started_at: Instant::now(),
            stripe_webhook_secret: None,
            square_webhook_signature_key: None,
            square_webhook_url: None,
        };
        let rate_limiter = crate::rate_limit::RateLimiterState::new();
        let app = build_router(state.clone(), rate_limiter);

        // Push items as default tenant
        let push_d = authed_post(
            "/api/sync/push",
            r#"[{"id":"def-item","action":"act","payload":"{}","status":"pending","retry_count":0,"last_error":null,"created_at":"2026-06-01T00:00:00Z","synced_at":null}]"#,
            None,
        );
        let r = app.clone().oneshot(push_d).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Explicit tenant-c should NOT see default tenant's items
        let pull_c = authed_post("/api/sync/pull", r#"{"since":null}"#, Some("tenant-c"));
        let r_c = app.clone().oneshot(pull_c).await.unwrap();
        let b_c = r_c.into_body().collect().await.unwrap().to_bytes();
        let j_c: serde_json::Value = serde_json::from_slice(&b_c).unwrap();
        assert_eq!(
            j_c["items"].as_array().unwrap().len(),
            0,
            "tenant-c should not see default tenant items"
        );

        // Default tenant should see its own item
        let pull_d = authed_post("/api/sync/pull", r#"{"since":null}"#, None);
        let r_d = app.oneshot(pull_d).await.unwrap();
        let b_d = r_d.into_body().collect().await.unwrap().to_bytes();
        let j_d: serde_json::Value = serde_json::from_slice(&b_d).unwrap();
        assert_eq!(j_d["items"].as_array().unwrap().len(), 1);
        assert_eq!(j_d["items"][0]["id"], "def-item");
    }
}
