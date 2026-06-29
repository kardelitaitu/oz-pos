//! PostgreSQL Sync Daemon — background task that periodically pushes pending
//! offline mutations directly to a remote PostgreSQL database.
//!
//! Operates similarly to [`crate::daemon::SyncDaemon`] but uses a
//! [`PgTransport`] instead of HTTP transport. Configuration is read from
//! the local settings table on every tick, so changes take effect without
//! restarting.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, RwLock, watch};

use oz_core::db::Store;
use oz_core::settings::Settings;

use crate::pg_transport::PgTransport;

/// Default interval between PG sync cycles (60 seconds — PG sync is
/// typically less time-sensitive than HTTP sync).
const DEFAULT_PG_SYNC_INTERVAL: Duration = Duration::from_secs(60);

/// Snapshot of the PG daemon's current state, observable via
/// [`PgSyncDaemon::status`].
#[derive(Debug, Clone, Default)]
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
}

impl PgSyncDaemon {
    /// Create a new PostgreSQL sync daemon.
    pub fn new() -> Self {
        Self {
            interval: DEFAULT_PG_SYNC_INTERVAL,
            status: Arc::new(RwLock::new(PgDaemonStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new PostgreSQL sync daemon with a custom interval.
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            status: Arc::new(RwLock::new(PgDaemonStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
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
                        Self::run_tick(&db, &daemon_status).await;
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
    async fn run_tick(db: &DbConnection, daemon_status: &Arc<RwLock<PgDaemonStatus>>) {
        // Phase 1: Read PG settings + pending items from local DB (blocking)
        let db_clone = db.clone();
        let (pg_config, pending) = tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            let store = Store::new(&conn);

            let enabled = Settings::is_pg_sync_enabled(&conn).unwrap_or(false);
            let pending = store.list_pending_offline().unwrap_or_default();

            let pg_config = if enabled && !pending.is_empty() {
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

                if !host.is_empty() && !dbname.is_empty() {
                    Some((host, port, dbname, user, password))
                } else {
                    None
                }
            } else {
                None
            };

            (pg_config, pending)
        })
        .await
        .unwrap_or_default();

        // Phase 2: Do async PG sync if configured
        let mut pushed = 0usize;
        let pulled = 0usize;
        let mut sync_error: Option<String> = None;

        if let Some((host, port, dbname, user, password)) = &pg_config {
            let port_u16: u16 = port.parse().unwrap_or(5432);
            match PgTransport::new(host, port_u16, dbname, user, password) {
                Ok(transport) => {
                    match transport.push_items(&pending).await {
                        Ok(results) => {
                            pushed = results.len();
                            // Phase 3: Apply results to local DB (blocking)
                            let db_clone = db.clone();
                            let ids: Vec<String> = pending.iter().map(|i| i.id.clone()).collect();
                            let outcome = tokio::task::spawn_blocking(move || {
                                let conn = db_clone.blocking_lock();
                                let store = Store::new(&conn);
                                for (i, outcome) in ids.iter().zip(results.iter()) {
                                    match outcome {
                                        crate::transport::PushOutcome::Accepted => {
                                            let _ = store.mark_offline_synced(i);
                                        }
                                        crate::transport::PushOutcome::Rejected { reason } => {
                                            let _ = store.mark_offline_failed(i, reason);
                                        }
                                        crate::transport::PushOutcome::Conflict(remote) => {
                                            let _ = store.mark_offline_synced(i);
                                            let _ = store
                                                .enqueue_offline(&remote.action, &remote.payload);
                                        }
                                    }
                                }
                            })
                            .await;

                            if let Err(e) = outcome {
                                sync_error = Some(format!("apply phase: {e}"));
                            }
                        }
                        Err(e) => {
                            sync_error = Some(e.to_string());
                        }
                    }
                }
                Err(e) => {
                    sync_error = Some(format!("failed to create pg transport: {e}"));
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
        s.last_error = sync_error.clone();

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

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn setup_db() -> DbConnection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        Arc::new(Mutex::new(conn))
    }

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
}
