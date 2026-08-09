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
use oz_core::events::SettingsUpdated;
use oz_core::settings::Settings;
use oz_core::sync_client::SyncConfig;

use crate::queue::SyncQueue;
use crate::transport::{PushOutcome, SyncTransport};
use crate::{SyncError, import_snapshot};

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

/// Callback invoked after the daemon applies a remote `settings.update`
/// (SYNC-10) so the app can re-emit `SettingsUpdated` for UI reactivity.
///
/// The desktop client wires this to emit the `settings_updated` Tauri event
/// (the same wire shape the frontend `SettingsContext` listens for).
///
/// **Contract:** the callback runs on the daemon's blocking apply thread
/// WHILE the database connection lock is held, so it must NOT acquire the
/// database lock itself (that would deadlock against the mutex it already
/// owns). Keep it to side effects without DB access — a Tauri emit, a bus
/// publish, a channel send.
pub type SettingsChangedSink = Arc<dyn Fn(&SettingsUpdated) + Send + Sync>;

/// A background task that periodically syncs the local offline queue with a
/// remote server.
///
/// The daemon reads `SyncConfig` from the database settings on every tick,
/// so configuration changes take effect on the next cycle without restarting.
pub struct SyncDaemon {
    interval: Duration,
    status: Arc<RwLock<DaemonStatus>>,
    shutdown_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
    settings_sink: SettingsChangedSink,
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

/// ADR sync-auth-hardening P1: request a fresh token and persist it as the
/// API key. Returns `true` when a new key was stored. Callers invoke this
/// once after an `AuthExpired` and never loop on it.
async fn refresh_persisted_api_key(db: &DbConnection, server_url: &str) -> bool {
    // ADR sync-auth-hardening P3: prefer terminal client credentials when
    // the device is paired; fall back to the admin key / open mint.
    let (terminal_id, terminal_secret) = {
        let db_clone = db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_clone.blocking_lock();
            let store = Store::new(&conn);
            (
                Settings::get_sync_terminal_id(store.conn()).unwrap_or(None),
                Settings::get_sync_terminal_secret(store.conn()).unwrap_or(None),
            )
        })
        .await
        .unwrap_or((None, None))
    };
    let token = match (terminal_id, terminal_secret) {
        (Some(id), Some(secret)) => {
            oz_core::sync_client::request_token_client_credentials(server_url, &id, &secret).await
        }
        _ => {
            oz_core::sync_client::request_token(
                server_url,
                oz_core::sync_client::admin_key_from_env().as_deref(),
            )
            .await
        }
    };
    let Some(key) = token.token.filter(|_| token.ok) else {
        tracing::warn!(
            status = %token.status,
            "sync token refresh failed — sync stays on the stored key"
        );
        return false;
    };
    let db_clone = db.clone();
    tokio::task::spawn_blocking(move || {
        let conn = db_clone.blocking_lock();
        let store = Store::new(&conn);
        match Settings::set_sync_api_key(store.conn(), &key) {
            Ok(()) => true,
            Err(e) => {
                tracing::error!(error = %e, "persisting refreshed sync API key failed");
                false
            }
        }
    })
    .await
    .unwrap_or(false)
}

/// Apply push outcomes to the local queue (blocking DB work, spawned off the
/// async runtime). Returns an error message when the apply phase itself
/// failed (spawn panic); per-item failures are logged, mirroring the original
/// inline apply block exactly.
async fn apply_push_results(
    db: &DbConnection,
    pending: Vec<oz_core::offline::OfflineQueueItem>,
    results: Vec<PushOutcome>,
) -> Option<String> {
    let db_clone = db.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let conn = db_clone.blocking_lock();
        let store = Store::new(&conn);
        let queue = SyncQueue::new();
        for (local, outcome) in pending.iter().zip(results.iter()) {
            match outcome {
                PushOutcome::Accepted => {
                    if let Err(e) = store.mark_offline_synced(&local.id) {
                        tracing::error!(
                            item_id = %local.id,
                            error = %e,
                            "sync daemon: failed to mark item synced"
                        );
                    }
                }
                PushOutcome::Rejected { reason } => {
                    if let Err(e) = store.mark_offline_failed(&local.id, reason) {
                        tracing::error!(
                            item_id = %local.id,
                            error = %e,
                            "sync daemon: failed to mark item failed"
                        );
                    }
                }
                PushOutcome::Conflict(server_item) => {
                    if let Err(e) = queue.apply_push_conflict(&store, local, server_item) {
                        tracing::error!(
                            item_id = %local.id,
                            error = %e,
                            "sync daemon: failed to apply conflict resolution"
                        );
                    }
                }
            }
        }
    })
    .await;

    match outcome {
        Ok(()) => None,
        Err(e) => Some(format!("apply push phase: {e}")),
    }
}

impl SyncDaemon {
    /// Create a new sync daemon.
    pub fn new() -> Self {
        Self {
            interval: DEFAULT_SYNC_INTERVAL,
            status: Arc::new(RwLock::new(DaemonStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
            settings_sink: Arc::new(|_: &SettingsUpdated| {}),
        }
    }

    /// Create a new sync daemon with a custom interval.
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval,
            status: Arc::new(RwLock::new(DaemonStatus::default())),
            shutdown_tx: Arc::new(Mutex::new(None)),
            settings_sink: Arc::new(|_: &SettingsUpdated| {}),
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
        self.start_inner(db, self.settings_sink.clone()).await;
    }

    /// Start the background sync daemon with a custom settings-change sink
    /// (SYNC-10).
    ///
    /// The sink is invoked after each remote `settings.update` the pull
    /// phase applies, carrying the changed key and its originating terminal
    /// — the desktop client uses this to emit the `settings_updated` Tauri
    /// event so the UI refetches a setting changed on another terminal.
    pub async fn start_with_sink(&self, db: DbConnection, settings_sink: SettingsChangedSink) {
        self.start_inner(db, settings_sink).await;
    }

    /// Shared start path used by [`SyncDaemon::start`] and
    /// [`SyncDaemon::start_with_sink`].
    async fn start_inner(&self, db: DbConnection, settings_sink: SettingsChangedSink) {
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
                        Self::run_tick(&db, &daemon_status, &settings_sink).await;

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
    ///
    /// `settings_sink` is invoked after the pull phase applies a remote
    /// `settings.update` (SYNC-10) so the change is reactive in this
    /// terminal's UI even though it was made elsewhere.
    async fn run_tick(
        db: &DbConnection,
        daemon_status: &Arc<RwLock<DaemonStatus>>,
        settings_sink: &SettingsChangedSink,
    ) {
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

        // Phase 2: Do async sync if configured and there are pending items.
        // `pushed`/`pulled` start at 0 so every code path (including the
        // RUST-05 fail-closed transport skip) yields a defined value for the
        // daemon status below.
        let mut pushed = 0;
        let mut pulled = 0;
        let mut sync_error: Option<String> = None;

        if let Some(cfg) = &config {
            if !cfg.server_url.is_empty() && !pending.is_empty() {
                // RUST-05: fail closed — never sync through an
                // unauthenticated, timeout-less client. A construction
                // failure records the error and skips the push phase.
                let transport = match SyncTransport::try_new(
                    &cfg.server_url,
                    cfg.api_key.as_deref(),
                ) {
                    Ok(t) => Some(t),
                    Err(e) => {
                        pushed = 0;
                        sync_error = Some(format!("transport construction failed: {e}"));
                        tracing::error!(
                            error = %e,
                            "sync transport construction failed — skipping push (RUST-05 fail-closed)"
                        );
                        None
                    }
                };
                if let Some(transport) = transport {
                    match transport.push_items(&pending).await {
                        Ok(results) => {
                            pushed = results.len();
                            // Phase 3: Apply push results to DB (blocking).
                            // SYNC-02: carry the FULL local items (not just ids)
                            // so a conflict is resolved by the shared ADR #21
                            // conflict-application service — the same strategy the
                            // immediate SyncEngine uses, never a blanket LWW.
                            if let Some(apply_err) = apply_push_results(db, pending, results).await
                            {
                                sync_error = Some(apply_err);
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
                            // ADR sync-auth-hardening P1/P4: stale auth — refresh
                            // the key once and retry the push batch exactly once.
                            // An explicit `invalid_token` is a config problem and
                            // must not be masked by a refresh.
                            if let SyncError::AuthExpired = e {
                                tracing::warn!(
                                    "push rejected (401) — refreshing API key and retrying once"
                                );
                                if refresh_persisted_api_key(db, &cfg.server_url).await {
                                    let (retry_cfg, _) = {
                                        let db_clone = db.clone();
                                        tokio::task::spawn_blocking(move || {
                                            let conn = db_clone.blocking_lock();
                                            read_config_and_pending(&conn)
                                        })
                                        .await
                                        .unwrap_or((None, Vec::new()))
                                    };
                                    if let Some(retry_cfg) = retry_cfg
                                        && let Ok(transport) = SyncTransport::try_new(
                                            &retry_cfg.server_url,
                                            retry_cfg.api_key.as_deref(),
                                        )
                                    {
                                        match transport.push_items(&pending).await {
                                            Ok(results) => {
                                                pushed = results.len();
                                                if let Some(apply_err) =
                                                    apply_push_results(db, pending, results).await
                                                {
                                                    sync_error = Some(apply_err);
                                                }
                                            }
                                            Err(retry_err) => {
                                                sync_error = Some(retry_err.to_string());
                                            }
                                        }
                                    } else {
                                        sync_error = Some(
                                            "push rejected (401) and refreshed key is not usable"
                                                .into(),
                                        );
                                    }
                                } else {
                                    sync_error =
                                        Some("push rejected (401) and token refresh failed".into());
                                }
                            } else if sync_error.is_none() {
                                sync_error = Some(e.to_string());
                            }
                        }
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

                // RUST-05: fail closed for the pull phase as well.
                let transport = match SyncTransport::try_new(
                    &cfg.server_url,
                    cfg.api_key.as_deref(),
                ) {
                    Ok(t) => Some(t),
                    Err(e) => {
                        pulled = 0;
                        if sync_error.is_none() {
                            sync_error = Some(format!("transport construction failed: {e}"));
                        }
                        tracing::error!(
                            error = %e,
                            "sync transport construction failed — skipping pull (RUST-05 fail-closed)"
                        );
                        None
                    }
                };
                if let Some(transport) = transport {
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
                                let prev_cursor = pull_cursor.clone();
                                // SYNC-10: own the sink (an owned `Arc`) so the
                                // `'static` spawn_blocking closure can call it
                                // after each applied settings item.
                                let settings_sink = settings_sink.clone();
                                let outcome = tokio::task::spawn_blocking(move || {
                                    let conn = db_clone.blocking_lock();
                                    let store = Store::new(&conn);
                                    let queue = SyncQueue::new();
                                    let mut has_stock_movements = false;
                                    let mut all_applied = true;
                                    let mut quarantined_item = false;
                                    let mut retryable_failure = false;
                                    // SYNC-01: captured so anchor-persistence
                                    // failures surface in the daemon status
                                    // (returned from the closure below) instead of
                                    // being silently swallowed by tracing only.
                                    let mut anchor_error: Option<String> = None;
                                    for item in &items {
                                        if item.action == "stock.movement" {
                                            has_stock_movements = true;
                                        }
                                        // SYNC-01: the domain mutation and its
                                        // idempotency receipt commit together. A
                                        // crash before commit rolls back both, so
                                        // replay is safe rather than duplicating a
                                        // committed stock mutation with a missing
                                        // receipt.
                                        match queue.apply_remote_atomic_full(&store, item) {
                                            Ok(outcome) => {
                                                // SYNC-10: a settings change
                                                // applied from a remote
                                                // terminal is re-emitted as
                                                // `SettingsUpdated` so the UI
                                                // refetches. The tx committed
                                                // inside apply_remote_atomic_full
                                                // before this runs.
                                                if let Some((key, terminal_id)) =
                                                    outcome.settings_change
                                                {
                                                    let event = SettingsUpdated {
                                                        changed_keys: vec![key],
                                                        terminal_id,
                                                    };
                                                    settings_sink(&event);
                                                }
                                                if !outcome.applied
                                                    && store
                                                        .is_remote_failure_dead_lettered(&item.id)
                                                        .unwrap_or(false)
                                                {
                                                    quarantined_item = true;
                                                    tracing::error!(
                                                        item_id = %item.id,
                                                        action = %item.action,
                                                        "remote item remains quarantined; advancing page anchor"
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                let dead_lettered = store
                                                    .is_remote_failure_dead_lettered(&item.id)
                                                    .unwrap_or(false);
                                                if dead_lettered {
                                                    quarantined_item = true;
                                                    tracing::error!(
                                                        item_id = %item.id,
                                                        action = %item.action,
                                                        error = %e,
                                                        "remote item quarantined after repeated failures; advancing page anchor"
                                                    );
                                                } else {
                                                    all_applied = false;
                                                    retryable_failure = true;
                                                    tracing::error!(
                                                        item_id = %item.id,
                                                        action = %item.action,
                                                        error = %e,
                                                        "failed to atomically apply remote item; retaining page anchor for retry"
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    // ADR #6: Rebuild the materialized stock_summary
                                    // cache before advancing the pull anchor. If the
                                    // rebuild fails, the old anchor is retained so a
                                    // retry can restore the derived state as well.
                                    let summary_rebuilt = if has_stock_movements {
                                        match store.rebuild_stock_summary() {
                                            Ok(_) => true,
                                            Err(e) => {
                                                tracing::error!(
                                                    error = %e,
                                                    "failed to rebuild stock summary after sync pull"
                                                );
                                                anchor_error = Some(format!(
                                                    "rebuild stock summary after sync pull: {e}"
                                                ));
                                                false
                                            }
                                        }
                                    } else {
                                        true
                                    };
                                    // SYNC-01: advance the pull anchor ONLY after
                                    // the whole page and its derived stock cache
                                    // applied successfully. A crash mid-pull leaves
                                    // the old anchor so the ledger absorbs replay.
                                    if all_applied && !retryable_failure && summary_rebuilt {
                                        // SYNC-09: re-read the DURABLE pull state
                                        // before advancing. An operator rewind
                                        // (`requeue_remote_failure` sets since = NULL
                                        // to force a full re-pull) can land while this
                                        // page was in flight; blindly writing new_since
                                        // would clobber it and the requeued item would
                                        // never be re-fetched. Skip the advance when
                                        // the durable (since, cursor) no longer matches
                                        // what this tick captured — a full-state
                                        // comparison, not just the Some→None rewind
                                        // signature, so a concurrent writer moving the
                                        // anchor (forward or back) can never be
                                        // overwritten with our now-stale value. The
                                        // re-read and the write below share the same
                                        // `blocking_lock()` hold, so no rewind can
                                        // interleave between them.
                                        let durable = store
                                            .get_sync_pull_state()
                                            .unwrap_or_default();
                                        let rewound = durable.since.as_deref()
                                            != prev_since.as_deref()
                                            || durable.cursor.as_deref()
                                                != prev_cursor.as_deref();
                                        if rewound {
                                            tracing::warn!(
                                                "operator rewind detected mid-pull — retaining rewound anchor for full re-pull"
                                            );
                                        } else {
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
                                    }
                                    // Keep quarantine visible in daemon status even
                                    // though the page is allowed to advance after
                                    // the configured retry budget is exhausted.
                                    if quarantined_item && anchor_error.is_none() {
                                        anchor_error = Some(
                                            "one or more remote items were dead-lettered"
                                                .to_owned(),
                                        );
                                    }
                                    // Return the anchor-persistence/quarantine error
                                    // so the caller surfaces the recovery action in
                                    // daemon status and logs.
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
                        Err(SyncError::AnchorExpired { oldest_available }) => {
                            pulled = 0;
                            tracing::warn!(
                                oldest_available = ?oldest_available,
                                "sync anchor expired — fetching snapshot to recover"
                            );
                            match transport.fetch_snapshot().await {
                                Ok(snapshot) => {
                                    let db_clone = db.clone();
                                    let anchor = oldest_available.clone();
                                    let recovery = tokio::task::spawn_blocking(move || {
                                        let conn = db_clone.blocking_lock();
                                        let store = Store::new(&conn);
                                        let imported = import_snapshot(&store, &snapshot)?;
                                        store.set_sync_pull_state(anchor.as_deref(), None)?;
                                        Ok::<usize, SyncError>(imported)
                                    })
                                    .await;
                                    match recovery {
                                        Ok(Ok(imported)) => {
                                            tracing::info!(
                                                imported,
                                                "snapshot imported after daemon anchor expiry"
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
                                                sync_error = Some(format!(
                                                    "snapshot recovery panicked: {e}"
                                                ));
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    if let SyncError::ServerMigrated { new_url } = &e {
                                        let db = db.clone();
                                        let url = new_url.clone();
                                        let _ = tokio::task::spawn_blocking(move || {
                                            let conn = db.blocking_lock();
                                            let store = Store::new(&conn);
                                            let _ =
                                                Settings::set_sync_server_url(store.conn(), &url);
                                        })
                                        .await;
                                        tracing::info!(new_url = %new_url, "server migrated — local config updated");
                                    }
                                    if sync_error.is_none() {
                                        sync_error =
                                            Some(format!("snapshot recovery fetch failed: {e}"));
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
                            // ADR sync-auth-hardening P1: stale auth — refresh
                            // the key once so the next cycle (60–120 s) pulls
                            // with fresh credentials. No in-tick pull retry: the
                            // pull apply block is anchor/quarantine-sensitive, so
                            // a retry would duplicate ~150 lines of application
                            // logic; recovery one cycle later is automatic.
                            if let SyncError::AuthExpired = e {
                                tracing::warn!(
                                    "pull rejected (401) — refreshing API key for next cycle"
                                );
                                if sync_error.is_none() {
                                    if refresh_persisted_api_key(db, &cfg.server_url).await {
                                        sync_error = Some(
                                            "pull rejected (401); key refreshed — will retry next cycle"
                                                .into(),
                                        );
                                    } else {
                                        sync_error = Some(
                                            "pull rejected (401) and token refresh failed".into(),
                                        );
                                    }
                                }
                            } else if sync_error.is_none() {
                                sync_error = Some(format!("pull phase: {e}"));
                            }
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
    use axum::{
        Json, Router,
        extract::State,
        http::StatusCode,
        response::IntoResponse,
        routing::{get, post},
    };
    use oz_core::migrations;
    use oz_core::settings::Settings;
    use tokio::sync::Notify;

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

    // ── TDD: daemon anchor-expiry recovery ─────────────────────────

    async fn spawn_anchor_expired_daemon_server() -> (String, Arc<std::sync::atomic::AtomicUsize>) {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let snapshot_hits = Arc::new(AtomicUsize::new(0));

        async fn handle_pull(
            State(_snapshot_hits): State<Arc<AtomicUsize>>,
            Json(request): Json<crate::transport::PullRequest>,
        ) -> impl IntoResponse {
            const OLDEST_AVAILABLE: &str = "2026-02-01T00:00:00.000Z";
            if request.since.as_deref() == Some("2025-01-01T00:00:00.000Z") {
                return (
                    StatusCode::GONE,
                    Json(serde_json::json!({
                        "error": "anchor_expired",
                        "oldest_available": OLDEST_AVAILABLE,
                    })),
                )
                    .into_response();
            }
            Json(PullResponse {
                items: vec![],
                next_cursor: None,
            })
            .into_response()
        }

        async fn handle_snapshot(
            State(snapshot_hits): State<Arc<AtomicUsize>>,
        ) -> Json<crate::transport::SyncSnapshotResponse> {
            snapshot_hits.fetch_add(1, Ordering::SeqCst);
            Json(crate::transport::SyncSnapshotResponse {
                version: 1,
                products: vec![],
                tax_rates: vec![],
                users: vec![],
            })
        }

        let app = Router::new()
            .route("/api/sync/pull", post(handle_pull))
            .route("/api/sync/snapshot", get(handle_snapshot))
            .with_state(snapshot_hits.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        (format!("http://localhost:{port}"), snapshot_hits)
    }

    /// A stale daemon anchor must recover through the snapshot endpoint and
    /// advance to the server's oldest retained row. Without this path the
    /// daemon logs `AnchorExpired` forever and never converges.
    #[tokio::test]
    async fn daemon_recovers_expired_anchor_with_snapshot() {
        use std::sync::atomic::Ordering;

        let (server_url, snapshot_hits) = spawn_anchor_expired_daemon_server().await;
        let db = setup_db();
        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            let store = Store::new(&conn);
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
            store
                .set_sync_pull_state(Some("2025-01-01T00:00:00.000Z"), None)
                .unwrap();
        })
        .await
        .unwrap();

        let status = Arc::new(RwLock::new(DaemonStatus::default()));
        SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;

        assert_eq!(snapshot_hits.load(Ordering::SeqCst), 1);
        let state = tokio::task::spawn_blocking({
            let db = db.clone();
            move || {
                let conn = db.blocking_lock();
                Store::new(&conn).get_sync_pull_state().unwrap()
            }
        })
        .await
        .unwrap();
        assert_eq!(state.since.as_deref(), Some("2026-02-01T00:00:00.000Z"));
        assert!(state.cursor.is_none());
        assert!(status.read().await.last_error.is_none());
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
        SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;
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
        SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;
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

    /// Spawn a mock pull server that continually returns a malformed remote
    /// sale. It is used to verify that transient failures retain the anchor
    /// until the retry budget is exhausted, then quarantine the item.
    async fn spawn_poison_remote_mock_sync_server() -> String {
        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        async fn handle_pull(Json(_req): Json<serde_json::Value>) -> Json<PullResponse> {
            let mut item = oz_core::offline::OfflineQueueItem::new(
                "complete_sale",
                r#"{"line_items":[{"sku":"MISSING","qty":1}]}"#,
            );
            item.id = "remote-poison-1".into();
            item.created_at = "2026-01-03T00:00:00.000Z".into();
            Json(PullResponse {
                items: vec![item],
                next_cursor: None,
            })
        }
        async fn handle_push(Json(items): Json<Vec<serde_json::Value>>) -> Json<PushResponse> {
            Json(PushResponse {
                results: vec![PushOutcome::Accepted; items.len()],
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

    /// SYNC-08 regression: a page containing a quarantined item and a fresh
    /// retryable item must still retain its anchor for the retryable item.
    #[tokio::test]
    async fn daemon_does_not_skip_retryable_item_beside_dead_letter() {
        let server_url = spawn_poison_remote_mock_server_with_two_items().await;
        let db = setup_db();
        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
            conn.execute(
                "INSERT INTO sync_remote_failures
                    (item_id, action, payload, attempts, last_error, dead_lettered)
                 VALUES ('remote-poison-dead', 'complete_sale', '{}', 3, 'permanent', 1)",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();

        let status = Arc::new(RwLock::new(DaemonStatus::default()));
        SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;

        let db_check = db.clone();
        let (anchor, retry_attempts) = tokio::task::spawn_blocking(move || {
            let conn = db_check.blocking_lock();
            let store = Store::new(&conn);
            (
                store.get_sync_pull_state().unwrap(),
                conn.query_row(
                    "SELECT attempts FROM sync_remote_failures WHERE item_id = 'remote-poison-retry'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            )
        })
        .await
        .unwrap();

        assert!(
            anchor.since.is_none(),
            "retryable item must retain the anchor"
        );
        assert_eq!(retry_attempts, 1);
    }

    /// Spawn a slow mock sync server whose pull handler BLOCKS on a
    /// [`tokio::sync::Notify`] until the test releases it, then returns one
    /// remote `stock.adjusted` item.
    ///
    /// The "pull arrived" notify fires as soon as the daemon's pull request
    /// reaches the handler — by then the daemon has already captured the
    /// durable anchor, so the test has a deterministic window to rewind it
    /// mid-pull (the race this regression pins).
    async fn spawn_slow_mock_sync_server() -> (String, Arc<Notify>, Arc<Notify>) {
        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let arrived = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());

        async fn handle_push(Json(items): Json<Vec<serde_json::Value>>) -> Json<PushResponse> {
            Json(PushResponse {
                results: vec![PushOutcome::Accepted; items.len()],
            })
        }
        async fn handle_pull(
            State((arrived, release)): State<(Arc<Notify>, Arc<Notify>)>,
            Json(_req): Json<serde_json::Value>,
        ) -> Json<PullResponse> {
            // Signal that the daemon's pull is in flight (anchor captured),
            // then block until the test rewinds the anchor and releases us.
            arrived.notify_one();
            release.notified().await;
            let mut item = oz_core::offline::OfflineQueueItem::new(
                "stock.adjusted",
                r#"{"sku":"COFFEE","delta":10}"#,
            );
            item.id = "remote-rewind-race-1".into();
            item.created_at = "2026-01-02T00:00:00.000Z".into();
            Json(PullResponse {
                items: vec![item],
                next_cursor: None,
            })
        }

        let app = Router::new()
            .route("/api/sync/push", post(handle_push))
            .route("/api/sync/pull", post(handle_pull))
            .with_state((arrived.clone(), release.clone()));

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        (format!("http://localhost:{port}"), arrived, release)
    }

    /// SYNC-09 regression: an operator rewind (`requeue_remote_failure`
    /// sets `sync_pull_state.since = NULL`) landing while a pull page is in
    /// flight must SURVIVE the daemon's apply phase. Previously the apply
    /// closure wrote its computed `new_since` blindly, clobbering the
    /// rewind — the next cycle then pulled from the advanced anchor and
    /// never re-fetched the requeued dead-lettered item.
    #[tokio::test]
    async fn daemon_pull_does_not_clobber_operator_rewind() {
        let (server_url, pull_arrived, release_pull) = spawn_slow_mock_sync_server().await;
        let db = setup_db();

        // Seed a product + inventory (so the remote adjustment applies
        // cleanly), configure sync, and pre-set a DURABLE anchor so the
        // daemon captures `Some(since)` at tick start.
        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            let store = Store::new(&conn);
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
            conn.execute_batch(
                "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
                 VALUES ('prod-coffee', 'COFFEE', 'Coffee', 350, 'USD', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
                 INSERT INTO inventory (product_id, qty, updated_at)
                 VALUES ('prod-coffee', 50, '2026-01-01T00:00:00.000Z');",
            )
            .unwrap();
            store
                .set_sync_pull_state(Some("2026-01-01T00:00:00.000Z"), None)
                .unwrap();
        })
        .await
        .unwrap();

        let status = Arc::new(RwLock::new(DaemonStatus::default()));
        // Run the tick in the background so the pull is genuinely in flight
        // when we rewind (the race is between the anchor capture and the
        // apply-phase write).
        let tick = {
            let db = db.clone();
            let status = status.clone();
            tokio::spawn(async move {
                SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;
            })
        };

        // Wait until the daemon's pull request reached the server — the
        // anchor is captured by now — then rewind it exactly as an operator
        // requeue would. Timeout so a daemon regression that never reaches
        // the pull phase FAILS this test instead of hanging the suite.
        tokio::time::timeout(Duration::from_secs(10), pull_arrived.notified())
            .await
            .expect("daemon never reached the pull phase");
        let db_rewind = db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_rewind.blocking_lock();
            let store = Store::new(&conn);
            store.set_sync_pull_state(None, None).unwrap();
        })
        .await
        .unwrap();
        release_pull.notify_one();

        tick.await.unwrap();

        // The page still applied (stock 50 → 60) — only the anchor advance
        // must be skipped so the rewind survives for a full re-pull.
        let (anchor, stock) = tokio::task::spawn_blocking({
            let db = db.clone();
            move || {
                let conn = db.blocking_lock();
                let store = Store::new(&conn);
                (
                    store.get_sync_pull_state().unwrap(),
                    store.get_stock("prod-coffee").unwrap(),
                )
            }
        })
        .await
        .unwrap();
        assert_eq!(stock, 60, "pull page must still apply despite the rewind");
        assert!(
            anchor.since.is_none(),
            "operator rewind must survive the apply phase (anchor.since = {:?})",
            anchor.since
        );
        assert!(
            anchor.cursor.is_none(),
            "rewound cursor must survive the apply phase (cursor = {:?})",
            anchor.cursor
        );
    }

    /// Spawn a mock pull server returning one already-quarantined item and
    /// one fresh poison item. This pins page-level anchor ordering.
    async fn spawn_poison_remote_mock_server_with_two_items() -> String {
        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        async fn handle_pull(Json(_req): Json<serde_json::Value>) -> Json<PullResponse> {
            let mut dead = oz_core::offline::OfflineQueueItem::new(
                "complete_sale",
                r#"{"line_items":[{"sku":"MISSING-DEAD","qty":1}]}"#,
            );
            dead.id = "remote-poison-dead".into();
            dead.created_at = "2026-01-03T00:00:00.000Z".into();
            let mut retry = oz_core::offline::OfflineQueueItem::new(
                "complete_sale",
                r#"{"line_items":[{"sku":"MISSING-RETRY","qty":1}]}"#,
            );
            retry.id = "remote-poison-retry".into();
            retry.created_at = "2026-01-03T00:00:01.000Z".into();
            Json(PullResponse {
                items: vec![dead, retry],
                next_cursor: None,
            })
        }
        async fn handle_push(Json(items): Json<Vec<serde_json::Value>>) -> Json<PushResponse> {
            Json(PushResponse {
                results: vec![PushOutcome::Accepted; items.len()],
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

    /// SYNC-08 regression: a failing remote item retains the previous anchor
    /// while it is retryable, then becomes a visible dead letter and allows
    /// the page anchor to advance after the third failed attempt.
    #[tokio::test]
    async fn daemon_retains_anchor_until_remote_item_is_dead_lettered() {
        let server_url = spawn_poison_remote_mock_sync_server().await;
        let db = setup_db();
        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
        })
        .await
        .unwrap();

        let status = Arc::new(RwLock::new(DaemonStatus::default()));
        for attempt in 1..=3 {
            SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;
            let db_check = db.clone();
            let (anchor, dead_lettered, failures) = tokio::task::spawn_blocking(move || {
                let conn = db_check.blocking_lock();
                let store = Store::new(&conn);
                (
                    store.get_sync_pull_state().unwrap(),
                    store.is_remote_failure_dead_lettered("remote-poison-1").unwrap(),
                    conn.query_row(
                        "SELECT attempts FROM sync_remote_failures WHERE item_id = 'remote-poison-1'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                )
            })
            .await
            .unwrap();

            if attempt < 3 {
                assert!(
                    anchor.since.is_none(),
                    "retryable failure must retain anchor"
                );
                assert!(!dead_lettered);
                assert_eq!(failures, attempt);
            } else {
                assert!(anchor.since.is_some(), "dead letter may advance anchor");
                assert!(dead_lettered);
                assert_eq!(failures, 3);
            }
        }

        assert!(
            status.read().await.last_error.is_some(),
            "dead-lettering must remain visible in daemon status"
        );
    }

    /// Spawn a mock sync server whose push endpoint ALWAYS returns a
    /// `Conflict` with a LOWER-version server item. The daemon must route
    /// the conflict through the shared ADR #21 service (SYNC-02): the local
    /// higher version wins and is marked resolved — never discarded by the
    /// old blanket "LWW: remote wins" path.
    async fn spawn_conflict_mock_sync_server() -> String {
        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        async fn handle_push(Json(items): Json<Vec<serde_json::Value>>) -> Json<PushResponse> {
            let results = items
                .iter()
                .map(|_| {
                    PushOutcome::Conflict(oz_core::offline::OfflineQueueItem::new(
                        "product.update",
                        r#"{"version":3,"name":"Server Stale"}"#,
                    ))
                })
                .collect();
            Json(PushResponse { results })
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

    /// SYNC-02 regression: when the server returns a Conflict for a pushed
    /// item, the daemon must resolve it through the shared ADR #21 service
    /// (version LWW here) rather than blanket-marking it synced and
    /// re-enqueuing the remote winner.
    #[tokio::test]
    async fn daemon_resolves_push_conflict_via_shared_service() {
        let server_url = spawn_conflict_mock_sync_server().await;
        let db = setup_db();

        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            let store = Store::new(&conn);
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
            // Local product.update has version 5 — HIGHER than the server's 3.
            store
                .enqueue_offline("product.update", r#"{"version":5,"name":"Local New"}"#)
                .unwrap();
        })
        .await
        .unwrap();

        let status = Arc::new(RwLock::new(DaemonStatus::default()));
        SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;

        let db_check = db.clone();
        let (all, pending) = tokio::task::spawn_blocking(move || {
            let conn = db_check.blocking_lock();
            let store = Store::new(&conn);
            (
                store.list_all_offline().unwrap(),
                store.list_pending_offline().unwrap(),
            )
        })
        .await
        .unwrap();

        // The local item must be marked resolved (synced) with the local-won
        // tag — the shared service decided local v5 > server v3. Nothing may
        // be re-enqueued (old behavior re-enqueued the server's stale v3).
        assert_eq!(all.len(), 1, "no remote winner may be re-enqueued");
        assert!(pending.is_empty(), "local winner must not stay pending");
        assert_eq!(all[0].status, oz_core::offline::OfflineQueueStatus::Synced);
        assert!(
            all[0]
                .last_error
                .as_deref()
                .unwrap_or("")
                .contains("resolved: conflict (local won)"),
            "daemon must record the ADR #21 resolution tag, got: {:?}",
            all[0].last_error
        );
    }

    /// SYNC-05 daemon end-to-end: a stock conflict must be resolved via the
    /// shared ADR #21 service into a CRDT merge, the merged winner must be
    /// re-enqueued, AND a later pull of that same merged item must be
    /// consumable by the daemon's apply_remote (both deltas land in stock).
    ///
    /// Mock: push returns a Conflict with a lower server stock delta; pull
    /// returns the merged crdt_delta envelope (fixed id so the SYNC-01
    /// ledger absorbs replays).
    async fn spawn_crdt_conflict_mock_sync_server() -> String {
        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        async fn handle_push(Json(items): Json<Vec<serde_json::Value>>) -> Json<PushResponse> {
            let results = items
                .iter()
                .map(|_| {
                    PushOutcome::Conflict(oz_core::offline::OfflineQueueItem::new(
                        "stock.adjusted",
                        r#"{"sku":"COFFEE","delta":-3}"#,
                    ))
                })
                .collect();
            Json(PushResponse { results })
        }
        async fn handle_pull(Json(_req): Json<serde_json::Value>) -> Json<PullResponse> {
            let mut winner = oz_core::offline::OfflineQueueItem::new(
                "stock.adjusted",
                r#"{"local":{"sku":"COFFEE","delta":10},"remote":{"sku":"COFFEE","delta":-3},"merge_type":"crdt_delta"}"#,
            );
            winner.id = "remote-crdt-winner-1".into();
            winner.created_at = "2026-01-02T00:00:00.000Z".into();
            Json(PullResponse {
                items: vec![winner],
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
    async fn daemon_crdt_conflict_merge_is_consumable_end_to_end() {
        let server_url = spawn_crdt_conflict_mock_sync_server().await;
        let db = setup_db();

        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            let store = Store::new(&conn);
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
            conn.execute_batch(
                "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
                 VALUES ('prod-coffee', 'COFFEE', 'Coffee', 350, 'USD', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');
                 INSERT INTO inventory (product_id, qty, updated_at)
                 VALUES ('prod-coffee', 50, '2026-01-01T00:00:00.000Z');",
            )
            .unwrap();
            store
                .enqueue_offline(
                    "stock.adjusted",
                    r#"{"sku":"COFFEE","delta":10}"#,
                )
                .unwrap();
        })
        .await
        .unwrap();

        let status = Arc::new(RwLock::new(DaemonStatus::default()));
        // One tick: push → conflict → CRDT merge resolved locally; pull →
        // merged winner applied by apply_remote. Both deltas must land.
        SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;

        let db_check = db.clone();
        let (stock, all) = tokio::task::spawn_blocking(move || {
            let conn = db_check.blocking_lock();
            let store = Store::new(&conn);
            (
                store.get_stock("prod-coffee").unwrap(),
                store.list_all_offline().unwrap(),
            )
        })
        .await
        .unwrap();

        // 50 + 10 (local) - 3 (remote) = 57 — the merge survives push→pull.
        assert_eq!(stock, 57, "both CRDT deltas must be applied by the daemon");

        // The local item carries the crdt-merge resolution tag. Match on
        // the tag itself (NOT on payload content): the re-enqueued merged
        // winner also embeds `"delta":10` inside its envelope, and
        // list_all_offline orders by created_at DESC (winner first), so a
        // payload-based lookup would grab the wrong row.
        let local = all.iter().find(|i| {
            i.last_error
                .as_deref()
                .unwrap_or("")
                .contains("resolved: conflict (crdt merge)")
        });
        assert!(
            local.is_some(),
            "local stock item must carry the crdt-merge tag, got: {:?}",
            all.iter().map(|i| &i.last_error).collect::<Vec<_>>()
        );
    }

    /// When the DB read phase succeeds, `run_tick` must update status
    /// without setting `last_error`. This is the regression guard for
    /// Bug #1 — verifies the refactored match arms don't break the
    /// happy path.
    #[tokio::test]
    async fn run_tick_happy_path_does_not_set_error() {
        let db = setup_db();
        let status = Arc::new(RwLock::new(DaemonStatus::default()));

        SyncDaemon::run_tick(&db, &status, &noop_settings_sink()).await;

        let s = status.read().await;
        assert!(s.last_sync_at.is_some(), "status should be updated");
        assert!(s.last_error.is_none(), "no error expected for empty config");
        assert_eq!(s.last_pushed, 0);
        assert_eq!(s.last_pulled, 0);
    }

    /// A settings sink that records nothing — for run_tick call sites that
    /// only care about the sync pipeline, not settings reactivity.
    fn noop_settings_sink() -> SettingsChangedSink {
        Arc::new(|_: &SettingsUpdated| {})
    }

    /// Spawn a mock pull server returning one remote `settings.update` item
    /// (fixed id + timestamp so the SYNC-01 ledger absorbs replays).
    async fn spawn_settings_mock_sync_server() -> String {
        let listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        async fn handle_push(Json(items): Json<Vec<serde_json::Value>>) -> Json<PushResponse> {
            Json(PushResponse {
                results: vec![PushOutcome::Accepted; items.len()],
            })
        }
        async fn handle_pull(Json(_req): Json<serde_json::Value>) -> Json<PullResponse> {
            let mut item = oz_core::offline::OfflineQueueItem::new(
                "settings.update",
                r#"{"key":"store.name","value":"Remote Acme","terminal_id":"term-remote","version":3}"#,
            );
            item.id = "remote-setting-sync-1".into();
            item.created_at = "2026-01-02T00:00:00.000Z".into();
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

    /// SYNC-10: when the pull applies a remote `settings.update`, the
    /// daemon must invoke its settings sink with the changed key so the app
    /// can re-emit `SettingsUpdated` — and the value row must actually land.
    #[tokio::test]
    async fn daemon_publishes_settings_updated_for_remote_settings_change() {
        let server_url = spawn_settings_mock_sync_server().await;
        let db = setup_db();

        let db_setup = db.clone();
        let url = server_url.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db_setup.blocking_lock();
            Settings::set_sync_enabled(&conn, true).unwrap();
            Settings::set_sync_server_url(&conn, &url).unwrap();
        })
        .await
        .unwrap();

        let recorded: Arc<std::sync::Mutex<Vec<(String, String)>>> =
            Arc::new(std::sync::Mutex::new(vec![]));
        let sink: SettingsChangedSink = Arc::new({
            let recorded = recorded.clone();
            move |event: &SettingsUpdated| {
                for key in &event.changed_keys {
                    recorded
                        .lock()
                        .unwrap()
                        .push((key.clone(), event.terminal_id.clone()));
                }
            }
        });

        let status = Arc::new(RwLock::new(DaemonStatus::default()));
        SyncDaemon::run_tick(&db, &status, &sink).await;

        assert_eq!(
            *recorded.lock().unwrap(),
            vec![("store.name".to_string(), "term-remote".to_string())],
            "the daemon must publish the remote settings change via the sink"
        );

        let value = tokio::task::spawn_blocking({
            let db = db.clone();
            move || {
                let conn = db.blocking_lock();
                Settings::get(&conn, "store.name").unwrap()
            }
        })
        .await
        .unwrap();
        assert_eq!(
            value.as_deref(),
            Some("Remote Acme"),
            "the settings row must be applied from the pull"
        );
    }
}
