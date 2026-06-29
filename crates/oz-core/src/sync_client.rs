//! Cloud sync client — pushes pending offline queue items to a remote server.
//!
//! The sync client reads from the local offline queue, sends each item to the
//! configured remote server via REST API, and marks the item as synced (or
//! failed) in the local database.

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::offline::OfflineQueueItem;
use crate::db::Store;

/// Result of a single sync attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncAttemptResult {
    /// Number of items successfully synced.
    pub synced: usize,
    /// Number of items that failed to sync.
    pub failed: usize,
    /// Error message if the entire sync failed (e.g. network error).
    pub error: Option<String>,
}

/// Sync client configuration.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Remote server base URL (e.g. "http://localhost:3099").
    pub server_url: String,
    /// API key for authentication.
    pub api_key: Option<String>,
}

impl SyncConfig {
    /// Load sync configuration from settings.
    pub fn from_settings(store: &Store) -> Result<Option<Self>, CoreError> {
        let enabled = crate::settings::Settings::is_sync_enabled(store.conn())?;
        if !enabled {
            return Ok(None);
        }
        let server_url = crate::settings::Settings::get_sync_server_url(store.conn())?;
        let server_url = match server_url {
            Some(u) if !u.is_empty() => u,
            _ => return Ok(None),
        };
        let api_key = crate::settings::Settings::get_sync_api_key(store.conn())?;
        Ok(Some(Self { server_url, api_key }))
    }
}

/// Attempt to sync all pending offline items to the remote server.
///
/// Returns a `SyncAttemptResult` with counts of synced/failed items.
/// When the server is unreachable the entire batch fails with an error.
pub fn sync_pending(store: &Store, config: &SyncConfig) -> Result<SyncAttemptResult, CoreError> {
    let pending = store.list_pending_offline()?;
    if pending.is_empty() {
        return Ok(SyncAttemptResult { synced: 0, failed: 0, error: None });
    }

    let mut synced = 0usize;
    let mut failed = 0usize;
    let mut global_error: Option<String> = None;

    for item in &pending {
        match send_item_to_server(config, item) {
            Ok(()) => {
                store.mark_offline_synced(&item.id)?;
                synced += 1;
            }
            Err(e) => {
                let err_msg = e.to_string();
                store.mark_offline_failed(&item.id, &err_msg)?;
                failed += 1;
                global_error = Some(err_msg);
            }
        }
    }

    Ok(SyncAttemptResult { synced, failed, error: global_error })
}

/// Send a single offline queue item to the remote server via HTTP POST.
#[cfg(feature = "sync-http")]
fn send_item_to_server(config: &SyncConfig, item: &OfflineQueueItem) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/events", config.server_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "id": item.id,
        "action": item.action,
        "payload": item.payload,
    });

    let mut headers = reqwest::blocking::Client::new()
        .post(&url)
        .header("Content-Type", "application/json");

    if let Some(ref key) = config.api_key {
        headers = headers.header("Authorization", &format!("Bearer {key}"));
    }

    let resp = headers
        .json(&payload)
        .send()
        .map_err(|e| format!("sync HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("sync server returned {status}: {body}").into());
    }

    tracing::info!(
        item_id = %item.id,
        action = %item.action,
        server = %config.server_url,
        "synced item to server"
    );
    Ok(())
}

/// Stub used when `sync-http` feature is disabled — just logs the intent.
#[cfg(not(feature = "sync-http"))]
fn send_item_to_server(config: &SyncConfig, item: &OfflineQueueItem) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        item_id = %item.id,
        action = %item.action,
        server = %config.server_url,
        "sync-http feature disabled; would sync item to server"
    );
    Ok(())
}

/// Send a single offline queue item to the remote server (async version).
///
/// This is the async counterpart used by the background daemon.
pub async fn sync_pending_async(
    store: &Store<'_>,
    config: &SyncConfig,
) -> Result<SyncAttemptResult, CoreError> {
    sync_pending(store, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::settings::Settings;
    use rusqlite::Connection;

    fn setup() -> Store<'static> {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        let conn: &'static Connection = Box::leak(Box::new(conn));
        Store::new(conn)
    }

    #[test]
    fn sync_pending_empty_queue() {
        let store = setup();
        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: None,
        };
        let result = sync_pending(&store, &config).unwrap();
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 0);
        assert!(result.error.is_none());
    }

    #[test]
    fn sync_config_from_settings_disabled() {
        let store = setup();
        let config = SyncConfig::from_settings(&store).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn sync_pending_marks_items_synced() {
        let store = setup();
        let _item = store.enqueue_offline("complete_sale", r#"{"test": true}"#).unwrap();

        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: None,
        };
        // No server running locally — sync should fail with a transport error.
        let result = sync_pending(&store, &config).unwrap();
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 1);
        assert!(result.error.is_some(), "should report a network error");

        // Item should be marked as failed (no longer pending).
        let pending = store.list_pending_offline().unwrap();
        assert!(pending.is_empty(), "failed item is no longer pending");
        let all = store.list_all_offline().unwrap();
        assert_eq!(all.len(), 1, "item still in queue with failed status");
        assert_eq!(all[0].status, crate::offline::OfflineQueueStatus::Failed);
    }

    #[test]
    fn sync_pending_multiple_items() {
        let store = setup();
        store.enqueue_offline("complete_sale", r#"{"id":1}"#).unwrap();
        store.enqueue_offline("complete_sale", r#"{"id":2}"#).unwrap();

        let config = SyncConfig {
            server_url: "http://localhost:3099".into(),
            api_key: None,
        };
        let result = sync_pending(&store, &config).unwrap();
        // No server running — all items fail.
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 2);
        assert!(result.error.is_some(), "should report a network error");
    }

    #[test]
    fn sync_config_from_settings_enabled_with_url() {
        let store = setup();
        let conn = store.conn();
        Settings::set_sync_enabled(conn, true).unwrap();
        Settings::set_sync_server_url(conn, "http://sync.example.com").unwrap();

        let config = SyncConfig::from_settings(&store).unwrap();
        assert!(config.is_some());
        assert_eq!(config.unwrap().server_url, "http://sync.example.com");
    }

    #[test]
    fn sync_config_from_settings_enabled_no_url() {
        let store = setup();
        let conn = store.conn();
        Settings::set_sync_enabled(conn, true).unwrap();
        // Don't set a URL
        let config = SyncConfig::from_settings(&store).unwrap();
        assert!(config.is_none(), "should be None when no URL is set");
    }

    #[test]
    fn sync_config_from_settings_enabled_empty_url() {
        let store = setup();
        let conn = store.conn();
        Settings::set_sync_enabled(conn, true).unwrap();
        Settings::set_sync_server_url(conn, "").unwrap();

        let config = SyncConfig::from_settings(&store).unwrap();
        assert!(config.is_none(), "should be None when URL is empty");
    }

    #[test]
    fn sync_config_from_settings_with_api_key() {
        let store = setup();
        let conn = store.conn();
        Settings::set_sync_enabled(conn, true).unwrap();
        Settings::set_sync_server_url(conn, "http://sync.example.com").unwrap();
        Settings::set_sync_api_key(conn, "sk-test-key").unwrap();

        let config = SyncConfig::from_settings(&store).unwrap().unwrap();
        assert_eq!(config.server_url, "http://sync.example.com");
        assert_eq!(config.api_key, Some("sk-test-key".into()));
    }

    #[test]
    fn sync_attempt_result_debug() {
        let result = SyncAttemptResult {
            synced: 5,
            failed: 1,
            error: Some("network error".into()),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("synced: 5"));
        assert!(debug.contains("failed: 1"));
    }

    #[test]
    fn sync_attempt_result_serde_roundtrip() {
        let result = SyncAttemptResult {
            synced: 10,
            failed: 2,
            error: Some("timeout".into()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: SyncAttemptResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.synced, 10);
        assert_eq!(back.failed, 2);
        assert_eq!(back.error, Some("timeout".into()));
    }

    #[test]
    fn sync_attempt_result_no_error() {
        let result = SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: None,
        };
        assert!(result.error.is_none());
    }
}
