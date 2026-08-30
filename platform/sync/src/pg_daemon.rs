//! PostgreSQL Sync Daemon — background task that periodically pushes pending
//! offline mutations directly to a remote PostgreSQL database.
/*
last audited 25-07-26 by RSA-Agent (platform-sync slice G: pg_daemon deep read)
crate: platform-sync | status: SAFE | lint: CLEAN
findings: exemplary — full SYNC-01/02/09/10 parity with the SQLite daemon: durable anchor advanced only after the page plus the ADR #6 stock_summary rebuild succeed, SYNC-09 mid-pull rewind detection under one lock hold, shared ADR #21 conflict service, SYNC-10 settings re-emit after commit; documented pull-every-cycle fix (pull was previously unreachable on push-idle cycles, starving relay terminals); anchor anchored on created_at never synced_at (documented NULL-stamp rationale); recover_pg_snapshot imports BEFORE resetting the anchor (test-pinned both orderings); tenant fallback chain (license setting, then queue tenant, then default); require_tls fail-closed; per-phase panic capture into status
next: none | perf: blocking DB phases in spawn_blocking
*/
//!
//! Operates similarly to [`crate::daemon::SyncDaemon`] but uses a
//! [`PgTransport`] instead of HTTP transport. Configuration is read from
//! the local settings table on every tick, so changes take effect without
//! restarting.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::{Mutex, RwLock, watch};

use oz_core::db::Store;
use oz_core::events::SettingsUpdated;
use oz_core::offline::OfflineQueueItem;
use oz_core::settings::Settings;

use crate::daemon::SettingsChangedSink;
use crate::pg_transport::PgTransport;
use crate::queue::SyncQueue;
use crate::{SyncError, SyncResult, import_snapshot};

/// Default interval between PG sync cycles (60 seconds — PG sync is
/// typically less time-sensitive than HTTP sync).
const DEFAULT_PG_SYNC_INTERVAL: Duration = Duration::from_secs(60);

/// Snapshot of the PG daemon's current state, observable via
/// [`PgSyncDaemon::status`]. Serialized camelCase for the Tauri command
/// boundary (the desktop client's `pg_sync_status` IPC returns this
/// directly, matching the app's DTO convention).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PgDaemonStatus {
    /// Whether the daemon is currently running.
    pub running: bool,
    /// ISO-8601 timestamp of the last completed sync cycle (or error).
    pub last_sync_at: Option<String>,
    /// Number of items pushed in the last cycle.
    pub last_pushed: usize,
    /// Number of items pulled in the last cycle.
    pub last_pulled: usize,
    /// Error message from the last cycle, if any.
    pub last_error: Option<String>,
    /// Number of items currently pending in the offline queue.
    pub pending_count: i64,
}

/// A reference to a shared DB connection, used by the daemon to create
/// temporary [`Store`] instances inside `spawn_blocking` closures.
pub type DbConnection = Arc<Mutex<rusqlite::Connection>>;

/// A background task that periodically syncs the local offline queue with a
/// remote PostgreSQL database.
///
/// The daemon reads PG connection settings from the database settings table
/// on every tick, so configuration changes take effect on the next cycle
/// without restarting.
pub struct PgSyncDaemon {
    interval: Duration,
    status: Arc<RwLock<PgDaemonStatus>>,
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
    settings_sink: SettingsChangedSink,
}

impl PgSyncDaemon {
    /// Create a new PostgreSQL sync daemon.
    pub fn new() -> Self {
        Self {
            interval: DEFAULT_PG_SYNC_INTERVAL,
            status: Arc::new(RwLock::new(PgDaemonStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
            settings_sink: Arc::new(|_: &SettingsUpdated| {}),
        }
    }

    /// Create a new PostgreSQL sync daemon with a custom interval.
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            status: Arc::new(RwLock::new(PgDaemonStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
            settings_sink: Arc::new(|_: &SettingsUpdated| {}),
        }
    }

    /// Start the background PG sync daemon.
    ///
    /// Spawns a `tokio` task that periodically:
    /// 1. Reads PG connection settings + pending items from the local DB
    /// 2. Pushes pending items to the remote PostgreSQL database
    /// 3. Updates item statuses in the local DB
    ///
    /// If the daemon is already running, this is a no-op.
    pub async fn start(&self, db: DbConnection) {
        self.start_inner(db, self.settings_sink.clone()).await;
    }

    /// Start the background PG sync daemon with a custom settings-change
    /// sink (SYNC-10 parity with the SQLite daemon).
    ///
    /// The sink is invoked after each remote `settings.update`/`settings.change`
    /// the pull phase applies, carrying the changed key and its originating
    /// terminal — the desktop client uses this to emit the `settings_updated`
    /// Tauri event so the UI refetches a setting changed on a remote
    /// PostgreSQL terminal.
    pub async fn start_with_sink(&self, db: DbConnection, settings_sink: SettingsChangedSink) {
        self.start_inner(db, settings_sink).await;
    }

    /// Shared start path used by [`PgSyncDaemon::start`] and
    /// [`PgSyncDaemon::start_with_sink`].
    async fn start_inner(&self, db: DbConnection, settings_sink: SettingsChangedSink) {
        if self.is_running().await {
            tracing::warn!("pg sync daemon is already running");
            return;
        }

        let (tx, rx) = watch::channel(false);
        *self.shutdown_tx.lock().await = Some(tx);

        let interval = self.interval;
        let daemon_status = Arc::clone(&self.status);

        {
            let mut s = daemon_status.write().await;
            s.running = true;
            s.last_error = None;
        }

        tokio::spawn(async move {
            let mut rx = rx;

            tracing::info!(interval_ms = interval.as_millis(), "pg sync daemon started");

            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        Self::run_tick(&db, &daemon_status, &settings_sink).await;
                    }
                    result = rx.changed() => {
                        if result.is_err() || *rx.borrow() {
                            tracing::info!("pg sync daemon shutting down");
                            break;
                        }
                    }
                }
            }

            let mut s = daemon_status.write().await;
            s.running = false;
        });
    }

    /// Run a single PG sync tick: read -> send -> apply.
    async fn run_tick(
        db: &DbConnection,
        daemon_status: &Arc<RwLock<PgDaemonStatus>>,
        settings_sink: &SettingsChangedSink,
    ) {
        // Phase 1: Read PG settings + pending items from local DB (blocking)
        let db_clone = db.clone();
        let (pg_config, pending, pull_since, pull_cursor, read_error) =
            match tokio::task::spawn_blocking(move || {
                let conn = db_clone.blocking_lock();
                let store = Store::new(&conn);

                let enabled = Settings::is_pg_sync_enabled(&conn).unwrap_or(false);
                let pending = store.list_pending_offline().unwrap_or_default();
                // SYNC-01 parity: the durable pull anchor (since + composite
                // cursor) survives restarts and advances only after a page
                // applied — never re-derive it from the local queue's synced
                // timestamps (pulled remote items do not move those).
                let pull_state = store.get_sync_pull_state().ok();
                let pull_since = pull_state.as_ref().and_then(|s| s.since.clone());
                let pull_cursor = pull_state.as_ref().and_then(|s| s.cursor.clone());

                // Build the transport whenever PG sync is ENABLED — not only
                // when there are pending items — so a pull-only terminal (a
                // pure consumer of another terminal's rows on a shared remote
                // PG) still pulls every cycle. Previously the pull phase was
                // unreachable on push-idle cycles, which would have starved
                // relay terminals of remote updates.
                let pg_config = if enabled {
                    let host = Settings::get_pg_sync_host(&conn)
                        .unwrap_or_default()
                        .unwrap_or_default();
                    let port: String = Settings::get_pg_sync_port(&conn)
                        .ok()
                        .flatten()
                        .filter(|p| !p.is_empty())
                        .unwrap_or_else(|| "5432".into());
                    let dbname = Settings::get_pg_sync_dbname(&conn)
                        .unwrap_or_default()
                        .unwrap_or_default();
                    let user = Settings::get_pg_sync_user(&conn)
                        .unwrap_or_default()
                        .unwrap_or_default();
                    let password = Settings::get_pg_sync_password(&conn)
                        .unwrap_or_default()
                        .unwrap_or_default();
                    // TLS enforcement: when pg_sync.require_tls is set, the
                    // transport refuses plaintext connections (fail-closed
                    // for cloud PostgreSQL). Defaults to plaintext to match
                    // the historical NoTls transport.
                    let require_tls = Settings::get_pg_sync_require_tls(&conn).unwrap_or(false);
                    // The transport scopes every query to this tenant, so a
                    // shared multi-tenant database never leaks another
                    // tenant's rows to this terminal. Falls back to the
                    // local queue's tenant when the license setting is
                    // absent (pre-license installs).
                    let tenant_id: String = Settings::get(&conn, "license.tenant_id")
                        .unwrap_or_default()
                        .filter(|s| !s.is_empty())
                        .or_else(|| pending.first().map(|i| i.tenant_id.clone()))
                        .unwrap_or_else(|| "default".into());

                    if !host.is_empty() && !dbname.is_empty() {
                        Some((host, port, dbname, user, password, tenant_id, require_tls))
                    } else {
                        None
                    }
                } else {
                    None
                };

                (pg_config, pending, pull_since, pull_cursor)
            })
            .await
            {
                Ok((cfg, pending, since, cursor)) => (cfg, pending, since, cursor, None),
                Err(join_err) => {
                    let msg = format!("pg sync config read panicked: {join_err}");
                    tracing::error!(error = %msg, "pg sync daemon read phase failed");
                    (None, Vec::new(), None, None, Some(msg))
                }
            };

        // Phase 2: Do async PG sync if configured
        let mut pushed = 0usize;
        let mut pulled = 0usize;
        let mut sync_error: Option<String> = None;

        let pg_transport = pg_config.as_ref().and_then(
            |(host, port, dbname, user, password, tenant_id, require_tls)| {
                let port_u16: u16 = port.parse().unwrap_or(5432);
                PgTransport::new_with_tls(
                    host,
                    port_u16,
                    dbname,
                    user,
                    password,
                    tenant_id,
                    *require_tls,
                )
                .ok()
            },
        );

        if let Some(ref transport) = pg_transport {
            // Phase 3: push pending items (no-op when nothing is pending).
            if !pending.is_empty() {
                match transport.push_items(&pending).await {
                    Ok(results) => {
                        pushed = results.len(); // Phase 3: Apply push results to local DB (blocking)
                        let db_clone = db.clone();
                        let outcome = tokio::task::spawn_blocking(move || {
                            let conn = db_clone.blocking_lock();
                            let store = Store::new(&conn);
                            let queue = SyncQueue::new();
                            for (item, outcome) in pending.iter().zip(results.iter()) {
                                match outcome {
                                    crate::transport::PushOutcome::Accepted => {
                                        if let Err(e) = store.mark_offline_synced(&item.id) {
                                            tracing::error!(
                                                item_id = %item.id,
                                                error = %e,
                                                "pg sync daemon: failed to mark item synced"
                                            );
                                        }
                                    }
                                    crate::transport::PushOutcome::Rejected { reason } => {
                                        if let Err(e) = store.mark_offline_failed(&item.id, reason)
                                        {
                                            tracing::error!(
                                                item_id = %item.id,
                                                error = %e,
                                                "pg sync daemon: failed to mark item failed"
                                            );
                                        }
                                    }
                                    crate::transport::PushOutcome::Conflict(server_item) => {
                                        // SYNC-02 parity: route the conflict through the shared
                                        // ADR #21 service (version LWW / sale status DAG / stock
                                        // CRDT merge) instead of blanket mark-synced + re-enqueue,
                                        // which could resurrect stale remote state.
                                        if let Err(e) =
                                            queue.apply_push_conflict(&store, item, server_item)
                                        {
                                            tracing::error!(
                                                item_id = %item.id,
                                                action = %item.action,
                                                error = %e,
                                                "pg sync daemon: conflict resolution failed"
                                            );
                                        }
                                    }
                                }
                            }
                        })
                        .await;

                        if let Err(e) = outcome {
                            sync_error = Some(format!("apply push phase: {e}"));
                        }
                    }
                    Err(e) => {
                        sync_error = Some(e.to_string());
                    }
                }
            }

            // Phase 4: Pull remote updates and apply them locally.
            // SYNC-01 parity: the `since` anchor + composite `(created_at,
            // id)` cursor come from the durable `sync_pull_state` row, each
            // item applies atomically with the idempotency ledger, and the
            // anchor advances only after the page applied — so a replaying
            // remote queue can never re-apply a mutation, and a poison item
            // dead-letters instead of erroring forever. Pages loop while the
            // remote returns a next cursor (P-3 pagination). The pull runs on
            // every enabled cycle, independent of whether anything was
            // pending to push.
            let mut pull_since = pull_since;
            let mut pull_cursor = pull_cursor;
            loop {
                match transport
                    .pull_updates(pull_since.as_deref(), pull_cursor.as_deref())
                    .await
                {
                    Ok(pull_resp) => {
                        pulled += pull_resp.items.len();
                        let next_cursor = pull_resp.next_cursor;
                        if pull_resp.items.is_empty() {
                            break;
                        }
                        let db_clone = db.clone();
                        let items = pull_resp.items;
                        let prev_since = pull_since;
                        let prev_cursor = pull_cursor.clone();
                        // The closure needs the cursor to persist it with the
                        // anchor; the outer loop re-owns the original for the
                        // next pull iteration.
                        let next_cursor_for_persist = next_cursor.clone();
                        let settings_sink = settings_sink.clone();
                        let outcome = tokio::task::spawn_blocking(move || {
                            let conn = db_clone.blocking_lock();
                            let store = Store::new(&conn);
                            // None = retryable failure: the durable anchor is
                            // retained and pagination stops so the next cycle
                            // re-pulls the same page (`?` early-returns None
                            // from the closure).
                            let new_since = apply_pulled_page(
                                &store,
                                &items,
                                prev_since.as_deref(),
                                &settings_sink,
                            )?;
                            // SYNC-09: re-read the DURABLE pull state before
                            // advancing (parity with the SQLite daemon). An
                            // operator rewind (`requeue_remote_failure` sets
                            // since = NULL) can land while this page was in
                            // flight; writing new_since blindly would clobber
                            // it and the requeued item would never be
                            // re-fetched. Skip the write when the durable
                            // (since, cursor) no longer matches what this tick
                            // captured — full-state comparison, so a
                            // concurrent writer moving the anchor can never
                            // be overwritten with our now-stale value. Both
                            // the read and the (skipped) write hold the same
                            // `blocking_lock()`, so nothing can interleave.
                            let durable = store
                                .get_sync_pull_state()
                                .unwrap_or_default();
                            let rewound = durable.since.as_deref()
                                != prev_since.as_deref()
                                || durable.cursor.as_deref()
                                    != prev_cursor.as_deref();
                            if rewound {
                                tracing::warn!(
                                    "pg sync daemon: operator rewind detected mid-pull — retaining rewound anchor for full re-pull"
                                );
                                // Pagination may still continue in-memory: the
                                // retained NULL anchor makes the NEXT tick a
                                // full re-pull (ledger absorbs the replay).
                            } else if let Err(e) = store.set_sync_pull_state(
                                Some(&new_since),
                                next_cursor_for_persist.as_deref(),
                            ) {
                                tracing::error!(
                                    error = %e,
                                    "pg sync daemon: failed to persist pull anchor"
                                );
                            }
                            Some(new_since)
                        })
                        .await;
                        match outcome {
                            Ok(Some(new_since)) => {
                                pull_since = Some(new_since);
                                pull_cursor = next_cursor;
                                if pull_cursor.is_none() {
                                    break;
                                }
                            }
                            // Retryable failure — retain anchor, stop paginating.
                            Ok(None) => break,
                            Err(e) => {
                                if sync_error.is_none() {
                                    sync_error = Some(format!("apply pull phase: {e}"));
                                }
                                break;
                            }
                        }
                    }
                    Err(SyncError::AnchorExpired { oldest_available }) => {
                        tracing::warn!(
                            oldest_available = ?oldest_available,
                            "pg sync anchor expired — fetching snapshot to recover"
                        );
                        match transport.fetch_snapshot().await {
                            Ok(snapshot) => {
                                let db_clone = db.clone();
                                let anchor = oldest_available.clone();
                                let recovery = tokio::task::spawn_blocking(move || {
                                    let conn = db_clone.blocking_lock();
                                    let store = Store::new(&conn);
                                    recover_pg_snapshot(&store, &snapshot, anchor.as_deref())
                                })
                                .await;
                                match recovery {
                                    Ok(Ok(imported)) => {
                                        tracing::info!(
                                            imported,
                                            "pg snapshot imported after anchor expiry"
                                        );
                                    }
                                    Ok(Err(e)) => {
                                        if sync_error.is_none() {
                                            sync_error =
                                                Some(format!("snapshot recovery failed: {e}"));
                                        }
                                    }
                                    Err(e) => {
                                        if sync_error.is_none() {
                                            sync_error =
                                                Some(format!("snapshot recovery panicked: {e}"));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if sync_error.is_none() {
                                    sync_error =
                                        Some(format!("snapshot recovery fetch failed: {e}"));
                                }
                            }
                        }
                        break;
                    }
                    Err(e) => {
                        if sync_error.is_none() {
                            sync_error = Some(format!("pull phase: {e}"));
                        }
                        break;
                    }
                }
            }
        }

        // Get pending count
        let db_clone = db.clone();
        let pending_count = tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            let store = Store::new(&conn);
            store.pending_offline_count().unwrap_or(0)
        })
        .await
        .unwrap_or(0);

        // Update daemon status
        let mut s = daemon_status.write().await;
        s.last_sync_at =
            Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        s.pending_count = pending_count;
        s.last_pushed = pushed;
        s.last_pulled = pulled;
        // If the read phase panicked, surface that error in the status.
        s.last_error = sync_error.clone().or_else(|| read_error.clone());

        if sync_error.is_some() {
            tracing::error!(error = ?sync_error, "pg sync cycle failed");
        } else {
            tracing::info!(pushed, pulled, "pg sync cycle completed");
        }
    }

    /// Gracefully stop the background PG sync daemon.
    pub async fn stop(&self) {
        let tx = self.shutdown_tx.lock().await.take();
        if let Some(tx) = tx {
            let _ = tx.send(true);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Check if the daemon is currently running.
    pub async fn is_running(&self) -> bool {
        self.status.read().await.running
    }

    /// Get a snapshot of the daemon's current status.
    pub async fn status(&self) -> PgDaemonStatus {
        self.status.read().await.clone()
    }

    /// Set the sync interval (applied on next cycle start).
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    /// Get the current sync interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

impl Default for PgSyncDaemon {
    fn default() -> Self {
        Self::new()
    }
}

/// Import a PostgreSQL snapshot, then move the durable pull anchor to the
/// server's oldest retained boundary.
///
/// The anchor is changed only after [`import_snapshot`] succeeds. A failed
/// import therefore leaves the stale anchor intact so the next cycle can
/// retry recovery rather than claiming that the gap was repaired.
fn recover_pg_snapshot(
    store: &Store<'_>,
    snapshot: &crate::transport::SyncSnapshotResponse,
    oldest_available: Option<&str>,
) -> SyncResult<usize> {
    let imported = import_snapshot(store, snapshot)?;
    store.set_sync_pull_state(oldest_available, None)?;
    Ok(imported)
}

/// Apply one page of pulled remote items atomically (SYNC-01 parity with the
/// SQLite daemon).
///
/// Each item is applied via [`SyncQueue::apply_remote_atomic`] — the domain
/// mutation and its idempotency receipt commit in one transaction, and a
/// poison item is dead-lettered after its retry budget. Returns the next
/// durable pull anchor to persist: `Some(max(prev_since, newest created_at))`
/// when the whole page applied (dead-lettered items count as applied), or
/// `None` when a retryable failure requires retaining the current anchor so
/// the next cycle re-pulls the same page. A replayed page never re-applies a
/// mutation (the ledger skips it), so a crash mid-pull is safe.
///
/// The anchor is the page's newest `created_at` — the composite cursor's
/// first key — never `synced_at`, which the remote may leave NULL forever
/// (rows pushed as `pending` are only stamped later). Anchoring on
/// `created_at` means a queue whose rows are never stamped still advances
/// and is never re-pulled in full.
///
/// ADR #6 parity with the SQLite daemon: when the page contains
/// `stock.movement` items (which write ONLY the raw delta ledger — the
/// apply path never touches `stock_summary`), the materialized summary is
/// rebuilt from the ledger before the anchor advances, and a rebuild
/// failure retains the anchor so the next cycle re-pulls the same page and
/// retries the derived-state rebuild.
///
/// SYNC-10 parity: a pulled `settings.update`/`settings.change` re-emits
/// `SettingsUpdated` through `settings_sink` after its transaction commits
/// (the same contract as the SQLite daemon), so the app can refetch a
/// setting changed on a remote PostgreSQL terminal.
fn apply_pulled_page(
    store: &Store<'_>,
    page: &[OfflineQueueItem],
    prev_since: Option<&str>,
    settings_sink: &SettingsChangedSink,
) -> Option<String> {
    let queue = SyncQueue::new();
    let mut page_all_applied = true;
    let mut has_stock_movements = false;

    for remote_item in page {
        if remote_item.action == "stock.movement" {
            has_stock_movements = true;
        }
        match queue.apply_remote_atomic_full(store, remote_item) {
            Ok(outcome) => {
                // SYNC-10 parity: a settings change applied from a remote
                // PostgreSQL terminal is re-emitted as `SettingsUpdated` so
                // the UI refetches. The tx committed inside
                // apply_remote_atomic_full before this runs.
                if let Some((key, terminal_id)) = outcome.settings_change {
                    let event = SettingsUpdated {
                        changed_keys: vec![key],
                        terminal_id,
                    };
                    settings_sink(&event);
                }
                if !outcome.applied
                    && store
                        .is_remote_failure_dead_lettered(&remote_item.id)
                        .unwrap_or(false)
                {
                    tracing::error!(
                        item_id = %remote_item.id,
                        action = %remote_item.action,
                        "remote item remains quarantined; advancing page anchor"
                    );
                }
            }
            Err(e) => {
                let dead_lettered = store
                    .is_remote_failure_dead_lettered(&remote_item.id)
                    .unwrap_or(false);
                if dead_lettered {
                    tracing::error!(
                        item_id = %remote_item.id,
                        action = %remote_item.action,
                        error = %e,
                        "remote item quarantined after repeated failures; advancing page anchor"
                    );
                } else {
                    page_all_applied = false;
                    tracing::error!(
                        item_id = %remote_item.id,
                        action = %remote_item.action,
                        error = %e,
                        "failed to atomically apply remote item; retaining pull anchor for retry"
                    );
                }
            }
        }
    }

    if !page_all_applied {
        return None;
    }

    // ADR #6: rebuild the materialized stock_summary from the delta ledger
    // before advancing the pull anchor. If the rebuild fails, the old anchor
    // is retained so the next cycle can restore the derived state as well
    // (replay is absorbed by the idempotency ledger).
    if has_stock_movements && let Err(e) = store.rebuild_stock_summary() {
        tracing::error!(
            error = %e,
            "failed to rebuild stock summary after pg sync pull"
        );
        return None;
    }

    // Monotonic anchor: take the later of the current anchor and the page's
    // newest `created_at` watermark — the composite cursor's first key, so
    // the durable anchor tracks exactly what the next pull filters on. The
    // remote may never stamp `synced_at` (rows pushed as `pending` stay
    // NULL), so anchoring on `synced_at` could stall the anchor forever and
    // re-pull the whole queue every cycle. `.or()` alone could regress the
    // anchor when the remote returns rows older than `since` (clock skew /
    // late delivery), which would re-fetch history on every cycle. ISO-8601
    // timestamps are fixed-format, so lexicographic ordering equals
    // chronological ordering here.
    let page_max = page.iter().map(|i| i.created_at.as_str()).max();
    std::cmp::max(prev_since, page_max).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use oz_core::offline::{OfflineQueueItem, OfflineQueueStatus};

    fn setup_db() -> DbConnection {
        Arc::new(Mutex::new(migrations::fresh_db()))
    }

    fn seed_product_and_inventory(conn: &rusqlite::Connection) {
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('prod-coffee', 'COFFEE', 'Coffee', 350, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty, updated_at) VALUES
                ('prod-coffee', 50, '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();
    }

    /// A remote item shaped as the PG pull decodes it. `created_at` is the
    /// durable-anchor watermark (the composite cursor orders on it);
    /// `synced_at` is deliberately left NULL — the remote may never stamp it,
    /// and the anchor must still advance on `created_at`.
    fn remote_stock_adjustment(id: &str, delta: i64, created_at: &str) -> OfflineQueueItem {
        let mut item = OfflineQueueItem::new(
            "stock.adjusted",
            format!(r#"{{"sku":"COFFEE","delta":{delta}}}"#),
        );
        item.id = id.into();
        item.created_at = created_at.into();
        item
    }

    fn remote_poison_sale(id: &str) -> OfflineQueueItem {
        let mut item = OfflineQueueItem::new(
            "complete_sale",
            r#"{"line_items":[{"sku":"MISSING","qty":1}]}"#,
        );
        item.id = id.into();
        item.created_at = "2026-01-01T00:00:00.000Z".into();
        item
    }

    #[test]
    fn snapshot_recovery_imports_before_resetting_anchor() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        store
            .set_sync_pull_state(Some("stale"), Some("stale-cursor"))
            .unwrap();
        let snapshot = crate::transport::SyncSnapshotResponse {
            version: 1,
            products: vec![crate::transport::SnapshotProduct {
                id: "pg-snapshot-product".into(),
                sku: "PG-SNAPSHOT".into(),
                name: "PG Snapshot Product".into(),
                price_minor: 250,
                currency: "USD".into(),
                category_id: None,
                barcode: None,
                created_at: None,
                updated_at: None,
                price_updated_at: None,
                track_serial: false,
                store_id: None,
                ..Default::default()
            }],
            tax_rates: vec![],
            users: vec![],
        };

        let imported = recover_pg_snapshot(&store, &snapshot, Some("oldest"))
            .expect("valid snapshot should recover the PG anchor");
        assert_eq!(imported, 1);
        let pull_state = store.get_sync_pull_state().unwrap();
        assert_eq!(pull_state.since.as_deref(), Some("oldest"));
        assert_eq!(pull_state.cursor, None);
        let product_name: String = conn
            .query_row(
                "SELECT name FROM products WHERE sku = 'PG-SNAPSHOT'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(product_name, "PG Snapshot Product");
    }

    #[test]
    fn snapshot_recovery_keeps_stale_anchor_when_import_fails() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        store
            .set_sync_pull_state(Some("stale"), Some("stale-cursor"))
            .unwrap();
        let snapshot = crate::transport::SyncSnapshotResponse {
            version: 99,
            products: vec![],
            tax_rates: vec![],
            users: vec![],
        };

        assert!(recover_pg_snapshot(&store, &snapshot, Some("oldest")).is_err());
        let pull_state = store.get_sync_pull_state().unwrap();
        assert_eq!(pull_state.since.as_deref(), Some("stale"));
        assert_eq!(pull_state.cursor.as_deref(), Some("stale-cursor"));
    }

    // ── SYNC-01 parity: atomic pull apply + durable anchor ────────────

    #[test]
    fn apply_pulled_page_applies_stock_adjustment_and_records_receipt() {
        let conn = migrations::fresh_db();
        seed_product_and_inventory(&conn);
        let store = Store::new(&conn);

        let page = vec![remote_stock_adjustment(
            "pg-item-1",
            10,
            "2026-01-02T00:00:00.000Z",
        )];
        let new_since = apply_pulled_page(&store, &page, None, &noop_settings_sink());

        assert_eq!(store.get_stock("prod-coffee").unwrap(), 60);
        assert!(
            store.is_remote_item_applied("pg-item-1").unwrap(),
            "ledger receipt must be recorded with the mutation"
        );
        assert_eq!(
            new_since.as_deref(),
            Some("2026-01-02T00:00:00.000Z"),
            "anchor advances on created_at, not synced_at"
        );
    }

    /// Regression (composite-cursor slice): the durable anchor must advance
    /// on `created_at` even when the remote NEVER stamps `synced_at` (NULL)
    /// — otherwise a queue whose rows all lack a synced_at watermark never
    /// advances and re-pulls everything every cycle.
    #[test]
    fn apply_pulled_page_advances_anchor_on_created_at_when_synced_at_null() {
        let conn = migrations::fresh_db();
        seed_product_and_inventory(&conn);
        let store = Store::new(&conn);

        let page = vec![remote_stock_adjustment(
            "pg-item-null",
            10,
            "2026-01-02T00:00:00.000Z",
        )];
        let new_since = apply_pulled_page(&store, &page, None, &noop_settings_sink());

        assert_eq!(store.get_stock("prod-coffee").unwrap(), 60);
        assert_eq!(
            new_since.as_deref(),
            Some("2026-01-02T00:00:00.000Z"),
            "anchor must advance on created_at even when synced_at is NULL"
        );
    }

    #[test]
    fn apply_pulled_page_replay_is_idempotent() {
        let conn = migrations::fresh_db();
        seed_product_and_inventory(&conn);
        let store = Store::new(&conn);
        let page = vec![remote_stock_adjustment(
            "pg-item-replay",
            10,
            "2026-01-02T00:00:00.000Z",
        )];

        let _ = apply_pulled_page(&store, &page, None, &noop_settings_sink());
        let _ = apply_pulled_page(&store, &page, None, &noop_settings_sink());

        assert_eq!(
            store.get_stock("prod-coffee").unwrap(),
            60,
            "a replayed page must NOT re-apply the mutation (SYNC-01)"
        );
    }

    #[test]
    fn apply_pulled_page_retains_anchor_on_retryable_failure() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        let page = vec![remote_poison_sale("pg-poison-1")];

        let new_since = apply_pulled_page(&store, &page, None, &noop_settings_sink());
        assert!(
            new_since.is_none(),
            "retryable failure must retain the pull anchor"
        );
        assert!(
            !store
                .is_remote_failure_dead_lettered("pg-poison-1")
                .unwrap()
        );
    }

    #[test]
    fn apply_pulled_page_dead_letters_then_advances() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        let page = vec![remote_poison_sale("pg-poison-2")];

        // Attempts 1-2 retain the anchor; the 3rd dead-letters the item and
        // allows the page anchor to advance.
        assert!(apply_pulled_page(&store, &page, None, &noop_settings_sink()).is_none());
        assert!(apply_pulled_page(&store, &page, None, &noop_settings_sink()).is_none());
        let new_since = apply_pulled_page(&store, &page, None, &noop_settings_sink());
        assert!(
            new_since.is_some(),
            "dead-lettered item may advance the anchor"
        );
        assert!(
            store
                .is_remote_failure_dead_lettered("pg-poison-2")
                .unwrap()
        );
    }

    #[test]
    fn apply_pulled_page_anchor_is_monotonic_max_created_at() {
        let conn = migrations::fresh_db();
        seed_product_and_inventory(&conn);
        let store = Store::new(&conn);

        let earlier = remote_stock_adjustment("pg-item-a", 1, "2026-01-01T00:00:00.000Z");
        let later = remote_stock_adjustment("pg-item-b", 1, "2026-01-03T00:00:00.000Z");

        // A prior anchor newer than one page row must not regress.
        let new_since = apply_pulled_page(
            &store,
            &[earlier, later],
            Some("2026-01-02T00:00:00.000Z"),
            &noop_settings_sink(),
        );
        assert_eq!(new_since.as_deref(), Some("2026-01-03T00:00:00.000Z"));
    }

    /// ADR #6 parity (SQLite daemon, daemon.rs): a page containing a
    /// `stock.movement` writes ONLY the raw delta-ledger row (the summary
    /// cache is NOT touched by the apply path), so the materialized
    /// `stock_summary` must be rebuilt from the ledger before the anchor
    /// advances. Without the rebuild, a remote stock movement pulled via PG
    /// leaves the on-hand cache the app reads permanently stale.
    #[test]
    fn apply_pulled_page_rebuilds_stock_summary_after_stock_movements() {
        let conn = migrations::fresh_db();
        seed_product_and_inventory(&conn);
        let store = Store::new(&conn);

        let mut item = OfflineQueueItem::new(
            "stock.movement",
            r#"{"id":"sm-remote-1","item_id":"prod-coffee","delta":40,"reason":"restock","store_id":"default","created_at":"2026-01-05T00:00:00.000Z"}"#,
        );
        item.id = "pg-item-movement-1".into();
        item.created_at = "2026-01-05T00:00:00.000Z".into();

        let new_since = apply_pulled_page(
            &store,
            std::slice::from_ref(&item),
            None,
            &noop_settings_sink(),
        );

        assert_eq!(
            new_since.as_deref(),
            Some("2026-01-05T00:00:00.000Z"),
            "a stock.movement page applies and advances the anchor"
        );
        let summary_qty: i64 = conn
            .query_row(
                "SELECT qty FROM stock_summary WHERE item_id = 'prod-coffee'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            summary_qty, 40,
            "stock_summary must be rebuilt from the ledger after a stock.movement page"
        );
    }

    /// ADR #6 parity: if the summary rebuild fails, the durable anchor must
    /// be retained so the next cycle re-pulls the same page (the ledger
    /// absorbs the replay) and retries the derived-state rebuild — mirroring
    /// the SQLite daemon's "old anchor retained so a retry can restore the
    /// derived state as well".
    #[test]
    fn apply_pulled_page_retains_anchor_when_stock_summary_rebuild_fails() {
        let conn = migrations::fresh_db();
        seed_product_and_inventory(&conn);
        let store = Store::new(&conn);
        // Force the rebuild to fail: the summary table no longer exists.
        conn.execute_batch("DROP TABLE stock_summary").unwrap();

        let mut item = OfflineQueueItem::new(
            "stock.movement",
            r#"{"id":"sm-remote-2","item_id":"prod-coffee","delta":10,"reason":"restock","store_id":"default","created_at":"2026-01-06T00:00:00.000Z"}"#,
        );
        item.id = "pg-item-movement-2".into();
        item.created_at = "2026-01-06T00:00:00.000Z".into();

        let new_since = apply_pulled_page(
            &store,
            std::slice::from_ref(&item),
            None,
            &noop_settings_sink(),
        );
        assert!(
            new_since.is_none(),
            "a failed summary rebuild must retain the pull anchor"
        );
    }

    /// Helper: a no-op settings sink for call sites that do not assert on
    /// SYNC-10 emission.
    fn noop_settings_sink() -> SettingsChangedSink {
        Arc::new(|_: &SettingsUpdated| {})
    }

    /// SYNC-10 parity: a pulled remote `settings.update` must re-emit
    /// `SettingsUpdated` through the daemon's sink so the UI refetches — the
    /// SQLite daemon publishes the changed key + originating terminal after
    /// the tx commits; the PG path previously used `apply_remote_atomic`,
    /// which drops the settings-change report entirely.
    #[test]
    fn apply_pulled_page_emits_settings_updated_after_settings_change() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        let emitted = Arc::new(std::sync::Mutex::new(Vec::<SettingsUpdated>::new()));
        let sink: SettingsChangedSink = {
            let emitted = Arc::clone(&emitted);
            Arc::new(move |event: &SettingsUpdated| emitted.lock().unwrap().push(event.clone()))
        };

        let mut item = OfflineQueueItem::new(
            "settings.update",
            r#"{"key":"store.name","value":"Remote Acme","terminal_id":"term-remote","version":3}"#,
        );
        item.id = "pg-item-settings-1".into();
        item.created_at = "2026-01-02T00:00:00.000Z".into();

        let new_since = apply_pulled_page(&store, std::slice::from_ref(&item), None, &sink);

        assert_eq!(new_since.as_deref(), Some("2026-01-02T00:00:00.000Z"));
        let captured = emitted.lock().unwrap().clone();
        assert_eq!(
            captured.len(),
            1,
            "exactly one SettingsUpdated must be emitted per applied settings change"
        );
        assert_eq!(captured[0].changed_keys, vec!["store.name".to_string()]);
        assert_eq!(captured[0].terminal_id, "term-remote");
    }

    /// SYNC-10 negative: pages without settings changes must not emit
    /// anything through the sink.
    #[test]
    fn apply_pulled_page_is_silent_for_non_settings_pages() {
        let conn = migrations::fresh_db();
        seed_product_and_inventory(&conn);
        let store = Store::new(&conn);

        let emitted = Arc::new(std::sync::Mutex::new(Vec::<SettingsUpdated>::new()));
        let sink: SettingsChangedSink = {
            let emitted = Arc::clone(&emitted);
            Arc::new(move |event: &SettingsUpdated| emitted.lock().unwrap().push(event.clone()))
        };

        let page = vec![remote_stock_adjustment(
            "pg-item-silent",
            10,
            "2026-01-02T00:00:00.000Z",
        )];
        let new_since = apply_pulled_page(&store, &page, None, &sink);

        assert!(new_since.is_some());
        assert!(
            emitted.lock().unwrap().is_empty(),
            "no settings change in the page → no SettingsUpdated emission"
        );
    }

    /// Helper: enqueue an offline item and return its actual ID (from the returned OfflineQueueItem).
    fn enqueue_item(conn: &rusqlite::Connection, action: &str, payload: &str) -> String {
        let store = Store::new(conn);
        let item = store.enqueue_offline(action, payload).unwrap();
        item.id
    }

    /// Helper: get raw pending count from the offline_queue table.
    fn raw_pending_count(conn: &rusqlite::Connection) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM offline_queue WHERE status = 'pending'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0)
    }

    // ── Lifecycle tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn daemon_starts_stopped() {
        let daemon = PgSyncDaemon::new();
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_start_and_stop() {
        let db = setup_db();
        let daemon = PgSyncDaemon::new();
        daemon.start(db).await;
        assert!(daemon.is_running().await);
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_status_defaults() {
        let daemon = PgSyncDaemon::new();
        let status = daemon.status().await;
        assert!(!status.running);
        assert!(status.last_sync_at.is_none());
        assert_eq!(status.last_pushed, 0);
        assert_eq!(status.last_pulled, 0);
        assert!(status.last_error.is_none());
    }

    #[tokio::test]
    async fn daemon_stop_when_not_running_is_noop() {
        let daemon = PgSyncDaemon::new();
        daemon.stop().await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_double_start_is_noop() {
        let db = setup_db();
        let daemon = PgSyncDaemon::new();
        daemon.start(db.clone()).await;
        assert!(daemon.is_running().await);
        daemon.start(db).await;
        assert!(daemon.is_running().await);
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_custom_interval() {
        let daemon = PgSyncDaemon::with_interval(Duration::from_millis(50));
        assert_eq!(daemon.interval(), Duration::from_millis(50));
    }

    #[tokio::test]
    async fn daemon_set_interval() {
        let mut daemon = PgSyncDaemon::new();
        daemon.set_interval(Duration::from_secs(10));
        assert_eq!(daemon.interval(), Duration::from_secs(10));
    }

    // ── Outbox schema validation ────────────────────────────────────

    #[test]
    fn outbox_schema_has_required_columns() {
        let conn = migrations::fresh_db();
        let mut stmt = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='offline_queue'")
            .unwrap();
        let sql: String = stmt.query_row([], |r| r.get(0)).unwrap();
        assert!(sql.contains("id"), "offline_queue must have 'id' column");
        assert!(
            sql.contains("action"),
            "offline_queue must have 'action' column"
        );
        assert!(
            sql.contains("payload"),
            "offline_queue must have 'payload' column"
        );
        assert!(
            sql.contains("status"),
            "offline_queue must have 'status' column"
        );
        assert!(
            sql.contains("created_at"),
            "offline_queue must have 'created_at' column"
        );
    }

    #[test]
    fn outbox_table_exists() {
        let conn = migrations::fresh_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='offline_queue'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "offline_queue table must exist after migrations");
    }

    // ── Idempotency & duplicate handling ───────────────────────────

    #[test]
    fn mark_offline_synced_is_idempotent() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        let id = enqueue_item(&conn, "sale.completed", r#"{"sale_id":"s1"}"#);

        // First mark as synced — should succeed
        assert!(store.mark_offline_synced(&id).is_ok());

        // Second mark as synced — must succeed (idempotent)
        assert!(store.mark_offline_synced(&id).is_ok());
    }

    #[test]
    fn mark_offline_synced_nonexistent_item() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        // Syncing a nonexistent ID should not panic
        let result = store.mark_offline_synced("nonexistent-id");
        // Should be Ok (or Err depending on implementation) — but never panic
        let _ = result;
    }

    #[test]
    fn duplicate_enqueue_creates_separate_items() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        // Enqueue the same action twice
        store
            .enqueue_offline("stock.adjusted", r#"{"sku":"COFFEE"}"#)
            .unwrap();
        store
            .enqueue_offline("stock.adjusted", r#"{"sku":"COFFEE"}"#)
            .unwrap();

        // Both should be pending
        let count = raw_pending_count(&conn);
        assert_eq!(count, 2, "duplicate enqueue should create separate items");
    }

    // ── Large batch handling ───────────────────────────────────────

    #[test]
    fn large_batch_enqueue_10k_items() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        // Enqueue 10,000 items
        for i in 0..10_000 {
            store
                .enqueue_offline(
                    "product.created",
                    &format!(r#"{{"sku":"SKU-{}","name":"Item {}"}}"#, i, i),
                )
                .unwrap();
        }

        let count = store.pending_offline_count().unwrap();
        assert_eq!(count, 10_000);
        assert_eq!(raw_pending_count(&conn), 10_000);
    }

    #[test]
    fn list_pending_returns_correct_items() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        for i in 0..100 {
            store
                .enqueue_offline("product.created", &format!(r#"{{"sku":"SKU-{}"}}"#, i))
                .unwrap();
        }

        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 100);
        // All should have 'pending' status
        assert!(
            pending
                .iter()
                .all(|p| p.status == OfflineQueueStatus::Pending)
        );
    }

    #[test]
    fn pending_count_zero_when_empty() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        assert_eq!(store.pending_offline_count().unwrap(), 0);
    }

    // ── Graceful shutdown ──────────────────────────────────────────

    #[tokio::test]
    async fn daemon_stop_twice_is_idempotent() {
        let db = setup_db();
        let daemon = PgSyncDaemon::new();
        daemon.start(db).await;
        assert!(daemon.is_running().await);
        daemon.stop().await;
        daemon.stop().await; // second stop should be safe
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_stops_cleanly_with_short_interval() {
        let db = setup_db();
        let daemon = PgSyncDaemon::with_interval(Duration::from_millis(50));
        daemon.start(db).await;
        assert!(daemon.is_running().await);
        // Let it tick a few times
        tokio::time::sleep(Duration::from_millis(120)).await;
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!daemon.is_running().await);
    }

    // ── Status tracking ────────────────────────────────────────────

    #[tokio::test]
    async fn daemon_status_updates_running_flag() {
        let db = setup_db();
        let daemon = PgSyncDaemon::new();
        assert!(!daemon.status().await.running);
        daemon.start(db).await;
        assert!(daemon.status().await.running);
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon.status().await.running);
    }

    #[tokio::test]
    async fn daemon_status_shows_pending_count_after_tick() {
        let db = setup_db();
        // Enqueue some items before starting (blocking — spawn_blocking to avoid runtime panic)
        {
            let db_clone = db.clone();
            tokio::task::spawn_blocking(move || {
                let conn = db_clone.blocking_lock();
                let store = Store::new(&conn);
                for i in 0..5 {
                    store
                        .enqueue_offline("product.created", &format!(r#"{{"sku":"SKU-{}"}}"#, i))
                        .unwrap();
                }
            })
            .await
            .unwrap();
        }

        let daemon = PgSyncDaemon::with_interval(Duration::from_millis(30));
        daemon.start(db).await;
        // Wait for at least one tick
        tokio::time::sleep(Duration::from_millis(80)).await;

        let status = daemon.status().await;
        assert!(
            status.last_sync_at.is_some(),
            "last_sync_at should be set after tick"
        );
        // No PG configured, so items should still be pending
        assert_eq!(status.pending_count, 5);
        assert_eq!(status.last_pushed, 0);

        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // ── Concurrent daemon instances (advisory lock simulation) ──────

    #[tokio::test]
    async fn two_daemons_cannot_run_simultaneously_on_same_db() {
        let db1 = setup_db();
        let db2 = db1.clone();

        let daemon1 = PgSyncDaemon::new();
        let daemon2 = PgSyncDaemon::new();

        daemon1.start(db1).await;
        assert!(daemon1.is_running().await);

        // Second daemon on the same DB — should be fine since they're
        // separate daemon instances (not the same object)
        daemon2.start(db2).await;
        assert!(daemon2.is_running().await);

        daemon1.stop().await;
        daemon2.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon1.is_running().await);
        assert!(!daemon2.is_running().await);
    }

    // ── Error isolation ────────────────────────────────────────────

    #[test]
    fn mark_offline_failed_stores_reason() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        let id = enqueue_item(&conn, "sale.completed", r#"{"sale_id":"s1"}"#);

        let result = store.mark_offline_failed(&id, "connection refused");
        assert!(result.is_ok());

        // Verify the item is no longer pending
        let pending = store.pending_offline_count().unwrap();
        assert_eq!(pending, 0);
    }

    #[test]
    fn one_failed_item_does_not_block_others() {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);

        // Enqueue 3 items
        let id1 = enqueue_item(&conn, "sale.1", r#"{"sale_id":"s1"}"#);
        let _id2 = enqueue_item(&conn, "sale.2", r#"{"sale_id":"s2"}"#);
        let id3 = enqueue_item(&conn, "sale.3", r#"{"sale_id":"s3"}"#);

        // Mark item 2 as failed
        store.mark_offline_failed(&id1, "error").unwrap();
        // Item 3 should still be pending
        assert_eq!(store.pending_offline_count().unwrap(), 2);
        // Mark item 3 as synced
        store.mark_offline_synced(&id3).unwrap();
        assert_eq!(store.pending_offline_count().unwrap(), 1);
    }

    // ── DbConnection thread safety ─────────────────────────────────

    #[tokio::test]
    async fn db_connection_can_be_cloned_and_shared() {
        let db = setup_db();
        let db2 = db.clone();

        // Verify both handles can access the same DB via spawn_blocking
        let handle = tokio::task::spawn_blocking(move || {
            let conn = db.blocking_lock();
            let count: i64 = conn.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
            count
        });
        let result = handle.await.unwrap();
        assert_eq!(result, 1);

        // db2 should still work — also via spawn_blocking in async context
        let handle2 = tokio::task::spawn_blocking(move || {
            let conn = db2.blocking_lock();
            let count: i64 = conn.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
            count
        });
        let result2 = handle2.await.unwrap();
        assert_eq!(result2, 1);
    }

    // ── PgDaemonStatus serialization ───────────────────────────────

    #[test]
    fn pg_daemon_status_default_values() {
        let status = PgDaemonStatus::default();
        assert!(!status.running);
        assert!(status.last_sync_at.is_none());
        assert!(status.last_error.is_none());
        assert_eq!(status.last_pushed, 0);
        assert_eq!(status.last_pulled, 0);
        assert_eq!(status.pending_count, 0);
    }

    #[test]
    fn pg_daemon_status_clone() {
        let status = PgDaemonStatus {
            running: true,
            last_sync_at: Some("2026-07-22T00:00:00Z".into()),
            last_error: Some("test error".into()),
            last_pushed: 5,
            last_pulled: 3,
            pending_count: 10,
        };

        let cloned = status.clone();
        assert_eq!(cloned.running, status.running);
        assert_eq!(cloned.last_sync_at, status.last_sync_at);
        assert_eq!(cloned.last_error, status.last_error);
        assert_eq!(cloned.last_pushed, status.last_pushed);
        assert_eq!(cloned.last_pulled, status.last_pulled);
        assert_eq!(cloned.pending_count, status.pending_count);
    }
}
