use super::*;
use std::path::PathBuf;

fn temp_image_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oz-local-api-{tag}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn resolve_port_defaults_and_validates() {
    let conn = oz_core::migrations::fresh_db();
    assert_eq!(resolve_port(&conn), DEFAULT_PORT);
    oz_core::Settings::set(&conn, SETTINGS_PORT, "8080").unwrap();
    assert_eq!(resolve_port(&conn), 8080);
    // Below the registered range → default.
    oz_core::Settings::set(&conn, SETTINGS_PORT, "80").unwrap();
    assert_eq!(resolve_port(&conn), DEFAULT_PORT);
    // Garbage → default.
    oz_core::Settings::set(&conn, SETTINGS_PORT, "not-a-port").unwrap();
    assert_eq!(resolve_port(&conn), DEFAULT_PORT);
}

#[test]
fn is_enabled_requires_explicit_one() {
    let conn = oz_core::migrations::fresh_db();
    assert!(!is_enabled(&conn), "default is off");
    oz_core::Settings::set(&conn, SETTINGS_ENABLED, "1").unwrap();
    assert!(is_enabled(&conn));
    oz_core::Settings::set(&conn, SETTINGS_ENABLED, "0").unwrap();
    assert!(!is_enabled(&conn));
}

#[test]
fn secret_is_generated_once_and_stable() {
    let conn = oz_core::migrations::fresh_db();
    let first = load_or_create_secret(&conn).unwrap();
    assert_eq!(first.len(), 64, "two simple UUIDs = 64 hex chars");
    assert!(first.bytes().all(|b| b.is_ascii_hexdigit()));
    let second = load_or_create_secret(&conn).unwrap();
    assert_eq!(first, second, "second load must not rotate the secret");
}

#[tokio::test]
async fn rotate_secret_replaces_persisted_value_and_invalidates_old_tokens() {
    let conn = oz_core::migrations::fresh_db();
    let original = load_or_create_secret(&conn).unwrap();
    let rotated = rotate_secret(&conn).unwrap();
    assert_ne!(original, rotated);
    assert_eq!(rotated.len(), 64);
    // Later loads see the rotated value — rotation does not re-fire.
    assert_eq!(load_or_create_secret(&conn).unwrap(), rotated);
    // A token minted under the old key no longer validates against the
    // new one (this is why the UI warns before rotating).
    let stale = mint_token(&original, "stale", Some(1)).unwrap();
    assert!(
        oz_api::auth::validate_token_with_secret(&stale.token, Some(&rotated))
            .await
            .is_err(),
        "rotated secret must invalidate previously minted tokens"
    );
    let fresh = mint_token(&rotated, "fresh", Some(1)).unwrap();
    assert!(
        oz_api::auth::validate_token_with_secret(&fresh.token, Some(&rotated))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn mint_token_roundtrip_and_clamp() {
    let secret = "a".repeat(32);
    let resp = mint_token(&secret, "my-script", Some(2)).unwrap();
    let claims = oz_api::auth::validate_token_with_secret(&resp.token, Some(&secret))
        .await
        .unwrap();
    assert_eq!(claims.sub, "my-script");

    // Out-of-range expiry is clamped, not rejected.
    let long = mint_token(&secret, "x", Some(999_999)).unwrap();
    let claims = oz_api::auth::validate_token_with_secret(&long.token, Some(&secret))
        .await
        .unwrap();
    let hours = (claims.exp as i64 - claims.iat as i64) / 3600;
    assert_eq!(hours, MAX_TOKEN_HOURS);

    // Blank label falls back.
    let blank = mint_token(&secret, "   ", None).unwrap();
    let claims = oz_api::auth::validate_token_with_secret(&blank.token, Some(&secret))
        .await
        .unwrap();
    assert_eq!(claims.sub, "local-script");
}

#[tokio::test]
async fn minted_token_rejected_under_wrong_secret() {
    let secret = "b".repeat(32);
    let resp = mint_token(&secret, "t", Some(1)).unwrap();
    assert!(
        oz_api::auth::validate_token_with_secret(&resp.token, Some(&"c".repeat(32)))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn server_serves_health_protected_routes_and_stops() {
    let dir = temp_image_dir("serve");
    let db = Arc::new(Mutex::new(oz_core::migrations::fresh_db()));
    let secret = "d".repeat(32);
    let handle = start(
        db.clone(),
        PathBuf::from(":memory:"),
        dir.clone(),
        secret.clone(),
        0, // OS-assigned port
    )
    .await
    .unwrap();
    let base = format!("http://127.0.0.1:{}", handle.port);

    // Health is public.
    let resp = reqwest::get(format!("{base}/api/v1/health")).await.unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    // Self-documenting OpenAPI: the shared base surface, all scope
    // "both", cloud-only paths absent.
    let resp = reqwest::get(format!("{base}/api/openapi.json"))
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    // MED-4: the route is registered inside router()'s layer scope, so
    // it carries the same security headers as every other endpoint.
    assert_eq!(
        resp.headers()
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok())
            .as_deref(),
        Some("nosniff"),
        "local docs route must sit inside the security-headers layer"
    );
    let spec: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(spec["info"]["title"], "OZ-POS Local Terminal API");
    assert_eq!(
        spec["servers"][0]["url"].as_str(),
        Some(format!("http://127.0.0.1:{}", handle.port).as_str())
    );
    assert!(
        spec["paths"]["/api/v1/products"].is_object(),
        "shared surface present"
    );
    assert!(
        spec["paths"]["/api/sync/push"].is_null(),
        "cloud-only path leaked into local spec"
    );
    assert!(
        spec["paths"]["/api/docs"].is_null(),
        "docs UI leaked into local spec"
    );
    for (path, item) in spec["paths"].as_object().unwrap() {
        for (verb, op) in item.as_object().unwrap() {
            if matches!(verb.as_str(), "get" | "post" | "put" | "patch" | "delete") {
                assert_eq!(
                    op["x-oz-scope"].as_str(),
                    Some("both"),
                    "{verb} {path} in the local spec must be scope both"
                );
            }
        }
    }

    // Protected route without a token → 401.
    let resp = reqwest::get(format!("{base}/api/v1/products"))
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    // Protected route with a UI-minted token → 200.
    let token = mint_token(&secret, "test-script", Some(1)).unwrap().token;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/api/v1/products"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    // A token forged with the known dev constant is rejected — the
    // per-install secret is the whole point of the stateful middleware.
    let forged = oz_api::auth::create_token("forged", Some(1), None, None)
        .unwrap()
        .token;
    let resp = client
        .get(format!("{base}/api/v1/products"))
        .header("Authorization", format!("Bearer {forged}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    // Token minting over HTTP requires X-Admin-Key (= the secret).
    let mint = reqwest::Client::new()
        .post(format!("{base}/api/v1/tokens"))
        .json(&serde_json::json!({"label": "no-key"}))
        .send()
        .await
        .unwrap();
    assert_eq!(mint.status(), reqwest::StatusCode::UNAUTHORIZED);
    let mint = reqwest::Client::new()
        .post(format!("{base}/api/v1/tokens"))
        .header("X-Admin-Key", &secret)
        .json(&serde_json::json!({"label": "with-key", "expiry_hours": 1}))
        .send()
        .await
        .unwrap();
    assert_eq!(mint.status(), reqwest::StatusCode::OK);

    // Stop → the listener socket dies with the task.
    handle.stop();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        reqwest::get(format!("{base}/api/v1/health")).await.is_err(),
        "connection must fail after stop"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn start_reports_port_conflict_as_error() {
    let dir = temp_image_dir("conflict");
    let db = Arc::new(Mutex::new(oz_core::migrations::fresh_db()));
    let first = start(
        db.clone(),
        PathBuf::from(":memory:"),
        dir.clone(),
        "e".repeat(32),
        0,
    )
    .await
    .unwrap();
    // Same explicit port twice → the second bind must fail cleanly.
    let taken = first.port;
    let dup = start(
        db,
        PathBuf::from(":memory:"),
        dir.clone(),
        "e".repeat(32),
        taken,
    )
    .await;
    assert!(
        dup.is_err(),
        "re-binding a taken port must error, not panic"
    );
    first.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

// ── Primary-store targeting (review HIGH-1 regression) ─────────────

#[test]
fn primary_store_id_resolves_and_falls_back() {
    let global = oz_core::migrations::fresh_db();
    // Migration 025 seeds 'default' with is_primary = 0 → fallback.
    assert_eq!(primary_store_id(&global), "default");
    global
        .execute(
            "UPDATE store_profiles SET is_primary = 1 WHERE id = 'default'",
            [],
        )
        .unwrap();
    assert_eq!(primary_store_id(&global), "default");
    // A flagged non-default store wins.
    global
        .execute(
            "INSERT INTO store_profiles (id, name, address, tax_id, currency, timezone, is_primary, created_at, updated_at)
             VALUES ('store-b', 'B', '', '', 'USD', 'UTC', 0, 'x', 'x')",
            [],
        )
        .unwrap();
    global
        .execute(
            "UPDATE store_profiles SET is_primary = 0 WHERE id = 'default'",
            [],
        )
        .unwrap();
    global
        .execute(
            "UPDATE store_profiles SET is_primary = 1 WHERE id = 'store-b'",
            [],
        )
        .unwrap();
    assert_eq!(primary_store_id(&global), "store-b");
}

#[tokio::test]
async fn serves_the_primary_store_database_not_the_global_one() {
    let tmp = std::env::temp_dir().join(format!("oz-local-api-store-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp).unwrap();
    let global = oz_core::migrations::fresh_db();
    global
        .execute(
            "UPDATE store_profiles SET is_primary = 1 WHERE id = 'default'",
            [],
        )
        .unwrap();
    let manager = platform_core::StoreDatabaseManager::new(tmp.clone(), oz_core::migrations::ALL);

    let store_id = primary_store_id(&global);
    let (api_db, api_path) = open_api_store_connection(&manager, &store_id).unwrap();
    assert_eq!(api_path, tmp.join("store-default.sqlite"));

    let secret = "c".repeat(32);
    let handle = start(api_db, api_path, tmp.join("images"), secret.clone(), 0)
        .await
        .unwrap();
    let base = format!("http://127.0.0.1:{}", handle.port);

    // Create a product through the API (JWT + operator admin key).
    let token = mint_token(&secret, "t", Some(1)).unwrap().token;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/v1/products"))
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Admin-Key", &secret)
        .json(&serde_json::json!({
            "sku": "API-001", "name": "From API",
            "price": {"minor_units": 100, "currency": "USD"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);

    // The write must be visible through the MANAGER's connection — the
    // exact path every scoped Tauri command uses (resolve_scope).
    let ui_conn = manager.open_store("default").unwrap();
    let in_store: i64 = ui_conn
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM products WHERE sku = 'API-001'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        in_store, 1,
        "API write must land in the store DB the UI reads"
    );

    // …and must NOT have touched the global identity DB.
    let in_global: i64 = global
        .query_row(
            "SELECT COUNT(*) FROM products WHERE sku = 'API-001'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(in_global, 0, "API must not write to the global DB");

    let stopped_port = handle.port;
    handle.stop_async().await;
    // stop_async guarantees the listener is gone: an immediate re-bind
    // of the same port must not race OS socket teardown (review MED-3).
    let again = start(
        Arc::new(Mutex::new(oz_core::migrations::fresh_db())),
        PathBuf::from(":memory:"),
        tmp.join("images2"),
        secret,
        stopped_port,
    )
    .await;
    assert!(
        again.is_ok(),
        "re-bind of the just-stopped port must succeed: {:?}",
        again.err()
    );
    if let Ok(h) = again {
        h.stop_async().await;
    }
    let _ = std::fs::remove_dir_all(&tmp);
}
