//! Tests for `redis_backend.rs` — key derivation, Lua source, and the
//! fallback contract (connection refusal → `Ok(None)` → in-process).
//!
//! Integration tests that require a live Redis are gated behind the
//! `OZ_TEST_REDIS_URL` environment variable (mirroring the PG integration
//! tests' `OZ_TEST_PG_URL` pattern): they skip when Redis is unavailable.

use super::*;

use std::time::Duration;

// ── Pure helpers ─────────────────────────────────────────────────────

#[test]
fn snapshot_keys_are_namespaced_per_tenant() {
    assert_eq!(snapshot_version_key("t1"), "oz:snapshot:t1:ver");
    assert_eq!(snapshot_data_key("t1"), "oz:snapshot:t1:data");
    // Tenants must not collide.
    assert_ne!(snapshot_version_key("t1"), snapshot_version_key("t2"));
    assert_ne!(snapshot_data_key("t1"), snapshot_data_key("t2"));
    // Version and data keys for the same tenant are distinct.
    assert_ne!(snapshot_version_key("t1"), snapshot_data_key("t1"));
}

#[test]
fn lua_source_is_non_empty_and_contains_key_operations() {
    let src = token_bucket_lua_source();
    assert!(!src.is_empty(), "Lua source must not be empty");
    // The script must use Redis Hash ops and EXPIRE.
    assert!(src.contains("HGET"), "script must read the bucket hash");
    assert!(src.contains("HSET"), "script must write the bucket hash");
    assert!(src.contains("EXPIRE"), "script must set a TTL");
    assert!(
        src.contains("math.min"),
        "script must cap refill at capacity"
    );
    // Both outcome shapes are returned.
    assert!(src.contains("{1, 0}"), "allowed outcome must be {{1, 0}}");
    assert!(
        src.contains("{0, retry_after}"),
        "denied outcome must include retry_after"
    );
}

#[test]
fn rate_limit_key_is_namespaced() {
    assert_eq!(RedisBackend::rate_limit_key("a|b"), "oz:rate_limit:a|b");
}

// ── Connection fallback contract ────────────────────────────────────

#[tokio::test]
async fn connect_rejects_malformed_url() {
    // A URL that cannot even be parsed is a configuration error → Err,
    // NOT a silent fallback.
    let result = RedisBackend::connect("not a url").await;
    assert!(result.is_err(), "malformed URL must surface as an error");
}

#[tokio::test]
#[ignore = "slow connection timeout without Redis"]
async fn connect_returns_none_when_refused() {
    // Point at a port that is (almost certainly) not listening. The
    // connection manager will retry, so this can take a moment — the
    // contract is `Ok(None)` (fall back to in-process), never a panic.
    let url = "redis://127.0.0.1:1";
    match RedisBackend::connect(url).await {
        Ok(None) => {} // expected fallback path
        Ok(Some(_)) => {
            // Something is actually listening on port 1 — treat as success.
        }
        Err(e) => panic!("refused connection must not return Err: {e}"),
    }
}

// ── Integration (gated on OZ_TEST_REDIS_URL) ────────────────────────

async fn test_backend() -> Option<RedisBackend> {
    let Ok(url) = std::env::var("OZ_TEST_REDIS_URL") else {
        return None;
    };
    match RedisBackend::connect(&url).await {
        Ok(Some(b)) => Some(b),
        Ok(None) => {
            eprintln!("Redis integration test skipped: no server at {url}");
            None
        }
        Err(e) => {
            eprintln!("Redis integration test skipped: {e}");
            None
        }
    }
}

#[tokio::test]
#[ignore = "Redis integration tests disabled in dev CI"]
async fn integration_snapshot_roundtrip() {
    let Some(backend) = test_backend().await else {
        return;
    };
    let tenant = format!("t-{}", uuid::Uuid::now_v7());
    // Use a key-scoped DB index so we don't collide with real data.

    // Miss before store.
    assert_eq!(backend.snapshot_get(&tenant).await.unwrap(), None);

    // Store + hit.
    backend
        .snapshot_set(&tenant, "v7", b"{\"products\":[]}", 30)
        .await
        .unwrap();
    let (version, bytes) = backend.snapshot_get(&tenant).await.unwrap().unwrap();
    assert_eq!(version, "v7");
    assert_eq!(bytes, b"{\"products\":[]}");

    // Overwrite.
    backend
        .snapshot_set(&tenant, "v8", b"{\"products\":[1]}", 30)
        .await
        .unwrap();
    let (version, bytes) = backend.snapshot_get(&tenant).await.unwrap().unwrap();
    assert_eq!(version, "v8");
    assert_eq!(bytes, b"{\"products\":[1]}");
}

#[tokio::test]
#[ignore = "Redis integration tests disabled in dev CI"]
async fn integration_rate_limit_allows_then_denies() {
    let Some(backend) = test_backend().await else {
        return;
    };
    let key = format!("it-{}", uuid::Uuid::now_v7());
    // Capacity 2, refill 2/sec — the burst of 2 is allowed, the third is not.
    assert_eq!(backend.check_rate_limit(&key, 2, 2.0).await.unwrap(), None);
    assert_eq!(backend.check_rate_limit(&key, 2, 2.0).await.unwrap(), None);
    let denied = backend.check_rate_limit(&key, 2, 2.0).await.unwrap();
    assert!(denied.is_some(), "third consume must be denied");
}

#[tokio::test]
#[ignore = "Redis integration tests disabled in dev CI"]
async fn integration_snapshot_ttl_expires() {
    let Some(backend) = test_backend().await else {
        return;
    };
    let tenant = format!("ttl-{}", uuid::Uuid::now_v7());
    backend.snapshot_set(&tenant, "v1", b"x", 1).await.unwrap();
    assert_eq!(
        backend.snapshot_get(&tenant).await.unwrap().unwrap().0,
        "v1"
    );
    // Wait past the 1s TTL.
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(
        backend.snapshot_get(&tenant).await.unwrap(),
        None,
        "entry must expire after TTL"
    );
}
