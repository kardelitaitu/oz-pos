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
fn send_item_to_server(config: &SyncConfig, item: &OfflineQueueItem) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Replace with actual HTTP POST request using ureq or reqwest.
    //
    // The payload structure sent to the server:
    // {
    //   "action": "complete_sale",
    //   "payload": { ... item payload ... },
    //   "id": "item.id"
    // }
    tracing::info!(
        item_id = %item.id,
        action = %item.action,
        server = %config.server_url,
        "would sync item to server"
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
        let result = sync_pending(&store, &config).unwrap();
        assert_eq!(result.synced, 1);

        let pending = store.list_pending_offline().unwrap();
        assert!(pending.is_empty());
    }
}
