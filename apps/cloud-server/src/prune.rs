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

use crate::metrics;
use oz_core::db::Store;
use rusqlite::Connection;
use tokio::sync::Mutex;
use tracing::{error, info};

/// The retention horizon for sync data (P-1): 90 days.
const RETENTION_DAYS: i64 = 90;
/// Rows deleted per batch, so neither backend holds a long DELETE transaction.
const PRUNE_BATCH_SIZE: i64 = 500;

/// ISO-8601 cutoff timestamp (seconds precision) for the retention horizon.
fn retention_cutoff() -> String {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(RETENTION_DAYS);
    cutoff.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

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
        let cutoff_str = retention_cutoff();
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
            // Retention observability (round 123): surface the deleted count
            // on a counter so operators can confirm old rows are being aged
            // out — a flat counter while rows age past the horizon signals
            // the retention path is not covering them.
            metrics::PRUNE_QUEUE_DELETED_TOTAL.inc_by(deleted as f64);

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

/// Start the background prune loop on a Postgres pool (Phase 1.5).
///
/// The Postgres loop only applies P-1 offline-queue retention. The
/// `archive_stock_movements` rollup and SQLite's `incremental_vacuum` are
/// SQLite-specific: Postgres reclaims space via autovacuum, and the stock
/// movement rollup has no Postgres port yet (tracked in the plan).
pub fn start_prune_loop_pg(pool: deadpool_postgres::Pool) {
    tokio::spawn(async move {
        info!("prune loop (Postgres) started (interval = 1 hour)");

        // Run immediately on startup so old data doesn't accumulate.
        run_prune_cycle_pg(&pool).await;

        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        interval.tick().await;

        loop {
            interval.tick().await;
            run_prune_cycle_pg(&pool).await;
        }
    });
}

/// Execute a single Postgres prune cycle: delete `offline_queue` rows older
/// than the 90-day horizon in cursor-based batches (P-1 Retention).
async fn run_prune_cycle_pg(pool: &deadpool_postgres::Pool) {
    let cutoff = retention_cutoff();
    let mut queue_deleted: usize = 0;

    loop {
        let client = match pool.get().await {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "prune (pg): failed to acquire connection");
                break;
            }
        };

        // Select up to 500 old IDs in a stable order.
        let ids: Vec<String> = match client
            .query(
                "SELECT id FROM offline_queue WHERE created_at < $1 ORDER BY id LIMIT $2",
                &[&cutoff, &PRUNE_BATCH_SIZE],
            )
            .await
        {
            Ok(rows) => rows.iter().map(|r| r.get(0)).collect(),
            Err(e) => {
                error!(error = %e, "prune (pg): failed to select batch");
                break;
            }
        };

        if ids.is_empty() {
            break;
        }

        // Delete the batch. ids are bound as a text array parameter — never
        // interpolated — because they originate from client pushes and must
        // always be treated as data, never SQL.
        let deleted = match client
            .execute("DELETE FROM offline_queue WHERE id = ANY($1)", &[&ids])
            .await
        {
            Ok(count) => count,
            Err(e) => {
                error!(error = %e, "prune (pg): batch delete failed");
                break;
            }
        };

        queue_deleted += deleted as usize;
        metrics::PRUNE_QUEUE_DELETED_TOTAL.inc_by(deleted as f64);
    }

    if queue_deleted > 0 {
        info!(
            queue_deleted = queue_deleted,
            "prune cycle (Postgres) completed"
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use serial_test::serial;

    /// The prune DELETE must treat ids as data, never as SQL. The cloud
    /// server accepts client-supplied ids verbatim in `push_handler` (no
    /// UUID validation), so a hostile id sitting in an old synced row must
    /// not execute arbitrary statements when the hourly prune runs — the
    /// "IDs are UUIDv7 — safe" comment is an assumption, not an invariant.
    #[serial]
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
    #[serial]
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
    /// The prune must record every deleted row on the retention counter so
    /// operators can observe that old queue rows are actually being aged
    /// out (round-121 follow-up: retention observability).
    #[serial]
    #[test]
    fn prune_records_deleted_rows_on_retention_counter() {
        let conn = oz_core::migrations::fresh_db();
        conn.execute_batch(
            "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority) VALUES
             ('old-1', 'act', '{}', 'pending', 0, NULL, '2025-01-01T00:00:00Z', NULL, 't1', 1),
             ('old-2', 'act', '{}', 'synced', 0, NULL, '2025-01-02T00:00:00Z', '2025-01-03T00:00:00Z', 't1', 1),
             ('fresh', 'act', '{}', 'pending', 0, NULL, '2026-08-09T00:00:00Z', NULL, 't1', 1)"
        )
        .unwrap();

        // Delta around the cycle: other prune tests (serialized via
        // #[serial]) may have incremented the shared counter earlier.
        let before = crate::metrics::PRUNE_QUEUE_DELETED_TOTAL.get();
        let db = Arc::new(Mutex::new(conn));
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(run_prune_cycle(&db));
        let after = crate::metrics::PRUNE_QUEUE_DELETED_TOTAL.get();

        assert_eq!(
            (after - before) as u64,
            2,
            "the prune must record the two deleted rows on the retention counter"
        );
    }

    /// Integration test: the Postgres prune cycle applies P-1 offline-queue
    /// retention — old rows (any status) are deleted, recent rows survive.
    /// Skips when no reachable Postgres is configured, so the suite stays
    /// green on machines without one.
    #[tokio::test]
    async fn pg_integration_prune_ages_out_old_rows() {
        let url = std::env::var("OZ_TEST_PG_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());

        let pool = match crate::db::DbPool::connect_postgres(&url, false, 20).await {
            Ok(crate::db::DbPool::Postgres(pool)) => pool,
            Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
            Err(e) => {
                eprintln!("PG prune integration test skipped: {e}");
                return;
            }
        };

        let tenant = format!("pg-prune-test-{}", uuid::Uuid::now_v7());
        let recent = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        {
            let client = pool.get().await.unwrap();
            client
                .execute(
                    "INSERT INTO offline_queue (id, action, payload, status, retry_count, last_error, created_at, synced_at, tenant_id, priority)
                     VALUES
                     ($1, 'act', '{}', 'pending', 0, NULL, '2025-01-01T00:00:00Z', NULL, $4, 1),
                     ($2, 'act', '{}', 'synced', 0, NULL, '2025-01-02T00:00:00Z', '2025-01-03T00:00:00Z', $4, 1),
                     ($3, 'act', '{}', 'pending', 0, NULL, $5, NULL, $4, 1)",
                    &[
                        &format!("old-pending-{tenant}"),
                        &format!("old-synced-{tenant}"),
                        &format!("recent-{tenant}"),
                        &tenant,
                        &recent,
                    ],
                )
                .await
                .unwrap();
        }

        super::run_prune_cycle_pg(&pool).await;

        {
            let client = pool.get().await.unwrap();
            let rows = client
                .query(
                    "SELECT id FROM offline_queue WHERE tenant_id = $1 ORDER BY id",
                    &[&tenant],
                )
                .await
                .unwrap();
            let ids: Vec<String> = rows.iter().map(|r| r.get(0)).collect();
            assert_eq!(
                ids,
                vec![format!("recent-{tenant}")],
                "old pending and old synced rows must be pruned; the recent row survives"
            );

            client
                .execute("DELETE FROM offline_queue WHERE tenant_id = $1", &[&tenant])
                .await
                .unwrap();
        }
    }
}
