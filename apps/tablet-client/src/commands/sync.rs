//! Cloud sync commands — configure and trigger sync from the UI.
//!
//! The `trigger_sync` command runs a sync cycle immediately (instead of
//! waiting for the background daemon's interval). The settings commands
//! let the user configure the server URL and API key.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::settings::Settings;
use oz_core::sync_client::{self, SyncAttemptResult, SyncConfig};

use crate::error::AppError;
use crate::state::AppState;

/// Get the current sync configuration settings.
#[derive(Debug, Serialize)]
pub struct SyncSettingsDto {
    pub server_url: Option<String>,
    pub has_api_key: bool,
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
    pub server_url: Option<String>,
    pub api_key: Option<String>,
    pub enabled: bool,
}

#[command]
pub async fn update_sync_settings(
    args: UpdateSyncSettingsArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    if let Some(url) = &args.server_url {
        Settings::set_sync_server_url(&db, url)?;
    }
    if let Some(key) = &args.api_key {
        Settings::set_sync_api_key(&db, key)?;
    }
    Settings::set_sync_enabled(&db, args.enabled)?;
    drop(db);
    Ok(())
}

/// Immediately trigger a sync cycle.
#[command]
pub async fn trigger_sync(state: State<'_, AppState>) -> Result<SyncAttemptResult, AppError> {
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
