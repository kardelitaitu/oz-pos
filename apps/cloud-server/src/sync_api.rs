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

use crate::metrics;
use crate::rate_limit::{RateLimiterState, rate_limit_middleware};

/// Snapshot cache entry: (generation timestamp, serialised JSON bytes).
type CacheEntry = (std::time::Instant, Vec<u8>);
/// Per-tenant snapshot cache map.
type SnapshotCache = Arc<Mutex<std::collections::HashMap<String, CacheEntry>>>;

/// Shared state for sync handlers — a database connection behind `Arc<Mutex<>>`.
#[derive(Clone)]
pub struct SyncState {
    pub db: Arc<Mutex<Connection>>,
    /// Snapshot cache: keyed by tenant_id, stores (generated_at, JSON bytes).
    /// P-3 Step 4: in-memory cache with 5-minute TTL.
    pub snapshot_cache: SnapshotCache,
    /// P8-1: Per-tenant rate limiter for sync endpoints.
    pub rate_limiter: RateLimiterState,
}

impl SyncState {
    /// Create a new SyncState from a CloudServerState and an existing RateLimiterState.
    /// This ensures the rate limiter instance is shared with the cleanup task.
    pub fn from_with_rate_limiter(
        state: super::CloudServerState,
        rate_limiter: RateLimiterState,
    ) -> Self {
        Self {
            db: state.db,
            snapshot_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            rate_limiter,
        }
    }
}

impl From<super::CloudServerState> for SyncState {
    fn from(state: super::CloudServerState) -> Self {
        Self::from_with_rate_limiter(state, RateLimiterState::new())
    }
}

/// Build the sync router with all four endpoints, protected by JWT auth
/// and per-tenant rate limiting (P8-1).
///
/// Middleware order (axum: first `.layer()` = outermost, runs FIRST):
///
///   `.layer(axum::Extension(rate_limiter.clone()))` — makes RateLimiterState available
///   `.layer(middleware::from_fn(auth_middleware))`        ← outermost (injects ApiTokenClaims)
///   `.layer(middleware::from_fn(rate_limit_middleware))`  ← innermost (reads claims)
///
/// Execution order: auth_middleware → rate_limit_middleware → handler
/// Axum layers are applied from outside to inside, so the LAST .layer() is the
/// innermost (closest to the handler).
pub fn sync_router(state: SyncState) -> Router {
    let rate_limiter = state.rate_limiter.clone();
    Router::new()
        .route("/api/sync/push", post(push_handler))
        .route("/api/sync/pull", post(pull_handler))
        .route("/api/sync/status", get(status_handler))
        .route("/api/sync/snapshot", get(snapshot_handler))
        .with_state(state)
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(middleware::from_fn(auth_middleware))
        .layer(axum::Extension(rate_limiter))
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
    let start = std::time::Instant::now();
    use oz_core::offline::OfflineQueueStatus;

    // Tenant isolation: use the tenant_id from the JWT claims, not the
    // incoming JSON body, to prevent tenant spoofing.
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");

    let db_start = std::time::Instant::now();
    let conn = state.db.lock().await;
    metrics::DB_CONTENTION_SECONDS
        .with_label_values(&["push"])
        .observe(db_start.elapsed().as_secs_f64());
    let mut results = Vec::with_capacity(items.len());

    // Estimate batch size for metrics.
    let batch_bytes = serde_json::to_vec(&items).map(|v| v.len()).unwrap_or(0) as f64;
    metrics::SYNC_BATCH_SIZE_BYTES.observe(batch_bytes);

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
            Ok(_) => {
                metrics::SYNC_PUSHES_TOTAL.with_label_values(&["accepted"]).inc();
                results.push(PushOutcome::Accepted)
            }
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    metrics::SYNC_PUSHES_TOTAL.with_label_values(&["conflict"]).inc();
                    results.push(PushOutcome::Rejected {
                        reason: format!("duplicate id: {}", item.id),
                    });
                } else {
                    metrics::SYNC_PUSHES_TOTAL.with_label_values(&["rejected"]).inc();
                    results.push(PushOutcome::Rejected {
                        reason: format!("database error: {e}"),
                    });
                }
            }
        }
    }

    metrics::SYNC_PUSH_DURATION_MS.observe(start.elapsed().as_secs_f64() * 1000.0);
    Ok(axum::Json(PushResponse { results }))
}

/// `POST /api/sync/pull` — return items changed since the given timestamp.
///
/// Supports cursor-based pagination (P-3): the client passes an opaque
/// `cursor` from the previous page's `next_cursor` to fetch the next page.
/// Each page returns at most 500 items. When `next_cursor` is null, all
/// pages have been consumed.
async fn pull_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
    axum::Json(req): axum::Json<PullRequest>,
) -> Result<axum::Json<PullResponse>, (axum::http::StatusCode, String)> {
    let start = std::time::Instant::now();
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    let db_start = std::time::Instant::now();
    let conn = state.db.lock().await;
    metrics::DB_CONTENTION_SECONDS
        .with_label_values(&["pull"])
        .observe(db_start.elapsed().as_secs_f64());

    // P-1 retention: if the client's anchor (`since`) is older than the
    // oldest retained row, the requested data has been pruned. Skip this
    // check when using a cursor (subsequent pages don't re-check anchor).
    if req.cursor.is_none()
        && let Some(ref since) = req.since
    {
        let oldest: Option<String> = conn
            .query_row(
                "SELECT MIN(created_at) FROM offline_queue WHERE tenant_id = ?1",
                params![tenant_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        if let Some(ref oldest_ts) = oldest
            && since < oldest_ts
        {
            metrics::SYNC_ANCHOR_EXPIRED_TOTAL.inc();
            return Err((
                axum::http::StatusCode::GONE,
                serde_json::json!({
                    "error": "anchor_expired",
                    "oldest_available": oldest_ts,
                })
                .to_string(),
            ));
        }
    }

    // P-3: decode cursor if present. Format: "created_at|id".
    let (cursor_ts, cursor_id) = if let Some(ref cursor) = req.cursor {
        let parts: Vec<&str> = cursor.splitn(2, '|').collect();
        if parts.len() == 2 {
            (Some(parts[0].to_owned()), Some(parts[1].to_owned()))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // Build paginated query. Fetch one extra row (501) to detect more pages.
    let limit = 501i64;
    let mut items: Vec<oz_core::offline::OfflineQueueItem> = if let (Some(ts), Some(cid)) =
        (&cursor_ts, &cursor_id)
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority
                 FROM offline_queue
                 WHERE tenant_id = ?1 AND created_at >= ?2 AND (created_at > ?3 OR (created_at = ?3 AND id > ?4))
                 ORDER BY created_at ASC, id ASC
                 LIMIT ?5",
            )
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows = stmt
            .query_map(
                params![
                    tenant_id,
                    req.since.as_deref().unwrap_or(""),
                    ts,
                    cid,
                    limit
                ],
                row_to_item,
            )
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    } else if let Some(ref since) = req.since {
        let mut stmt = conn
            .prepare(
                "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority
                 FROM offline_queue
                 WHERE created_at >= ?1 AND tenant_id = ?2
                 ORDER BY created_at ASC, id ASC
                 LIMIT ?3",
            )
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows = stmt
            .query_map(params![since, tenant_id, limit], row_to_item)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority
                 FROM offline_queue
                 WHERE tenant_id = ?1
                 ORDER BY created_at ASC, id ASC
                 LIMIT ?2",
            )
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows = stmt
            .query_map(params![tenant_id, limit], row_to_item)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    // P-3: Detect if there are more pages (501st row exists).
    let next_cursor = if items.len() > 500 {
        items.truncate(500);
        let last = items.last().unwrap();
        Some(format!("{}|{}", last.created_at, last.id))
    } else {
        None
    };

    metrics::SYNC_PULL_DURATION_MS.observe(start.elapsed().as_secs_f64() * 1000.0);
    Ok(axum::Json(PullResponse { items, next_cursor }))
}

/// `GET /api/sync/snapshot` — return reference data baseline for a tenant (P-3).
///
/// Called by clients whose sync anchor has expired. Returns all products,
/// tax rates, and users for the requesting tenant (scoped by `tenant_id`
/// from JWT claims). Responses are cached in-memory per-tenant with a
/// 5-min TTL.
///
/// TODO: When oz-api adds POST endpoints for tax_rates and users, those
/// handlers must stamp `tenant_id` from JWT claims — same pattern as
/// `create_product` in oz-api/src/routes/products.rs. Without it, new
/// tax rates and users default to 'default' and leak across tenants.
async fn snapshot_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
) -> axum::Json<serde_json::Value> {
    let start = std::time::Instant::now();
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");

    // Helper: build an error JSON response.
    let error_json = |msg: &str| -> axum::Json<serde_json::Value> {
        axum::Json(serde_json::json!({"error": msg}))
    };

    // P-3 Step 4: check in-memory cache (5-min TTL).
    {
        let cache = state.snapshot_cache.lock().await;
        if let Some((cached_at, cached_bytes)) = cache.get(tenant_id)
            && cached_at.elapsed().as_secs() < 300
            && let Ok(json) = serde_json::from_slice::<serde_json::Value>(cached_bytes)
        {
            return axum::Json(json);
        }
    }

    let db_start = std::time::Instant::now();
    let conn = state.db.lock().await;
    metrics::DB_CONTENTION_SECONDS
        .with_label_values(&["snapshot"])
        .observe(db_start.elapsed().as_secs_f64());

    // Query products — scoped to the requesting tenant.
    let products: Vec<serde_json::Value> = match (|| -> Result<_, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, sku, name, price_minor, currency, category_id, barcode, created_at, updated_at, price_updated_at, track_serial
                 FROM products WHERE tenant_id = ?1"
            )
            .map_err(|e| e.to_string())?;
        Ok(stmt
            .query_map(params![tenant_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>("id")?,
                    "sku": row.get::<_, String>("sku")?,
                    "name": row.get::<_, String>("name")?,
                    "price_minor": row.get::<_, i64>("price_minor")?,
                    "currency": row.get::<_, String>("currency")?,
                    "category_id": row.get::<_, Option<String>>("category_id")?,
                    "barcode": row.get::<_, Option<String>>("barcode")?,
                    "created_at": row.get::<_, String>("created_at")?,
                    "updated_at": row.get::<_, String>("updated_at")?,
                    "price_updated_at": row.get::<_, String>("price_updated_at")?,
                    "track_serial": row.get::<_, bool>("track_serial")?
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect())
    })() {
        Ok(v) => v,
        Err(e) => return error_json(&e),
    };

    // Query tax rates — scoped to the requesting tenant.
    let tax_rates: Vec<serde_json::Value> = match (|| -> Result<_, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, rate_bps, is_default, is_inclusive, created_at, updated_at FROM tax_rates WHERE tenant_id = ?1"
            )
            .map_err(|e| e.to_string())?;
        Ok(stmt
            .query_map(params![tenant_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>("id")?,
                    "name": row.get::<_, String>("name")?,
                    "rate_bps": row.get::<_, i64>("rate_bps")?,
                    "is_default": row.get::<_, bool>("is_default")?,
                    "is_inclusive": row.get::<_, bool>("is_inclusive")?,
                    "created_at": row.get::<_, Option<String>>("created_at")?,
                    "updated_at": row.get::<_, Option<String>>("updated_at")?
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect())
    })() {
        Ok(v) => v,
        Err(e) => return error_json(&e),
    };

    // Query users — scoped to the requesting tenant.
    let users: Vec<serde_json::Value> = match (|| -> Result<_, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at FROM users WHERE tenant_id = ?1"
            )
            .map_err(|e| e.to_string())?;
        Ok(stmt
            .query_map(params![tenant_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>("id")?,
                    "username": row.get::<_, String>("username")?,
                    "pin_hash": row.get::<_, String>("pin_hash")?,
                    "display_name": row.get::<_, String>("display_name")?,
                    "role_id": row.get::<_, String>("role_id")?,
                    "is_active": row.get::<_, bool>("is_active")?,
                    "created_at": row.get::<_, Option<String>>("created_at")?,
                    "updated_at": row.get::<_, Option<String>>("updated_at")?
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect())
    })() {
        Ok(v) => v,
        Err(e) => return error_json(&e),
    };

    let snapshot = serde_json::json!({
        "products": products,
        "tax_rates": tax_rates,
        "users": users,
    });

    // Cache the result.
    if let Ok(cached_bytes) = serde_json::to_vec(&snapshot) {
        let mut cache = state.snapshot_cache.lock().await;
        cache.insert(
            tenant_id.to_owned(),
            (std::time::Instant::now(), cached_bytes),
        );
    }

    metrics::SYNC_PULL_DURATION_MS.observe(start.elapsed().as_secs_f64() * 1000.0);
    axum::Json(snapshot)
}

/// `GET /api/sync/status` — return server health, version, and pending queue depth.
async fn status_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
) -> axum::Json<SyncStatusResponse> {
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    let (pending_count, total_tenants) = {
        let conn = state.db.lock().await;
        let pending = conn
            .query_row(
                "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending' AND tenant_id = ?1",
                params![tenant_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);
        let tenants = conn
            .query_row(
                "SELECT COUNT(DISTINCT tenant_id) FROM offline_queue",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);
        (pending, tenants)
    };

    // P-3: Tiered heartbeat — server tells client how often to poll.
    // < 1000 tenants → 120s, 1000-5000 → 300s, 5000+ → max(300, 10k/count*60).
    let heartbeat_interval_secs = match total_tenants {
        0..=999 => 120,
        1000..=5000 => 300,
        _ => (10_000 / total_tenants * 60).max(300),
    };

    axum::Json(SyncStatusResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        pending_count,
        heartbeat_interval_secs: heartbeat_interval_secs as u64,
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
    /// Recommended heartbeat interval in seconds (P-3 tiered heartbeat).
    pub heartbeat_interval_secs: u64,
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
        priority: row
            .get::<_, i32>("priority")
            .map(oz_core::offline::SyncPriority::from)
            .unwrap_or(oz_core::offline::SyncPriority::Normal),
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
    use std::collections::HashMap;
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
            snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: RateLimiterState::new(),
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
    async fn snapshot_tenant_isolation() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
            snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: RateLimiterState::new(),
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
            snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: RateLimiterState::new(),
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
            snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: RateLimiterState::new(),
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

    // ── Anchor expiry (P-1 retention) ────────────────────────────

    #[tokio::test]
    async fn pull_returns_410_when_anchor_expired() {
        let state = SyncState {
            db: Arc::new(Mutex::new(fresh_db())),
            snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: RateLimiterState::new(),
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
}
