//! Cloud sync commands — configure and trigger sync from the UI.
//!
//! The `sync_run` command runs a push cycle immediately (instead of
//! waiting for the background daemon's interval). `sync_pull` fetches
//! the server's snapshot of products / tax rates / users and replaces
//! the local cache. The settings commands let the user configure the
//! server URL and API key.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::settings::Settings;
use oz_core::sync_client::{self, PullResult, SyncAttemptResult, SyncConfig};

use crate::error::AppError;
use crate::state::AppState;

/// Get the current sync configuration settings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
    // Always update server URL (passing `null` or empty string clears it).
    let url = args.server_url.as_deref().unwrap_or("");
    Settings::set_sync_server_url(&db, url)?;
    // Only update API key if `Some(key)` was passed from the UI.
    // When `args.api_key` is `None` (the masked API field on the front-end was not modified),
    // preserve the existing key stored in the database.
    if let Some(ref key) = args.api_key {
        Settings::set_sync_api_key(&db, key)?;
    }
    Settings::set_sync_enabled(&db, args.enabled)?;
    drop(db);
    Ok(())
}

/// Immediately run a sync cycle that pushes pending sales, credit, and
/// other queued offline transactions to the configured cloud server.
///
/// Uses a three-phase split (read → async HTTP → write) so the DB
/// lock is not held during the network round-trip, avoiding the
/// "Cannot drop a runtime in a context where blocking is not allowed"
/// panic that reqwest::blocking triggers inside Tauri's async runtime.
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
/// If `url` is provided (from the front-end text field), it is used
/// directly so users can request a token before saving. Otherwise the
/// saved value from settings is used.
#[command]
pub async fn request_sync_token(
    url: Option<String>,
    state: State<'_, AppState>,
) -> Result<sync_client::TokenResult, AppError> {
    // Resolve the URL first (may briefly lock DB), then drop the lock
    // before making the async HTTP call.
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

/// Test the cloud sync connection by pinging the configured server's
/// `/health` endpoint.
///
/// If `url` is provided (from the front-end text field), it is used
/// directly so users can test a URL before saving. Otherwise the
/// saved value from settings is used.
#[command]
pub async fn test_sync_connection(
    url: Option<String>,
    state: State<'_, AppState>,
) -> Result<sync_client::PingResult, AppError> {
    // Resolve the URL first (may briefly lock DB), then drop the lock
    // before making the async HTTP call.
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

    #[test]
    fn update_sync_settings_deserialize() {
        let json =
            r#"{"serverUrl":"https://sync.example.com","apiKey":"sk-abc123","enabled":true}"#;
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
}
