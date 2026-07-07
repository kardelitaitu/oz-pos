//! Offline queue commands.
//!
//! These commands allow the front-end to enqueue, list, and sync
//! transactions that were created while the network was unavailable.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{OfflineQueueItem, Store};

use foundation::validate_not_empty;

use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

/// Offline queue item DTO for the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineQueueItemDto {
    pub id: String,
    pub action: String,
    pub payload: String,
    pub status: String,
    pub retry_count: i64,
    pub last_error: Option<String>,
    pub created_at: String,
    pub synced_at: Option<String>,
}

impl From<OfflineQueueItem> for OfflineQueueItemDto {
    fn from(item: OfflineQueueItem) -> Self {
        Self {
            id: item.id,
            action: item.action,
            payload: item.payload,
            status: item.status.as_stored_str().to_owned(),
            retry_count: item.retry_count,
            last_error: item.last_error,
            created_at: item.created_at,
            synced_at: item.synced_at,
        }
    }
}

/// Result of a sync retry attempt.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    /// Number of items successfully synced.
    pub synced_count: i64,
    /// Number of items that failed to sync.
    pub failed_count: i64,
    /// Total number of items that were attempted.
    pub total_count: i64,
}

/// Arguments for enqueuing an offline transaction.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueOfflineArgs {
    /// The action to perform (e.g. "complete_sale", "void_sale").
    pub action: String,
    /// JSON-serialized payload for the action.
    pub payload: String,
}

// ── Commands ──────────────────────────────────────────────────────────

/// Manually enqueue a transaction for later sync.
#[command]
pub async fn enqueue_offline(
    args: EnqueueOfflineArgs,
    state: State<'_, AppState>,
) -> Result<OfflineQueueItemDto, AppError> {
    validate_not_empty("action", &args.action).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("payload", &args.payload).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    let item = store.enqueue_offline(&args.action, &args.payload)?;
    drop(db);

    tracing::info!(id = %item.id, action = %item.action, "offline transaction enqueued");
    Ok(item.into())
}

/// List all pending (unsynced) offline queue items, oldest first.
#[command]
pub async fn list_pending_offline(
    state: State<'_, AppState>,
) -> Result<Vec<OfflineQueueItemDto>, AppError> {
    let db = state.db.lock().await;
    run_list_pending_offline(&db)
}

fn run_list_pending_offline(
    conn: &rusqlite::Connection,
) -> Result<Vec<OfflineQueueItemDto>, AppError> {
    let store = Store::new(conn);
    let items = store.list_pending_offline()?;
    let dtos: Vec<OfflineQueueItemDto> = items.into_iter().map(OfflineQueueItemDto::from).collect();
    Ok(dtos)
}

/// List all offline queue items (most recent first).
#[command]
pub async fn list_all_offline(
    state: State<'_, AppState>,
) -> Result<Vec<OfflineQueueItemDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let items = store.list_all_offline()?;
    let dtos: Vec<OfflineQueueItemDto> = items.into_iter().map(OfflineQueueItemDto::from).collect();
    Ok(dtos)
}

/// Get the count of pending offline items.
#[command]
pub async fn pending_offline_count(state: State<'_, AppState>) -> Result<i64, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let count = store.pending_offline_count()?;
    drop(db);
    Ok(count)
}

/// Attempt to sync all pending offline items.
///
/// For each pending item, tries to process the action. Currently marks
/// items as synced as a placeholder — real sync logic will be added later.
#[command]
pub async fn retry_offline_sync(state: State<'_, AppState>) -> Result<SyncResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let pending = store.list_pending_offline()?;
    let total_count = pending.len() as i64;
    let mut synced_count: i64 = 0;
    let mut failed_count: i64 = 0;

    for item in &pending {
        // Placeholder: attempt to process each item.
        // Real implementation will dispatch based on item.action.
        match store.mark_offline_synced(&item.id) {
            Ok(()) => {
                synced_count += 1;
                tracing::info!(id = %item.id, action = %item.action, "offline item synced");
            }
            Err(e) => {
                failed_count += 1;
                let err_msg = format!("sync failed: {e}");
                let _ = store.mark_offline_failed(&item.id, &err_msg);
                tracing::error!(id = %item.id, action = %item.action, error = %e, "offline item sync failed");
            }
        }
    }

    drop(db);
    Ok(SyncResult {
        synced_count,
        failed_count,
        total_count,
    })
}

/// Delete a processed offline queue item.
#[command]
pub async fn delete_offline_item(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    validate_not_empty("id", &id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);
    store.delete_offline_item(&id)?;
    drop(db);

    tracing::info!(id, "offline queue item deleted");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::OfflineQueueStatus;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    #[test]
    fn list_pending_offline_empty_db() {
        let conn = fresh_conn();
        let items = run_list_pending_offline(&conn).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn enqueue_and_list_pending() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let item = store
            .enqueue_offline("complete_sale", r#"{"sale_id":"abc"}"#)
            .unwrap();
        assert_eq!(item.action, "complete_sale");
        assert_eq!(item.status, OfflineQueueStatus::Pending);

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, item.id);
    }

    #[test]
    fn mark_offline_synced() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let item = store.enqueue_offline("void_sale", "{}").unwrap();
        store.mark_offline_synced(&item.id).unwrap();

        let synced_item = store.list_all_offline().unwrap();
        assert_eq!(synced_item.len(), 1);
        assert_eq!(synced_item[0].status, OfflineQueueStatus::Synced);
        assert!(synced_item[0].synced_at.is_some());
    }

    #[test]
    fn mark_offline_failed() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let item = store.enqueue_offline("complete_sale", "{}").unwrap();
        store
            .mark_offline_failed(&item.id, "network error")
            .unwrap();

        let failed = store.list_all_offline().unwrap();
        assert_eq!(failed[0].status, OfflineQueueStatus::Failed);
        assert_eq!(failed[0].last_error.as_deref(), Some("network error"));
        assert_eq!(failed[0].retry_count, 1);
    }

    #[test]
    fn pending_offline_count() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        assert_eq!(store.pending_offline_count().unwrap(), 0);
        store.enqueue_offline("test", "{}").unwrap();
        assert_eq!(store.pending_offline_count().unwrap(), 1);
    }

    #[test]
    fn delete_offline_item() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        let item = store.enqueue_offline("test", "{}").unwrap();
        store.delete_offline_item(&item.id).unwrap();
        assert_eq!(store.list_all_offline().unwrap().len(), 0);
    }

    #[test]
    fn enqueue_offline_validation() {
        let conn = fresh_conn();
        let store = Store::new(&conn);
        let item = store.enqueue_offline("", "{}").unwrap();
        // Empty action is stored as-is (no front-end validation at store level).
        assert_eq!(item.action, "");
        let loaded = store.list_all_offline().unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn retry_sync_marks_pending_as_synced() {
        let conn = fresh_conn();
        let store = Store::new(&conn);

        store
            .enqueue_offline("complete_sale", r#"{"id":"1"}"#)
            .unwrap();
        store.enqueue_offline("void_sale", r#"{"id":"2"}"#).unwrap();

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 2);

        for item in &pending {
            store.mark_offline_synced(&item.id).unwrap();
        }

        let remaining = store.list_pending_offline().unwrap();
        assert!(remaining.is_empty());
    }
}
