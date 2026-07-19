//! Offline Queue — enqueue, list, mark, delete offline sync items.

use rusqlite::params;

use crate::error::CoreError;
use crate::offline::{OfflineQueueItem, OfflineQueueStatus, SyncPriority};

use super::Store;

impl Store<'_> {
    /// Enqueue a transaction for later sync (default tenant).
    pub fn enqueue_offline(
        &self,
        action: &str,
        payload: &str,
    ) -> Result<OfflineQueueItem, CoreError> {
        self.enqueue_offline_with_tenant(action, payload, "default")
    }

    /// Enqueue a transaction with dedup by action + payload.
    ///
    /// If a pending item with the same `action` and `payload` already
    /// exists, returns `Ok(None)` — no duplicate is created.
    /// Otherwise, enqueues normally and returns `Ok(Some(item))`.
    ///
    /// This prevents duplicate entries when the same sale completion,
    /// void, or adjustment is enqueued multiple times (e.g. due to
    /// network retry or cross-terminal propagation).
    pub fn enqueue_offline_dedup(
        &self,
        action: &str,
        payload: &str,
    ) -> Result<Option<OfflineQueueItem>, CoreError> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM offline_queue
                  WHERE status = 'pending' AND action = ?1 AND payload = ?2)",
                params![action, payload],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            return Ok(None);
        }
        self.enqueue_offline(action, payload).map(Some)
    }

    /// Enqueue a transaction for later sync, scoped to the given tenant.
    pub fn enqueue_offline_with_tenant(
        &self,
        action: &str,
        payload: &str,
        tenant_id: &str,
    ) -> Result<OfflineQueueItem, CoreError> {
        self.enqueue_offline_inner(action, payload, tenant_id, SyncPriority::Normal)
    }

    /// Enqueue a transaction with a specific sync priority (P-2).
    pub fn enqueue_offline_priority(
        &self,
        action: &str,
        payload: &str,
        priority: SyncPriority,
    ) -> Result<OfflineQueueItem, CoreError> {
        self.enqueue_offline_inner(action, payload, "default", priority)
    }

    fn enqueue_offline_inner(
        &self,
        action: &str,
        payload: &str,
        tenant_id: &str,
        priority: SyncPriority,
    ) -> Result<OfflineQueueItem, CoreError> {
        let mut item = OfflineQueueItem::with_tenant(action, payload, tenant_id);
        item.priority = priority;
        self.conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![item.id, item.action, item.payload, item.status.as_stored_str(), item.retry_count, item.last_error, item.created_at, item.synced_at, item.tenant_id, item.priority as i32],
        )?;
        Ok(item)
    }

    /// List all pending (unsynced) offline queue items, oldest first.
    pub fn list_pending_offline(&self) -> Result<Vec<OfflineQueueItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority
             FROM offline_queue WHERE status = 'pending' ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], Self::row_to_offline_queue_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List all offline queue items.
    pub fn list_all_offline(&self) -> Result<Vec<OfflineQueueItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority
             FROM offline_queue ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], Self::row_to_offline_queue_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List pending offline items scoped to a tenant.
    pub fn list_pending_offline_for_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<OfflineQueueItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority
             FROM offline_queue WHERE status = 'pending' AND tenant_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id], Self::row_to_offline_queue_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Mark an offline queue item as synced.
    pub fn mark_offline_synced(&self, id: &str) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE offline_queue SET status = 'synced', synced_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "offline_queue",
                id: id.to_owned(),
            });
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
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    /// Delete a processed offline queue item.
    pub fn delete_offline_item(&self, id: &str) -> Result<(), CoreError> {
        self.conn
            .execute("DELETE FROM offline_queue WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn row_to_offline_queue_item(row: &rusqlite::Row) -> rusqlite::Result<OfflineQueueItem> {
        let status_str: String = row.get("status")?;
        Ok(OfflineQueueItem {
            id: row.get("id")?,
            action: row.get("action")?,
            payload: row.get("payload")?,
            status: OfflineQueueStatus::from_stored_str(&status_str)
                .unwrap_or(OfflineQueueStatus::Pending),
            retry_count: row.get("retry_count")?,
            last_error: row.get("last_error")?,
            created_at: row.get("created_at")?,
            synced_at: row.get("synced_at")?,
            tenant_id: row.get("tenant_id")?,
            priority: row
                .get::<_, i32>("priority")
                .map(crate::offline::SyncPriority::from)
                .unwrap_or(crate::offline::SyncPriority::Normal),
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
        migrations::fresh_db()
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

    // ── Additional edge cases ─────────────────────────────────────

    #[test]
    fn enqueue_offline_with_tenant_sets_tenant_id() {
        let conn = fresh();
        let s = store(&conn);
        let item = s
            .enqueue_offline_with_tenant("sale.create", "{}", "tenant-a")
            .unwrap();
        assert_eq!(item.tenant_id, "tenant-a");
        assert_eq!(item.action, "sale.create");
    }

    #[test]
    fn enqueue_offline_priority_roundtrip() {
        let conn = fresh();
        let s = store(&conn);
        let item = s
            .enqueue_offline_priority("payment.sync", "{}", SyncPriority::Critical)
            .unwrap();
        assert_eq!(item.priority, SyncPriority::Critical);
        let item = s
            .enqueue_offline_priority("audit.log", "{}", SyncPriority::Low)
            .unwrap();
        assert_eq!(item.priority, SyncPriority::Low);
        // Default is Normal.
        let item = s
            .enqueue_offline_priority("default", "{}", SyncPriority::Normal)
            .unwrap();
        assert_eq!(item.priority, SyncPriority::Normal);
    }

    #[test]
    fn list_pending_offline_for_tenant_filters() {
        let conn = fresh();
        let s = store(&conn);

        // Enqueue items for different tenants.
        s.enqueue_offline_with_tenant("sale.create", "{}", "tenant-a")
            .unwrap();
        s.enqueue_offline_with_tenant("product.update", "{}", "tenant-b")
            .unwrap();
        s.enqueue_offline_with_tenant("sale.void", "{}", "tenant-a")
            .unwrap();

        let a_items = s.list_pending_offline_for_tenant("tenant-a").unwrap();
        assert_eq!(a_items.len(), 2);
        assert!(a_items.iter().all(|i| i.tenant_id == "tenant-a"));

        let b_items = s.list_pending_offline_for_tenant("tenant-b").unwrap();
        assert_eq!(b_items.len(), 1);
    }

    #[test]
    fn list_pending_offline_for_tenant_empty() {
        let conn = fresh();
        let s = store(&conn);
        let items = s.list_pending_offline_for_tenant("no-such-tenant").unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn mark_offline_failed_nonexistent_noop() {
        let conn = fresh();
        let s = store(&conn);
        // mark_offline_failed doesn't check affected rows, so this should be a no-op.
        s.mark_offline_failed("nonexistent", "test error").unwrap();
        // Verify state unchanged.
        let count = s.pending_offline_count().unwrap();
        assert_eq!(count, 0);
    }

    // ── Dedup tests ───────────────────────────────────────────────────

    #[test]
    fn enqueue_dedup_first_call_inserts() {
        let conn = fresh();
        let s = store(&conn);
        let result = s
            .enqueue_offline_dedup("complete_sale", r#"{"sale_id":"s-1"}"#)
            .unwrap();
        assert!(result.is_some(), "first call should enqueue");
        let count = s.pending_offline_count().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn enqueue_dedup_second_call_skips() {
        let conn = fresh();
        let s = store(&conn);

        // First call — inserts
        let first = s
            .enqueue_offline_dedup("complete_sale", r#"{"sale_id":"s-1"}"#)
            .unwrap();
        assert!(first.is_some());

        // Second call — dedup skips
        let second = s
            .enqueue_offline_dedup("complete_sale", r#"{"sale_id":"s-1"}"#)
            .unwrap();
        assert!(second.is_none(), "duplicate should be deduped");

        let count = s.pending_offline_count().unwrap();
        assert_eq!(count, 1, "only one item should be pending");
    }

    #[test]
    fn enqueue_dedup_same_action_different_payload_passes() {
        let conn = fresh();
        let s = store(&conn);

        let first = s
            .enqueue_offline_dedup("complete_sale", r#"{"sale_id":"s-1"}"#)
            .unwrap();
        assert!(first.is_some());

        // Different sale_id — should insert
        let second = s
            .enqueue_offline_dedup("complete_sale", r#"{"sale_id":"s-2"}"#)
            .unwrap();
        assert!(second.is_some(), "different payload should not be deduped");

        let count = s.pending_offline_count().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn enqueue_dedup_different_action_same_payload_passes() {
        let conn = fresh();
        let s = store(&conn);

        let first = s
            .enqueue_offline_dedup("complete_sale", r#"{"id":"x"}"#)
            .unwrap();
        assert!(first.is_some());

        // Different action — should insert
        let second = s
            .enqueue_offline_dedup("void_sale", r#"{"id":"x"}"#)
            .unwrap();
        assert!(second.is_some(), "different action should not be deduped");

        let count = s.pending_offline_count().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn enqueue_dedup_synced_item_does_not_block() {
        let conn = fresh();
        let s = store(&conn);

        // Enqueue, mark synced, then try to enqueue same again
        let first = s
            .enqueue_offline_dedup("complete_sale", r#"{"sale_id":"s-1"}"#)
            .unwrap();
        assert!(first.is_some());
        let id = first.as_ref().unwrap().id.clone();
        s.mark_offline_synced(&id).unwrap();

        // Same action+payload — but the original is synced, not pending
        let second = s
            .enqueue_offline_dedup("complete_sale", r#"{"sale_id":"s-1"}"#)
            .unwrap();
        // The original item is synced so this should be treated as a new item.
        // (We only dedup against items still pending.)
        assert!(second.is_some(), "synced item should not block re-enqueue");
    }

    #[test]
    fn enqueue_dedup_cross_terminal_scenario() {
        // Simulate: Terminal A enqueues sale, Terminal B receives it via
        // sync and tries to re-enqueue. The dedup should prevent the
        // duplicate if the payload is byte-identical.
        let conn = fresh();
        let s = store(&conn);

        // Terminal A completes the sale
        let payload = r#"{"sale_id":"s-A-1","items":[{"sku":"COFFEE","qty":2}]}"#;
        let result = s.enqueue_offline_dedup("complete_sale", payload).unwrap();
        assert!(result.is_some(), "Terminal A: first enqueue should succeed");

        // Same sale arrives from Terminal B via sync (byte-identical payload)
        let result = s.enqueue_offline_dedup("complete_sale", payload).unwrap();
        assert!(result.is_none(), "Terminal B: duplicate should be deduped");

        let count = s.pending_offline_count().unwrap();
        assert_eq!(count, 1, "only one pending item after cross-terminal dedup");
    }

    #[test]
    fn list_all_offline_empty_db() {
        let conn = fresh();
        let items = store(&conn).list_all_offline().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn delete_offline_item_only_removes_target() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let s = store(&conn);

        s.delete_offline_item("oq-1").unwrap();
        let remaining = s.list_all_offline().unwrap();
        assert_eq!(remaining.len(), 3);
        assert!(remaining.iter().all(|i| i.id != "oq-1"));
        // oq-2, oq-3, oq-4 should still be present.
        assert!(remaining.iter().any(|i| i.id == "oq-2"));
        assert!(remaining.iter().any(|i| i.id == "oq-3"));
        assert!(remaining.iter().any(|i| i.id == "oq-4"));
    }
}
