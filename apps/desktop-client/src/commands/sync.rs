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
    let server_url = Settings::get_sync_server_url(&db)?;
    let api_key = Settings::get_sync_api_key(&db)?;
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
    // Always write — passing `null` from the front-end is the UI's
    // signal to clear the field. `SyncConfig::from_settings` treats
    // an empty string as "not configured", so the round-trip is safe.
    let url = args.server_url.as_deref().unwrap_or("");
    let key = args.api_key.as_deref().unwrap_or("");
    Settings::set_sync_server_url(&db, url)?;
    Settings::set_sync_api_key(&db, key)?;
    Settings::set_sync_enabled(&db, args.enabled)?;
    drop(db);
    Ok(())
}

/// Immediately run a sync cycle that pushes pending sales, credit, and
/// other queued offline transactions to the configured cloud server.
#[command]
pub async fn sync_run(state: State<'_, AppState>) -> Result<SyncAttemptResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let config = SyncConfig::from_settings(&store)?;
    let result = match config {
        Some(cfg) => sync_client::sync_pending(&store, &cfg)?,
        None => SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: Some("Sync is not configured or disabled".into()),
        },
    };
    drop(db);
    Ok(result)
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

/// Pull a server snapshot and overwrite the local cache for products,
/// tax rates, and users. The UI is expected to confirm the overwrite
/// before invoking this command.
#[command]
pub async fn sync_pull(state: State<'_, AppState>) -> Result<PullResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let config = SyncConfig::from_settings(&store)?;
    let result = match config {
        Some(cfg) => sync_client::pull_snapshot(&store, &cfg)?,
        None => PullResult {
            products_pulled: 0,
            tax_rates_pulled: 0,
            users_pulled: 0,
            error: Some("Sync is not configured or disabled".into()),
        },
    };
    drop(db);
    Ok(result)
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
        assert_eq!(json["server_url"], "https://sync.example.com");
        assert_eq!(json["has_api_key"], true);
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
        assert!(json["server_url"].is_null());
        assert_eq!(json["has_api_key"], false);
        assert_eq!(json["enabled"], false);
    }

    #[test]
    fn update_sync_settings_deserialize() {
        let json =
            r#"{"server_url":"https://sync.example.com","api_key":"sk-abc123","enabled":true}"#;
        let args: UpdateSyncSettingsArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.server_url.unwrap(), "https://sync.example.com");
        assert_eq!(args.api_key.unwrap(), "sk-abc123");
        assert!(args.enabled);
    }

    #[test]
    fn update_sync_settings_deserialize_no_key() {
        let json = r#"{"server_url":null,"api_key":null,"enabled":false}"#;
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
