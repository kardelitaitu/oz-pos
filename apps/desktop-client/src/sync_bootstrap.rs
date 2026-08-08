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
/// Returns `true` only when no non-empty server URL is configured. Once a
/// user (or a previous provisioning) has set a URL, auto-provision must
/// never touch the configuration again.
fn should_auto_provision(configured_url: Option<&str>) -> bool {
    match configured_url {
        Some(url) => url.trim().is_empty(),
        None => true,
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
    // 1. Never touch an already-configured install. This guard runs before
    //    any network I/O so a configured production URL can never be
    //    clobbered by a local dev server. A settings READ error is treated
    //    as "leave it alone", not as "not configured" — we must never
    //    provision over an install we couldn't inspect.
    {
        let conn = db.lock().await;
        let configured = match Settings::get_sync_server_url(&conn) {
            Ok(url) => url,
            Err(e) => {
                tracing::warn!("reading sync settings failed — leaving sync unconfigured: {e}");
                return;
            }
        }
        .map(|u| u.trim().to_string());
        if !should_auto_provision(configured.as_deref()) {
            tracing::debug!(
                server_url = %configured.unwrap_or_default(),
                "sync already configured — skipping auto-provision"
            );
            return;
        }
    }

    // 2. Bounded probe + token request against the local dev server. The
    //    retry absorbs a cold-start docker container that is still warming
    //    up when the app boots.
    for attempt in 1..=PROBE_ATTEMPTS {
        let ping = sync_client::ping_server(LOCAL_SYNC_URL).await;
        if ping.ok {
            let token = sync_client::request_token(LOCAL_SYNC_URL).await;
            if let (true, Some(key)) = (token.ok, token.token) {
                let mut conn = db.lock().await;
                match persist_provisioned_sync(&mut conn, LOCAL_SYNC_URL, &key) {
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
                url = LOCAL_SYNC_URL,
                "local sync server not reachable — leaving sync unconfigured"
            );
            return;
        }
        tokio::time::sleep(PROBE_RETRY_DELAY).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;

    #[test]
    fn should_provision_when_no_url_is_configured() {
        assert!(should_auto_provision(None));
        assert!(should_auto_provision(Some("")));
        assert!(should_auto_provision(Some("   ")));
    }

    #[test]
    fn should_not_provision_when_a_url_is_already_configured() {
        assert!(!should_auto_provision(Some("http://localhost:3099")));
        assert!(!should_auto_provision(Some("https://cloud.example.com")));
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
}
