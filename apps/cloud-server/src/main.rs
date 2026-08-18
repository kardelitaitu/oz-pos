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
//! | `OZ_ADMIN_KEY` | — | Admin key gating `POST /api/v1/tokens` (ADR sync-auth-hardening P2). When unset the token endpoint stays open (dev mode); set it in production so only callers with the matching `X-Admin-Key` header can mint tokens. |
//! | `OZ_ENFORCE_PLANS` | — | When `1`/`true`/`on`, sync requests from tenants on the `free` plan (or with no plan row) are rejected with `403 plan_required` (ADR sync-plan-gating). When unset, plan gating is off — dev mode keeps working as before. |
//! | `OZ_REDIRECT_ONLY` | — | Run in redirect-only mode (ADR #11). Requires `OZ_SYNC_REDIRECT_URL`. Skips DB, prune, metrics, API — only serves the migration redirect. |
//! | `OZ_SYNC_REDIRECT_URL` | — | New server URL for migration redirect. When set, all `/api/sync/*` requests return `{"error":"server_migrated","new_url":"<url>"}` with HTTP 421. |
//! | `RUST_LOG` | `info` | Log level filter (e.g. `debug`, `oz_cloud_server=debug`) |

mod config;
mod db;
mod email;
mod email_pg;
mod metrics;
mod openapi;
mod prune;
mod rate_limit;
mod redirect;
mod shutdown;
mod sync_api;
mod sync_store;
mod webhooks;

use std::sync::Arc;
use std::time::Instant;

use axum::{Json, Router};
use rusqlite::Connection;
use serde::Serialize;
use tokio::sync::Mutex;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;
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
    /// Optional Postgres pool (Phase 1.2). `Some` on the Postgres branch;
    /// the health handler reads the sync queue from it instead of the
    /// (empty, in-memory) SQLite fallback.
    pub pg: Option<deadpool_postgres::Pool>,
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
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ── tokio-console (RUSTFLAGS="--cfg tokio_unstable" + feature "console") ─
    // console-subscriber panics if tokio was not built with `tokio_unstable`,
    // so the init is gated on BOTH the feature and the cfg — `--all-features`
    // without RUSTFLAGS must not crash startup.
    #[cfg(all(feature = "console", tokio_unstable))]
    {
        console_subscriber::init();
        tracing::info!("tokio-console subscriber initialised");
    }
    #[cfg(not(all(feature = "console", tokio_unstable)))]
    {
        tracing::debug!(
            "tokio-console disabled — compile with `--features console` + RUSTFLAGS=\"--cfg tokio_unstable\" to enable"
        );
    }

    // ── Configuration ────────────────────────────────────────────────
    let config =
        config::CloudServerConfig::from_env().map_err(|e| format!("invalid configuration: {e}"))?;

    // ── Logging ──────────────────────────────────────────────────────
    match config.log_format {
        config::LogFormat::Json => {
            oz_logging::try_init_json().map_err(|e| format!("logging init_json failed: {e}"))?;
        }
        config::LogFormat::Plain => {
            oz_logging::try_init().map_err(|e| format!("logging init failed: {e}"))?;
        }
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
    if config.redirect_only {
        info!("running in redirect-only mode (ADR #11)");
        let redirect_url = config
            .sync_redirect_url
            .clone()
            // SAFETY: redirect_only mode validates sync_redirect_url at
            // construction, so the URL is always present here — SAFETY.
            .expect("validated at construction");
        let redirect_router = Router::new()
            .fallback(|| async { axum::http::StatusCode::MISDIRECTED_REQUEST })
            .layer(axum::middleware::from_fn_with_state(
                Some(redirect_url),
                redirect::redirect_middleware,
            ));
        serve(redirect_router, config).await?;
        return Ok(());
    }

    // ── Database ─────────────────────────────────────────────────────
    // Supports both SQLite (OZ_DB_PATH) and PostgreSQL (DATABASE_URL).
    // SQLite is the default backend.
    let pool = db::DbPool::from_config(&config)
        .await
        .map_err(|e| format!("failed to initialise database: {e}"))?;

    match &pool {
        db::DbPool::Sqlite(conn) => {
            info!("running with SQLite backend");
            let state = CloudServerState {
                db: conn.clone(),
                pg: None,
                started_at: Instant::now(),
                stripe_webhook_secret: config.stripe_webhook_secret.clone(),
                square_webhook_signature_key: config.square_webhook_signature_key.clone(),
                square_webhook_url: config.square_webhook_url.clone(),
            };
            // Start the background prune loop (ADR #6 Q4 / P-1 Ledger Retention).
            prune::start_prune_loop(conn.clone());

            // P55-3: Start the scheduled report sender loop.
            email::start_report_sender_loop(conn.clone());

            // P8-1: Per-tenant rate limiter state + background cleanup.
            let rate_limiter = RateLimiterState::new();
            start_rate_limit_cleanup(rate_limiter.clone());

            let app = build_router(state, rate_limiter, &config, None);
            serve(app, config).await?;
        }
        db::DbPool::Postgres(pg_pool) => {
            info!("running with PostgreSQL backend");
            // The oz-api REST handlers dispatch on `state.pg` (Some →
            // Postgres data layer, None → the SQLite `Store` path), so the
            // API layer reads/writes Postgres here. The in-memory SQLite is
            // only a never-touched fallback for handlers that were never
            // ported (health is PG-aware) — production data never lands in
            // it.
            let conn = db::DbPool::connect_sqlite_in_memory()
                .map_err(|e| format!("failed to create in-memory SQLite for API: {e}"))?;
            let state = CloudServerState {
                db: conn.sqlite_conn(),
                pg: Some(pg_pool.clone()),
                started_at: Instant::now(),
                stripe_webhook_secret: config.stripe_webhook_secret.clone(),
                square_webhook_signature_key: config.square_webhook_signature_key.clone(),
                square_webhook_url: config.square_webhook_url.clone(),
            };

            // P8-1: Per-tenant rate limiter state + background cleanup.
            let rate_limiter = RateLimiterState::new();
            start_rate_limit_cleanup(rate_limiter.clone());

            // Phase 1.5: P-1 offline-queue retention on Postgres (the SQLite
            // prune loop does not run on this branch).
            prune::start_prune_loop_pg(pg_pool.clone());

            // Phase 1.5: scheduled report sender on Postgres (the SQLite
            // loop reads the same settings/analytics surface via rusqlite).
            email_pg::start_report_sender_loop_pg(pg_pool.clone());

            let app = build_router(state, rate_limiter, &config, Some(pg_pool.clone()));
            serve(app, config).await?;
        }
    }
    Ok(())
}

/// Start the HTTP server on the configured port with graceful shutdown.
///
/// Listens for SIGTERM (Docker/K8s) or Ctrl+C. On receiving the signal:
/// 1. Stops accepting new connections
/// 2. Drains in-flight connections with a 30-second timeout
/// 3. Logs the shutdown and exits cleanly
async fn serve(
    app: Router,
    config: config::CloudServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = config.port;

    // RUST-07: a port bind failure (e.g. port already in use) is a recoverable
    // operational error and now propagates to main instead of panicking.
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .map_err(|e| format!("failed to bind port {port}: {e}"))?;
    info!(port, "OZ-POS cloud server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::shutdown_signal())
        .await
        .map_err(|e| format!("server exited with error: {e}"))?;

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
    Ok(())
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

    // P8-3: all DB queries in a single lock acquisition. On the Postgres
    // branch the queue depth / last-sync are read from the real database
    // (the in-memory SQLite fallback is empty by design).
    let (db_connected, db_latency_us, sync_queue_depth, last_sync_at, db_kind) =
        if let Some(pool) = &state.pg {
            let db_start = std::time::Instant::now();
            let (connected, depth, last) = match pool.get().await {
                Ok(client) => {
                    let depth = client
                        .query_one(
                            "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending'",
                            &[],
                        )
                        .await
                        .map(|r| r.get::<_, i64>(0))
                        .unwrap_or(0);
                    let last = client
                        .query_one(
                            "SELECT MAX(synced_at) FROM offline_queue \
                             WHERE synced_at IS NOT NULL",
                            &[],
                        )
                        .await
                        .map(|r| r.get::<_, Option<String>>(0))
                        .unwrap_or(None);
                    (true, depth, last)
                }
                Err(_) => (false, 0, None),
            };
            let latency = db_start.elapsed().as_micros() as u64;
            (connected, latency, depth, last, "postgres")
        } else {
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

            (connected, latency, depth, last, "sqlite")
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
        db: db_kind.into(),
        uptime_seconds: uptime,
        db_connected,
        db_latency_us,
        sync_queue_depth,
        last_sync_at,
    })
}

/// Build the combined router: REST API + sync endpoints + rate limiting.
///
/// `pg` is the optional Postgres pool for the sync data layer (Phase 1.2).
/// When set, push/pull/status/snapshot/plan read and write Postgres; when
/// `None`, the sync function keeps using the shared SQLite connection.
pub fn build_router(
    state: CloudServerState,
    rate_limiter: RateLimiterState,
    config: &config::CloudServerConfig,
    pg: Option<deadpool_postgres::Pool>,
) -> Router {
    // CORS allowlist shared with the oz-api router (unify-auth-and-sync.md
    // §11): documented defaults, overridable via OZ_CORS_ORIGINS.
    let cors_origins = oz_api::cors_origins_from_env();
    let cors = oz_api::build_cors(&cors_origins);

    // Build the oz-api router (products, categories, sales, health, tokens).
    let api_state = oz_api::AppState {
        db: state.db.clone(),
        // Phase 1.2: the REST handlers read/write Postgres on the cloud
        // branch instead of the in-memory SQLite fallback.
        pg: pg.clone(),
        // ADR sync-auth-hardening P2: gate token minting with the admin key
        // when configured; open in dev mode when unset.
        admin_key: config.admin_key.clone(),
        api_secret: config.api_secret.clone().unwrap_or_default(),
        db_path: config.db_path.clone(),
        port: config.port,
        cors_origins: cors_origins.clone(),
    };
    let api_router = oz_api::router(api_state);

    // P8-3: Rate-limit token minting per client IP. This needs its own clone
    // of the limiter because /api/v1/tokens runs BEFORE auth (it mints the
    // JWT), so the per-tenant limiter has no claims to key on.
    let token_rate_limiter = rate_limiter.clone();

    // Clone state for the health endpoint BEFORE SyncState::from consumes the original.
    let health_state = state.clone();

    // Build the sync router (push/pull endpoints) from sync_api module.
    // P8-1: Share the same RateLimiterState with the cleanup task.
    let mut sync_state = SyncState::from_with_rate_limiter(state.clone(), rate_limiter);
    sync_state.pg = pg;
    let sync_router = sync_router(sync_state, config.enforce_plans);

    // Build the webhook router (unauthenticated — HMAC signature verification).
    let webhook_router = webhooks::webhooks_router(state.clone());

    // P-2: Per-route-group concurrency limits prevent sync bursts
    // from starving the product/sales/health API routes.
    // API: 10 concurrent, sync: 40 concurrent.
    let api_router = api_router
        .layer(ConcurrencyLimitLayer::new(10))
        .layer(axum::middleware::from_fn(
            crate::rate_limit::token_rate_limit_middleware,
        ))
        .layer(axum::Extension(token_rate_limiter));
    let sync_router = sync_router.layer(ConcurrencyLimitLayer::new(40));

    // OpenAPI documentation routes — Swagger UI + Scalar + raw OpenAPI JSON.
    let docs_router = Router::new()
        .route(
            "/api/openapi.json",
            axum::routing::get(openapi::openapi_json_handler),
        )
        .route("/api/docs", axum::routing::get(openapi::swagger_ui_handler))
        .route(
            "/api/docs/scalar",
            axum::routing::get(openapi::scalar_ui_handler),
        );

    Router::new()
        .route("/health", axum::routing::get(health_handler))
        .route("/api/health", axum::routing::get(health_handler))
        .route("/metrics", axum::routing::get(metrics_handler))
        .with_state(health_state)
        .merge(docs_router)
        .merge(api_router)
        .merge(sync_router)
        .merge(webhook_router)
        .layer(axum::middleware::from_fn_with_state(
            config.sync_redirect_url.clone(),
            redirect::redirect_middleware,
        ))
        .layer(CompressionLayer::new().gzip(true))
        .layer(cors)
        .layer(axum::middleware::from_fn(
            oz_api::security_headers_middleware,
        ))
} // ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)] #[path = "main_tests.rs"] mod tests;
