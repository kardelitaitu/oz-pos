//! Offline Queue — enqueue, list, mark, delete offline sync items.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::offline::{OfflineQueueItem, OfflineQueueStatus, SyncPriority};

use super::Store;

/// Summary of offline queue status — counts by status and sync timing.
/// Used by P1-6 sync observability dashboard widgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusSummary {
    /// Number of pending (unsynced) items.
    pub pending_count: i64,
    /// Number of successfully synced items.
    pub synced_count: i64,
    /// Number of failed items.
    pub failed_count: i64,
    /// Total retry count across all failed items.
    pub total_retry_count: i64,
    /// ISO-8601 timestamp of the most recently synced item, if any.
    pub last_synced_at: Option<String>,
    /// ISO-8601 timestamp of the oldest pending item, if any.
    pub oldest_pending_at: Option<String>,
    /// Number of items resolved via conflict during the last sync cycle.
    /// (P1-3: items whose last_error starts with "resolved: conflict").
    pub conflict_count: i64,
}

/// Durable pull anchor/cursor for the background sync daemon (SYNC-01).
///
/// Persisted in the single-row `sync_pull_state` table so the daemon only
/// fetches remote updates newer than the last successfully-applied page
/// (plus the opaque pagination cursor for the next page, P-3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncPullState {
    /// ISO-8601 anchor timestamp of the last successfully applied page.
    pub since: Option<String>,
    /// Opaque pagination cursor for the next page (P-3). `None` when the
    /// previous page was the final one.
    pub cursor: Option<String>,
}

/// A retained failure from applying a remote sync item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteSyncFailure {
    /// Remote item identifier.
    pub item_id: String,
    /// Remote action name.
    pub action: String,
    /// Original payload retained for operator inspection.
    pub payload: String,
    /// Number of failed application attempts.
    pub attempts: i64,
    /// Most recent application error.
    pub last_error: String,
    /// Whether retry is exhausted and the item is quarantined.
    pub dead_lettered: bool,
}

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

    /// Enqueue a transaction scoped to a tenant with a specific priority.
    ///
    /// OFF-09: this is the combined tenant + priority entry point so the
    /// command boundary can preserve both multi-store isolation and the
    /// P-2 priority tier in a single call.
    pub fn enqueue_offline_scoped(
        &self,
        action: &str,
        payload: &str,
        tenant_id: &str,
        priority: SyncPriority,
    ) -> Result<OfflineQueueItem, CoreError> {
        self.enqueue_offline_inner(action, payload, tenant_id, priority)
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

    /// Mark an offline queue item as synced, scoped to a tenant (SYNC-07).
    ///
    /// Returns [`CoreError::NotFound`] when the id does not exist **or**
    /// belongs to a different tenant — a cross-tenant mutation is treated
    /// exactly like a missing item so the client queue boundary is safe by
    /// construction even in a multi-tenant process.
    pub fn mark_offline_synced_for_tenant(
        &self,
        id: &str,
        tenant_id: &str,
    ) -> Result<(), CoreError> {
        let affected = self.conn.execute(
            "UPDATE offline_queue SET status = 'synced', synced_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND tenant_id = ?2",
            params![id, tenant_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "offline_queue",
                id: id.to_owned(),
            });
        }
        Ok(())
    }

    /// Mark an offline queue item as resolved via conflict (P1-3).
    ///
    /// Sets status to 'synced' and records the resolution type in
    /// `last_error` so the status summary can count conflict resolutions.
    pub fn mark_offline_resolved(&self, id: &str, resolution: &str) -> Result<(), CoreError> {
        let marker = format!("resolved: conflict ({})", resolution);
        let affected = self.conn.execute(
            "UPDATE offline_queue SET status = 'synced', synced_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), last_error = ?1 WHERE id = ?2",
            params![marker, id],
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

    /// Mark an offline queue item as failed, scoped to a tenant (SYNC-07).
    ///
    /// A cross-tenant id is a no-op (`Ok(())`), matching the unscoped
    /// variant's lenient semantics but never mutating another tenant's row.
    pub fn mark_offline_failed_for_tenant(
        &self,
        id: &str,
        tenant_id: &str,
        error: &str,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "UPDATE offline_queue SET status = 'failed', last_error = ?1, retry_count = retry_count + 1
             WHERE id = ?2 AND tenant_id = ?3",
            params![error, id, tenant_id],
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

    /// Get the count of pending offline items scoped to a tenant (SYNC-07).
    pub fn pending_offline_count_for_tenant(&self, tenant_id: &str) -> Result<i64, CoreError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending' AND tenant_id = ?1",
                params![tenant_id],
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

    /// Delete an offline queue item, scoped to a tenant (SYNC-07).
    ///
    /// A cross-tenant id is a no-op — the row (if any) is left untouched.
    pub fn delete_offline_item_for_tenant(
        &self,
        id: &str,
        tenant_id: &str,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM offline_queue WHERE id = ?1 AND tenant_id = ?2",
            params![id, tenant_id],
        )?;
        Ok(())
    }

    /// Get a summary of the offline queue status (P1-6 sync observability).
    ///
    /// Returns counts by status, total retry count, last sync timestamp,
    /// and oldest pending timestamp — all in a single query.
    pub fn offline_queue_status_summary(&self) -> Result<SyncStatusSummary, CoreError> {
        // Status counts
        let counts: Vec<(String, i64)> = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM offline_queue GROUP BY status")?
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut pending_count: i64 = 0;
        let mut synced_count: i64 = 0;
        let mut failed_count: i64 = 0;
        for (status, count) in &counts {
            match status.as_str() {
                "pending" => pending_count = *count,
                "synced" => synced_count = *count,
                "failed" => failed_count = *count,
                _ => {}
            }
        }

        // Total retry count across all failed items
        let total_retry_count: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(retry_count), 0) FROM offline_queue WHERE status = 'failed'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Last synced at (most recent synced_at timestamp)
        let last_synced_at: Option<String> = self
            .conn
            .query_row(
                "SELECT synced_at FROM offline_queue WHERE status = 'synced' AND synced_at IS NOT NULL ORDER BY synced_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        // Oldest pending at (earliest created_at among pending items)
        let oldest_pending_at: Option<String> = self
            .conn
            .query_row(
                "SELECT created_at FROM offline_queue WHERE status = 'pending' ORDER BY created_at ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        // P1-3: Count items resolved via conflict (last_error starts with "resolved: conflict")
        let conflict_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM offline_queue WHERE last_error LIKE 'resolved: conflict%'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(SyncStatusSummary {
            pending_count,
            synced_count,
            failed_count,
            total_retry_count,
            last_synced_at,
            oldest_pending_at,
            conflict_count,
        })
    }

    /// Read the persisted sync pull anchor and cursor (SYNC-01).
    ///
    /// Returns the `since` timestamp and `cursor` from the last
    /// successfully-applied page. Both are `None` on first sync (pull
    /// everything). A missing row (pre-114 database) defaults to `None`.
    pub fn get_sync_pull_state(&self) -> Result<SyncPullState, CoreError> {
        use rusqlite::OptionalExtension;
        self.conn
            .query_row(
                "SELECT since, cursor FROM sync_pull_state WHERE id = 1",
                [],
                |row| {
                    Ok(SyncPullState {
                        since: row.get(0)?,
                        cursor: row.get(1)?,
                    })
                },
            )
            .optional()
            .map(|row| row.unwrap_or_default())
            .map_err(Into::into)
    }

    /// Persist the sync pull anchor and cursor (SYNC-01).
    ///
    /// Called only AFTER a page of remote items was applied successfully,
    /// so a crash mid-pull replays safely — the idempotency ledger then
    /// skips any already-applied items.
    pub fn set_sync_pull_state(
        &self,
        since: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT INTO sync_pull_state (id, since, cursor) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET since = excluded.since, cursor = excluded.cursor",
            params![since, cursor],
        )?;
        Ok(())
    }

    /// Check whether a remote item has already been applied locally (SYNC-01).
    pub fn is_remote_item_applied(&self, item_id: &str) -> Result<bool, CoreError> {
        self.conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sync_applied_items WHERE item_id = ?1)",
                params![item_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    /// Record a remote item as applied locally (SYNC-01 idempotency ledger).
    ///
    /// `INSERT OR IGNORE` — re-recording the same id is a no-op, so replay
    /// of a page never double-counts a mutation.
    pub fn mark_remote_item_applied(&self, item_id: &str, action: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO sync_applied_items (item_id, action) VALUES (?1, ?2)",
            params![item_id, action],
        )?;
        Ok(())
    }

    /// Record a remote application failure and advance its retry/dead-letter state.
    ///
    /// The payload is retained for operator inspection. Once `max_attempts`
    /// is reached, the item is quarantined and no longer eligible for page
    /// application until a future explicit operator requeue workflow is added.
    pub fn record_remote_failure(
        &self,
        item_id: &str,
        action: &str,
        payload: &str,
        error: &str,
        max_attempts: i64,
    ) -> Result<bool, CoreError> {
        let max_attempts = max_attempts.max(1);
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO sync_remote_failures
                (item_id, action, payload, attempts, last_error, dead_lettered)
             VALUES (?1, ?2, ?3, 1, ?4, CASE WHEN 1 >= ?5 THEN 1 ELSE 0 END)
             ON CONFLICT(item_id) DO UPDATE SET
                action = excluded.action,
                payload = excluded.payload,
                attempts = sync_remote_failures.attempts + 1,
                last_error = excluded.last_error,
                last_failed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                dead_lettered = CASE
                    WHEN sync_remote_failures.attempts + 1 >= ?5 THEN 1
                    ELSE 0
                END",
            params![item_id, action, payload, error, max_attempts],
        )?;
        let dead_lettered: bool = tx.query_row(
            "SELECT dead_lettered FROM sync_remote_failures WHERE item_id = ?1",
            params![item_id],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(dead_lettered)
    }

    /// List retained remote application failures, newest failure first.
    pub fn list_remote_failures(&self) -> Result<Vec<RemoteSyncFailure>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT item_id, action, payload, attempts, last_error, dead_lettered
             FROM sync_remote_failures ORDER BY last_failed_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RemoteSyncFailure {
                item_id: row.get(0)?,
                action: row.get(1)?,
                payload: row.get(2)?,
                attempts: row.get(3)?,
                last_error: row.get(4)?,
                dead_lettered: row.get::<_, i64>(5)? != 0,
            })
        })?;
        rows.map(|row| row.map_err(CoreError::from)).collect()
    }

    /// Return whether a remote item has been quarantined as a dead letter.
    pub fn is_remote_failure_dead_lettered(&self, item_id: &str) -> Result<bool, CoreError> {
        self.conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sync_remote_failures WHERE item_id = ?1 AND dead_lettered = 1)",
                params![item_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    /// Clear a resolved remote failure after its item is applied successfully.
    pub fn clear_remote_failure(&self, item_id: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        self.clear_remote_failure_in_tx(&tx, item_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Clear a remote failure using a caller-owned transaction.
    pub fn clear_remote_failure_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        item_id: &str,
    ) -> Result<(), CoreError> {
        tx.execute(
            "DELETE FROM sync_remote_failures WHERE item_id = ?1",
            params![item_id],
        )?;
        Ok(())
    }

    /// Record a remote item using a caller-owned transaction.
    ///
    /// The sync applier uses this method in the same transaction as the
    /// domain mutation, preventing a crash between mutation and receipt from
    /// causing a second application on replay.
    pub fn mark_remote_item_applied_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        item_id: &str,
        action: &str,
    ) -> Result<(), CoreError> {
        tx.execute(
            "INSERT OR IGNORE INTO sync_applied_items (item_id, action) VALUES (?1, ?2)",
            params![item_id, action],
        )?;
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
    fn enqueue_offline_scoped_combines_tenant_and_priority() {
        // OFF-09: the combined tenant + priority entry point the command
        // boundary uses must persist both fields on the same row.
        let conn = fresh();
        let s = store(&conn);
        let item = s
            .enqueue_offline_scoped("complete_sale", "{}", "store-a", SyncPriority::Critical)
            .unwrap();
        assert_eq!(item.tenant_id, "store-a");
        assert_eq!(item.priority, SyncPriority::Critical);

        let loaded = s.list_pending_offline_for_tenant("store-a").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].priority, SyncPriority::Critical);

        // A different tenant never sees it.
        let other = s.list_pending_offline_for_tenant("store-b").unwrap();
        assert!(other.is_empty());
    }

    #[test]
    fn pending_batch_orders_critical_before_normal_before_low() {
        // OFF-09: the retry command sorts the batch by priority so Critical
        // items always transmit first. Pins the ordering contract on the
        // raw items returned by the store.
        let conn = fresh();
        let s = store(&conn);
        s.enqueue_offline_scoped("settings.change", "{}", "default", SyncPriority::Low)
            .unwrap();
        s.enqueue_offline_scoped("complete_sale", "{}", "default", SyncPriority::Critical)
            .unwrap();
        s.enqueue_offline_scoped("product.update", "{}", "default", SyncPriority::Normal)
            .unwrap();

        let mut batch = s.list_pending_offline().unwrap();
        batch.sort_by_key(|i| i.priority);
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[0].priority, SyncPriority::Critical);
        assert_eq!(batch[1].priority, SyncPriority::Normal);
        assert_eq!(batch[2].priority, SyncPriority::Low);
    }

    #[test]
    fn list_pending_offline_for_tenant_empty() {
        let conn = fresh();
        let s = store(&conn);
        let items = s.list_pending_offline_for_tenant("no-such-tenant").unwrap();
        assert!(items.is_empty());
    }

    // ── SYNC-07: two-tenant boundary through the client queue ────────

    #[test]
    fn tenant_scoped_count_isolates_tenants() {
        let conn = fresh();
        let s = store(&conn);

        s.enqueue_offline_with_tenant("sale.create", "{}", "tenant-a")
            .unwrap();
        s.enqueue_offline_with_tenant("sale.create", "{}", "tenant-a")
            .unwrap();
        s.enqueue_offline_with_tenant("product.update", "{}", "tenant-b")
            .unwrap();

        assert_eq!(s.pending_offline_count_for_tenant("tenant-a").unwrap(), 2);
        assert_eq!(s.pending_offline_count_for_tenant("tenant-b").unwrap(), 1);
        assert_eq!(s.pending_offline_count_for_tenant("tenant-c").unwrap(), 0);
    }

    #[test]
    fn tenant_scoped_mark_synced_refuses_cross_tenant() {
        let conn = fresh();
        let s = store(&conn);

        let item = s
            .enqueue_offline_with_tenant("sale.create", "{}", "tenant-a")
            .unwrap();

        // Correct tenant: succeeds.
        s.mark_offline_synced_for_tenant(&item.id, "tenant-a")
            .unwrap();
        assert_eq!(s.pending_offline_count().unwrap(), 0);

        // Cross-tenant (re-insert, then attempt from tenant-b): NotFound,
        // and the row stays untouched (still pending).
        let item2 = s
            .enqueue_offline_with_tenant("sale.create", "{}", "tenant-a")
            .unwrap();
        let err = s
            .mark_offline_synced_for_tenant(&item2.id, "tenant-b")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "offline_queue"));
        assert_eq!(s.pending_offline_count_for_tenant("tenant-a").unwrap(), 1);
    }

    #[test]
    fn tenant_scoped_mark_failed_does_not_touch_other_tenant() {
        let conn = fresh();
        let s = store(&conn);

        let item = s
            .enqueue_offline_with_tenant("sale.create", "{}", "tenant-a")
            .unwrap();

        // Cross-tenant mark-failed is a no-op: status stays pending.
        s.mark_offline_failed_for_tenant(&item.id, "tenant-b", "boom")
            .unwrap();
        let all = s.list_all_offline().unwrap();
        let row = all.into_iter().find(|i| i.id == item.id).unwrap();
        assert_eq!(row.status, OfflineQueueStatus::Pending);
        assert_eq!(row.retry_count, 0);

        // Correct tenant: marks failed + increments retry.
        s.mark_offline_failed_for_tenant(&item.id, "tenant-a", "boom")
            .unwrap();
        let all = s.list_all_offline().unwrap();
        let row = all.into_iter().find(|i| i.id == item.id).unwrap();
        assert_eq!(row.status, OfflineQueueStatus::Failed);
        assert_eq!(row.retry_count, 1);
    }

    #[test]
    fn tenant_scoped_delete_does_not_touch_other_tenant() {
        let conn = fresh();
        let s = store(&conn);

        let item = s
            .enqueue_offline_with_tenant("sale.create", "{}", "tenant-a")
            .unwrap();

        // Cross-tenant delete is a no-op: row survives.
        s.delete_offline_item_for_tenant(&item.id, "tenant-b")
            .unwrap();
        assert_eq!(s.pending_offline_count().unwrap(), 1);

        // Correct tenant: row removed.
        s.delete_offline_item_for_tenant(&item.id, "tenant-a")
            .unwrap();
        assert_eq!(s.pending_offline_count().unwrap(), 0);
    }

    // ── SYNC-01: durable pull anchor + idempotency ledger ────────────

    #[test]
    fn sync_pull_state_defaults_to_none() {
        let conn = fresh();
        let st = store(&conn).get_sync_pull_state().unwrap();
        assert!(st.since.is_none());
        assert!(st.cursor.is_none());
    }

    #[test]
    fn sync_pull_state_roundtrip() {
        let conn = fresh();
        let s = store(&conn);
        s.set_sync_pull_state(Some("2026-01-01T00:00:00Z"), None)
            .unwrap();
        let st = s.get_sync_pull_state().unwrap();
        assert_eq!(st.since.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert!(st.cursor.is_none());

        // Single-row guard: overwrite, never insert a second row.
        s.set_sync_pull_state(
            Some("2026-02-01T00:00:00Z"),
            Some("2026-02-01T00:00:00Z|abc"),
        )
        .unwrap();
        let st = s.get_sync_pull_state().unwrap();
        assert_eq!(st.since.as_deref(), Some("2026-02-01T00:00:00Z"));
        assert_eq!(st.cursor.as_deref(), Some("2026-02-01T00:00:00Z|abc"));

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_pull_state", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "sync_pull_state must stay a single row");
    }

    #[test]
    fn sync_applied_items_tracks_ids() {
        let conn = fresh();
        let s = store(&conn);
        assert!(!s.is_remote_item_applied("item-1").unwrap());

        s.mark_remote_item_applied("item-1", "stock.adjusted")
            .unwrap();
        assert!(s.is_remote_item_applied("item-1").unwrap());

        // INSERT OR IGNORE — replay is a no-op.
        s.mark_remote_item_applied("item-1", "stock.adjusted")
            .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_applied_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count, 1,
            "re-applying the same item must not duplicate the ledger row"
        );
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

    // ── P1-6: SyncStatusSummary tests ────────────────────────────────

    #[test]
    fn status_summary_empty_db() {
        let conn = fresh();
        let s = store(&conn);
        let summary = s.offline_queue_status_summary().unwrap();
        assert_eq!(summary.pending_count, 0);
        assert_eq!(summary.synced_count, 0);
        assert_eq!(summary.failed_count, 0);
        assert_eq!(summary.total_retry_count, 0);
        assert!(summary.last_synced_at.is_none());
        assert!(summary.oldest_pending_at.is_none());
    }

    #[test]
    fn status_summary_with_seeded_data() {
        let conn = fresh();
        seed_pending_and_synced(&conn);
        let s = store(&conn);
        let summary = s.offline_queue_status_summary().unwrap();

        // oq-1 (pending), oq-2 (pending), oq-3 (synced), oq-4 (failed)
        assert_eq!(summary.pending_count, 2);
        assert_eq!(summary.synced_count, 1);
        assert_eq!(summary.failed_count, 1);
        // oq-4 has retry_count = 3
        assert_eq!(summary.total_retry_count, 3);

        // oq-3 is synced at '2025-01-01T11:01:00.000Z'
        assert_eq!(
            summary.last_synced_at.as_deref(),
            Some("2025-01-01T11:01:00.000Z")
        );

        // oq-1 is the oldest pending at '2025-01-01T12:00:00.000Z'
        assert_eq!(
            summary.oldest_pending_at.as_deref(),
            Some("2025-01-01T12:00:00.000Z")
        );
    }

    #[test]
    fn status_summary_updates_after_operations() {
        let conn = fresh();
        let s = store(&conn);

        // Empty
        let summary = s.offline_queue_status_summary().unwrap();
        assert_eq!(summary.pending_count, 0);

        // Enqueue an item
        let item = s.enqueue_offline("test", "{}").unwrap();
        let summary = s.offline_queue_status_summary().unwrap();
        assert_eq!(summary.pending_count, 1);
        assert_eq!(summary.synced_count, 0);
        assert!(summary.oldest_pending_at.is_some());

        // Mark it synced
        s.mark_offline_synced(&item.id).unwrap();
        let summary = s.offline_queue_status_summary().unwrap();
        assert_eq!(summary.pending_count, 0);
        assert_eq!(summary.synced_count, 1);
        assert!(summary.last_synced_at.is_some());
    }

    #[test]
    fn status_summary_total_retry_across_multiple_failed() {
        let conn = fresh();
        let s = store(&conn);

        // Insert two failed items with retry counts
        s.enqueue_offline("a", "{}").unwrap();
        let b = s.enqueue_offline("b", "{}").unwrap();
        s.mark_offline_failed(&b.id, "err").unwrap();
        s.mark_offline_failed(&b.id, "err").unwrap();

        let summary = s.offline_queue_status_summary().unwrap();
        assert_eq!(summary.failed_count, 1);
        assert_eq!(summary.total_retry_count, 2);
    }

    #[test]
    fn status_summary_serde_roundtrip() {
        let summary = SyncStatusSummary {
            pending_count: 5,
            synced_count: 10,
            failed_count: 2,
            total_retry_count: 7,
            last_synced_at: Some("2025-06-01T12:00:00Z".into()),
            oldest_pending_at: None,
            conflict_count: 0,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let rt: SyncStatusSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.pending_count, 5);
        assert_eq!(rt.synced_count, 10);
        assert_eq!(rt.failed_count, 2);
        assert_eq!(rt.total_retry_count, 7);
    }

    #[test]
    fn status_summary_debug_output() {
        let summary = SyncStatusSummary {
            pending_count: 1,
            synced_count: 2,
            failed_count: 0,
            total_retry_count: 0,
            last_synced_at: None,
            oldest_pending_at: None,
            conflict_count: 0,
        };
        let debug = format!("{summary:?}");
        assert!(debug.contains("pending_count: 1"));
        assert!(debug.contains("synced_count: 2"));
    }
}
