//! Per-tenant token-bucket rate limiter (P8-1).
//!
//! Provides an axum middleware that limits request rates per tenant
//! (identified via JWT claims) using the token-bucket algorithm.
//! A background task periodically removes stale buckets.
//!
//! # Rate limits
//!
//! | Endpoint | Limit | Window |
//! |---|---|---|
//! | `POST /api/sync/push` | 100 | per minute |
//! | `POST /api/sync/pull` | 300 | per minute |
//! | `GET  /api/sync/status` | 300 | per minute |
//! | `GET  /api/sync/snapshot` | 50 | per minute |
//! | `POST /api/v1/tokens` | 30 | per minute, per client IP |
//!
//! When exceeded, returns `429 Too Many Requests` with a `Retry-After` header.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::RwLock;
use tracing::warn;

// ── Rate limit configuration ──────────────────────────────────────

/// Rate limit configuration for a single route.
#[derive(Debug, Clone, Copy)]
struct RateLimitConfig {
    /// Maximum tokens (burst capacity).
    capacity: u32,
    /// Tokens replenished per second.
    refill_per_sec: f64,
}

/// Map of URI path prefixes to their rate limit configs.
/// Order matters: more specific prefixes first.
const RATE_LIMITS: &[(&str, RateLimitConfig)] = &[
    (
        "/api/sync/snapshot",
        RateLimitConfig {
            capacity: 50,
            refill_per_sec: 50.0 / 60.0, // 50/min
        },
    ),
    (
        "/api/sync/push",
        RateLimitConfig {
            capacity: 100,
            refill_per_sec: 100.0 / 60.0, // 100/min
        },
    ),
    (
        "/api/sync/pull",
        RateLimitConfig {
            capacity: 300,
            refill_per_sec: 300.0 / 60.0, // 300/min
        },
    ),
    (
        "/api/sync/status",
        RateLimitConfig {
            capacity: 300,
            refill_per_sec: 300.0 / 60.0, // 300/min
        },
    ),
];

/// Rate limit for `POST /api/v1/tokens` — 30 mint attempts per minute per
/// client IP. Minting is rare in normal operation (once per terminal boot or
/// on token expiry), so this is generous for legitimate clients while still
/// blocking brute-force attacks on the admin key / terminal credentials.
const TOKEN_RATE_LIMIT: RateLimitConfig = RateLimitConfig {
    capacity: 30,
    refill_per_sec: 30.0 / 60.0, // 30/min
};

// ── Token bucket ──────────────────────────────────────────────────

/// A single token bucket for one tenant + endpoint combination.
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current token count (capped at capacity).
    tokens: f64,
    /// Maximum tokens (burst capacity).
    capacity: u32,
    /// Tokens replenished per second.
    refill_per_sec: f64,
    /// Last time tokens were refilled.
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u32, refill_per_sec: f64) -> Self {
        Self {
            tokens: capacity as f64,
            capacity,
            refill_per_sec,
            last_refill: Instant::now(),
        }
    }

    /// Attempt to consume one token. Returns `true` if allowed.
    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time since last refill.
    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity as f64);
            self.last_refill = Instant::now();
        }
    }

    /// Return the time in seconds until one token is available.
    fn time_until_token(&self) -> f64 {
        let deficit = 1.0 - self.tokens;
        if deficit <= 0.0 {
            return 0.0;
        }
        deficit / self.refill_per_sec
    }
}

// ── Rate limiter state ────────────────────────────────────────────

/// Shared per-tenant rate limiter state.
#[derive(Clone)]
pub struct RateLimiterState {
    /// Per-endpoint key → per-tenant → TokenBucket.
    /// Key format: `"{tenant_id}|{endpoint_key}"`.
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
}

impl RateLimiterState {
    /// Create a new empty rate limiter state.
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Try to consume a token for the given tenant and URI path.
    /// Returns `Ok(())` if allowed, or `Err(retry_after_seconds)` if rate-limited.
    pub async fn check_rate_limit(&self, tenant_id: &str, path: &str) -> Result<(), f64> {
        let Some((prefix, config)) = RATE_LIMITS
            .iter()
            .find(|(prefix, _)| path.starts_with(prefix))
        else {
            return Ok(()); // No rate limit configured for this path
        };

        self.check_keyed(
            format!("{tenant_id}|{prefix}"),
            config.capacity,
            config.refill_per_sec,
        )
        .await
    }

    /// Try to consume a token for the token-mint endpoint, keyed by client IP.
    ///
    /// `/api/v1/tokens` mints the JWT, so there are no `ApiTokenClaims` to key
    /// the per-tenant limiter on; it is throttled by client IP instead to stop
    /// brute-forcing of the admin key / terminal client credentials.
    pub async fn check_token_rate_limit(&self, ip: &str) -> Result<(), f64> {
        self.check_keyed(
            format!("ip:{ip}"),
            TOKEN_RATE_LIMIT.capacity,
            TOKEN_RATE_LIMIT.refill_per_sec,
        )
        .await
    }

    /// Consume one token from the bucket identified by `key`, creating it on
    /// demand with the given capacity and refill rate.
    async fn check_keyed(
        &self,
        key: String,
        capacity: u32,
        refill_per_sec: f64,
    ) -> Result<(), f64> {
        let mut buckets = self.buckets.write().await;
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(capacity, refill_per_sec));

        if bucket.try_consume() {
            Ok(())
        } else {
            Err(bucket.time_until_token())
        }
    }

    /// Remove buckets that haven't been used in more than `max_age`.
    pub async fn cleanup_stale_buckets(&self, max_age: Duration) {
        let mut buckets = self.buckets.write().await;
        let cutoff = Instant::now() - max_age;
        buckets.retain(|_, bucket| bucket.last_refill > cutoff);
    }

    /// Return the number of active buckets (for metrics/debugging).
    pub async fn bucket_count(&self) -> usize {
        self.buckets.read().await.len()
    }
}

// ── Axum middleware ───────────────────────────────────────────────

/// Axum middleware that rate-limits requests per tenant.
///
/// Must be applied AFTER the auth middleware (which injects `ApiTokenClaims`)
/// and AFTER the `RateLimiterState` extension is added to the router.
///
/// Accesses tenant_id via `request.extensions().get::<ApiTokenClaims>()`
/// (injected by auth_middleware) and the RateLimiterState via `Extension`
/// (injected by the router layer in sync_api.rs).
///
/// Returns `429 Too Many Requests` with `Retry-After` header when rate-limited.
pub async fn rate_limit_middleware(
    Extension(rate_limiter): Extension<RateLimiterState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();

    // Tenant ID comes from the auth middleware's extension — if not present,
    // fall through without rate limiting (auth middleware will reject anyway).
    let tenant_id = request
        .extensions()
        .get::<oz_api::auth::ApiTokenClaims>()
        .and_then(|claims| claims.tenant_id.as_deref())
        .unwrap_or("default");

    match rate_limiter.check_rate_limit(tenant_id, path).await {
        Ok(()) => next.run(request).await,
        Err(retry_after) => {
            let retry_secs = retry_after.ceil() as u64;
            warn!(
                tenant_id,
                path,
                retry_after_secs = retry_secs,
                "rate limit exceeded"
            );
            crate::metrics::RATE_LIMIT_429_TOTAL
                .with_label_values(&["sync"])
                .inc();
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", &retry_secs.to_string())],
                axum::Json(serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "retry_after_seconds": retry_secs,
                })),
            )
                .into_response()
        }
    }
}

/// Axum middleware that rate-limits `POST /api/v1/tokens` per client IP.
///
/// Unlike [`rate_limit_middleware`], this runs BEFORE authentication — the
/// token endpoint mints the JWT, so there are no `ApiTokenClaims` to key on.
///
/// The client IP is resolved from the `X-Forwarded-For` / `X-Real-IP` headers,
/// which the reverse proxy must overwrite (per the plan the proxy is the single
/// public entry point). Without a proxy header, all requests share one
/// `"unknown"` bucket.
pub async fn token_rate_limit_middleware(
    Extension(rate_limiter): Extension<RateLimiterState>,
    request: Request,
    next: Next,
) -> Response {
    // Only the token-mint endpoint is throttled here; every other route on
    // the API router passes straight through.
    if request.uri().path() != "/api/v1/tokens" {
        return next.run(request).await;
    }

    let ip = client_ip(&request);
    match rate_limiter.check_token_rate_limit(&ip).await {
        Ok(()) => next.run(request).await,
        Err(retry_after) => {
            let retry_secs = retry_after.ceil() as u64;
            warn!(
                ip = %ip,
                path = "/api/v1/tokens",
                retry_after_secs = retry_secs,
                "token mint rate limit exceeded"
            );
            crate::metrics::RATE_LIMIT_429_TOTAL
                .with_label_values(&["token"])
                .inc();
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", &retry_secs.to_string())],
                axum::Json(serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "retry_after_seconds": retry_secs,
                })),
            )
                .into_response()
        }
    }
}

/// Resolve the client IP for token-mint rate limiting. Prefers the
/// `X-Forwarded-For` header (first hop, set/overwritten by the reverse proxy),
/// then `X-Real-IP`, and finally a shared `"unknown"` bucket when neither is
/// present.
fn client_ip(request: &Request) -> String {
    if let Some(ip) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return ip.to_string();
    }
    if let Some(ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return ip.to_string();
    }
    "unknown".to_string()
}

// ── Background cleanup ────────────────────────────────────────────

/// Start a background task that removes stale rate-limit buckets every 60 seconds.
///
/// Buckets unused for more than 5 minutes are removed. The task runs until
/// the returned `tokio::sync::watch::Sender` is dropped or the application shuts down.
pub fn start_rate_limit_cleanup(state: RateLimiterState) -> tokio::sync::watch::Sender<bool> {
    let (tx, mut rx) = tokio::sync::watch::channel::<bool>(false);

    tokio::spawn(async move {
        // Wait for initial shutdown signal
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    state.cleanup_stale_buckets(Duration::from_secs(300)).await;
                }
                _ = rx.changed() => {
                    // Shutdown signal received
                    break;
                }
            }
        }
    });

    tx
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn token_bucket_allows_initial_burst() {
        let mut bucket = TokenBucket::new(100, 100.0 / 60.0);
        for i in 0..100 {
            assert!(bucket.try_consume(), "request {i} should be allowed");
        }
        // 101st request should be denied immediately
        assert!(!bucket.try_consume(), "101st request should be denied");
    }

    #[tokio::test]
    async fn token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(10, 10.0); // 10 tokens, refill 10/sec

        // Consume all 10
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }
        assert!(!bucket.try_consume(), "11th should be denied");

        // Advance time by 500ms (should have 5 tokens)
        bucket.last_refill = Instant::now() - Duration::from_millis(500);
        assert!(bucket.try_consume(), "should have refilled after 500ms");
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume(), "6th after refill should be denied");
    }

    #[tokio::test]
    async fn token_bucket_respects_capacity() {
        let mut bucket = TokenBucket::new(5, 100.0); // cap 5, refill 100/sec

        // Advance time by 1 hour (should have capacity'd tokens)
        bucket.last_refill = Instant::now() - Duration::from_secs(3600);
        bucket.refill();
        assert!(
            (bucket.tokens - 5.0).abs() < f64::EPSILON,
            "should be capped at 5, got {}",
            bucket.tokens
        );
    }

    #[tokio::test]
    async fn token_bucket_time_until_token() {
        let mut bucket = TokenBucket::new(1, 1.0); // 1 token, refill 1/sec

        // Consume the only token
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());

        // Time until next token should be ~1 second
        let wait = bucket.time_until_token();
        assert!(wait > 0.9 && wait < 1.1, "wait should be ~1s, got {wait}s");
    }

    #[tokio::test]
    async fn rate_limiter_allows_within_limit() {
        let limiter = RateLimiterState::new();

        for i in 0..100 {
            let result = limiter.check_rate_limit("tenant-a", "/api/sync/push").await;
            assert!(
                result.is_ok(),
                "request {i} should be allowed, got: {result:?}"
            );
        }

        // 101st should be rate-limited
        let result = limiter.check_rate_limit("tenant-a", "/api/sync/push").await;
        assert!(result.is_err(), "101st request should be rate-limited");
    }

    #[tokio::test]
    async fn rate_limiter_isolates_tenants() {
        let limiter = RateLimiterState::new();

        // Exhaust tenant-a's push limit
        for _ in 0..100 {
            assert!(
                limiter
                    .check_rate_limit("tenant-a", "/api/sync/push")
                    .await
                    .is_ok()
            );
        }
        assert!(
            limiter
                .check_rate_limit("tenant-a", "/api/sync/push")
                .await
                .is_err()
        );

        // Tenant-b should still have its own bucket (unaffected)
        for _ in 0..100 {
            assert!(
                limiter
                    .check_rate_limit("tenant-b", "/api/sync/push")
                    .await
                    .is_ok(),
                "tenant-b should have its own limit"
            );
        }
        assert!(
            limiter
                .check_rate_limit("tenant-b", "/api/sync/push")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn rate_limiter_isolates_endpoints() {
        let limiter = RateLimiterState::new();

        // Exhaust push limit
        for _ in 0..100 {
            assert!(
                limiter
                    .check_rate_limit("t", "/api/sync/push")
                    .await
                    .is_ok()
            );
        }
        assert!(
            limiter
                .check_rate_limit("t", "/api/sync/push")
                .await
                .is_err()
        );

        // Pull should still be allowed (different limit)
        for _ in 0..300 {
            assert!(
                limiter
                    .check_rate_limit("t", "/api/sync/pull")
                    .await
                    .is_ok(),
                "pull should have its own limit"
            );
        }
        assert!(
            limiter
                .check_rate_limit("t", "/api/sync/pull")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn rate_limiter_unknown_path_allowed() {
        let limiter = RateLimiterState::new();
        let result = limiter.check_rate_limit("t", "/api/unknown").await;
        assert!(result.is_ok(), "unknown paths should be allowed");
    }

    #[tokio::test]
    async fn rate_limiter_snapshot_limit() {
        let limiter = RateLimiterState::new();

        for i in 0..50 {
            assert!(
                limiter
                    .check_rate_limit("t", "/api/sync/snapshot")
                    .await
                    .is_ok(),
                "snapshot request {i} should be allowed"
            );
        }
        assert!(
            limiter
                .check_rate_limit("t", "/api/sync/snapshot")
                .await
                .is_err(),
            "51st snapshot should be rate-limited"
        );
    }

    #[tokio::test]
    async fn rate_limiter_cleanup_removes_stale() {
        let limiter = RateLimiterState::new();

        // Create some buckets by making requests
        assert!(
            limiter
                .check_rate_limit("t1", "/api/sync/push")
                .await
                .is_ok()
        );
        assert!(
            limiter
                .check_rate_limit("t2", "/api/sync/pull")
                .await
                .is_ok()
        );

        assert_eq!(limiter.bucket_count().await, 2);

        // Cleanup with zero max_age (forces removal)
        limiter.cleanup_stale_buckets(Duration::from_secs(0)).await;
        assert_eq!(limiter.bucket_count().await, 0);
    }

    #[tokio::test]
    async fn rate_limiter_cleanup_preserves_recent() {
        let limiter = RateLimiterState::new();

        assert!(
            limiter
                .check_rate_limit("t1", "/api/sync/push")
                .await
                .is_ok()
        );
        assert_eq!(limiter.bucket_count().await, 1);

        // Cleanup with 1 hour max_age (should NOT remove recent buckets)
        limiter
            .cleanup_stale_buckets(Duration::from_secs(3600))
            .await;
        assert_eq!(limiter.bucket_count().await, 1);
    }

    #[tokio::test]
    async fn rate_limiter_retry_after_header_value() {
        let limiter = RateLimiterState::new();

        // Exhaust tenant's push limit
        for _ in 0..100 {
            let _ = limiter
                .check_rate_limit("retry-tenant", "/api/sync/push")
                .await;
        }

        let result = limiter
            .check_rate_limit("retry-tenant", "/api/sync/push")
            .await;
        match result {
            Err(retry_after) => {
                // Should be a positive number (at least some fraction of a second)
                assert!(
                    retry_after > 0.0,
                    "retry-after should be positive, got {retry_after}"
                );
            }
            Ok(_) => panic!("should be rate-limited"),
        }
    }

    // ── Token-mint rate limiting (P8-3) ────────────────────────────

    #[tokio::test]
    async fn token_rate_limit_allows_then_blocks() {
        let limiter = RateLimiterState::new();
        for i in 0..30 {
            assert!(
                limiter.check_token_rate_limit("203.0.113.4").await.is_ok(),
                "mint {i} should be allowed"
            );
        }
        assert!(
            limiter.check_token_rate_limit("203.0.113.4").await.is_err(),
            "31st mint should be rate-limited"
        );
    }

    #[tokio::test]
    async fn token_rate_limit_isolates_ips() {
        let limiter = RateLimiterState::new();
        for _ in 0..30 {
            assert!(limiter.check_token_rate_limit("203.0.113.4").await.is_ok());
        }
        assert!(limiter.check_token_rate_limit("203.0.113.4").await.is_err());
        assert!(
            limiter.check_token_rate_limit("203.0.113.5").await.is_ok(),
            "a different IP has its own bucket"
        );
    }

    #[test]
    fn client_ip_prefers_x_forwarded_for() {
        let req = Request::builder()
            .uri("/api/v1/tokens")
            .header("x-forwarded-for", "203.0.113.9, 10.0.0.1")
            .header("x-real-ip", "198.51.100.7")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req), "203.0.113.9");
    }

    #[test]
    fn client_ip_falls_back_to_x_real_ip_then_unknown() {
        let req = Request::builder()
            .uri("/api/v1/tokens")
            .header("x-real-ip", "198.51.100.7")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(client_ip(&req), "198.51.100.7");

        let bare = Request::builder()
            .uri("/api/v1/tokens")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(client_ip(&bare), "unknown");
    }
}
