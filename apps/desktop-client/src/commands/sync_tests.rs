use super::*;
use tauri::Manager as _;

#[test]
fn sync_settings_serialize() {
    let s = SyncSettingsDto {
        server_url: Some("https://sync.example.com".into()),
        has_api_key: true,
        enabled: true,
    };
    let json = serde_json::to_value(&s).unwrap();
    assert_eq!(json["serverUrl"], "https://sync.example.com");
    assert_eq!(json["hasApiKey"], true);
    assert_eq!(json["enabled"], true);
}

#[test]
fn sync_settings_no_url_disabled() {
    let s = SyncSettingsDto {
        server_url: None,
        has_api_key: false,
        enabled: false,
    };
    let json = serde_json::to_value(&s).unwrap();
    assert!(json["serverUrl"].is_null());
    assert_eq!(json["hasApiKey"], false);
    assert_eq!(json["enabled"], false);
}
#[cfg(debug_assertions)]
#[test]
fn sync_probe_never_uses_local_dev_server_while_cloud_testing() {
    // TEMPORARILY DISABLED (2026-08-16): the local-Docker fallback is
    // commented out while testing against the deployed cloud server, so
    // an unconfigured sync stays unconfigured — the probe resolves None
    // regardless of the allow_local_fallback flag.
    let resolved = resolve_sync_probe_url(None, None, true);
    assert_eq!(resolved.as_deref(), None);
    assert_eq!(
        resolve_sync_probe_url(None, Some(String::new()), true).as_deref(),
        None
    );
    assert_eq!(resolve_sync_probe_url(None, None, false), None);
}

#[test]
fn update_sync_settings_deserialize() {
    let json = r#"{"serverUrl":"https://sync.example.com","apiKey":"sk-abc123","enabled":true}"#;
    let args: UpdateSyncSettingsArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.server_url.unwrap(), "https://sync.example.com");
    assert_eq!(args.api_key.unwrap(), "sk-abc123");
    assert!(args.enabled);
}

#[test]
fn update_sync_settings_deserialize_no_key() {
    let json = r#"{"serverUrl":null,"apiKey":null,"enabled":false}"#;
    let args: UpdateSyncSettingsArgs = serde_json::from_str(json).unwrap();
    assert!(args.server_url.is_none());
    assert!(args.api_key.is_none());
    assert!(!args.enabled);
}

#[test]
fn update_sync_settings_data_clear_url_writes_empty_row() {
    // The UI sends server_url: None when the user clears the field.
    // The command must write an empty row (Some("")) rather than
    // leaving the stale URL (which would keep auto-provision from ever
    // repairing a broken URL) or deleting the row (which would make a
    // cleared + disabled install look like a fresh one and re-trigger
    // provisioning on the next debug launch). THIS app is where the
    // should_auto_provision discriminator runs, so the pin belongs
    // here, not just on the tablet twin.
    let conn = oz_core::migrations::fresh_db();
    Settings::set_sync_server_url(&conn, "https://sync.example.com").unwrap();
    Settings::set_sync_enabled(&conn, false).unwrap();

    let args = UpdateSyncSettingsArgs {
        server_url: None,
        api_key: None,
        enabled: false,
    };
    update_sync_settings_data(&conn, &args).unwrap();

    assert_eq!(
        Settings::get_sync_server_url(&conn).unwrap(),
        Some("".into())
    );
}

#[test]
fn update_sync_settings_debug() {
    let args = UpdateSyncSettingsArgs {
        server_url: Some("https://sync.example.com".into()),
        api_key: None,
        enabled: true,
    };
    let debug = format!("{args:?}");
    assert!(debug.contains("sync.example.com"));
    assert!(debug.contains("true"));
}

#[test]
fn sync_pull_args_deserialize() {
    let json = r#"{"confirmDestructive":true}"#;
    let args: SyncPullArgs = serde_json::from_str(json).unwrap();
    assert!(args.confirm_destructive);
}

#[test]
fn sync_pull_args_deserialize_false() {
    let json = r#"{"confirmDestructive":false}"#;
    let args: SyncPullArgs = serde_json::from_str(json).unwrap();
    assert!(!args.confirm_destructive);
}

#[test]
fn sync_pull_args_missing_consent_fails() {
    // SYNC-03: a payload with no consent key must not silently
    // default to true — serde errors on the missing field.
    let result = serde_json::from_str::<SyncPullArgs>(r#"{}"#);
    assert!(
        result.is_err(),
        "missing confirm_destructive must fail deserialization"
    );
}

#[test]
fn validate_pull_consent_accepts_true() {
    let args = SyncPullArgs {
        confirm_destructive: true,
    };
    assert!(validate_pull_consent(&args).is_ok());
}

#[test]
fn validate_pull_consent_rejects_false() {
    let args = SyncPullArgs {
        confirm_destructive: false,
    };
    let err = validate_pull_consent(&args).unwrap_err();
    assert!(err.to_string().contains("confirm_destructive"));
}

#[test]
fn pull_result_serialize_no_error() {
    let r = PullResult {
        products_pulled: 10,
        tax_rates_pulled: 2,
        users_pulled: 3,
        error: None,
    };
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["products_pulled"], 10);
    assert_eq!(json["tax_rates_pulled"], 2);
    assert_eq!(json["users_pulled"], 3);
    assert!(json["error"].is_null());
}

#[test]
fn pull_result_serialize_with_error() {
    let r = PullResult {
        products_pulled: 0,
        tax_rates_pulled: 0,
        users_pulled: 0,
        error: Some("network unreachable".into()),
    };
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["products_pulled"], 0);
    assert_eq!(json["error"], "network unreachable");
}

#[test]
fn pull_result_deserialize() {
    let json = r#"{"products_pulled":5,"tax_rates_pulled":1,"users_pulled":2,"error":null}"#;
    let r: PullResult = serde_json::from_str(json).unwrap();
    assert_eq!(r.products_pulled, 5);
    assert_eq!(r.tax_rates_pulled, 1);
    assert_eq!(r.users_pulled, 2);
    assert!(r.error.is_none());
}

#[tokio::test]
async fn sync_run_uses_persisted_settings_and_reports_empty_queue_success() {
    // Phase 4 bootstrap contract: once the Tauri settings database has
    // the URL, API key, and enabled flag written by auto-provisioning,
    // the real command must read that persisted state and return an
    // explicit successful result when there is nothing to push.
    let conn = oz_core::migrations::fresh_db();
    update_sync_settings_data(
        &conn,
        &UpdateSyncSettingsArgs {
            server_url: Some("http://localhost:3099".into()),
            api_key: Some("test-jwt".into()),
            enabled: true,
        },
    )
    .unwrap();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = sync_run(app.state()).await.unwrap();

    assert_eq!(result.synced, 0);
    assert_eq!(result.failed, 0);
    assert!(result.error.is_none());
}

async fn spawn_push_test_server() -> (
    String,
    Arc<tokio::sync::Mutex<Option<String>>>,
    tokio::task::JoinHandle<()>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let captured = Arc::new(tokio::sync::Mutex::new(None));
    let captured_by_server = captured.clone();
    let task = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut buffer = vec![0_u8; 16 * 1024];
        let bytes_read = socket.read(&mut buffer).await.unwrap_or(0);
        *captured_by_server.lock().await =
            Some(String::from_utf8_lossy(&buffer[..bytes_read]).into_owned());
        let body = r#"{"results":[{"outcome":"accepted"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = socket.write_all(response.as_bytes()).await;
    });
    (url, captured, task)
}

#[tokio::test]
async fn sync_run_enqueues_one_item_and_observes_server_acceptance() {
    // Full isolated harness: a temporary AppState owns the queue, the
    // real command performs the HTTP push, and the test server captures
    // the authenticated request and returns an accepted outcome.
    let (server_url, captured, server_task) = spawn_push_test_server().await;
    let conn = oz_core::migrations::fresh_db();
    update_sync_settings_data(
        &conn,
        &UpdateSyncSettingsArgs {
            server_url: Some(server_url),
            api_key: Some("test-jwt".into()),
            enabled: true,
        },
    )
    .unwrap();
    {
        let store = Store::new(&conn);
        store
            .enqueue_offline("phase4.e2e", r#"{"probe":true}"#)
            .unwrap();
    }
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = sync_run(app.state()).await.unwrap();
    server_task.await.unwrap();

    assert_eq!(result.synced, 1);
    assert_eq!(result.failed, 0);
    assert!(result.error.is_none());
    let request = captured.lock().await.clone().unwrap();
    assert!(request.starts_with("POST /api/sync/push HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-jwt"),
        "request did not carry the configured bearer token: {request}"
    );

    let state = app.state::<AppState>();
    let db = state.db.lock().await;
    let items = Store::new(&db).list_all_offline().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].status,
        oz_core::offline::OfflineQueueStatus::Synced
    );
}

#[tokio::test]
async fn sync_run_plan_required_keeps_items_pending_and_flags_upgrade() {
    // ADR sync-plan-gating: when the server rejects with a structured
    // 403 plan_required, sync_run must (a) report plan_required so the
    // UI shows an upgrade prompt, (b) NOT mark items failed — they stay
    // pending and sync automatically after an upgrade.
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut buffer = vec![0_u8; 16 * 1024];
        let _ = socket.read(&mut buffer).await;
        let body = r#"{"error":"plan_required"}"#;
        let response = format!(
            "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = socket.write_all(response.as_bytes()).await;
    });

    let conn = oz_core::migrations::fresh_db();
    update_sync_settings_data(
        &conn,
        &UpdateSyncSettingsArgs {
            server_url: Some(server_url),
            api_key: Some("test-jwt".into()),
            enabled: true,
        },
    )
    .unwrap();
    {
        let store = Store::new(&conn);
        store
            .enqueue_offline("complete_sale", r#"{"id":"plan-gate"}"#)
            .unwrap();
    }
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = sync_run(app.state()).await.unwrap();
    task.await.unwrap();

    assert!(result.plan_required, "must flag plan_required for the UI");
    assert_eq!(result.synced, 0);
    assert_eq!(result.failed, 0, "a plan gate is not a failure");

    let state = app.state::<AppState>();
    let db = state.db.lock().await;
    let items = Store::new(&db).list_all_offline().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].status,
        oz_core::offline::OfflineQueueStatus::Pending,
        "plan-gated items must stay pending so they sync after upgrade"
    );
}

#[tokio::test]
async fn sync_run_refreshes_token_and_retries_once_after_401() {
    // ADR sync-auth-hardening P1: when the server rejects the stored
    // token with 401, the command must mint a fresh token, persist it,
    // and retry the push exactly once — no operator action, no loop.
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_url = format!("http://{}", listener.local_addr().unwrap());
    let retry_auth: Arc<tokio::sync::Mutex<Option<String>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    let retry_auth_server = retry_auth.clone();
    let task = tokio::spawn(async move {
        let mut auth_of_retry: Option<String> = None;
        for attempt in 0..3 {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let mut buffer = vec![0_u8; 16 * 1024];
            let n = socket.read(&mut buffer).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
            let path = request
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or_default();
            let response = if path == "/api/sync/push" && attempt == 0 {
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string()
            } else if path == "/api/v1/tokens" {
                let body = r#"{"token":{"token":"fresh-jwt-456","expires_at":"2026-08-10T00:00:00Z","token_id":"uuid-1"}}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
            } else {
                auth_of_retry = request
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("authorization:"))
                    .map(|l| l.to_string());
                let body = r#"{"results":[{"outcome":"accepted"}]}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
            };
            let _ = socket.write_all(response.as_bytes()).await;
        }
        *retry_auth_server.lock().await = auth_of_retry;
    });

    let conn = oz_core::migrations::fresh_db();
    update_sync_settings_data(
        &conn,
        &UpdateSyncSettingsArgs {
            server_url: Some(server_url),
            api_key: Some("stale-jwt".into()),
            enabled: true,
        },
    )
    .unwrap();
    {
        let store = Store::new(&conn);
        store
            .enqueue_offline("phase1.refresh", r#"{"probe":true}"#)
            .unwrap();
    }
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let result = sync_run(app.state()).await.unwrap();
    task.await.unwrap();

    assert_eq!(result.synced, 1);
    assert_eq!(result.failed, 0);
    assert!(result.error.is_none());

    // The retried push must carry the freshly minted token.
    let auth = retry_auth.lock().await.clone().unwrap_or_default();
    assert!(
        auth.to_ascii_lowercase().contains("bearer fresh-jwt-456"),
        "retried push did not carry the fresh token: {auth}"
    );

    // The refreshed key was persisted and the item reached synced.
    let state = app.state::<AppState>();
    let db = state.db.lock().await;
    assert_eq!(
        Settings::get_sync_api_key(&db).unwrap().as_deref(),
        Some("fresh-jwt-456")
    );
    let items = Store::new(&db).list_all_offline().unwrap();
    assert_eq!(
        items[0].status,
        oz_core::offline::OfflineQueueStatus::Synced
    );
}

#[tokio::test]
async fn request_token_sends_admin_key_header_when_provided() {
    // ADR sync-auth-hardening P2: a gated server (OZ_ADMIN_KEY set)
    // only mints tokens when the request carries the matching
    // X-Admin-Key header. Pin the wire contract here.
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_url = format!("http://{}", listener.local_addr().unwrap());
    let captured: Arc<tokio::sync::Mutex<Option<String>>> = Arc::new(tokio::sync::Mutex::new(None));
    let captured_server = captured.clone();
    let task = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut buffer = vec![0_u8; 16 * 1024];
        let n = socket.read(&mut buffer).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
        *captured_server.lock().await = Some(request);
        let body = r#"{"token":{"token":"jwt-1","expires_at":null,"token_id":"u1"}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = socket.write_all(response.as_bytes()).await;
    });

    let result = sync_client::request_token(&server_url, Some("sekret")).await;
    task.await.unwrap();

    assert!(result.ok, "token request failed: {}", result.status);
    let request = captured.lock().await.clone().unwrap();
    assert!(
        request.to_ascii_lowercase().contains("x-admin-key: sekret"),
        "token request did not carry the admin key: {request}"
    );
}

// ── PostgreSQL sync settings & daemon commands ────────────────

#[test]
fn pg_sync_settings_dto_serialize_camel_case() {
    let dto = PgSyncSettingsDto {
        enabled: true,
        host: Some("db.example.com".into()),
        port: Some("5432".into()),
        dbname: Some("oz_sync".into()),
        user: Some("sync_user".into()),
        has_password: true,
        require_tls: true,
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["enabled"], true);
    assert_eq!(json["host"], "db.example.com");
    assert_eq!(json["port"], "5432");
    assert_eq!(json["dbname"], "oz_sync");
    assert_eq!(json["user"], "sync_user");
    assert_eq!(json["hasPassword"], true);
    assert_eq!(json["requireTls"], true);
}

#[test]
fn update_pg_sync_settings_args_deserialize() {
    let json = r#"{"enabled":true,"host":"db.example.com","port":"5432","dbname":"oz_sync","user":"sync_user","password":"secret","requireTls":true}"#;
    let args: UpdatePgSyncSettingsArgs = serde_json::from_str(json).unwrap();
    assert!(args.enabled);
    assert_eq!(args.host.as_deref(), Some("db.example.com"));
    assert_eq!(args.port.as_deref(), Some("5432"));
    assert_eq!(args.dbname.as_deref(), Some("oz_sync"));
    assert_eq!(args.user.as_deref(), Some("sync_user"));
    assert_eq!(args.password.as_deref(), Some("secret"));
    assert_eq!(args.require_tls, Some(true));
}

#[test]
fn update_pg_sync_settings_data_roundtrip() {
    let conn = oz_core::migrations::fresh_db();
    let args = UpdatePgSyncSettingsArgs {
        enabled: true,
        host: Some("db.example.com".into()),
        port: Some("5433".into()),
        dbname: Some("oz_sync".into()),
        user: Some("sync_user".into()),
        password: Some("secret".into()),
        require_tls: Some(true),
    };
    update_pg_sync_settings_data(&conn, &args).unwrap();

    let dto = run_get_pg_sync_settings(&conn).unwrap();
    assert!(dto.enabled);
    assert_eq!(dto.host.as_deref(), Some("db.example.com"));
    assert_eq!(dto.port.as_deref(), Some("5433"));
    assert_eq!(dto.dbname.as_deref(), Some("oz_sync"));
    assert_eq!(dto.user.as_deref(), Some("sync_user"));
    assert!(dto.has_password);
    assert!(dto.require_tls);
}

#[test]
fn update_pg_sync_settings_data_disabled_default() {
    let conn = oz_core::migrations::fresh_db();
    let dto = run_get_pg_sync_settings(&conn).unwrap();
    assert!(!dto.enabled);
    assert!(dto.host.is_none());
    assert!(dto.port.is_none());
    assert!(dto.dbname.is_none());
    assert!(dto.user.is_none());
    assert!(!dto.has_password);
    // TLS defaults to off, matching the historical NoTls transport.
    assert!(!dto.require_tls);
}

#[test]
fn update_pg_sync_settings_data_none_clears_optional_fields() {
    let conn = oz_core::migrations::fresh_db();
    update_pg_sync_settings_data(
        &conn,
        &UpdatePgSyncSettingsArgs {
            enabled: true,
            host: Some("db.example.com".into()),
            port: Some("5432".into()),
            dbname: Some("oz_sync".into()),
            user: Some("sync_user".into()),
            password: None,
            require_tls: Some(true),
        },
    )
    .unwrap();
    // A later save with None clears the connection fields (same
    // contract as the HTTP sync URL handling).
    update_pg_sync_settings_data(
        &conn,
        &UpdatePgSyncSettingsArgs {
            enabled: false,
            host: None,
            port: None,
            dbname: None,
            user: None,
            password: None,
            require_tls: None,
        },
    )
    .unwrap();

    let dto = run_get_pg_sync_settings(&conn).unwrap();
    assert!(!dto.enabled);
    assert!(dto.host.is_none());
    assert!(dto.port.is_none());
    assert!(dto.dbname.is_none());
    assert!(dto.user.is_none());
    // require_tls is written on every update; the second save's None
    // defaults to false.
    assert!(!dto.require_tls);
}

#[test]
fn update_pg_sync_settings_data_password_preserved_when_none() {
    let conn = oz_core::migrations::fresh_db();
    update_pg_sync_settings_data(
        &conn,
        &UpdatePgSyncSettingsArgs {
            enabled: true,
            host: None,
            port: None,
            dbname: None,
            user: None,
            password: Some("secret".into()),
            require_tls: None,
        },
    )
    .unwrap();
    // A later save without a password must keep the stored secret —
    // the UI sends None for the untouched masked field, mirroring the
    // HTTP sync API-key handling.
    update_pg_sync_settings_data(
        &conn,
        &UpdatePgSyncSettingsArgs {
            enabled: true,
            host: Some("db.example.com".into()),
            port: None,
            dbname: None,
            user: None,
            password: None,
            require_tls: Some(true),
        },
    )
    .unwrap();

    let dto = run_get_pg_sync_settings(&conn).unwrap();
    assert!(dto.has_password);
    assert!(dto.require_tls);
}

#[tokio::test]
async fn pg_sync_settings_command_roundtrip() {
    let conn = oz_core::migrations::fresh_db();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    update_pg_sync_settings(
        UpdatePgSyncSettingsArgs {
            enabled: true,
            host: Some("db.example.com".into()),
            port: None,
            dbname: Some("oz_sync".into()),
            user: None,
            password: Some("secret".into()),
            require_tls: Some(true),
        },
        app.state(),
    )
    .await
    .unwrap();

    let dto = get_pg_sync_settings(app.state()).await.unwrap();
    assert!(dto.enabled);
    assert_eq!(dto.host.as_deref(), Some("db.example.com"));
    assert_eq!(dto.dbname.as_deref(), Some("oz_sync"));
    assert!(dto.has_password);
    assert!(dto.require_tls);
}

#[tokio::test]
async fn pg_sync_status_returns_default_on_fresh_state() {
    let conn = oz_core::migrations::fresh_db();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    let status = pg_sync_status(app.state()).await.unwrap();
    assert!(!status.running);
    assert_eq!(status.last_pushed, 0);
    assert_eq!(status.last_pulled, 0);
    assert_eq!(status.pending_count, 0);
    assert!(status.last_error.is_none());
}

#[tokio::test]
async fn pg_sync_stop_on_stopped_daemon_is_noop() {
    let conn = oz_core::migrations::fresh_db();
    let app = tauri::test::mock_builder()
        .manage(AppState::for_test_with_conn(conn))
        .build(tauri::generate_context!())
        .unwrap();

    // Stopping a daemon that was never started must succeed quietly.
    pg_sync_stop(app.state()).await.unwrap();
    let status = pg_sync_status(app.state()).await.unwrap();
    assert!(!status.running);
}
