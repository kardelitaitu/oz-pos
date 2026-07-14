//! Background pruning task for the cloud sync server.
//!
//! Runs on an hourly interval, calling [`Store::archive_stock_movements`]
//! to consolidate delta ledger rows older than 90 days into the archive
//! table (ADR #6 Q4 / P-1 Ledger Retention).
//!
//! Also prunes the `offline_queue` table — deleting synced and failed
//! items older than 90 days (P-1 Retention).

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
/// 2. Deletes `offline_queue` rows older than 90 days (synced/failed status only).
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

        // Delete old offline queue items (synced or failed, > 90 days).
        let queue_deleted =
            match conn.execute(
                "DELETE FROM offline_queue
                 WHERE status IN ('synced', 'failed')
                   AND created_at < datetime('now', '-90 days')",
                [],
            ) {
                Ok(count) => count,
                Err(e) => {
                    error!(error = %e, "prune: offline_queue cleanup failed");
                    0
                }
            };

        (stock_archived, queue_deleted)
    })
    .await;

    match result {
        Ok((stock, queue)) => {
            if stock > 0 || queue > 0 {
                info!(stock_archived = stock, queue_deleted = queue, "prune cycle completed");
            }
        }
        Err(e) => {
            error!(error = %e, "prune spawn_blocking panicked");
        }
    }
}
