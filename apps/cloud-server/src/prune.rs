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

/// Execute a single Postgres prune cycle:
/// 1. delete `offline_queue` rows older than the 90-day horizon in
///    cursor-based batches (P-1 Retention);
/// 2. delete `sent_reports` claims older than the same horizon — the dedup
///    table grows one row per (tenant, period) forever, and a claim is only
///    useful while a crash-recovery retry window could still collide.
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

    // Age out `sent_reports` claims. Single DELETE — the table is small
    // (one row per tenant/period per cadence) and the same 90-day horizon
    // bounds its size, so batching is unnecessary.
    let sent_reports_deleted = match pool.get().await {
        Ok(client) => match client
            .execute("DELETE FROM sent_reports WHERE sent_at < $1", &[&cutoff])
            .await
        {
            Ok(count) => count as usize,
            Err(e) => {
                error!(error = %e, "prune (pg): sent_reports sweep failed");
                0
            }
        },
        Err(e) => {
            error!(error = %e, "prune (pg): failed to acquire connection for sent_reports sweep");
            0
        }
    };
    if sent_reports_deleted > 0 {
        metrics::PRUNE_SENT_REPORTS_DELETED_TOTAL.inc_by(sent_reports_deleted as f64);
    }

    if queue_deleted > 0 || sent_reports_deleted > 0 {
        info!(
            queue_deleted = queue_deleted,
            sent_reports_deleted = sent_reports_deleted,
            "prune cycle (Postgres) completed"
        );
    }
}
#[cfg(test)] #[path = "prune_tests.rs"] mod tests;
