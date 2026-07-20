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
        // Find matching rate limit config
        let config = RATE_LIMITS
            .iter()
            .find(|(prefix, _)| path.starts_with(prefix))
            .map(|(_, config)| config);

        let config = match config {
            Some(c) => c,
            None => return Ok(()), // No rate limit configured for this path
        };

        let key = format!(
            "{tenant_id}|{}",
            RATE_LIMITS
                .iter()
                .find(|(prefix, _)| path.starts_with(prefix))
                .map(|(prefix, _)| *prefix)
                .unwrap_or(path)
        );

        let mut buckets = self.buckets.write().await;
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(config.capacity, config.refill_per_sec));

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
}
