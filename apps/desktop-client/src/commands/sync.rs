//! Cloud sync commands — configure and trigger sync from the UI.
//!
//! The `sync_run` command runs a push cycle immediately (instead of
//! waiting for the background daemon's interval). `sync_pull` fetches
//! the server's snapshot of products / tax rates / users and replaces
//! the local cache. The settings commands let the user configure the
//! server URL and API key.

use std::sync::Arc;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use oz_core::db::Store;
use oz_core::events::SettingsUpdated;
use oz_core::settings::Settings;
use oz_core::sync_client::{self, PullResult, SyncAttemptResult, SyncConfig};
use platform_sync::daemon::SettingsChangedSink;
use platform_sync::pg_daemon::PgDaemonStatus;

use crate::error::AppError;
use crate::state::AppState;

/// Get the current sync configuration settings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSettingsDto {
    /// Server Url.
    pub server_url: Option<String>,
    /// Has Api Key.
    pub has_api_key: bool,
    /// Enabled.
    pub enabled: bool,
}

/// Get sync settings.
#[tauri::command]
pub async fn get_sync_settings(state: State<'_, AppState>) -> Result<SyncSettingsDto, AppError> {
    let db = state.db.lock().await;
    let server_url = Settings::get_sync_server_url(&db)?.filter(|s| !s.is_empty());
    let api_key = Settings::get_sync_api_key(&db)?.filter(|k| !k.is_empty());
    let enabled = Settings::is_sync_enabled(&db)?;
    drop(db);
    Ok(SyncSettingsDto {
        server_url,
        has_api_key: api_key.is_some(),
        enabled,
    })
}

/// Get sync settings resolved from a session token. ADR #7.
#[tauri::command]
pub async fn get_sync_settings_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<SyncSettingsDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let server_url = Settings::get_sync_server_url(&db)?.filter(|s| !s.is_empty());
    let api_key = Settings::get_sync_api_key(&db)?.filter(|k| !k.is_empty());
    let enabled = Settings::is_sync_enabled(&db)?;
    drop(db);
    Ok(SyncSettingsDto {
        server_url,
        has_api_key: api_key.is_some(),
        enabled,
    })
}

/// Update sync settings.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSyncSettingsArgs {
    /// Server Url.
    pub server_url: Option<String>,
    /// Api Key.
    pub api_key: Option<String>,
    /// Enabled.
    pub enabled: bool,
}

#[tauri::command]
/// Update sync settings.
pub async fn update_sync_settings(
    args: UpdateSyncSettingsArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    update_sync_settings_data(&db, &args)?;
    drop(db);
    Ok(())
}

/// Persist sync settings (server URL, API key, enabled flag) atomically.
///
/// All three writes execute inside a single SQLite transaction so a
/// failure on any one rolls back the others — the same atomicity fix the
/// tablet client landed. Clearing the server URL (passing `null` or an
/// empty string) writes an EMPTY row rather than deleting it: that
/// row-presence contract is what `sync_bootstrap::should_auto_provision`
/// relies on to distinguish a cleared+disabled install from a fresh one.
///
/// Extracted as a free function so the atomicity + clearing contract can
/// be tested without a Tauri runtime
/// (see `update_sync_settings_data_clear_url_writes_empty_row`).
pub fn update_sync_settings_data(
    conn: &Connection,
    args: &UpdateSyncSettingsArgs,
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    // Always update server URL (passing `null` or empty string clears it).
    let url = args.server_url.as_deref().unwrap_or("");
    Settings::set_sync_server_url(&tx, url)?;
    // Only update API key if `Some(key)` was passed from the UI.
    // When `args.api_key` is `None` (the masked API field on the front-end was not modified),
    // preserve the existing key stored in the database.
    if let Some(ref key) = args.api_key {
        Settings::set_sync_api_key(&tx, key)?;
    }
    Settings::set_sync_enabled(&tx, args.enabled)?;
    tx.commit()?;
    Ok(())
}

// ── PostgreSQL sync settings & daemon commands ──────────────────

/// PostgreSQL sync configuration (the PG transport's connection settings).
/// `has_password` reports whether a secret is stored — the password itself
/// is never echoed back to the front-end.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PgSyncSettingsDto {
    /// Whether PostgreSQL sync is enabled.
    pub enabled: bool,
    /// PostgreSQL hostname or IP.
    pub host: Option<String>,
    /// PostgreSQL port.
    pub port: Option<String>,
    /// PostgreSQL database name.
    pub dbname: Option<String>,
    /// PostgreSQL user.
    pub user: Option<String>,
    /// Whether a password is stored (never echoed back).
    pub has_password: bool,
}

/// Get PG sync settings.
#[tauri::command]
pub async fn get_pg_sync_settings(
    state: State<'_, AppState>,
) -> Result<PgSyncSettingsDto, AppError> {
    let db = state.db.lock().await;
    run_get_pg_sync_settings(&db)
}

/// Business logic for `get_pg_sync_settings` (extracted for testing).
fn run_get_pg_sync_settings(conn: &Connection) -> Result<PgSyncSettingsDto, AppError> {
    Ok(PgSyncSettingsDto {
        enabled: Settings::is_pg_sync_enabled(conn)?,
        host: Settings::get_pg_sync_host(conn)?.filter(|s| !s.is_empty()),
        port: Settings::get_pg_sync_port(conn)?.filter(|s| !s.is_empty()),
        dbname: Settings::get_pg_sync_dbname(conn)?.filter(|s| !s.is_empty()),
        user: Settings::get_pg_sync_user(conn)?.filter(|s| !s.is_empty()),
        has_password: Settings::get_pg_sync_password(conn)?.is_some_and(|s| !s.is_empty()),
    })
}

/// Update PG sync settings.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePgSyncSettingsArgs {
    /// Whether PostgreSQL sync is enabled.
    pub enabled: bool,
    /// PostgreSQL hostname or IP (`None` clears).
    pub host: Option<String>,
    /// PostgreSQL port (`None` clears).
    pub port: Option<String>,
    /// PostgreSQL database name (`None` clears).
    pub dbname: Option<String>,
    /// PostgreSQL user (`None` clears).
    pub user: Option<String>,
    /// PostgreSQL password — written only when `Some`, so the UI's masked
    /// untouched field never blanks the stored secret (mirror of the
    /// HTTP sync API-key handling).
    pub password: Option<String>,
}

/// Update PG sync settings.
#[tauri::command]
pub async fn update_pg_sync_settings(
    args: UpdatePgSyncSettingsArgs,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    update_pg_sync_settings_data(&db, &args)?;
    drop(db);
    Ok(())
}

/// Persist PG sync settings atomically in a single transaction.
///
/// Extracted as a free function so the persistence contract (optional
/// field clearing + password preservation) can be tested without a Tauri
/// runtime, mirroring `update_sync_settings_data`.
pub fn update_pg_sync_settings_data(
    conn: &Connection,
    args: &UpdatePgSyncSettingsArgs,
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    Settings::set_pg_sync_enabled(&tx, args.enabled)?;
    // `None` (or an empty string) clears the field — the same row-presence
    // contract the HTTP sync URL handling uses.
    Settings::set_pg_sync_host(&tx, args.host.as_deref().unwrap_or(""))?;
    Settings::set_pg_sync_port(&tx, args.port.as_deref().unwrap_or(""))?;
    Settings::set_pg_sync_dbname(&tx, args.dbname.as_deref().unwrap_or(""))?;
    Settings::set_pg_sync_user(&tx, args.user.as_deref().unwrap_or(""))?;
    if let Some(ref password) = args.password {
        Settings::set_pg_sync_password(&tx, password)?;
    }
    tx.commit()?;
    Ok(())
}

/// SYNC-10 settings sink shared by the SQLite and PG daemons: a settings
/// change applied by sync is re-emitted as the `settings_updated` Tauri
/// event (the same wire shape the frontend SettingsContext listens for)
/// so the UI refetches the changed scope. Local saves already publish the
/// domain event; this closes the loop for the sync-applied path.
pub fn settings_changed_sink(app: &tauri::AppHandle) -> SettingsChangedSink {
    let app_handle = app.clone();
    Arc::new(move |event: &SettingsUpdated| {
        let payload = serde_json::json!({
            "changed_keys": event.changed_keys,
            "terminal_id": event.terminal_id,
        });
        let _ = app_handle.emit("settings_updated", payload);
    })
}

/// Get the PG daemon's current status snapshot.
#[tauri::command]
pub async fn pg_sync_status(state: State<'_, AppState>) -> Result<PgDaemonStatus, AppError> {
    Ok(state.pg_sync_daemon.status().await)
}

/// Start the background PG sync daemon. No-op when already running.
#[tauri::command]
pub async fn pg_sync_start(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let db = state.db.clone();
    let sink = settings_changed_sink(&app_handle);
    state.pg_sync_daemon.start_with_sink(db, sink).await;
    Ok(())
}

/// Stop the background PG sync daemon. No-op when not running.
#[tauri::command]
pub async fn pg_sync_stop(state: State<'_, AppState>) -> Result<(), AppError> {
    state.pg_sync_daemon.stop().await;
    Ok(())
}

/// Immediately run a sync cycle that pushes pending sales, credit, and
/// other queued offline transactions to the configured cloud server.
///
/// Uses a three-phase split (read → async HTTP → write) so the DB
/// lock is not held during the network round-trip, avoiding the
/// "Cannot drop a runtime in a context where blocking is not allowed"
/// panic that reqwest::blocking triggers inside Tauri's async runtime.
#[tauri::command]
pub async fn sync_run(state: State<'_, AppState>) -> Result<SyncAttemptResult, AppError> {
    // Phase 1: Read pending items and config from DB (brief lock).
    let (pending_items, config_opt) = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let pending = store.list_pending_offline()?;
        let config = SyncConfig::from_settings(&store)?;
        (pending, config)
    };

    let config = match config_opt {
        Some(c) => c,
        None => {
            return Ok(SyncAttemptResult {
                synced: 0,
                failed: 0,
                error: Some("Sync is not configured or disabled".into()),
                plan_required: false,
            });
        }
    };

    if pending_items.is_empty() {
        return Ok(SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: None,
            plan_required: false,
        });
    }

    // Phase 2: Async HTTP push (no DB lock held).
    let mut outcomes = sync_client::send_items_to_server(&config, &pending_items).await;

    // ADR sync-auth-hardening P1: a 401 means the stored token is stale —
    // refresh it once and retry the push exactly once (never in a loop).
    if matches!(outcomes, Err(sync_client::SyncHttpError::AuthExpired)) {
        // ADR sync-auth-hardening P1: request a fresh token (async, no DB
        // lock), persist it under a brief lock, then retry exactly once.
        // P3: prefer terminal client credentials when the device is paired.
        let client_credentials = {
            let db = state.db.lock().await;
            match (
                Settings::get_sync_terminal_id(&db)?,
                Settings::get_sync_terminal_secret(&db)?,
            ) {
                (Some(id), Some(secret)) => Some((id, secret)),
                _ => None,
            }
        };
        let fresh_key = sync_client::request_refresh_token(
            &config.server_url,
            client_credentials
                .as_ref()
                .map(|(id, secret)| (id.as_str(), secret.as_str())),
        )
        .await;
        if let Some(fresh_key) = fresh_key {
            {
                let db = state.db.lock().await;
                sync_client::persist_refreshed_api_key(&db, &fresh_key)?;
            }
            let retry_config = {
                let db = state.db.lock().await;
                let store = Store::new(&db);
                SyncConfig::from_settings(&store)?
            };
            if let Some(cfg) = retry_config {
                outcomes = sync_client::send_items_to_server(&cfg, &pending_items).await;
            }
        }
    }

    // Phase 3: Write outcomes back to DB (brief lock).
    let db = state.db.lock().await;
    let store = Store::new(&db);
    match outcomes {
        Ok(outcomes) => Ok(sync_client::apply_sync_outcomes(
            &store,
            &pending_items,
            &outcomes,
        )?),
        // ADR sync-plan-gating: a free tenant is gated, not broken. Do NOT
        // mark the items failed — they stay `pending` and sync automatically
        // once the tenant upgrades. The UI shows an upgrade prompt instead.
        Err(sync_client::SyncHttpError::PlanRequired) => Ok(SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: Some("cloud sync requires a paid plan".into()),
            plan_required: true,
        }),
        Err(e) => Ok(sync_client::mark_all_failed(
            &store,
            &pending_items,
            &e.to_string(),
        )?),
    }
}

/// Get the pending sync count.
#[tauri::command]
pub async fn pending_sync_count(state: State<'_, AppState>) -> Result<i64, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let count = store.pending_offline_count()?;
    drop(db);
    Ok(count)
}

/// Request a new JWT API token from the cloud server's
/// `POST /api/v1/tokens` endpoint.
///
/// If `url` is provided (from the front-end text field), it is used
/// directly so users can request a token before saving. Otherwise the
/// saved value from settings is used.
#[tauri::command]
pub async fn request_sync_token(
    url: Option<String>,
    state: State<'_, AppState>,
) -> Result<sync_client::TokenResult, AppError> {
    // Resolve the URL first (may briefly lock DB), then drop the lock
    // before making the async HTTP call.
    let resolved = match url.filter(|u| !u.is_empty()) {
        Some(u) => Some(u),
        None => {
            let db = state.db.lock().await;
            Settings::get_sync_server_url(&db)?.filter(|s| !s.is_empty())
        }
    };
    match resolved {
        Some(u) => {
            Ok(sync_client::request_token(&u, sync_client::admin_key_from_env().as_deref()).await)
        }
        None => Ok(sync_client::TokenResult {
            ok: false,
            token: None,
            status: "No server URL configured".into(),
            expires_at: None,
        }),
    }
}

/// Read the caller's own sync plan from the server (ADR sync-plan-gating).
///
/// Resolves URL + API key from settings, then calls `GET
/// /api/v1/tenants/me/plan`. The endpoint is not plan-gated, so a free
/// tenant can read its own plan to render the upgrade prompt without
/// running a sync.
#[tauri::command]
pub async fn get_sync_plan(
    state: State<'_, AppState>,
) -> Result<sync_client::TenantPlanResult, AppError> {
    // Resolve URL + API key first (brief DB lock), then drop the lock
    // before the async HTTP call.
    let (url, api_key) = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let config = SyncConfig::from_settings(&store)?;
        match config {
            Some(c) => (Some(c.server_url), c.api_key),
            None => (None, None),
        }
    };
    match (url, api_key) {
        (Some(u), Some(key)) => Ok(sync_client::fetch_tenant_plan(&u, &key).await),
        _ => Ok(sync_client::TenantPlanResult {
            ok: false,
            plan: None,
            status: "Sync is not configured".into(),
        }),
    }
}

// TEMPORARILY DISABLED (2026-08-16): see the commented fallback in
// `resolve_sync_probe_url` — the local Docker dev URL must not be used
// while testing against the deployed cloud server. Re-enable together
// with the fallback block it feeds.
// #[cfg(debug_assertions)]
// const LOCAL_DEV_SYNC_URL: &str = "http://localhost:3099";

/// Resolve the URL used by the status-bar health probe.
///
/// Explicitly supplied and persisted URLs always win. The debug-only local
/// fallback is intentionally added here rather than in the frontend so the
/// status indicator can recover even while auto-provisioning is still writing
/// the persisted settings row.
fn resolve_sync_probe_url(
    candidate: Option<String>,
    saved: Option<String>,
    _allow_local_fallback: bool,
) -> Option<String> {
    if let Some(url) = candidate.filter(|url| !url.trim().is_empty()) {
        return Some(url);
    }
    if let Some(url) = saved.filter(|url| !url.trim().is_empty()) {
        return Some(url);
    }

    // The health indicator must be able to probe the local Docker server
    // before the asynchronous bootstrap has persisted URL/key settings.
    // Keep this fallback debug-only so production never probes localhost
    // behind the operator's back. An empty URL is unconfigured; an explicit
    // opt-out is represented by keeping a configured URL and disabling sync.
    //
    // TEMPORARILY DISABLED (2026-08-16): while testing against the deployed
    // cloud server the status indicator must not fall back to the local
    // Docker dev server. Re-enable by uncommenting the block below.
    // #[cfg(debug_assertions)]
    // if allow_local_fallback {
    //     return Some(LOCAL_DEV_SYNC_URL.to_string());
    // }

    None
}

/// Test the cloud sync connection by pinging the configured server's
/// `/health` endpoint.
///
/// If `url` is provided (from the front-end text field), it is used
/// directly so users can test a URL before saving. Otherwise the
/// saved value from settings is used.
#[tauri::command]
pub async fn test_sync_connection(
    url: Option<String>,
    state: State<'_, AppState>,
) -> Result<sync_client::PingResult, AppError> {
    // Resolve the URL first (may briefly lock DB), then drop the lock
    // before making the async HTTP call.
    let (saved, allow_local_fallback) =
        if url.as_ref().is_some_and(|value| !value.trim().is_empty()) {
            (None, true)
        } else {
            let db = state.db.lock().await;
            let saved = Settings::get_sync_server_url(&db)?;
            let allow_local_fallback = saved
                .as_deref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true);
            (saved, allow_local_fallback)
        };
    let resolved = resolve_sync_probe_url(url, saved, allow_local_fallback);
    match resolved {
        Some(u) => Ok(sync_client::ping_server(&u).await),
        None => Ok(sync_client::PingResult {
            ok: false,
            status: "No server URL configured".into(),
            latency_ms: None,
        }),
    }
}

/// Arguments for `sync_pull`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPullArgs {
    /// Must be `true` to proceed with the destructive pull.
    /// Prevents accidental local-data overwrite from UI double-clicks
    /// or programmatic calls without user consent (H-2).
    pub confirm_destructive: bool,
}

/// Reject a pull that lacks explicit destructive consent (H-2).
///
/// Extracted as a free function so the consent gate can be unit-tested
/// without a Tauri runtime.
fn validate_pull_consent(args: &SyncPullArgs) -> Result<(), AppError> {
    if !args.confirm_destructive {
        return Err(AppError::Invalid(
            "confirm_destructive must be true to proceed with sync pull".into(),
        ));
    }
    Ok(())
}

/// Pull a server snapshot and overwrite the local cache for products,
/// tax rates, and users.
///
/// The caller must explicitly acknowledge the destructive nature of this
/// operation by passing `confirm_destructive: true`. If false, the
/// command returns an error without fetching from the server.
///
/// Before applying the server snapshot, a backup of the current local
/// database is written to `<db_path>.sync-pull-<timestamp>.backup.db`.
/// This ensures the local state can be recovered if the pull overwrites
/// data unexpectedly (H-2).
///
/// Uses a three-phase split (read -> async HTTP -> write) so the DB
/// lock is not held during the network round-trip.
#[tauri::command]
pub async fn sync_pull(
    args: SyncPullArgs,
    state: State<'_, AppState>,
) -> Result<PullResult, AppError> {
    validate_pull_consent(&args)?;

    // Phase 1: Read config from DB (brief lock).
    let config_opt = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        SyncConfig::from_settings(&store)?
    };

    let config = match config_opt {
        Some(c) => c,
        None => {
            return Ok(PullResult {
                products_pulled: 0,
                tax_rates_pulled: 0,
                users_pulled: 0,
                error: Some("Sync is not configured or disabled".into()),
            });
        }
    };

    // Phase 2: Async HTTP fetch (no DB lock held).
    let mut snapshot = sync_client::fetch_snapshot_from_server(&config).await;

    // ADR sync-auth-hardening P1: refresh the token once and retry exactly
    // once when the server rejects our authentication.
    if matches!(snapshot, Err(sync_client::SyncHttpError::AuthExpired)) {
        // ADR sync-auth-hardening P1: request a fresh token (async, no DB
        // lock), persist it under a brief lock, then retry exactly once.
        // P3: prefer terminal client credentials when the device is paired.
        let client_credentials = {
            let db = state.db.lock().await;
            match (
                Settings::get_sync_terminal_id(&db)?,
                Settings::get_sync_terminal_secret(&db)?,
            ) {
                (Some(id), Some(secret)) => Some((id, secret)),
                _ => None,
            }
        };
        let fresh_key = sync_client::request_refresh_token(
            &config.server_url,
            client_credentials
                .as_ref()
                .map(|(id, secret)| (id.as_str(), secret.as_str())),
        )
        .await;
        if let Some(fresh_key) = fresh_key {
            {
                let db = state.db.lock().await;
                sync_client::persist_refreshed_api_key(&db, &fresh_key)?;
            }
            let retry_config = {
                let db = state.db.lock().await;
                let store = Store::new(&db);
                SyncConfig::from_settings(&store)?
            };
            if let Some(cfg) = retry_config {
                snapshot = sync_client::fetch_snapshot_from_server(&cfg).await;
            }
        }
    }

    // Phase 3: Create a pre-pull backup (defence in depth — H-2).
    // The backup file is timestamped so operators can correlate it with
    // a specific pull event.
    {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let mut backup_path = state.db_path.clone();
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let ext = format!("sync-pull-{timestamp}.backup.db");
        backup_path.set_extension(&ext);
        store
            .backup(&backup_path.display().to_string())
            .map_err(|e| {
                tracing::warn!(backup = %backup_path.display(), error = %e, "sync-pull backup failed");
                AppError::Internal(format!("sync-pull backup failed: {e}"))
            })?;
        tracing::info!(backup = %backup_path.display(), "pre-pull backup created");
    }

    // Phase 4: Apply snapshot to DB (brief lock).
    let db = state.db.lock().await;
    let store = Store::new(&db);
    match snapshot {
        Ok(s) => Ok(sync_client::apply_snapshot(&store, &s)?),
        Err(e) => Ok(PullResult {
            products_pulled: 0,
            tax_rates_pulled: 0,
            users_pulled: 0,
            error: Some(e.to_string()),
        }),
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
