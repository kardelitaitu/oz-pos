//! Offline Queue — enqueue, list, mark, delete offline sync items.

use rusqlite::params;

use crate::error::CoreError;
use crate::offline::{OfflineQueueItem, OfflineQueueStatus};

use super::Store;

impl Store<'_> {
    /// Enqueue a transaction for later sync.
    pub fn enqueue_offline(&self, action: &str, payload: &str) -> Result<OfflineQueueItem, CoreError> {
        let item = OfflineQueueItem::new(action, payload);
        self.conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![item.id, item.action, item.payload, item.status.as_stored_str(), item.retry_count, item.last_error, item.created_at, item.synced_at],
        )?;
        Ok(item)
    }

    /// List all pending (unsynced) offline queue items, oldest first.
    pub fn list_pending_offline(&self) -> Result<Vec<OfflineQueueItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at
             FROM offline_queue WHERE status = 'pending' ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map([], Self::row_to_offline_queue_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List all offline queue items.
    pub fn list_all_offline(&self) -> Result<Vec<OfflineQueueItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at
             FROM offline_queue ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], Self::row_to_offline_queue_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Mark an offline queue item as synced.
    pub fn mark_offline_synced(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE offline_queue SET status = 'synced', synced_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound { entity: "offline_queue", id: id.to_owned() });
        }
        Ok(())
    }

    /// Mark an offline queue item as failed with an error message.
    pub fn mark_offline_failed(&self, id: &str, error: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE offline_queue SET status = 'failed', last_error = ?1, retry_count = retry_count + 1 WHERE id = ?2",
            params![error, id],
        )?;
        Ok(())
    }

    /// Get the count of pending offline items.
    pub fn pending_offline_count(&self) -> Result<i64, CoreError> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending'", [], |row| row.get(0),
        ).map_err(Into::into)
    }

    /// Delete a processed offline queue item.
    pub fn delete_offline_item(&self, id: &str) -> Result<(), CoreError> {
        self.conn.execute("DELETE FROM offline_queue WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn row_to_offline_queue_item(row: &rusqlite::Row) -> rusqlite::Result<OfflineQueueItem> {
        let status_str: String = row.get("status")?;
        Ok(OfflineQueueItem {
            id: row.get("id")?,
            action: row.get("action")?,
            payload: row.get("payload")?,
            status: OfflineQueueStatus::from_stored_str(&status_str).unwrap_or(OfflineQueueStatus::Pending),
            retry_count: row.get("retry_count")?,
            last_error: row.get("last_error")?,
            created_at: row.get("created_at")?,
            synced_at: row.get("synced_at")?,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn seed_pending_and_synced(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at) VALUES
                ('oq-1', 'sale.create', '{\"total\":100}', 'pending', 0, '', '2025-01-01T12:00:00.000Z', ''),
                ('oq-2', 'product.update', '{}', 'pending', 2, 'timeout', '2025-01-01T12:05:00.000Z', ''),
                ('oq-3', 'sale.void', '{\"id\":\"s-1\"}', 'synced', 0, '', '2025-01-01T11:00:00.000Z', '2025-01-01T11:01:00.000Z'),
                ('oq-4', 'sale.create', '{\"total\":200}', 'failed', 3, 'server error', '2025-01-01T10:00:00.000Z', '');"
        ).unwrap();
    }

    // ── Enqueue ─────────────────────────────────────────────────────

    #[test]
    fn enqueue_offline_creates_pending_item() {
        let conn = fresh();
        let s = store(&conn);
        let item = s.enqueue_offline("sale.create", "{\"total\":50}").unwrap();
        assert_eq!(item.action, "sale.create");
        assert_eq!(item.payload, "{\"total\":50}");
        assert_eq!(item.status, OfflineQueueStatus::Pending);
        assert_eq!(item.retry_count, 0);
        assert!(!item.id.is_empty());
        assert!(!item.created_at.is_empty());
    }

    #[test]
    fn enqueue_offline_persists_to_db() {
        let conn = fresh();
        let s = store(&conn);
        s.enqueue_offline("sale.create", "{}").unwrap();

        let items = s.list_all_offline().unwrap();
        assert_eq!(items.len(), 1);
    }

    // ── List pending ────────────────────────────────────────────────

    #[test]
    fn list_pending_offline_empty() {
        let conn = fresh();
        let items = store(&conn).list_pending_offline().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn list_pending_offline_returns_only_pending_oldest_first() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let items = store(&conn).list_pending_offline().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "oq-1");
        assert_eq!(items[1].id, "oq-2");
        assert_eq!(items[0].retry_count, 0);
        assert_eq!(items[1].retry_count, 2);
        assert_eq!(items[1].last_error.as_deref(), Some("timeout"));
    }

    // ── List all ────────────────────────────────────────────────────

    #[test]
    fn list_all_offline_returns_all_statuses_most_recent_first() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let items = store(&conn).list_all_offline().unwrap();
        assert_eq!(items.len(), 4);
        // Most recent first (created_at DESC).
        assert_eq!(items[0].id, "oq-2");
        assert_eq!(items[3].id, "oq-4");
    }

    // ── Mark synced ─────────────────────────────────────────────────

    #[test]
    fn mark_offline_synced_updates_status() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let s = store(&conn);

        s.mark_offline_synced("oq-1").unwrap();

        let items = s.list_pending_offline().unwrap();
        assert_eq!(items.len(), 1, "only oq-2 should still be pending");
    }

    #[test]
    fn mark_offline_synced_not_found() {
        let conn = fresh();
        let err = store(&conn).mark_offline_synced("nonexistent").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "offline_queue"));
    }

    #[test]
    fn mark_offline_synced_sets_timestamp() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let s = store(&conn);

        s.mark_offline_synced("oq-2").unwrap();

        let all = s.list_all_offline().unwrap();
        let item = all.into_iter().find(|i| i.id == "oq-2").unwrap();
        assert_eq!(item.status, OfflineQueueStatus::Synced);
        assert!(item.synced_at.is_some(), "synced_at should be populated");
    }

    // ── Mark failed ─────────────────────────────────────────────────

    #[test]
    fn mark_offline_failed_increments_retry() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let s = store(&conn);

        s.mark_offline_failed("oq-1", "network error").unwrap();

        let all = s.list_all_offline().unwrap();
        let item = all.into_iter().find(|i| i.id == "oq-1").unwrap();
        assert_eq!(item.status, OfflineQueueStatus::Failed);
        assert_eq!(item.retry_count, 1);
        assert_eq!(item.last_error.as_deref(), Some("network error"));
    }

    #[test]
    fn mark_offline_failed_increments_existing_retry() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let s = store(&conn);

        s.mark_offline_failed("oq-2", "another error").unwrap();

        let all = s.list_all_offline().unwrap();
        let item = all.into_iter().find(|i| i.id == "oq-2").unwrap();
        assert_eq!(item.retry_count, 3, "should increment from 2 to 3");
    }

    // ── Pending count ───────────────────────────────────────────────

    #[test]
    fn pending_offline_count_zero() {
        let conn = fresh();
        let count = store(&conn).pending_offline_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn pending_offline_count_matches() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let count = store(&conn).pending_offline_count().unwrap();
        assert_eq!(count, 2);
    }

    // ── Delete ──────────────────────────────────────────────────────

    #[test]
    fn delete_offline_item_removes() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let s = store(&conn);

        s.delete_offline_item("oq-1").unwrap();

        let all = s.list_all_offline().unwrap();
        assert_eq!(all.len(), 3);
        assert!(all.into_iter().all(|i| i.id != "oq-1"));
    }

    #[test]
    fn delete_offline_item_nonexistent_does_not_error() {
        let conn = fresh();
        let s = store(&conn);
        // Deleting a non-existent item should succeed (no error).
        s.delete_offline_item("nonexistent").unwrap();
    }
}
