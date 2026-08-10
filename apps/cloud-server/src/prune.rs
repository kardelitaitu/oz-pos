//! Background pruning task for the cloud sync server.
//!
//! Runs on an hourly interval, calling [`Store::archive_stock_movements`]
//! to consolidate delta ledger rows older than 90 days into the archive
//! table (ADR #6 Q4 / P-1 Ledger Retention).
//!
//! Also prunes the `offline_queue` table — deleting items older than
//! 90 days regardless of status (P-1 Retention).

use std::sync::Arc;
use std::time::Duration;

use oz_core::db::Store;
use rusqlite::Connection;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Start the background prune loop on a shared database connection.
///
/// Spawns a `tokio` task that runs every hour. Each cycle:
/// 1. Archives `stock_movements` rows older than 90 days via rollup consolidation.
/// 2. Deletes `offline_queue` rows older than 90 days, regardless of status.
///
/// The task runs independently of the HTTP server and does not block requests.
/// The `DbConnection` type must match the one used by the sync daemon.
pub fn start_prune_loop(db: Arc<Mutex<Connection>>) {
    tokio::spawn(async move {
        info!("prune loop started (interval = 1 hour)");

        // Run immediately on startup so old data doesn't accumulate.
        run_prune_cycle(&db).await;

        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        // The first tick of `interval` fires immediately; skip it since we
        // already ran one cycle above.
        interval.tick().await;

        loop {
            interval.tick().await;
            run_prune_cycle(&db).await;
        }
    });
}

/// Execute a single prune cycle: archive stock movements + delete old offline queue items.
async fn run_prune_cycle(db: &Arc<Mutex<Connection>>) {
    let db = db.clone();

    let result = tokio::task::spawn_blocking(move || {
        let conn = db.blocking_lock();
        let store = Store::new(&conn);

        // Archive old stock movements (ADR #6 Q4).
        let stock_archived = match store.archive_stock_movements(90, 50) {
            Ok(count) => count,
            Err(e) => {
                error!(error = %e, "prune: archive_stock_movements failed");
                0
            }
        };

        // Delete old offline queue items in cursor-based batches
        // (P-1 Retention). This avoids long-running DELETE transactions
        // on large tables and lets incremental_vacuum reclaim space
        // between batches.
        let mut queue_deleted: usize = 0;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(90);
        let cutoff_str = cutoff.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        loop {
            // Select up to 500 old IDs in a stable order.
            let mut stmt = match conn.prepare(
                "SELECT id FROM offline_queue
                 WHERE created_at < ?1
                 ORDER BY id
                 LIMIT 500",
            ) {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "prune: failed to prepare batch select");
                    break;
                }
            };

            let ids: Vec<String> =
                match stmt.query_map(rusqlite::params![cutoff_str], |row| row.get(0)) {
                    Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                    Err(e) => {
                        error!(error = %e, "prune: failed to query batch");
                        break;
                    }
                };

            if ids.is_empty() {
                break;
            }

            // Delete the batch. The ids are bound as parameters — never
            // interpolated — because they originate from client pushes and
            // must always be treated as data, never SQL. Each DELETE runs
            // in its own implicit transaction, so a failure won't leave a
            // dangling transaction on the shared connection.
            let placeholders = vec!["?"; ids.len()].join(", ");
            let sql = format!("DELETE FROM offline_queue WHERE id IN ({placeholders})");
            let deleted = match conn.execute(&sql, rusqlite::params_from_iter(ids.iter())) {
                Ok(count) => count,
                Err(e) => {
                    error!(error = %e, "prune: batch delete failed");
                    break;
                }
            };

            queue_deleted += deleted;

            // Reclaim freed pages (P-1: incremental_vacuum after each batch).
            if let Err(e) = conn.execute_batch("PRAGMA incremental_vacuum(50);") {
                error!(error = %e, "prune: incremental_vacuum failed");
            }
        }

        (stock_archived, queue_deleted)
    })
    .await;

    match result {
        Ok((stock, queue)) => {
            if stock > 0 || queue > 0 {
                info!(
                    stock_archived = stock,
                    queue_deleted = queue,
                    "prune cycle completed"
                );
            }
        }
        Err(e) => {
            error!(error = %e, "prune spawn_blocking panicked");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    /// The prune DELETE must treat ids as data, never as SQL. The cloud
    /// server accepts client-supplied ids verbatim in `push_handler` (no
    /// UUID validation), so a hostile id sitting in an old synced row must
    /// not execute arbitrary statements when the hourly prune runs — the
    /// "IDs are UUIDv7 — safe" comment is an assumption, not an invariant.
    #[test]
    fn prune_delete_treats_hostile_id_as_data() {
        let conn = oz_core::migrations::fresh_db();
        // An old synced row whose id carries a statement terminator plus a
        // destructive CREATE. If the DELETE interpolates the id, `hacked`
        // appears in the schema.
        let hostile_id = "x'); CREATE TABLE hacked(id TEXT);--";
        conn.execute(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority)
             VALUES (?1, 'act', '{}', 'synced', 0, NULL, '2025-01-01T00:00:00Z', '2025-01-02T00:00:00Z', 't1', 1)",
            params![hostile_id],
        )
        .unwrap();

        let db = Arc::new(Mutex::new(conn));
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(run_prune_cycle(&db));

        let conn = db.blocking_lock();
        let hacked: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'hacked'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            hacked, 0,
            "hostile id must never execute SQL in the prune DELETE"
        );
    }
    /// P-1 retention must cover API-pushed rows. `push_handler` persists
    /// every accepted item with status `pending` and nothing ever
    /// transitions it server-side, so the old `status IN ('synced','failed')`
    /// filter exempted the entire push path — the cloud queue grew without
    /// bound. Retention applies to every status: rows older than the 90-day
    /// horizon are pruned (the anchor_expired -> snapshot recovery path is
    /// the designed guardrail for stragglers), recent rows survive.
    #[test]
    fn prune_ages_out_old_pending_rows_like_synced_ones() {
        let conn = oz_core::migrations::fresh_db();
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority) VALUES
             ('old-pending', 'act', '{}', 'pending', 0, NULL, '2025-01-01T00:00:00Z', NULL, 't1', 1),
             ('old-synced', 'act', '{}', 'synced', 0, NULL, '2025-01-02T00:00:00Z', '2025-01-03T00:00:00Z', 't1', 1),
             ('recent-pending', 'act', '{}', 'pending', 0, NULL, '2026-08-09T00:00:00Z', NULL, 't1', 1)"
        )
        .unwrap();

        let db = Arc::new(Mutex::new(conn));
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(run_prune_cycle(&db));

        let conn = db.blocking_lock();
        let remaining: Vec<String> = conn
            .prepare("SELECT id FROM offline_queue ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(
            remaining,
            vec!["recent-pending".to_string()],
            "old pending and old synced rows must be pruned; the recent pending row survives"
        );
    }
}
