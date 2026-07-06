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

            if interval == DEFAULT_SYNC_INTERVAL {
                tracing::info!("sync daemon started interval_range_secs=60..=120");
            } else {
                tracing::info!(interval_ms = interval.as_millis(), "sync daemon started");
            }

            loop {
                let sleep_dur = if interval == DEFAULT_SYNC_INTERVAL {
                    Duration::from_secs(rand::thread_rng().gen_range(60..=120))
                } else {
                    interval
                };
                tokio::select! {
                    _ = tokio::time::sleep(sleep_dur) => {
                        Self::run_tick(&db, &daemon_status).await;
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
                match transport.pull_updates(None).await {
                    Ok(pull_resp) => {
                        pulled = pull_resp.items.len();
                        if !pull_resp.items.is_empty() {
                            let db_clone = db.clone();
                            let items = pull_resp.items;
                            let outcome = tokio::task::spawn_blocking(move || {
                                let conn = db_clone.blocking_lock();
                                let store = Store::new(&conn);
                                let queue = SyncQueue::new();
                                for item in &items {
                                    if let Err(e) = queue.apply_remote(&store, item) {
                                        tracing::error!(
                                            item_id = %item.id,
                                            action = %item.action,
                                            error = %e,
                                            "failed to apply remote item"
                                        );
                                    }
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
    use oz_core::migrations;
    use oz_core::settings::Settings;
    use rusqlite::Connection;

    fn setup_db() -> DbConnection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        Arc::new(Mutex::new(conn))
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

    #[ignore]
    #[tokio::test(flavor = "multi_thread")]
    async fn daemon_runs_when_sync_configured() {
        let db = setup_db();
        {
            let conn = db.blocking_lock();
            let store = Store::new(&conn);
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, "http://localhost:3099").unwrap();
            store.enqueue_offline("test", r#"{}"#).unwrap();
        }
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
}
