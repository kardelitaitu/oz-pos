//! Cloud sync commands — configure and trigger sync from the UI.
//!
//! The `sync_run` command runs a sync cycle immediately (instead of
//! waiting for the background daemon's interval). The settings commands
//! let the user configure the server URL and API key.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::settings::Settings;
use oz_core::sync_client::{self, PullResult, SyncAttemptResult, SyncConfig};
use rusqlite::Connection;

use crate::error::AppError;
use crate::state::AppState;

/// Get the current sync configuration settings.
#[derive(Debug, Serialize)]
pub struct SyncSettingsDto {
    /// Server Url.
    pub server_url: Option<String>,
    /// Has Api Key.
    pub has_api_key: bool,
    /// Enabled.
    pub enabled: bool,
}

/// Get sync settings.
#[command]
pub async fn get_sync_settings(state: State<'_, AppState>) -> Result<SyncSettingsDto, AppError> {
    let db = state.db.lock().await;
    let server_url = Settings::get_sync_server_url(&db)?.filter(|s| !s.is_empty());
    let api_key = Settings::get_sync_api_key(&db)?.filter(|k| !k.is_empty());
    let enabled = Settings::is_sync_enabled(&db)?;
    drop(db);
    Ok(SyncSettingsDto {
        server_url,
        has_api_key: api_key.is_some(),
        enabled,
    })
}

/// Update sync settings.
#[derive(Debug, Deserialize)]
pub struct UpdateSyncSettingsArgs {
    /// Server Url.
    pub server_url: Option<String>,
    /// Api Key.
    pub api_key: Option<String>,
    /// Enabled.
    pub enabled: bool,
}

#[command]
/// Update sync settings.
pub async fn update_sync_settings(
    args: UpdateSyncSettingsArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    update_sync_settings_data(&db, &args)?;
    drop(db);
    Ok(())
}

/// Persist sync settings (server URL, API key, enabled flag) atomically.
///
/// All three writes execute inside a single SQLite transaction so a
/// failure on any one rolls back the others — preventing the
/// partially-updated state the previous sequential-write version could
/// leave behind (e.g. a new API key persisted while the `enabled` flag
/// still held its old value).
///
/// Extracted as a free function so the atomicity contract can be tested
/// without a Tauri runtime
/// (see `update_sync_settings_data_rolls_back_on_partial_failure`).
pub fn update_sync_settings_data(
    conn: &Connection,
    args: &UpdateSyncSettingsArgs,
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    // Always update server URL (passing `null` or empty string clears it).
    let url = args.server_url.as_deref().unwrap_or("");
    Settings::set_sync_server_url(&tx, url)?;
    // Only update API key if `Some(key)` was passed from the UI.
    // When `args.api_key` is `None` (the masked API field on the front-end was not modified),
    // preserve the existing key stored in the database.
    if let Some(ref key) = args.api_key {
        Settings::set_sync_api_key(&tx, key)?;
    }
    Settings::set_sync_enabled(&tx, args.enabled)?;
    tx.commit()?;
    Ok(())
}

/// Immediately run a sync cycle that pushes pending sales, credit, and
/// other queued offline transactions to the configured cloud server.
///
/// Uses a three-phase split (read → async HTTP → write) so the DB
/// lock is not held during the network round-trip.
#[command]
pub async fn sync_run(state: State<'_, AppState>) -> Result<SyncAttemptResult, AppError> {
    // Phase 1: Read pending items and config from DB (brief lock).
    let (pending_items, config_opt) = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let pending = store.list_pending_offline()?;
        let config = SyncConfig::from_settings(&store)?;
        (pending, config)
    };

    let config = match config_opt {
        Some(c) => c,
        None => {
            return Ok(SyncAttemptResult {
                synced: 0,
                failed: 0,
                error: Some("Sync is not configured or disabled".into()),
            });
        }
    };

    if pending_items.is_empty() {
        return Ok(SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: None,
        });
    }

    // Phase 2: Async HTTP push (no DB lock held).
    let outcomes = sync_client::send_items_to_server(&config, &pending_items).await;

    // Phase 3: Write outcomes back to DB (brief lock).
    let db = state.db.lock().await;
    let store = Store::new(&db);
    match outcomes {
        Ok(outcomes) => Ok(sync_client::apply_sync_outcomes(
            &store,
            &pending_items,
            &outcomes,
        )?),
        Err(e) => Ok(sync_client::mark_all_failed(
            &store,
            &pending_items,
            &e.to_string(),
        )?),
    }
}

/// Get the pending sync count.
#[command]
pub async fn pending_sync_count(state: State<'_, AppState>) -> Result<i64, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let count = store.pending_offline_count()?;
    drop(db);
    Ok(count)
}

/// Request a new JWT API token from the cloud server's
/// `POST /api/v1/tokens` endpoint.
///
/// Uses the URL from the front-end text field if provided,
/// otherwise falls back to saved settings.
#[command]
pub async fn request_sync_token(
    url: Option<String>,
    state: State<'_, AppState>,
) -> Result<sync_client::TokenResult, AppError> {
    let resolved = match url.filter(|u| !u.is_empty()) {
        Some(u) => Some(u),
        None => {
            let db = state.db.lock().await;
            Settings::get_sync_server_url(&db)?.filter(|s| !s.is_empty())
        }
    };
    match resolved {
        Some(u) => Ok(sync_client::request_token(&u).await),
        None => Ok(sync_client::TokenResult {
            ok: false,
            token: None,
            status: "No server URL configured".into(),
            expires_at: None,
        }),
    }
}

/// Test the cloud sync connection by pinging the configured server.
/// If `url` is provided from the front-end, it is used directly.
#[command]
pub async fn test_sync_connection(
    url: Option<String>,
    state: State<'_, AppState>,
) -> Result<sync_client::PingResult, AppError> {
    let resolved = match url.filter(|u| !u.is_empty()) {
        Some(u) => Some(u),
        None => {
            let db = state.db.lock().await;
            Settings::get_sync_server_url(&db)?.filter(|s| !s.is_empty())
        }
    };
    match resolved {
        Some(u) => Ok(sync_client::ping_server(&u).await),
        None => Ok(sync_client::PingResult {
            ok: false,
            status: "No server URL configured".into(),
            latency_ms: None,
        }),
    }
}

/// Pull a server snapshot and overwrite the local cache for products,
/// tax rates, and users. The UI is expected to confirm the overwrite
/// before invoking this command.
///
/// Uses a three-phase split (read → async HTTP → write) so the DB
/// lock is not held during the network round-trip.
#[command]
pub async fn sync_pull(state: State<'_, AppState>) -> Result<PullResult, AppError> {
    // Phase 1: Read config from DB (brief lock).
    let config_opt = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        SyncConfig::from_settings(&store)?
    };

    let config = match config_opt {
        Some(c) => c,
        None => {
            return Ok(PullResult {
                products_pulled: 0,
                tax_rates_pulled: 0,
                users_pulled: 0,
                error: Some("Sync is not configured or disabled".into()),
            });
        }
    };

    // Phase 2: Async HTTP fetch (no DB lock held).
    let snapshot = sync_client::fetch_snapshot_from_server(&config).await;

    // Phase 3: Apply snapshot to DB (brief lock).
    let db = state.db.lock().await;
    let store = Store::new(&db);
    match snapshot {
        Ok(s) => Ok(sync_client::apply_snapshot(&store, &s)?),
        Err(e) => Ok(PullResult {
            products_pulled: 0,
            tax_rates_pulled: 0,
            users_pulled: 0,
            error: Some(e.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_settings_serialize() {
        let dto = SyncSettingsDto {
            server_url: Some("https://sync.example.com".into()),
            has_api_key: true,
            enabled: true,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["server_url"], "https://sync.example.com");
        assert_eq!(json["has_api_key"], true);
        assert_eq!(json["enabled"], true);
    }

    #[test]
    fn sync_settings_no_url_disabled() {
        let dto = SyncSettingsDto {
            server_url: None,
            has_api_key: false,
            enabled: false,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert!(json["server_url"].is_null());
        assert!(!json["enabled"].as_bool().unwrap());
    }

    #[test]
    fn update_sync_settings_deserialize() {
        let json = r#"{"server_url":"https://sync.example.com","api_key":"sk-123","enabled":true}"#;
        let args: UpdateSyncSettingsArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.server_url.unwrap(), "https://sync.example.com");
        assert_eq!(args.api_key.unwrap(), "sk-123");
    }

    #[test]
    fn update_sync_settings_deserialize_no_key() {
        let json = r#"{"server_url":null,"api_key":null,"enabled":false}"#;
        let args: UpdateSyncSettingsArgs = serde_json::from_str(json).unwrap();
        assert!(args.server_url.is_none());
        assert!(args.api_key.is_none());
    }

    #[test]
    fn update_sync_settings_debug() {
        let args = UpdateSyncSettingsArgs {
            server_url: Some("url".into()),
            api_key: None,
            enabled: true,
        };
        let debug = format!("{:?}", args);
        assert!(debug.contains("url"));
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
        assert_eq!(json["error"], "network unreachable");
    }

    // ── TDD Bug Hunt: non-atomic sync settings writes ────────────────
    //
    // update_sync_settings_data persists three settings (server_url,
    // api_key, enabled). If the writes are NOT wrapped in a single
    // transaction, a failure on the third write leaves the first two
    // committed — a partially-updated, inconsistent state. This test
    // forces the third write to fail and asserts that the prior writes
    // were rolled back (i.e. the function is atomic).

    use oz_core::migrations;

    fn fresh_sync_conn() -> Connection {
        let conn = migrations::fresh_db();
        // Install triggers that reject any write to the `sync_enabled`
        // key — on both INSERT (fresh row) and UPDATE (existing row).
        // This simulates a disk/IO failure on the THIRD write only,
        // letting the first two (server_url, api_key) succeed.
        conn.execute_batch(
            "CREATE TRIGGER reject_sync_enabled_ins
             BEFORE INSERT ON settings
             WHEN NEW.key = 'sync_enabled'
             BEGIN
                 SELECT RAISE(ABORT, 'forced failure on sync_enabled insert');
             END;
             CREATE TRIGGER reject_sync_enabled_upd
             BEFORE UPDATE ON settings
             WHEN NEW.key = 'sync_enabled'
             BEGIN
                 SELECT RAISE(ABORT, 'forced failure on sync_enabled update');
             END;",
        )
        .expect("install reject trigger");
        conn
    }

    #[test]
    fn update_sync_settings_data_rolls_back_on_partial_failure() {
        let conn = fresh_sync_conn();
        let args = UpdateSyncSettingsArgs {
            server_url: Some("https://sync.example.com".into()),
            api_key: Some("sk-secret".into()),
            enabled: true,
        };

        // The third write (set_sync_enabled) hits the trigger and fails.
        let result = update_sync_settings_data(&conn, &args);
        assert!(
            result.is_err(),
            "update must surface the forced sync_enabled failure"
        );

        // Atomicity contract: because the function wraps all three writes
        // in one transaction, the failed third write must roll back the
        // first two. If they are NOT rolled back, the function is
        // non-atomic (the bug).
        let server_url = Settings::get_sync_server_url(&conn).unwrap();
        let api_key = Settings::get_sync_api_key(&conn).unwrap();

        // These assertions FAIL in the RED phase (no transaction → first
        // two writes persist) and PASS once a transaction wraps the batch.
        assert!(
            server_url.as_deref() != Some("https://sync.example.com"),
            "server_url must be rolled back, got: {server_url:?}"
        );
        assert!(
            api_key.as_deref() != Some("sk-secret"),
            "api_key must be rolled back, got: {api_key:?}"
        );
    }

    #[test]
    fn update_sync_settings_data_commits_all_when_no_failure() {
        // Happy path: all three writes succeed and persist together.
        let conn = migrations::fresh_db();
        let args = UpdateSyncSettingsArgs {
            server_url: Some("https://sync.example.com".into()),
            api_key: Some("sk-secret".into()),
            enabled: true,
        };
        update_sync_settings_data(&conn, &args).unwrap();

        assert_eq!(
            Settings::get_sync_server_url(&conn).unwrap().as_deref(),
            Some("https://sync.example.com")
        );
        assert_eq!(
            Settings::get_sync_api_key(&conn).unwrap().as_deref(),
            Some("sk-secret")
        );
        assert!(Settings::is_sync_enabled(&conn).unwrap());
    }

    #[test]
    fn update_sync_settings_data_preserves_api_key_when_none() {
        // When api_key is None, the existing key must be preserved.
        let conn = migrations::fresh_db();
        Settings::set_sync_api_key(&conn, "existing-key").unwrap();

        let args = UpdateSyncSettingsArgs {
            server_url: Some("https://x".into()),
            api_key: None,
            enabled: false,
        };
        update_sync_settings_data(&conn, &args).unwrap();

        assert_eq!(
            Settings::get_sync_api_key(&conn).unwrap().as_deref(),
            Some("existing-key")
        );
        assert!(!Settings::is_sync_enabled(&conn).unwrap());
    }
}
