//! Exchange rate auto-sync daemon.
//!
//! A background task that periodically fetches exchange rates from the
//! Frankfurter public API (`https://api.frankfurter.app`) and stores them
//! in the `exchange_rates` table using [`oz_core::db::Store::upsert_exchange_rate`].

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use oz_core::db::Store;
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock, watch};
use tracing;

/// A reference to a shared DB connection, used by the daemon to create
/// temporary [`Store`] instances inside `spawn_blocking` closures.
pub type DbConnection = Arc<std::sync::Mutex<rusqlite::Connection>>;

/// Snapshot of the daemon's current state.
#[derive(Debug, Clone, Default)]
pub struct RateSyncStatus {
    /// Whether the daemon is currently running.
    pub running: bool,
    /// ISO-8601 timestamp of the last completed sync cycle.
    pub last_sync_at: Option<String>,
    /// Number of rates updated in the last cycle.
    pub rates_updated: usize,
    /// Base currency used in the last cycle.
    pub base_currency: String,
    /// Error message from the last cycle, if any.
    pub last_error: Option<String>,
}

/// Response shape from the Frankfurter API.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FrankfurterResponse {
    amount: f64,
    base: String,
    date: String,
    rates: HashMap<String, f64>,
}

/// Default interval between sync cycles (6 hours).
const DEFAULT_SYNC_INTERVAL_MINUTES: u64 = 360;

/// A background task that periodically fetches exchange rates from the
/// Frankfurter public API and stores them in the database.
///
/// Settings are read from the database on every tick, so configuration
/// changes take effect on the next cycle without restarting.
pub struct RateSyncDaemon {
    interval: Duration,
    status: Arc<RwLock<RateSyncStatus>>,
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
}

impl RateSyncDaemon {
    /// Create a new rate sync daemon with the default interval (6 hours).
    pub fn new() -> Self {
        Self {
            interval: Duration::from_secs(DEFAULT_SYNC_INTERVAL_MINUTES * 60),
            status: Arc::new(RwLock::new(RateSyncStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new rate sync daemon with a custom interval.
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            status: Arc::new(RwLock::new(RateSyncStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the background rate sync daemon.
    ///
    /// Spawns a `tokio` task that periodically:
    /// 1. Reads settings from the DB (blocking)
    /// 2. Fetches rates from the Frankfurter API (async)
    /// 3. Stores rates in the DB (blocking)
    ///
    /// If the daemon is already running, this is a no-op.
    pub async fn start(&self, db: DbConnection) {
        if self.is_running().await {
            tracing::warn!("rate sync daemon is already running");
            return;
        }

        let (tx, rx) = watch::channel(false);
        *self.shutdown_tx.lock().await = Some(tx);

        let daemon_status = Arc::clone(&self.status);

        {
            let mut s = daemon_status.write().await;
            s.running = true;
            s.last_error = None;
        }

        let http_client = reqwest::Client::new();
        let interval = self.interval;

        tokio::spawn(async move {
            let mut rx = rx;

            tracing::info!("rate sync daemon started");

            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        Self::run_tick(&db, &daemon_status, &http_client).await;
                    }
                    res = rx.changed() => {
                        if res.is_err() || *rx.borrow() {
                            tracing::info!("rate sync daemon shutting down");
                            break;
                        }
                    }
                }
            }

            let mut s = daemon_status.write().await;
            s.running = false;
        });
    }

    async fn run_tick(
        db: &DbConnection,
        daemon_status: &Arc<RwLock<RateSyncStatus>>,
        client: &reqwest::Client,
    ) {
        let db_clone = db.clone();
        let (enabled, base_currency, interval_minutes) = tokio::task::spawn_blocking(move || {
            let conn = db_clone.lock().unwrap();
            let enabled = oz_core::settings::Settings::is_rate_sync_enabled(&conn).unwrap_or(false);
            let base = oz_core::settings::Settings::get_rate_sync_base_currency(&conn)
                .unwrap_or_else(|_| "USD".into());
            let interval = oz_core::settings::Settings::get_rate_sync_interval(&conn)
                .unwrap_or_else(|_| "360".into());
            (enabled, base, interval)
        })
        .await
        .unwrap_or((false, "USD".into(), "360".into()));

        // Update status with base currency
        {
            let mut s = daemon_status.write().await;
            s.base_currency = base_currency.clone();
        }

        if !enabled {
            tracing::debug!("rate sync is disabled, skipping cycle");
            return;
        }

        let _ = interval_minutes; // used for logging if needed

        // Fetch rates from the Frankfurter API
        let url = format!("https://api.frankfurter.app/latest?from={base_currency}");
        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("HTTP request failed: {e}");
                tracing::error!(error = ?err_msg, "rate sync fetch failed");
                let mut s = daemon_status.write().await;
                s.last_sync_at =
                    Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
                s.last_error = Some(err_msg);
                s.rates_updated = 0;
                return;
            }
        };

        let parsed: FrankfurterResponse = match resp.json().await {
            Ok(p) => p,
            Err(e) => {
                let err_msg = format!("JSON parse failed: {e}");
                tracing::error!(error = ?err_msg, "rate sync parse failed");
                let mut s = daemon_status.write().await;
                s.last_sync_at =
                    Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
                s.last_error = Some(err_msg);
                s.rates_updated = 0;
                return;
            }
        };

        // All rates returned are FROM base_currency TO target_currency
        let effective_date = parsed.date.clone();
        let source = "auto-sync";
        let rates = parsed.rates.clone();
        let count = rates.len();
        let base = parsed.base.clone();

        // Store rates in the DB (blocking)
        let db_clone = db.clone();
        let base_inner = base.clone();
        let date_inner = effective_date.clone();
        let result = tokio::task::spawn_blocking(move || {
            let conn = db_clone.lock().unwrap();
            let store = Store::new(&conn);
            let mut updated = 0usize;
            for (to_currency, rate) in &rates {
                if let Err(e) =
                    store.upsert_exchange_rate(&base_inner, to_currency, *rate, source, &date_inner)
                {
                    tracing::warn!(
                        from = %base_inner,
                        to = %to_currency,
                        error = %e,
                        "failed to upsert exchange rate"
                    );
                } else {
                    updated += 1;
                }
            }
            updated
        })
        .await
        .unwrap_or(0);

        tracing::info!(
            base_currency = %base,
            rates = count,
            updated = result,
            date = %effective_date,
            "rate sync cycle completed"
        );

        let mut s = daemon_status.write().await;
        s.last_sync_at =
            Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        s.rates_updated = result;
        s.last_error = None;
    }

    /// Gracefully stop the background rate sync daemon.
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
    pub async fn status(&self) -> RateSyncStatus {
        self.status.read().await.clone()
    }

    /// Set the sync interval (applied on next cycle).
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    /// Get the current sync interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

impl Default for RateSyncDaemon {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn daemon_starts_stopped() {
        let daemon = RateSyncDaemon::new();
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_start_and_stop() {
        let conn = oz_core::migrations::fresh_db();
        let db = Arc::new(std::sync::Mutex::new(conn));
        let daemon = RateSyncDaemon::new();
        daemon.start(db).await;
        assert!(daemon.is_running().await);
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_status_defaults() {
        let daemon = RateSyncDaemon::new();
        let status = daemon.status().await;
        assert!(!status.running);
        assert!(status.last_sync_at.is_none());
        assert_eq!(status.rates_updated, 0);
        assert!(status.last_error.is_none());
    }

    #[tokio::test]
    async fn daemon_custom_interval() {
        let daemon = RateSyncDaemon::with_interval(Duration::from_millis(50));
        assert_eq!(daemon.interval(), Duration::from_millis(50));
    }

    #[tokio::test]
    async fn daemon_set_interval() {
        let mut daemon = RateSyncDaemon::new();
        daemon.set_interval(Duration::from_secs(10));
        assert_eq!(daemon.interval(), Duration::from_secs(10));
    }

    #[tokio::test]
    async fn daemon_stop_when_not_running_is_noop() {
        let daemon = RateSyncDaemon::new();
        daemon.stop().await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn daemon_double_start_is_noop() {
        let conn = oz_core::migrations::fresh_db();
        let db = Arc::new(std::sync::Mutex::new(conn));
        let daemon = RateSyncDaemon::new();
        daemon.start(db.clone()).await;
        assert!(daemon.is_running().await);
        daemon.start(db).await;
        assert!(daemon.is_running().await);
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!daemon.is_running().await);
    }

    #[tokio::test]
    async fn frankfurter_response_deserialization() {
        let json = r#"{
            "amount": 1.0,
            "base": "USD",
            "date": "2026-06-30",
            "rates": {
                "EUR": 0.9234,
                "GBP": 0.7932,
                "JPY": 149.85
            }
        }"#;
        let resp: FrankfurterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.base, "USD");
        assert_eq!(resp.date, "2026-06-30");
        assert!((resp.rates["EUR"] - 0.9234).abs() < 0.0001);
        assert!((resp.rates["GBP"] - 0.7932).abs() < 0.0001);
        assert!((resp.rates["JPY"] - 149.85).abs() < 0.01);
        assert_eq!(resp.rates.len(), 3);
    }
}
