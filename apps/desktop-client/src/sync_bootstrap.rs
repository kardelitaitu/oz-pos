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
        let ping = sync_client::ping_server(LOCAL_SYNC_URL).await;
        if ping.ok {
            // ADR sync-auth-hardening P2: a server started with OZ_ADMIN_KEY
            // rejects minting without the matching header — pass it through
            // when the client environment carries it.
            let token = sync_client::request_token(
                LOCAL_SYNC_URL,
                sync_client::admin_key_from_env().as_deref(),
            )
            .await;
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
