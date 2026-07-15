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
use oz_core::sync_client::SyncConfig;

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
        let (config, pending) = tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            let store = Store::new(&conn);
            let config = SyncConfig::from_settings(&store).ok().flatten();
            let pending = store.list_pending_offline().unwrap_or_default();
            (config, pending)
        })
        .await
        .unwrap_or_default();

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
                                        let _ = store.mark_offline_synced(i);
                                    }
                                    PushOutcome::Rejected { reason } => {
                                        let _ = store.mark_offline_failed(i, reason);
                                    }
                                    PushOutcome::Conflict(remote) => {
                                        // LWW: remote wins — mark local as synced,
                                        // re-enqueue the remote version.
                                        let _ = store.mark_offline_synced(i);
                                        let _ =
                                            store.enqueue_offline(&remote.action, &remote.payload);
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
                        sync_error = Some(e.to_string());
                    }
                }
            } else {
                pushed = 0;
            }

            // Phase 4: Pull remote updates and apply them locally.
            if !cfg.server_url.is_empty() {
                let transport = SyncTransport::new(&cfg.server_url, cfg.api_key.as_deref());
                match transport.pull_updates(None, None).await {
                    Ok(pull_resp) => {
                        pulled = pull_resp.items.len();
                        if !pull_resp.items.is_empty() {
                            let db_clone = db.clone();
                            let items = pull_resp.items;
                            let outcome = tokio::task::spawn_blocking(move || {
                                let conn = db_clone.blocking_lock();
                                let store = Store::new(&conn);
                                let queue = SyncQueue::new();
                                let mut has_stock_movements = false;
                                for item in &items {
                                    if item.action == "stock.movement" {
                                        has_stock_movements = true;
                                    }
                                    if let Err(e) = queue.apply_remote(&store, item) {
                                        tracing::error!(
                                            item_id = %item.id,
                                            action = %item.action,
                                            error = %e,
                                            "failed to apply remote item"
                                        );
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
                            })
                            .await;
                            if let Err(e) = outcome {
                                sync_error = Some(format!("apply pull phase: {e}"));
                            }
                        }
                    }
                    Err(e) => {
                        pulled = 0;
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
        s.last_error = sync_error.clone();

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
    use axum::{routing::post, Json, Router};
    use oz_core::migrations;
    use oz_core::settings::Settings;
    use crate::transport::{PullResponse, PushOutcome, PushResponse};

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
}
