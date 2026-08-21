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

/// Sharding (SOTA finding E): concurrent requests across many tenants
/// must not corrupt buckets or deadlock, and each tenant keeps its own
/// limit even under parallel load. This exercises the per-shard locks —
/// a single shared `RwLock` would serialize but still pass; the point is
/// the sharded structure stays correct under contention.
#[tokio::test]
async fn rate_limiter_sharded_concurrent_tenants_stay_isolated() {
    let limiter = RateLimiterState::new();

    // 64 tenants × 100 concurrent pushes each (limit 100/min) — all must
    // be allowed; the 101st from any tenant must be rejected.
    let mut handles = Vec::new();
    for t in 0..64 {
        let limiter = limiter.clone();
        handles.push(tokio::spawn(async move {
            let tenant = format!("shard-tenant-{t}");
            for i in 0..100 {
                let result = limiter.check_rate_limit(&tenant, "/api/sync/push").await;
                assert!(
                    result.is_ok(),
                    "tenant {tenant} request {i} should be allowed"
                );
            }
            let result = limiter.check_rate_limit(&tenant, "/api/sync/push").await;
            assert!(
                result.is_err(),
                "tenant {tenant} 101st request should be rate-limited"
            );
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }

    // 64 tenants × 1 endpoint bucket each.
    assert_eq!(limiter.bucket_count().await, 64);
}

/// Two concurrent requests for the SAME bucket must not double-consume:
/// exactly 100 of 101 parallel attempts succeed for a 100-token bucket.
#[tokio::test]
async fn rate_limiter_sharded_same_bucket_parallel_consumption() {
    let limiter = RateLimiterState::new();

    let mut handles = Vec::new();
    for _ in 0..101 {
        let limiter = limiter.clone();
        handles.push(tokio::spawn(async move {
            limiter
                .check_rate_limit("single-tenant", "/api/sync/push")
                .await
                .is_ok()
        }));
    }
    let results: Vec<bool> = {
        let mut acc = Vec::with_capacity(handles.len());
        for handle in handles {
            acc.push(handle.await.unwrap());
        }
        acc
    };
    let allowed = results.iter().filter(|ok| **ok).count();
    assert_eq!(allowed, 100, "exactly 100 of 101 parallel attempts allowed");
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
