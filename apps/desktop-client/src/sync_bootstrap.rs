//! Debug-only bootstrap that auto-connects the desktop client to the
//! local dev sync server.
//!
//! `scripts/start-local-sync.bat` runs the cloud server in Docker on
//! `http://localhost:3099`, but a fresh app DB ships with an empty
//! `sync_server_url` and sync disabled — so the background sync daemon
//! silently no-ops (`SyncConfig::from_settings` returns `None`) until the
//! user manually configures Settings → Sync. This module closes that gap
//! for local development: on debug builds, if no server URL is configured
//! yet and the local server answers a health probe, we request a JWT and
//! persist the connection so sync works out of the box.
//!
//! Release builds never run this code (the call site in `lib.rs` is
//! `#[cfg(debug_assertions)]`-gated), so a production install's
//! configuration can never be touched by a stray local server.
//!
//! **Row-presence invariant:** the "deliberately disabled" detection in
//! [`should_auto_provision`] relies on `Settings::set_sync_server_url`
//! writing an empty-string row when the URL is cleared (never deleting
//! the row), together with retaining an existing API key. Keep both
//! contracts intact — a cleared row with no key is treated as an
//! unconfigured fresh install, while a cleared row with a key and sync
//! disabled is a deliberate opt-out.

use std::sync::Arc;
use std::time::Duration;

use oz_core::CoreError;
use oz_core::settings::Settings;
use oz_core::sync_client;
use rusqlite::Connection;
use tokio::sync::Mutex;

/// Default local dev sync server (`scripts/start-local-sync.bat`).
const LOCAL_SYNC_URL: &str = "http://localhost:3099";

/// How many probe + token attempts before giving up. The docker backend
/// can take a few seconds to answer on a cold start, so a bounded retry
/// lets the app connect even when it boots before the container is ready.
const PROBE_ATTEMPTS: u32 = 3;

/// Delay between probe attempts.
const PROBE_RETRY_DELAY: Duration = Duration::from_secs(2);

/// Decide whether auto-provisioning should run.
///
/// Returns `true` only when the install looks never-configured: no URL
/// row exists, or a cleared URL row has no API key, or sync is still
/// enabled (a broken half-configured state provisioning repairs). An
/// explicit disable — a URL that was set then cleared while sync is off
/// while retaining its API key — is respected.
fn should_auto_provision(
    configured_url: Option<&str>,
    sync_enabled: bool,
    has_api_key: bool,
) -> bool {
    match configured_url {
        // No settings row at all — a fresh install that has never been
        // configured. Provision regardless of the enabled flag (a fresh
        // DB ships with sync off but no URL row).
        None => true,
        // The URL row exists but was cleared. A retained API key plus sync
        // off means the user deliberately disabled it — respect that. No
        // key means the install is still unconfigured; sync on is a broken
        // half-configured state worth repairing.
        Some(url) if url.trim().is_empty() => sync_enabled || !has_api_key,
        // A real URL is configured — never touch it.
        Some(_) => false,
    }
}

/// Persist a provisioned sync connection: server URL + API key + enabled.
///
/// All three writes happen in one transaction so a failure partway can't
/// leave a half-provisioned state (e.g. a URL without a key) that would
/// block future auto-provisioning on the next launch.
fn persist_provisioned_sync(
    conn: &mut Connection,
    url: &str,
    api_key: &str,
) -> Result<(), CoreError> {
    let tx = conn.transaction()?;
    Settings::set_sync_server_url(&tx, url)?;
    Settings::set_sync_api_key(&tx, api_key)?;
    Settings::set_sync_enabled(&tx, true)?;
    tx.commit()?;
    Ok(())
}

/// Auto-connect the app to the local dev sync server (debug builds only).
///
/// 1. If a server URL is already configured, return immediately — an
///    existing install is never clobbered.
/// 2. Otherwise probe the local dev server; on success request a JWT and
///    persist URL + key + enabled so the background sync daemon picks the
///    connection up on its next tick.
/// 3. If the server is unreachable, leave settings untouched and return
///    quietly (sync stays unconfigured; the app still runs fine).
pub async fn auto_provision_local_sync(db: Arc<Mutex<Connection>>) {
    auto_provision_local_sync_with_url(db, LOCAL_SYNC_URL).await;
}

/// The provisioning loop, parameterised over the target server URL so tests
/// can run it against an ephemeral loopback server.
async fn auto_provision_local_sync_with_url(db: Arc<Mutex<Connection>>, server_url: &str) {
    // 1. Never touch an already-configured install. This guard runs before
    //    any network I/O so a configured production URL can never be
    //    clobbered by a local dev server. A settings READ error is treated
    //    as "leave it alone", not as "not configured" — we must never
    //    provision over an install we couldn't inspect.
    {
        let conn = db.lock().await;
        let (configured, enabled, has_api_key) = match (
            Settings::get_sync_server_url(&conn),
            Settings::is_sync_enabled(&conn),
            Settings::get_sync_api_key(&conn),
        ) {
            (Ok(url), Ok(enabled), Ok(api_key)) => (
                url.map(|u| u.trim().to_string()),
                enabled,
                api_key.is_some_and(|key| !key.trim().is_empty()),
            ),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                tracing::warn!("reading sync settings failed — leaving sync unconfigured: {e}");
                return;
            }
        };
        if !should_auto_provision(configured.as_deref(), enabled, has_api_key) {
            tracing::debug!(
                server_url = %configured.unwrap_or_default(),
                enabled,
                has_api_key,
                "sync already configured or deliberately disabled — skipping auto-provision"
            );
            return;
        }
    }

    // 2. Bounded probe + token request against the local dev server. The
    //    retry absorbs a cold-start docker container that is still warming
    //    up when the app boots.
    for attempt in 1..=PROBE_ATTEMPTS {
        let ping = sync_client::ping_server(server_url).await;
        if ping.ok {
            // ADR sync-auth-hardening P3: pair this terminal once (register
            // with the server and store the device secret), then mint tokens
            // with client credentials. Pairing is skipped when credentials
            // are already stored, and falls back to admin-key / open minting
            // when the server does not support registration.
            let client_credentials = resolve_terminal_credentials(&db, server_url).await;

            let token = match client_credentials {
                Some((client_id, client_secret)) => {
                    sync_client::request_token_client_credentials(
                        server_url,
                        &client_id,
                        &client_secret,
                    )
                    .await
                }
                None => {
                    // ADR sync-auth-hardening P2: a server started with
                    // OZ_ADMIN_KEY rejects minting without the matching
                    // header — pass it through when available.
                    sync_client::request_token(
                        server_url,
                        sync_client::admin_key_from_env().as_deref(),
                    )
                    .await
                }
            };
            if let (true, Some(key)) = (token.ok, token.token) {
                let mut conn = db.lock().await;
                match persist_provisioned_sync(&mut conn, server_url, &key) {
                    Ok(()) => tracing::info!(
                        expires_at = token.expires_at.as_deref().unwrap_or("unknown"),
                        "auto-provisioned local sync connection to {LOCAL_SYNC_URL}"
                    ),
                    Err(e) => tracing::warn!("persisting auto-provisioned sync failed: {e}"),
                }
                return;
            }
        }
        if attempt == PROBE_ATTEMPTS {
            tracing::debug!(
                url = server_url,
                "local sync server not reachable — leaving sync unconfigured"
            );
            return;
        }
        tokio::time::sleep(PROBE_RETRY_DELAY).await;
    }
}

/// Resolve this terminal's client credentials, pairing it with the server
/// on first run (ADR sync-auth-hardening P3).
///
/// Returns `Some((terminal_id, device_secret))` when the terminal is paired
/// (either already stored or freshly registered). Returns `None` when the
/// server rejected registration — the caller falls back to admin-key/open
/// minting so legacy dev servers keep working.
async fn resolve_terminal_credentials(
    db: &Arc<Mutex<Connection>>,
    server_url: &str,
) -> Option<(String, String)> {
    // Already paired — reuse the stored credentials.
    {
        let conn = db.lock().await;
        if let (Ok(Some(id)), Ok(Some(secret))) = (
            Settings::get_sync_terminal_id(&conn),
            Settings::get_sync_terminal_secret(&conn),
        ) {
            return Some((id, secret));
        }
    }

    // Fresh pair: generate a stable terminal id (kept even if the server is
    // unreachable, so a later launch retries registration with the same id).
    let terminal_id = {
        let conn = db.lock().await;
        match Settings::get_sync_terminal_id(&conn) {
            Ok(Some(id)) => id,
            _ => {
                let id = uuid::Uuid::new_v4().simple().to_string();
                let _ = Settings::set_sync_terminal_id(&conn, &id);
                id
            }
        }
    };

    let registration = sync_client::register_terminal(
        server_url,
        sync_client::admin_key_from_env().as_deref(),
        &terminal_id,
        "pos-terminal",
    )
    .await;
    if !registration.ok {
        tracing::debug!(
            status = %registration.status,
            "terminal registration failed — falling back to label minting"
        );
        return None;
    }

    let device_secret = registration.device_secret?;
    let conn = db.lock().await;
    if let Err(e) = Settings::set_sync_terminal_secret(&conn, &device_secret) {
        tracing::warn!(error = %e, "persisting terminal device secret failed");
        return None;
    }
    tracing::info!(terminal_id, "paired sync terminal with server");
    Some((terminal_id, device_secret))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;

    #[test]
    fn should_provision_when_no_url_is_configured() {
        // Fresh install: no settings row at all, sync off — provision.
        assert!(should_auto_provision(None, false, false));
        assert!(should_auto_provision(None, true, false));
    }

    #[test]
    fn should_not_provision_when_a_url_is_already_configured() {
        assert!(!should_auto_provision(
            Some("http://localhost:3099"),
            false,
            true
        ));
        assert!(!should_auto_provision(
            Some("https://cloud.example.com"),
            true,
            true
        ));
    }

    #[test]
    fn should_not_provision_when_sync_was_deliberately_disabled() {
        // The URL row exists but was cleared AND sync is off — the user's
        // "off" switch. Auto-provision must NOT silently re-enable it on
        // the next debug launch.
        assert!(!should_auto_provision(Some(""), false, true));
        assert!(!should_auto_provision(Some("   "), false, true));
    }

    #[test]
    fn should_provision_when_sync_is_enabled_but_url_was_cleared() {
        // Sync on but the URL field is empty — a broken half-configured
        // state. Provisioning repairs it and matches user intent.
        assert!(should_auto_provision(Some(""), true, true));
        assert!(should_auto_provision(Some("   "), true, true));
    }

    #[test]
    fn should_provision_when_empty_url_has_never_had_a_key() {
        // A fresh settings table may contain an empty URL row with sync
        // disabled. Without an API key this is still an unconfigured local
        // install, not an explicit opt-out, so debug bootstrap should heal it.
        assert!(should_auto_provision(Some(""), false, false));
        assert!(should_auto_provision(Some("   "), false, false));
    }

    #[test]
    fn persist_writes_url_key_and_enables_sync() {
        let mut conn = migrations::fresh_db();
        persist_provisioned_sync(&mut conn, LOCAL_SYNC_URL, "jwt-token").unwrap();
        assert_eq!(
            Settings::get_sync_server_url(&conn).unwrap().as_deref(),
            Some(LOCAL_SYNC_URL)
        );
        assert_eq!(
            Settings::get_sync_api_key(&conn).unwrap().as_deref(),
            Some("jwt-token")
        );
        assert!(Settings::is_sync_enabled(&conn).unwrap());
    }

    #[test]
    fn persist_is_idempotent() {
        let mut conn = migrations::fresh_db();
        persist_provisioned_sync(&mut conn, LOCAL_SYNC_URL, "token-a").unwrap();
        persist_provisioned_sync(&mut conn, LOCAL_SYNC_URL, "token-b").unwrap();
        assert_eq!(
            Settings::get_sync_api_key(&conn).unwrap().as_deref(),
            Some("token-b")
        );
    }

    #[tokio::test]
    async fn orchestrator_pairs_terminal_and_mints_with_client_credentials() {
        // ADR sync-auth-hardening P3: on first run the bootstrap registers
        // the terminal, stores the device secret, and mints its token with
        // client credentials — no admin key, no manual pairing.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let token_body: Arc<tokio::sync::Mutex<Option<String>>> =
            Arc::new(tokio::sync::Mutex::new(None));
        let token_body_server = token_body.clone();
        let task = tokio::spawn(async move {
            let mut echoed_terminal_id = String::new();
            for _ in 0..3 {
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
                let response = if path == "/health" {
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}"
                        .to_string()
                } else if path == "/api/v1/terminals" {
                    // Echo the client-generated terminal id back.
                    if let Some(line) = request.lines().rev().find(|l| !l.trim().is_empty())
                        && let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
                    {
                        echoed_terminal_id =
                            v["terminal_id"].as_str().unwrap_or_default().to_string();
                    }
                    let body = format!(
                        "{{\"terminal_id\":\"{echoed_terminal_id}\",\"device_secret\":\"dev-secret-xyz\"}}"
                    );
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                } else {
                    *token_body_server.lock().await = Some(request);
                    let body =
                        r#"{"token":{"token":"jwt-paired","expires_at":null,"token_id":"t1"}}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                };
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        let db = Arc::new(Mutex::new(migrations::fresh_db()));
        auto_provision_local_sync_with_url(db.clone(), &url).await;
        task.await.unwrap();

        let conn = db.try_lock().unwrap();
        assert_eq!(
            Settings::get_sync_server_url(&conn).unwrap().as_deref(),
            Some(url.as_str())
        );
        assert_eq!(
            Settings::get_sync_api_key(&conn).unwrap().as_deref(),
            Some("jwt-paired")
        );
        assert!(Settings::is_sync_enabled(&conn).unwrap());
        // The device secret was stored so later launches reuse it.
        assert_eq!(
            Settings::get_sync_terminal_secret(&conn)
                .unwrap()
                .as_deref(),
            Some("dev-secret-xyz")
        );
        let terminal_id = Settings::get_sync_terminal_id(&conn)
            .unwrap()
            .expect("terminal id must be persisted");
        assert_eq!(terminal_id.len(), 32);

        // The token request carried the client credentials, not the admin key.
        let token_request = token_body.lock().await.clone().unwrap();
        assert!(
            token_request.contains("\"client_id\":\"")
                && token_request.contains("\"client_secret\":\"dev-secret-xyz\""),
            "token request did not carry client credentials: {token_request}"
        );
        assert!(
            !token_request.to_ascii_lowercase().contains("x-admin-key"),
            "paired minting must not send the admin key"
        );
    }

    #[tokio::test]
    async fn orchestrator_never_clobbers_existing_configuration() {
        // Safety contract: an install that already has a server URL must
        // be left completely untouched — auto-provision must not overwrite
        // it, flip sync on, or inject a key.
        let db = Arc::new(Mutex::new(migrations::fresh_db()));
        {
            let conn = db.try_lock().unwrap();
            Settings::set_sync_server_url(&conn, "https://prod.example.com").unwrap();
            Settings::set_sync_enabled(&conn, false).unwrap();
        }

        auto_provision_local_sync(db.clone()).await;

        let conn = db.try_lock().unwrap();
        assert_eq!(
            Settings::get_sync_server_url(&conn).unwrap().as_deref(),
            Some("https://prod.example.com")
        );
        assert!(!Settings::is_sync_enabled(&conn).unwrap());
        assert!(Settings::get_sync_api_key(&conn).unwrap().is_none());
    }

    #[tokio::test]
    async fn orchestrator_does_not_reprovision_when_sync_was_disabled() {
        // The user cleared the URL and turned sync off. The guard must
        // fire BEFORE any network I/O, so this stays deterministic even
        // with a live dev server on :3099: sync stays off, no key is
        // injected, and the cleared URL row is untouched.
        let db = Arc::new(Mutex::new(migrations::fresh_db()));
        {
            let conn = db.try_lock().unwrap();
            Settings::set_sync_server_url(&conn, "").unwrap();
            Settings::set_sync_api_key(&conn, "existing-token").unwrap();
            Settings::set_sync_enabled(&conn, false).unwrap();
        }

        auto_provision_local_sync(db.clone()).await;

        let conn = db.try_lock().unwrap();
        assert_eq!(
            Settings::get_sync_server_url(&conn).unwrap().as_deref(),
            Some("")
        );
        assert!(!Settings::is_sync_enabled(&conn).unwrap());
        assert_eq!(
            Settings::get_sync_api_key(&conn).unwrap().as_deref(),
            Some("existing-token")
        );
    }
}
