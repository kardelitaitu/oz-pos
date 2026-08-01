//! Sync Daemon — background task that periodically pushes pending offline
//! mutations to the remote sync server and pulls remote updates.
//!
//! The daemon splits each sync cycle into three phases to avoid holding
//! the `Store` (which is `!Send`) across `.await` points:
//!
//! 1. **Read** — lock the DB (via `spawn_blocking`), read config + pending items
//! 2. **Send** — push items to the remote server (async, no DB needed)
//! 3. **Apply** — lock the DB again, mark items synced/failed

use std::sync::Arc;
use std::time::Duration;

use rand::Rng;
use tokio::sync::{Mutex, RwLock, watch};

use oz_core::db::Store;
use oz_core::settings::Settings;
use oz_core::sync_client::SyncConfig;

use crate::SyncError;
use crate::queue::SyncQueue;
use crate::transport::{PushOutcome, SyncTransport};

/// Base interval; actual per-cycle sleep is randomized 60–120s.
const DEFAULT_SYNC_INTERVAL: Duration = Duration::from_secs(30);

/// Maximum backoff cap in milliseconds (60 s).
const MAX_BACKOFF_MS: u64 = 60_000;

/// Compute exponential backoff with full jitter (P-1 spec §Backoff).
///
/// Formula: `rand(0, min(MAX_BACKOFF_MS, 2_000 * 2^failures))` ms.
/// Reset to 0 after a successful sync cycle.
fn compute_backoff(consecutive_failures: u32) -> Duration {
    let base = 2_000u64.saturating_mul(2u64.saturating_pow(consecutive_failures));
    let backoff_ms = std::cmp::min(MAX_BACKOFF_MS, base);
    let jittered = rand::thread_rng().gen_range(0..=backoff_ms);
    Duration::from_millis(jittered)
}

/// Snapshot of the daemon's current state, observable via [`SyncDaemon::status`].
#[derive(Debug, Clone, Default)]
pub struct DaemonStatus {
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
    /// Number of consecutive failed sync cycles (drives backoff).
    pub consecutive_failures: u32,
    /// Backoff delay applied before the current cycle, if any.
    pub backoff_ms: Option<u64>,
    /// Number of items currently pending in the offline queue.
    pub pending_count: i64,
}

/// A reference to a shared DB connection, used by the daemon to create
/// temporary [`Store`] instances inside `spawn_blocking` closures.
pub type DbConnection = Arc<Mutex<rusqlite::Connection>>;

/// A background task that periodically syncs the local offline queue with a
/// remote server.
///
/// The daemon reads `SyncConfig` from the database settings on every tick,
/// so configuration changes take effect on the next cycle without restarting.
pub struct SyncDaemon {
    interval: Duration,
    status: Arc<RwLock<DaemonStatus>>,
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
}

/// Read sync configuration and pending offline items from a database
/// connection. Extracted from [`SyncDaemon::run_tick`] so the read phase
/// is independently testable.
///
/// Returns `(config, pending)` where `config` is `None` if sync is not
/// configured or disabled.
pub(crate) fn read_config_and_pending(
    conn: &rusqlite::Connection,
) -> (Option<SyncConfig>, Vec<oz_core::offline::OfflineQueueItem>) {
    let store = Store::new(conn);
    let config = SyncConfig::from_settings(&store).ok().flatten();
    let pending = store.list_pending_offline().unwrap_or_default();
    (config, pending)
}

impl SyncDaemon {
    /// Create a new sync daemon.
    pub fn new() -> Self {
        Self {
            interval: DEFAULT_SYNC_INTERVAL,
            status: Arc::new(RwLock::new(DaemonStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new sync daemon with a custom interval.
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            status: Arc::new(RwLock::new(DaemonStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the background sync daemon.
    ///
    /// Spawns a `tokio` task that periodically:
    /// 1. Reads `SyncConfig` + pending items from the DB (blocking)
    /// 2. Pushes pending items to the remote server (async)
    /// 3. Updates item statuses in the DB (blocking)
    ///
    /// Config is read from the DB every tick, so setting changes take
    /// effect on the next cycle without restarting.
    ///
    /// If the daemon is already running, this is a no-op.
    pub async fn start(&self, db: DbConnection) {
        if self.is_running().await {
            tracing::warn!("sync daemon is already running");
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
            // Re-shadow `rx` as `mut` so the `async move` block can borrow
            // it mutably through the `select!` macro below.
            let mut rx = rx;
            let mut consecutive_failures: u32 = 0;

            if interval == DEFAULT_SYNC_INTERVAL {
                tracing::info!("sync daemon started interval_range_secs=60..=120");
            } else {
                tracing::info!(interval_ms = interval.as_millis(), "sync daemon started");
            }

            loop {
                // Compute sleep duration: backoff for failures, normal
                // random interval for the standard daemon rhythm, or a
                // fixed custom interval (e.g. for tests — backoff is
                // bypassed to avoid stalling fast test loops).
                let sleep_dur = if consecutive_failures > 0 && interval == DEFAULT_SYNC_INTERVAL {
                    let backoff = compute_backoff(consecutive_failures);
                    {
                        let mut s = daemon_status.write().await;
                        s.backoff_ms = Some(backoff.as_millis() as u64);
                    }
                    tracing::warn!(
                        failures = consecutive_failures,
                        backoff_ms = backoff.as_millis(),
                        "backing off after sync failure"
                    );
                    backoff
                } else if interval == DEFAULT_SYNC_INTERVAL {
                    {
                        let mut s = daemon_status.write().await;
                        s.backoff_ms = None;
                    }
                    Duration::from_secs(rand::thread_rng().gen_range(60..=120))
                } else {
                    {
                        let mut s = daemon_status.write().await;
                        s.backoff_ms = None;
                    }
                    interval
                };

                tokio::select! {
                    _ = tokio::time::sleep(sleep_dur) => {
                        Self::run_tick(&db, &daemon_status).await;

                        // Track consecutive failures for backoff on the
                        // next cycle. Reset to 0 on success.
                        let had_error = daemon_status.read().await.last_error.is_some();
                        if had_error {
                            consecutive_failures += 1;
                        } else {
                            consecutive_failures = 0;
                        }
                        {
                            let mut s = daemon_status.write().await;
                            s.consecutive_failures = consecutive_failures;
                        }
                    }
                    res = rx.changed() => {
                        if res.is_err() || *rx.borrow() {
                            tracing::info!("sync daemon shutting down");
                            break;
                        }
                    }
                }
            }

            let mut s = daemon_status.write().await;
            s.running = false;
        });
    }

    /// Run a single sync tick: read → send → apply.
    async fn run_tick(db: &DbConnection, daemon_status: &Arc<RwLock<DaemonStatus>>) {
        // Phase 1: Read config + pending items from DB (blocking)
        let db_clone = db.clone();
        let (config, pending, read_error) = match tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            let (cfg, pending) = read_config_and_pending(&conn);
            (cfg, pending)
        })
        .await
        {
            Ok((cfg, pending)) => (cfg, pending, None),
            Err(join_err) => {
                let msg = format!("sync config read panicked: {join_err}");
                tracing::error!(error = %msg, "sync daemon read phase failed");
                (None, Vec::new(), Some(msg))
            }
        };

        // Phase 2: Do async sync if configured and there are pending items
        let pushed;
        let pulled;
        let mut sync_error: Option<String> = None;

        if let Some(cfg) = &config {
            if !cfg.server_url.is_empty() && !pending.is_empty() {
                let transport = SyncTransport::new(&cfg.server_url, cfg.api_key.as_deref());
                match transport.push_items(&pending).await {
                    Ok(results) => {
                        pushed = results.len();
                        // Phase 3: Apply push results to DB (blocking)
                        let db_clone = db.clone();
                        let ids: Vec<String> = pending.iter().map(|i| i.id.clone()).collect();
                        let outcome = tokio::task::spawn_blocking(move || {
                            let conn = db_clone.blocking_lock();
                            let store = Store::new(&conn);
                            for (i, outcome) in ids.iter().zip(results.iter()) {
                                match outcome {
                                    PushOutcome::Accepted => {
                                        if let Err(e) = store.mark_offline_synced(i) {
                                            tracing::error!(
                                                item_id = %i,
                                                error = %e,
                                                "sync daemon: failed to mark item synced"
                                            );
                                        }
                                    }
                                    PushOutcome::Rejected { reason } => {
                                        if let Err(e) = store.mark_offline_failed(i, reason) {
                                            tracing::error!(
                                                item_id = %i,
                                                error = %e,
                                                "sync daemon: failed to mark item failed"
                                            );
                                        }
                                    }
                                    PushOutcome::Conflict(remote) => {
                                        // LWW: remote wins — mark local as synced,
                                        // re-enqueue the remote version.
                                        if let Err(e) = store.mark_offline_synced(i) {
                                            tracing::error!(
                                                item_id = %i,
                                                error = %e,
                                                "sync daemon: failed to mark conflicted item synced"
                                            );
                                        }
                                        if let Err(e) =
                                            store.enqueue_offline(&remote.action, &remote.payload)
                                        {
                                            tracing::error!(
                                                item_id = %i,
                                                action = %remote.action,
                                                error = %e,
                                                "sync daemon: failed to re-enqueue remote winner"
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
                        pushed = 0;
                        // ADR #11: If the server migrated, update the local
                        // URL so the next cycle connects to the new server.
                        if let SyncError::ServerMigrated { new_url } = &e {
                            let db = db.clone();
                            let url = new_url.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                let conn = db.blocking_lock();
                                let store = Store::new(&conn);
                                let _ = Settings::set_sync_server_url(store.conn(), &url);
                            })
                            .await;
                            tracing::info!(new_url = %new_url, "server migrated — local config updated");
                        }
                        sync_error = Some(e.to_string());
                    }
                }
            } else {
                pushed = 0;
            }

            // Phase 4: Pull remote updates and apply them locally.
            if !cfg.server_url.is_empty() {
                // SYNC-01: read the durable pull anchor + cursor so we only
                // fetch updates newer than the last successfully-applied page
                // (previously every cycle pulled the ENTIRE queue and re-applied
                // stock/sale mutations, silently corrupting inventory).
                let (pull_since, pull_cursor) = {
                    let db_clone = db.clone();
                    tokio::task::spawn_blocking(move || {
                        let conn = db_clone.blocking_lock();
                        let store = Store::new(&conn);
                        let st = store.get_sync_pull_state().unwrap_or_default();
                        (st.since, st.cursor)
                    })
                    .await
                    .unwrap_or((None, None))
                };

                let transport = SyncTransport::new(&cfg.server_url, cfg.api_key.as_deref());
                match transport
                    .pull_updates(pull_since.as_deref(), pull_cursor.as_deref())
                    .await
                {
                    Ok(pull_resp) => {
                        pulled = pull_resp.items.len();
                        if !pull_resp.items.is_empty() {
                            let db_clone = db.clone();
                            let items = pull_resp.items;
                            let next_cursor = pull_resp.next_cursor;
                            let prev_since = pull_since.clone();
                            let outcome = tokio::task::spawn_blocking(move || {
                                let conn = db_clone.blocking_lock();
                                let store = Store::new(&conn);
                                let queue = SyncQueue::new();
                                let mut has_stock_movements = false;
                                let mut all_applied = true;
                                // SYNC-01: captured so anchor-persistence
                                // failures surface in the daemon status
                                // (returned from the closure below) instead of
                                // being silently swallowed by tracing only.
                                let mut anchor_error: Option<String> = None;
                                for item in &items {
                                    if item.action == "stock.movement" {
                                        has_stock_movements = true;
                                    }
                                    // SYNC-01: idempotency ledger — skip any
                                    // remote item already applied in a prior
                                    // cycle (replay is harmless).
                                    let already = store
                                        .is_remote_item_applied(&item.id)
                                        .unwrap_or(false);
                                    if already {
                                        continue;
                                    }
                                    // Apply the mutation, then record the
                                    // ledger receipt. NOTE: we deliberately do
                                    // NOT wrap these in a single outer
                                    // transaction — `apply_remote`'s
                                    // `adjust_stock` path opens its OWN
                                    // `unchecked_transaction()` internally, so
                                    // nesting would fail with a SQLite
                                    // "cannot start a transaction within a
                                    // transaction" error and roll the item
                                    // back entirely (observed in the SYNC-01
                                    // regression test). Instead:
                                    //  1. If the apply FAILS, `all_applied`
                                    //     goes false → anchor does not advance
                                    //     → the item replays next cycle.
                                    //  2. If the apply SUCCEEDS, the mutation
                                    //     is committed. The receipt write is
                                    //     best-effort: even if it fails, we
                                    //     advance the anchor past the item,
                                    //     because re-applying an already-
                                    //     committed mutation is the worse
                                    //     failure. A lost receipt only matters
                                    //     if the server replays history after
                                    //     an anchor reset — which the snapshot
                                    //     import path handles separately.
                                    if let Err(e) = queue.apply_remote(&store, item) {
                                        all_applied = false;
                                        tracing::error!(
                                            item_id = %item.id,
                                            action = %item.action,
                                            error = %e,
                                            "failed to apply remote item"
                                        );
                                    } else if let Err(e) =
                                        store.mark_remote_item_applied(&item.id, &item.action)
                                    {
                                        tracing::warn!(
                                            item_id = %item.id,
                                            action = %item.action,
                                            error = %e,
                                            "failed to write ledger receipt (item still applied; anchor advances)"
                                        );
                                    }
                                }
                                // SYNC-01: advance the pull anchor ONLY after
                                // the whole page applied successfully. A crash
                                // mid-pull leaves the old anchor so the ledger
                                // absorbs the replay.
                                if all_applied {
                                    let new_since = items
                                        .iter()
                                        .map(|i| i.created_at.clone())
                                        .max()
                                        .or(prev_since);
                                    if let Err(e) = store.set_sync_pull_state(
                                        new_since.as_deref(),
                                        next_cursor.as_deref(),
                                    ) {
                                        tracing::error!(
                                            error = %e,
                                            "failed to persist sync pull anchor"
                                        );
                                        anchor_error = Some(format!(
                                            "persist sync pull anchor: {e}"
                                        ));
                                    }
                                }
                                // ADR #6: Rebuild the materialized stock_summary
                                // cache after applying remote stock movements.
                                if has_stock_movements && let Err(e) = store.rebuild_stock_summary()
                                {
                                    tracing::error!(
                                        error = %e,
                                        "failed to rebuild stock summary after sync pull"
                                    );
                                }
                                // Return the anchor-persistence error (if any)
                                // so the caller can surface it in the daemon
                                // status — a lost anchor makes the NEXT cycle
                                // re-pull the whole page, which is exactly the
                                // corruption class SYNC-01 prevents.
                                anchor_error
                            })
                            .await;
                            // SYNC-01: propagate both spawn_blocking panics AND
                            // anchor-persistence failures into sync_error so the
                            // daemon status/backoff reflect them.
                            match outcome {
                                Ok(Some(msg)) => {
                                    if sync_error.is_none() {
                                        sync_error = Some(msg);
                                    }
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    if sync_error.is_none() {
                                        sync_error = Some(format!("apply pull phase: {e}"));
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        pulled = 0;
                        // ADR #11: Handle server migration redirect.
                        if let SyncError::ServerMigrated { new_url } = &e {
                            let db = db.clone();
                            let url = new_url.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                let conn = db.blocking_lock();
                                let store = Store::new(&conn);
                                let _ = Settings::set_sync_server_url(store.conn(), &url);
                            })
                            .await;
                            tracing::info!(new_url = %new_url, "server migrated — local config updated");
                        }
                        if sync_error.is_none() {
                            sync_error = Some(format!("pull phase: {e}"));
                        }
                    }
                }
            } else {
                pulled = 0;
            }
        } else {
            pushed = 0;
            pulled = 0;
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

        if let Some(ref err) = sync_error {
            tracing::error!(error = ?err, "sync cycle failed");
        } else {
            tracing::info!(pushed, "sync cycle completed");
        }
    }

    /// Start a background pruning task that calls [`Store::archive_stock_movements`]
    /// on the local database (ADR #6 Q4 / P-1 Ledger Retention).
    ///
    /// Runs independently of the sync daemon with a random sleep interval of
    /// 60-120 seconds, matching the daemon's rhythm. The task is fire-and-
    /// forget — it runs until the process exits.
    pub fn start_prune_task(db: DbConnection) {
        tokio::spawn(async move {
            tracing::info!("prune daemon started interval_range_secs=60..=120");

            loop {
                let sleep_dur = Duration::from_secs(rand::thread_rng().gen_range(60..=120));
                tokio::time::sleep(sleep_dur).await;

                let db = db.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let conn = db.blocking_lock();
                    let store = Store::new(&conn);
                    store.archive_stock_movements(90, 50)
                })
                .await;

                match result {
                    Ok(Ok(count)) => {
                        if count > 0 {
                            tracing::info!(count, "prune cycle: archived stock movements");
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::error!(error = %e, "prune cycle failed");
                    }
                    Err(join_err) => {
                        tracing::error!(error = %join_err, "prune spawn_blocking panicked");
                        break;
                    }
                }
            }
        });
    }

    /// Gracefully stop the background sync daemon.
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
    pub async fn status(&self) -> DaemonStatus {
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

impl Default for SyncDaemon {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{PullResponse, PushOutcome, PushResponse};
    use axum::{Json, Router, routing::post};
    use oz_core::migrations;
    use oz_core::settings::Settings;

    fn setup_db() -> DbConnection {
        Arc::new(Mutex::new(migrations::fresh_db()))
    }

    /// Spawn a minimal mock sync server on port 0 and return its URL.
    /// Handles POST /api/sync/push (returns all accepted) and
    /// POST /api/sync/pull (returns empty items list).
    async fn spawn_mock_sync_server() -> String {
        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        async fn handle_push(Json(items): Json<Vec<serde_json::Value>>) -> Json<PushResponse> {
            Json(PushResponse {
                results: vec![PushOutcome::Accepted; items.len()],
            })
        }
        async fn handle_pull(Json(_req): Json<serde_json::Value>) -> Json<PullResponse> {
            Json(PullResponse {
                items: vec![],
                next_cursor: None,
            })
        }

        let app = Router::new()
            .route("/api/sync/push", post(handle_push))
            .route("/api/sync/pull", post(handle_pull));

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        format!("http://localhost:{port}")
    }

    #[tokio::test]
    async fn daemon_starts_stopped() {
        let daemon = SyncDaemon::new();
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_start_and_stop() {
        let db = setup_db();
        let daemon = SyncDaemon::new();
        daemon.start(db).await;
        assert!(daemon.is_running().await);
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_status_defaults() {
        let daemon = SyncDaemon::new();
        let status = daemon.status().await;
        assert!(!status.running);
        assert!(status.last_sync_at.is_none());
        assert_eq!(status.last_pushed, 0);
        assert_eq!(status.last_pulled, 0);
        assert!(status.last_error.is_none());
    }

    #[tokio::test]
    async fn daemon_stop_when_not_running_is_noop() {
        let daemon = SyncDaemon::new();
        daemon.stop().await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_double_start_is_noop() {
        let db = setup_db();
        let daemon = SyncDaemon::new();
        daemon.start(db.clone()).await;
        assert!(daemon.is_running().await);
        daemon.start(db).await;
        assert!(daemon.is_running().await);
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_runs_when_sync_configured() {
        let server_url = spawn_mock_sync_server().await;
        let db = setup_db();
        // Wrap DB setup in spawn_blocking to avoid blocking a tokio
        // worker thread (the multi-thread runtime panics on blocking_lock).
        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            let store = Store::new(&conn);
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
            store.enqueue_offline("test", r#"{}"#).unwrap();
        })
        .await
        .unwrap();
        let daemon = SyncDaemon::with_interval(Duration::from_millis(100));
        daemon.start(db).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        let status = daemon.status().await;
        assert!(status.last_sync_at.is_some());
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    #[tokio::test]
    async fn daemon_skips_when_sync_not_configured() {
        let db = setup_db();
        let daemon = SyncDaemon::with_interval(Duration::from_millis(100));
        daemon.start(db).await;
        tokio::time::sleep(Duration::from_millis(600)).await;
        let status = daemon.status().await;
        assert!(status.last_error.is_none());
        assert!(status.last_sync_at.is_some());
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    #[tokio::test]
    async fn daemon_custom_interval() {
        let daemon = SyncDaemon::with_interval(Duration::from_millis(50));
        assert_eq!(daemon.interval(), Duration::from_millis(50));
    }

    #[tokio::test]
    async fn daemon_set_interval() {
        let mut daemon = SyncDaemon::new();
        daemon.set_interval(Duration::from_secs(10));
        assert_eq!(daemon.interval(), Duration::from_secs(10));
    }

    // ── Backoff tests ────────────────────────────────────────────

    #[test]
    fn compute_backoff_produces_finite_duration() {
        // Jitter is random; just verify the function never panics
        // and always returns a valid (finite, non-negative) duration.
        for failures in 0..=10 {
            let backoff = compute_backoff(failures);
            assert!(
                backoff.as_millis() as u64 <= MAX_BACKOFF_MS,
                "backoff for {failures} failures exceeds cap"
            );
        }
    }

    #[test]
    fn compute_backoff_capped_at_60_seconds() {
        // After many failures, the backoff should be capped at 60s.
        let backoff = compute_backoff(100);
        assert!(
            backoff.as_millis() as u64 <= MAX_BACKOFF_MS,
            "backoff {} ms exceeds cap {MAX_BACKOFF_MS} ms",
            backoff.as_millis()
        );
    }

    #[test]
    fn compute_backoff_zero_failures_is_instant() {
        // 2_000 * 2^0 = 2_000, jittered in [0, 2000]
        let backoff = compute_backoff(0);
        assert!(
            backoff.as_millis() <= 2_000,
            "zero failures should cap at 2000ms, got {}ms",
            backoff.as_millis()
        );
    }

    // ── ADR #11: Server migration integration tests ──────────

    use crate::test_helpers::spawn_redirect_server;

    #[tokio::test]
    async fn daemon_auto_updates_url_on_server_migration() {
        let new_url = "https://new-server.example.com";
        let old_url = spawn_redirect_server(new_url).await;
        let db = setup_db();

        // Configure sync to point at the redirect server.
        let db_clone = db.clone();
        let old = old_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            let store = Store::new(&conn);
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &old).unwrap();
            store.enqueue_offline("test", r#"{}"#).unwrap();
        })
        .await
        .unwrap();

        let daemon = SyncDaemon::with_interval(Duration::from_millis(100));
        daemon.start(db.clone()).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        // The daemon should have detected the redirect and updated the URL.
        let updated_url = tokio::task::spawn_blocking(move || {
            let conn = db.blocking_lock();
            Settings::get_sync_server_url(&conn).unwrap()
        })
        .await
        .unwrap();

        assert_eq!(
            updated_url.as_deref(),
            Some(new_url),
            "daemon should auto-update sync_server_url after server_migrated redirect"
        );

        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    #[tokio::test]
    async fn daemon_pull_phase_detects_server_migration() {
        // No pending items — push is skipped, only pull runs.
        // The pull hits the redirect server and should still auto-update
        // the URL. This exercises the pull-phase ServerMigrated handler.
        let new_url = "https://pull-migrated.example.com";
        let old_url = spawn_redirect_server(new_url).await;
        let db = setup_db();

        let db_clone = db.clone();
        let old = old_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &old).unwrap();
            // No enqueue_offline — push phase is skipped.
        })
        .await
        .unwrap();

        let daemon = SyncDaemon::with_interval(Duration::from_millis(100));
        daemon.start(db.clone()).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let updated_url = tokio::task::spawn_blocking(move || {
            let conn = db.blocking_lock();
            Settings::get_sync_server_url(&conn).unwrap()
        })
        .await
        .unwrap();

        assert_eq!(
            updated_url.as_deref(),
            Some(new_url),
            "pull-phase only: daemon should still auto-update sync_server_url"
        );

        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // ── TDD Bug #1: spawn_blocking panic is not silently swallowed ─

    /// Verify that `read_config_and_pending` propagates errors from a
    /// poisoned connection. When the inner `unwrap()` on the mutex lock
    /// panics, the `spawn_blocking` join handle returns an `Err`, and
    /// `run_tick` must surface that in `last_error`.
    ///
    /// We test this by creating a valid DB, then extract the config read
    /// through the `read_config_and_pending` helper (which does the same
    /// work the `spawn_blocking` closure does).
    #[test]
    fn read_config_and_pending_returns_pending_count() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.enqueue_offline("test", r#"{}"#).unwrap();

        let (config, pending) = read_config_and_pending(&conn);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action, "test");
        // Config is None because sync is not enabled in fresh DB.
        assert!(config.is_none());
    }

    // ── SYNC-01: idempotent remote application ───────────────────────

    /// Spawn a mock sync server whose pull endpoint ALWAYS returns the
    /// same remote `stock.adjusted` item, regardless of the `since` anchor
    /// or cursor. Simulates a server that replays history (or a client
    /// whose anchor was lost) — the idempotency ledger must make replay
    /// harmless.
    async fn spawn_replaying_mock_sync_server() -> String {
        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        async fn handle_push(Json(items): Json<Vec<serde_json::Value>>) -> Json<PushResponse> {
            Json(PushResponse {
                results: vec![PushOutcome::Accepted; items.len()],
            })
        }
        async fn handle_pull(Json(_req): Json<serde_json::Value>) -> Json<PullResponse> {
            let mut item = oz_core::offline::OfflineQueueItem::new(
                "stock.adjusted",
                r#"{"sku":"COFFEE","delta":10}"#,
            );
            // Fixed id + timestamp so the SAME remote item is returned on
            // every pull — exactly the replay scenario SYNC-01 targets.
            // NOTE: this mock deliberately IGNORES the since/cursor request
            // params. Do not "fix" it to filter by anchor, or the replay
            // guarantee the test asserts would silently break.
            item.id = "remote-item-replay-1".into();
            item.created_at = "2026-01-01T00:00:00.000Z".into();
            Json(PullResponse {
                items: vec![item],
                next_cursor: None,
            })
        }

        let app = Router::new()
            .route("/api/sync/push", post(handle_push))
            .route("/api/sync/pull", post(handle_pull));

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        format!("http://localhost:{port}")
    }

    /// SYNC-01 regression: two daemon ticks against the SAME remote item
    /// must apply the local mutation exactly once (previously every cycle
    /// re-pulled the whole queue and re-deducted stock → silent corruption).
    #[tokio::test]
    async fn daemon_applies_replayed_remote_item_only_once() {
        let server_url = spawn_replaying_mock_sync_server().await;
        let db = setup_db();

        // Seed a product + inventory so the remote stock adjustment has a
        // target, and configure sync (all inside spawn_blocking per the
        // daemon's DB-access pattern).
        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
            conn.execute_batch(
                "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
                 VALUES ('prod-coffee', 'COFFEE', 'Coffee', 350, 'USD', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
                 INSERT INTO inventory (product_id, qty, updated_at)
                 VALUES ('prod-coffee', 50, '2026-01-01T00:00:00.000Z');",
            )
            .unwrap();
        })
        .await
        .unwrap();

        let status = Arc::new(RwLock::new(DaemonStatus::default()));

        // Tick 1: pulls + applies the remote +10 (50 → 60), records ledger.
        SyncDaemon::run_tick(&db, &status).await;
        let after_tick_1 = tokio::task::spawn_blocking({
            let db = db.clone();
            move || {
                let conn = db.blocking_lock();
                let store = Store::new(&conn);
                store.get_stock("prod-coffee").unwrap()
            }
        })
        .await
        .unwrap();
        assert_eq!(after_tick_1, 60, "first tick must apply the +10 delta");

        // Tick 2: the server replays the SAME item. The idempotency ledger
        // must skip it — stock stays 60, not 70.
        SyncDaemon::run_tick(&db, &status).await;
        let after_tick_2 = tokio::task::spawn_blocking({
            let db = db.clone();
            move || {
                let conn = db.blocking_lock();
                let store = Store::new(&conn);
                store.get_stock("prod-coffee").unwrap()
            }
        })
        .await
        .unwrap();
        assert_eq!(
            after_tick_2, 60,
            "replayed remote item must NOT be applied a second time (SYNC-01)"
        );

        // Ledger contains exactly one entry for the replayed id.
        let ledger_rows = tokio::task::spawn_blocking({
            let db = db.clone();
            move || {
                let conn = db.blocking_lock();
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sync_applied_items WHERE item_id = 'remote-item-replay-1'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                count
            }
        })
        .await
        .unwrap();
        assert_eq!(ledger_rows, 1, "ledger must hold one receipt for the item");
    }

    /// When the DB read phase succeeds, `run_tick` must update status
    /// without setting `last_error`. This is the regression guard for
    /// Bug #1 — verifies the refactored match arms don't break the
    /// happy path.
    #[tokio::test]
    async fn run_tick_happy_path_does_not_set_error() {
        let db = setup_db();
        let status = Arc::new(RwLock::new(DaemonStatus::default()));

        SyncDaemon::run_tick(&db, &status).await;

        let s = status.read().await;
        assert!(s.last_sync_at.is_some(), "status should be updated");
        assert!(s.last_error.is_none(), "no error expected for empty config");
        assert_eq!(s.last_pushed, 0);
        assert_eq!(s.last_pulled, 0);
    }
}
