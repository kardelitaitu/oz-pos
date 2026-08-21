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
    extract::{Extension, Request, State},
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use rusqlite::Connection;
use tokio::sync::Mutex;

use oz_api::auth::{ApiTokenClaims, auth_middleware};
use platform_sync::transport::{PullRequest, PullResponse, PushOutcome, PushResponse};

use crate::metrics;
use crate::rate_limit::{RateLimiterState, rate_limit_middleware};

/// Snapshot cache entry: (generation timestamp, serialised JSON bytes).
type CacheEntry = (std::time::Instant, Vec<u8>);
/// Per-tenant snapshot cache map.
type SnapshotCache = Arc<Mutex<std::collections::HashMap<String, CacheEntry>>>;

/// Cached global tenant count (status endpoint).
///
/// `distinct_tenant_count()` is a `COUNT(DISTINCT tenant_id)` over the
/// whole `offline_queue` — O(n) on a table bounded only by the 90-day
/// retention horizon. Every terminal polls `/api/sync/status` on its
/// heartbeat, so an uncached scan would repeat constantly on the hot
/// path. The count only feeds the tiered-heartbeat calculation, so a
/// bounded-stale value is fine: refresh at most once per
/// [`TENANT_COUNT_CACHE_TTL`](Self::TENANT_COUNT_CACHE_TTL) and serve
/// the cached value in between.
#[derive(Clone, Default)]
pub struct TenantCountCache(Arc<Mutex<Option<(std::time::Instant, i64)>>>);

impl TenantCountCache {
    /// How long a cached tenant count is trusted before the next refresh.
    const TENANT_COUNT_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

    /// Return the cached count, or `None` when no refresh has run yet.
    async fn cached(&self) -> Option<i64> {
        let guard = self.0.lock().await;
        guard
            .as_ref()
            .filter(|(at, _)| at.elapsed() < Self::TENANT_COUNT_CACHE_TTL)
            .map(|(_, count)| *count)
    }

    /// Store a freshly-computed count.
    async fn store(&self, count: i64) {
        let mut guard = self.0.lock().await;
        *guard = Some((std::time::Instant::now(), count));
    }
}

/// Shared state for sync handlers — a database connection behind `Arc<Mutex<>>`.
#[derive(Clone)]
pub struct SyncState {
    pub db: Arc<Mutex<Connection>>,
    /// Postgres pool for the sync data layer (Phase 1.2). `None` keeps the
    /// SQLite backend; `Some` routes push/pull/status/snapshot/plan through
    /// Postgres while the REST API continues to use the SQLite connection.
    pub pg: Option<deadpool_postgres::Pool>,
    /// Snapshot cache: keyed by tenant_id, stores (generated_at, JSON bytes).
    /// P-3 Step 4: in-memory cache with 15-minute TTL.
    pub snapshot_cache: SnapshotCache,
    /// P8-1: Per-tenant rate limiter for sync endpoints.
    pub rate_limiter: RateLimiterState,
    /// Skip UUID validation on push items. When true, items are inserted
    /// without parsing the id as UUID — saves ~0.02 core CPU at 200+ terminals.
    /// Set via OZ_SKIP_PUSH_VALIDATION=1.
    pub skip_push_validation: bool,
    /// TTL-cached global tenant count for the status endpoint.
    pub tenant_count_cache: TenantCountCache,
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
            pg: None,
            snapshot_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            rate_limiter,
            skip_push_validation: std::env::var("OZ_SKIP_PUSH_VALIDATION")
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false),
            tenant_count_cache: TenantCountCache::default(),
        }
    }

    /// Build the sync data backend for this state (Phase 1.2).
    ///
    /// Cheap to call: it clones the already-shared SQLite `Arc` or Postgres
    /// pool, so handlers can obtain a backend without holding state locks.
    fn store(&self) -> crate::sync_store::SyncStore {
        match &self.pg {
            Some(pool) => crate::sync_store::SyncStore::postgres(pool.clone()),
            None => crate::sync_store::SyncStore::sqlite(self.db.clone()),
        }
    }

    /// Global tenant count for the status endpoint, served from a
    /// TTL-bounded cache to avoid a `COUNT(DISTINCT tenant_id)` full scan
    /// on every heartbeat poll (SOTA: the count only feeds tiered
    /// heartbeat sizing, so up to 60s of staleness is harmless).
    async fn cached_tenant_count(&self) -> i64 {
        if let Some(count) = self.tenant_count_cache.cached().await {
            return count;
        }
        let count = self.store().distinct_tenant_count().await;
        self.tenant_count_cache.store(count).await;
        count
    }
}

impl From<super::CloudServerState> for SyncState {
    fn from(state: super::CloudServerState) -> Self {
        Self::from_with_rate_limiter(state, RateLimiterState::new())
    }
}

/// Build the sync router with all four endpoints, protected by JWT auth,
/// per-tenant rate limiting (P8-1), and optional plan enforcement
/// (ADR sync-plan-gating).
///
/// Middleware order (axum: first `.layer()` = outermost, runs FIRST):
///
///   `.layer(axum::Extension(rate_limiter.clone()))` — makes RateLimiterState available
///   `.layer(axum::Extension(store))`                  — makes the SyncStore available to plan_middleware
///   `.layer(axum::Extension(enforce_plans))`          — plan gate on/off
///   `.layer(middleware::from_fn(auth_middleware))`        ← outermost (injects ApiTokenClaims)
///   `.layer(middleware::from_fn(plan_middleware))`        ← reads claims, gates free tenants
///   `.layer(middleware::from_fn(rate_limit_middleware))`  ← innermost (reads claims)
///
/// Execution order: auth_middleware → plan_middleware → rate_limit_middleware → handler
/// Axum layers are applied from outside to inside, so the LAST .layer() is the
/// innermost (closest to the handler).
pub fn sync_router(state: SyncState, enforce_plans: bool) -> Router {
    sync_router_with_plan_enforcement(state, enforce_plans)
}

/// Build the sync router with an explicit plan-enforcement flag (used by
/// tests and by [`sync_router`], which reads `OZ_ENFORCE_PLANS`).
pub fn sync_router_with_plan_enforcement(state: SyncState, enforce_plans: bool) -> Router {
    let rate_limiter = state.rate_limiter.clone();
    let store = state.store();
    Router::new()
        .route("/api/sync/push", post(push_handler))
        .route("/api/sync/pull", post(pull_handler))
        .route("/api/sync/status", get(status_handler))
        .route("/api/sync/snapshot", get(snapshot_handler))
        .with_state(state)
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(middleware::from_fn(plan_middleware))
        .layer(middleware::from_fn(auth_middleware))
        .layer(axum::Extension(rate_limiter))
        .layer(axum::Extension(store))
        .layer(axum::Extension(enforce_plans))
}

/// Plan gate (ADR sync-plan-gating): when enforcement is enabled, a tenant
/// on the `free` plan (or with no assigned plan — fail closed) is rejected
/// with a structured 403 `{"error":"plan_required"}`. Runs after auth so
/// claims are available, before the handler.
#[allow(clippy::result_large_err)]
pub async fn plan_middleware(
    Extension(enforce_plans): Extension<bool>,
    Extension(store): Extension<crate::sync_store::SyncStore>,
    request: Request,
    next: middleware::Next,
) -> Result<axum::response::Response, axum::response::Response> {
    use axum::response::IntoResponse;
    use oz_core::TenantPlan;

    if !enforce_plans {
        return Ok(next.run(request).await);
    }

    let tenant_id = request
        .extensions()
        .get::<oz_api::auth::ApiTokenClaims>()
        .and_then(|claims| claims.tenant_id.as_deref())
        .unwrap_or("default");

    let plan = store.get_tenant_plan(tenant_id).await.map_err(|_| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": "internal" })),
        )
            .into_response()
    })?;

    if plan.unwrap_or(TenantPlan::Free) == TenantPlan::Free {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({ "error": "plan_required" })),
        )
            .into_response());
    }

    Ok(next.run(request).await)
}

/// `POST /api/sync/push` — receive and persist offline queue items.
///
/// Each item is inserted with its existing client-generated ID. Duplicate
/// IDs (UNIQUE constraint violation) are reported as `Rejected`.
#[tracing::instrument(skip(state, items), fields(tenant_id = claims.tenant_id.as_deref().unwrap_or("default"), item_count = items.len()))]
async fn push_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
    axum::Json(items): axum::Json<Vec<oz_core::offline::OfflineQueueItem>>,
) -> Result<axum::Json<PushResponse>, (axum::http::StatusCode, String)> {
    let start = std::time::Instant::now();

    // Tenant isolation: use the tenant_id from the JWT claims, not the
    // incoming JSON body, to prevent tenant spoofing.
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");

    // Estimate batch size for metrics.
    let batch_bytes = serde_json::to_vec(&items).map(|v| v.len()).unwrap_or(0) as f64;
    metrics::SYNC_BATCH_SIZE_BYTES.observe(batch_bytes);

    // Phase 1.2: INSERT goes through the sync store in a single transaction
    // (previously one transaction per item). `db_start` measures backend
    // access for the whole batch (mutex lock / pool acquisition).
    let store = state.store();
    let db_start = std::time::Instant::now();
    let mut results = Vec::with_capacity(items.len());

    // Phase 1: separate valid items from rejected UUIDs (cheap, no DB).
    let mut valid_items: Vec<oz_core::offline::OfflineQueueItem> = Vec::with_capacity(items.len());
    for item in &items {
        if !state.skip_push_validation && uuid::Uuid::parse_str(&item.id).is_err() {
            metrics::SYNC_PUSHES_TOTAL
                .with_label_values(&["rejected"])
                .inc();
            results.push(PushOutcome::Rejected {
                reason: format!("invalid id: {}", item.id),
            });
        } else {
            valid_items.push(item.clone());
        }
    }

    // Phase 2: single-transaction batch insert for the valid items.
    let batch_results = if valid_items.is_empty() {
        Vec::new()
    } else {
        store
            .push_batch(&valid_items, tenant_id)
            .await
            .map_err(|e| {
                metrics::SYNC_PUSHES_TOTAL
                    .with_label_values(&["rejected"])
                    .inc();
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)
            })?
    };
    for outcome in batch_results {
        let label = match &outcome {
            PushOutcome::Accepted => "accepted",
            PushOutcome::Rejected { reason } if reason.starts_with("duplicate id:") => "conflict",
            PushOutcome::Rejected { .. } => "rejected",
            PushOutcome::Conflict(..) => "conflict",
        };
        metrics::SYNC_PUSHES_TOTAL.with_label_values(&[label]).inc();
        results.push(outcome);
    }

    metrics::DB_CONTENTION_SECONDS
        .with_label_values(&["push"])
        .observe(db_start.elapsed().as_secs_f64());
    metrics::SYNC_PUSH_DURATION_MS.observe(start.elapsed().as_secs_f64() * 1000.0);
    Ok(axum::Json(PushResponse { results }))
}

/// `POST /api/sync/pull` — return items changed since the given timestamp.
///
/// Supports cursor-based pagination (P-3): the client passes an opaque
/// `cursor` from the previous page's `next_cursor` to fetch the next page.
/// Each page returns at most 500 items. When `next_cursor` is null, all
/// pages have been consumed.
#[tracing::instrument(skip(state, req), fields(tenant_id = claims.tenant_id.as_deref().unwrap_or("default"), since = req.since.as_deref().unwrap_or("null")))]
async fn pull_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
    axum::Json(req): axum::Json<PullRequest>,
) -> Result<axum::Json<PullResponse>, (axum::http::StatusCode, String)> {
    let start = std::time::Instant::now();
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    // Phase 1.2: anchor check + paginated pull go through the sync store,
    // so the SQLite and Postgres backends share one code path.
    let store = state.store();
    let db_start = std::time::Instant::now();

    // P-1 retention: if the client's anchor (`since`) is older than the
    // oldest retained row, the requested data has been pruned. Skip this
    // check when using a cursor (subsequent pages don't re-check anchor).
    if req.cursor.is_none()
        && let Some(ref since) = req.since
        && let Some(oldest_ts) = store.oldest_created_at(tenant_id).await
        && since.as_str() < oldest_ts.as_str()
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
    let cursor = match (&cursor_ts, &cursor_id) {
        (Some(ts), Some(cid)) => Some((ts.as_str(), cid.as_str())),
        _ => None,
    };
    let mut items: Vec<oz_core::offline::OfflineQueueItem> = store
        .pull_items(tenant_id, req.since.as_deref(), cursor, limit)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    metrics::DB_CONTENTION_SECONDS
        .with_label_values(&["pull"])
        .observe(db_start.elapsed().as_secs_f64());

    // P-3: Detect if there are more pages (501st row exists).
    // RUST-07: the pagination cursor is derived from the last *kept* row.
    // `get` (not `last().unwrap()`) so an empty slice yields `None` instead
    // of panicking.
    let next_cursor = if items.len() > 500 {
        items.truncate(500);
        items
            .last()
            .map(|last| format!("{}|{}", last.created_at, last.id))
    } else {
        None
    };

    metrics::SYNC_PULL_DURATION_MS.observe(start.elapsed().as_secs_f64() * 1000.0);
    Ok(axum::Json(PullResponse { items, next_cursor }))
}

/// Build a JSON response from pre-serialized bytes.
///
/// Serves the snapshot cache hit path without a
/// `serde_json::from_slice` → `axum::Json` re-serialize round trip
/// (SOTA finding C): the cache stores `Vec<u8>` precisely to avoid
/// JSON work on hits, so the hit path now returns the bytes as-is.
fn json_bytes_response(bytes: Vec<u8>) -> axum::response::Response {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        bytes,
    )
        .into_response()
}

/// `GET /api/sync/snapshot` — return reference data baseline for a tenant (P-3).
///
/// Called by clients whose sync anchor has expired. Returns all products,
/// tax rates, and users for the requesting tenant (scoped by `tenant_id`
/// from JWT claims). Responses are cached in-memory per-tenant with a
/// 15-min TTL; cache hits serve the stored bytes directly with zero JSON
/// processing.
///
/// Both `POST /api/v1/tax-rates` and `POST /api/v1/users` now stamp
/// `tenant_id` from JWT claims (same pattern as `create_product` in
/// `oz-api/src/routes/products.rs`). New tax rates and users are
/// correctly scoped per-tenant for snapshot isolation.
#[tracing::instrument(skip(state), fields(tenant_id = claims.tenant_id.as_deref().unwrap_or("default")))]
async fn snapshot_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
) -> Result<axum::response::Response, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let start = std::time::Instant::now();
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");

    // Helper: build an error JSON response with a non-2xx status (SYNC-09).
    // A failed snapshot must never look like a valid empty snapshot — the
    // client rejects non-success statuses before deserialising the body.
    let error_json = |msg: &str| -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
        tracing::error!(tenant_id, error = msg, "snapshot: query failed");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error": msg})),
        )
    };

    // P-3 Step 4: check in-memory cache (15-min TTL).
    // Reference data (products, tax rates, users) changes infrequently
    // during a shift. 15 min reduces cache misses by 3× vs 5 min.
    const SNAPSHOT_CACHE_TTL_SECS: u64 = 900;
    {
        let cache = state.snapshot_cache.lock().await;
        if let Some((cached_at, cached_bytes)) = cache.get(tenant_id)
            && cached_at.elapsed().as_secs() < SNAPSHOT_CACHE_TTL_SECS
        {
            // Cache hit: serve the stored bytes directly. No JSON
            // deserialize → re-serialize (SOTA finding C).
            return Ok(json_bytes_response(cached_bytes.clone()));
        }
    }

    // Phase 1.2: reference-data queries go through the sync store.
    // snapshot_all fetches products + tax_rates + users in a single
    // transaction (saves 3 round-trips vs separate calls).
    let store = state.store();
    let db_start = std::time::Instant::now();

    // SYNC-10: row decode failures fail the whole snapshot (5xx).
    let (products, tax_rates, users) = match store.snapshot_all(tenant_id).await {
        Ok(v) => v,
        Err(e) => return Err(error_json(&e)),
    };

    metrics::DB_CONTENTION_SECONDS
        .with_label_values(&["snapshot"])
        .observe(db_start.elapsed().as_secs_f64());

    let snapshot = serde_json::json!({
        "products": products,
        "tax_rates": tax_rates,
        "users": users,
    });

    // Serialize once: the bytes are both cached and served as the
    // response body (SOTA finding C — no second JSON pass).
    let bytes = match serde_json::to_vec(&snapshot) {
        Ok(bytes) => bytes,
        Err(e) => return Err(error_json(&e.to_string())),
    };

    // Cache the result, opportunistically pruning expired entries so a
    // tenant that stops polling cannot leave its bytes in memory forever
    // (unbounded growth under tenant churn). The TTL read-check above
    // only skips STALE reads; this eviction is what bounds the map size.
    let mut cache = state.snapshot_cache.lock().await;
    cache.retain(|_, (cached_at, _)| cached_at.elapsed().as_secs() < SNAPSHOT_CACHE_TTL_SECS);
    cache.insert(
        tenant_id.to_owned(),
        (std::time::Instant::now(), bytes.clone()),
    );

    metrics::SYNC_PULL_DURATION_MS.observe(start.elapsed().as_secs_f64() * 1000.0);
    Ok(json_bytes_response(bytes))
}

/// `GET /api/sync/status` — return server health, version, and pending queue depth.
#[tracing::instrument(skip(state), fields(tenant_id = claims.tenant_id.as_deref().unwrap_or("default")))]
async fn status_handler(
    State(state): State<SyncState>,
    Extension(claims): Extension<ApiTokenClaims>,
) -> axum::Json<SyncStatusResponse> {
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    let store = state.store();
    let (pending_count, total_tenants) = (
        store.pending_count(tenant_id).await,
        state.cached_tenant_count().await,
    );

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

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "sync_api_tests.rs"]
mod tests;
