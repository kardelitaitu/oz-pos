//! Sync Queue — local change log for offline-first replication.
//!
//! Wraps the `oz_core` offline queue Store methods into a clean interface
//! with additional tracking for conflict resolution and last-sync timing.

use oz_core::db::Store;
use oz_core::offline::{OfflineQueueItem, OfflineQueueStatus};
use oz_core::error::CoreError;

/// A resolved item after conflict resolution — may be accepted from either
/// the local or remote side, or a merged version.
#[derive(Debug, Clone)]
pub struct ResolvedItem {
    /// The original local item, if applicable.
    pub local: Option<OfflineQueueItem>,
    /// The original remote item, if applicable.
    pub remote: Option<OfflineQueueItem>,
    /// The winning item to persist.
    pub winner: OfflineQueueItem,
}

/// Wraps the offline queue database operations with sync-specific helpers.
pub struct SyncQueue;

impl SyncQueue {
    /// Create a new sync queue interface.
    pub fn new() -> Self {
        Self
    }

    /// List all pending (unsynced) items, oldest first.
    pub fn list_pending(&self, store: &Store<'_>) -> Result<Vec<OfflineQueueItem>, CoreError> {
        store.list_pending_offline()
    }

    /// List all items (most recent first).
    pub fn list_all(&self, store: &Store<'_>) -> Result<Vec<OfflineQueueItem>, CoreError> {
        store.list_all_offline()
    }

    /// Enqueue a new offline transaction.
    pub fn enqueue(
        &self,
        store: &Store<'_>,
        action: &str,
        payload: &str,
    ) -> Result<OfflineQueueItem, CoreError> {
        store.enqueue_offline(action, payload)
    }

    /// Mark an item as successfully synced.
    pub fn mark_synced(&self, store: &Store<'_>, id: &str) -> Result<(), CoreError> {
        store.mark_offline_synced(id)
    }

    /// Mark an item as failed with an error message.
    pub fn mark_failed(
        &self,
        store: &Store<'_>,
        id: &str,
        error: &str,
    ) -> Result<(), CoreError> {
        store.mark_offline_failed(id, error)
    }

    /// Get the count of pending items.
    pub fn pending_count(&self, store: &Store<'_>) -> Result<i64, CoreError> {
        store.pending_offline_count()
    }

    /// Delete an item from the queue.
    pub fn delete(&self, store: &Store<'_>, id: &str) -> Result<(), CoreError> {
        store.delete_offline_item(id)
    }

    /// Get the timestamp of the most recently synced item.
    ///
    /// Returns `None` if nothing has been synced yet.
    pub fn last_synced_at(&self, store: &Store<'_>) -> Result<Option<String>, CoreError> {
        let all = store.list_all_offline()?;
        Ok(all
            .iter()
            .filter(|i| matches!(i.status, OfflineQueueStatus::Synced))
            .filter_map(|i| i.synced_at.as_deref())
            .max_by(|a, b| a.cmp(b))
            .map(|s| s.to_owned()))
    }

    /// Apply a conflict-resolution outcome to the queue.
    ///
    /// If the local item lost, it is marked as failed/superseded. If the
    /// winner is a merged item, a new queue entry is created.
    pub fn apply_resolution(
        &self,
        store: &Store<'_>,
        resolved: &ResolvedItem,
    ) -> Result<(), CoreError> {
        // Mark the local item as synced (the conflict was resolved)
        if let Some(ref local) = resolved.local {
            store.mark_offline_synced(&local.id)?;
        }
        // If the winner is a merged item (neither purely local nor remote),
        // enqueue it for the next sync cycle.
        let is_new_winner = match (&resolved.local, &resolved.remote) {
            (Some(local), _) if resolved.winner.id == local.id => false,
            (_, Some(remote)) if resolved.winner.id == remote.id => false,
            _ => true,
        };
        if is_new_winner {
            store.enqueue_offline(&resolved.winner.action, &resolved.winner.payload)?;
        }
        Ok(())
    }

    /// Apply a remote item to the local store.
    ///
    /// For now, this is a no-op placeholder — the remote server handles
    /// writing its own data. In a full bidirectional sync, this would
    /// apply remote updates to the local database.
    pub fn apply_remote(
        &self,
        _store: &Store<'_>,
        _item: &OfflineQueueItem,
    ) -> Result<(), CoreError> {
        // TODO(bidirectional-sync): apply remote mutations to local DB
        // (e.g. update product catalog from server, sync staff changes, etc.)
        Ok(())
    }
}

impl Default for SyncQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn setup_store() -> Store<'static> {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        let conn: &'static Connection = Box::leak(Box::new(conn));
        Store::new(conn)
    }

    #[test]
    fn queue_empty_pending() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let pending = queue.list_pending(&store).unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn queue_enqueue_and_list() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "complete_sale", r#"{"sale_id":"s1"}"#).unwrap();
        assert_eq!(item.action, "complete_sale");

        let pending = queue.list_pending(&store).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, item.id);
    }

    #[test]
    fn queue_mark_synced() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue.mark_synced(&store, &item.id).unwrap();

        let pending = queue.list_pending(&store).unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn queue_mark_failed() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue.mark_failed(&store, &item.id, "network error").unwrap();

        let all = queue.list_all(&store).unwrap();
        assert_eq!(all[0].status, OfflineQueueStatus::Failed);
    }

    #[test]
    fn queue_last_synced_at_none() {
        let store = setup_store();
        let queue = SyncQueue::new();
        assert!(queue.last_synced_at(&store).unwrap().is_none());
    }

    #[test]
    fn queue_last_synced_at_after_sync() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue.mark_synced(&store, &item.id).unwrap();
        assert!(queue.last_synced_at(&store).unwrap().is_some());
    }

    #[test]
    fn queue_delete_removes_item() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item = queue.enqueue(&store, "test", "{}").unwrap();
        queue.delete(&store, &item.id).unwrap();
        let all = queue.list_all(&store).unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn queue_delete_nonexistent_does_not_error() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let result = queue.delete(&store, "nonexistent-id");
        assert!(result.is_ok());
    }

    #[test]
    fn queue_pending_count() {
        let store = setup_store();
        let queue = SyncQueue::new();
        assert_eq!(queue.pending_count(&store).unwrap(), 0);

        queue.enqueue(&store, "a", "{}").unwrap();
        queue.enqueue(&store, "b", "{}").unwrap();
        assert_eq!(queue.pending_count(&store).unwrap(), 2);

        // After marking one synced, count decreases.
        let pending = queue.list_pending(&store).unwrap();
        queue.mark_synced(&store, &pending[0].id).unwrap();
        assert_eq!(queue.pending_count(&store).unwrap(), 1);
    }

    #[test]
    fn queue_list_all_returns_all_statuses() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item1 = queue.enqueue(&store, "a", "{}").unwrap();
        let _item2 = queue.enqueue(&store, "b", "{}").unwrap();

        queue.mark_synced(&store, &item1.id).unwrap();

        let all = queue.list_all(&store).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn queue_list_pending_returns_oldest_first() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item1 = queue.enqueue(&store, "first", "{}").unwrap();
        let item2 = queue.enqueue(&store, "second", "{}").unwrap();

        let pending = queue.list_pending(&store).unwrap();
        assert_eq!(pending[0].id, item1.id, "oldest item should be first");
        assert_eq!(pending[1].id, item2.id);
    }

    #[test]
    fn queue_last_synced_at_multiple_items() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let item1 = queue.enqueue(&store, "a", "{}").unwrap();
        let item2 = queue.enqueue(&store, "b", "{}").unwrap();

        queue.mark_synced(&store, &item1.id).unwrap();
        let ts1 = queue.last_synced_at(&store).unwrap().unwrap();

        queue.mark_synced(&store, &item2.id).unwrap();
        let ts2 = queue.last_synced_at(&store).unwrap().unwrap();

        // The timestamp of the most recently synced item should be >= the earlier one.
        assert!(ts2 >= ts1, "last synced at should increase");
    }

    #[test]
    fn queue_apply_resolution_local_wins() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let local = queue.enqueue(&store, "test", "{}").unwrap();

        let remote = OfflineQueueItem {
            id: uuid::Uuid::new_v4().to_string(),
            action: "test".into(),
            payload: "{}".into(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            synced_at: None,
        };

        let resolved = ResolvedItem {
            local: Some(local.clone()),
            remote: Some(remote),
            winner: local.clone(),
        };

        queue.apply_resolution(&store, &resolved).unwrap();

        // Local item should be marked synced.
        let all = store.list_all_offline().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, OfflineQueueStatus::Synced);
    }

    #[test]
    fn queue_apply_resolution_remote_wins() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let local = queue.enqueue(&store, "test", "{}").unwrap();

        let remote = OfflineQueueItem {
            id: uuid::Uuid::new_v4().to_string(),
            action: "test".into(),
            payload: r#"{"from":"server"}"#.into(),
            status: OfflineQueueStatus::Pending,
            retry_count: 0,
            last_error: None,
            created_at: "2025-06-01T12:00:00.000Z".into(),
            synced_at: None,
        };

        let resolved = ResolvedItem {
            local: Some(local.clone()),
            remote: Some(remote.clone()),
            winner: remote,
        };

        queue.apply_resolution(&store, &resolved).unwrap();

        // Local item should be marked synced. No new item enqueued because
        // the winner is the remote item (not a merge).
        let all = store.list_all_offline().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, OfflineQueueStatus::Synced);
    }

    #[test]
    fn queue_apply_remote_is_noop() {
        let store = setup_store();
        let queue = SyncQueue::new();
        let remote = OfflineQueueItem::new("remote_update", r#"{"data":"test"}"#);
        let result = queue.apply_remote(&store, &remote);
        assert!(result.is_ok(), "apply_remote should succeed");
        // Verify no items were created in the queue.
        let all = store.list_all_offline().unwrap();
        assert!(all.is_empty());
    }
}
