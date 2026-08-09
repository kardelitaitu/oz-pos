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
            });
        }
    };

    if pending_items.is_empty() {
        return Ok(SyncAttemptResult {
            synced: 0,
            failed: 0,
            error: None,
        });
    }

    // Phase 2: Async HTTP push (no DB lock held).
    let mut outcomes = sync_client::send_items_to_server(&config, &pending_items).await;

    // ADR sync-auth-hardening P1: a 401 means the stored token is stale —
    // refresh it once and retry the push exactly once (never in a loop).
    if matches!(outcomes, Err(sync_client::SyncHttpError::AuthRejected)) {
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

/// Resolve the URL used by the status-bar health probe.
///
/// Explicitly supplied and persisted URLs always win. The debug-only local
/// fallback is intentionally added here rather than in the frontend so the
/// status indicator can recover even while auto-provisioning is still writing
/// the persisted settings row.
#[cfg(debug_assertions)]
const LOCAL_DEV_SYNC_URL: &str = "http://localhost:3099";

fn resolve_sync_probe_url(
    candidate: Option<String>,
    saved: Option<String>,
    allow_local_fallback: bool,
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
    // behind the operator's back. A cleared URL with a retained key and
    // sync disabled is an explicit opt-out and must not be overridden.
    #[cfg(debug_assertions)]
    if allow_local_fallback {
        return Some(LOCAL_DEV_SYNC_URL.to_string());
    }

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
            let enabled = Settings::is_sync_enabled(&db)?;
            let has_api_key =
                Settings::get_sync_api_key(&db)?.is_some_and(|key| !key.trim().is_empty());
            (saved, enabled || !has_api_key)
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
    if matches!(snapshot, Err(sync_client::SyncHttpError::AuthRejected)) {
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
mod tests {
    use super::*;
    use tauri::Manager as _;

    #[test]
    fn sync_settings_serialize() {
        let s = SyncSettingsDto {
            server_url: Some("https://sync.example.com".into()),
            has_api_key: true,
            enabled: true,
        };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["serverUrl"], "https://sync.example.com");
        assert_eq!(json["hasApiKey"], true);
        assert_eq!(json["enabled"], true);
    }

    #[test]
    fn sync_settings_no_url_disabled() {
        let s = SyncSettingsDto {
            server_url: None,
            has_api_key: false,
            enabled: false,
        };
        let json = serde_json::to_value(&s).unwrap();
        assert!(json["serverUrl"].is_null());
        assert_eq!(json["hasApiKey"], false);
        assert_eq!(json["enabled"], false);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn sync_probe_uses_local_dev_server_before_bootstrap_persists_settings() {
        let resolved = resolve_sync_probe_url(None, None, true);
        assert_eq!(resolved.as_deref(), Some("http://localhost:3099"));
        assert_eq!(resolve_sync_probe_url(None, None, false), None);
    }

    #[test]
    fn update_sync_settings_deserialize() {
        let json =
            r#"{"serverUrl":"https://sync.example.com","apiKey":"sk-abc123","enabled":true}"#;
        let args: UpdateSyncSettingsArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.server_url.unwrap(), "https://sync.example.com");
        assert_eq!(args.api_key.unwrap(), "sk-abc123");
        assert!(args.enabled);
    }

    #[test]
    fn update_sync_settings_deserialize_no_key() {
        let json = r#"{"serverUrl":null,"apiKey":null,"enabled":false}"#;
        let args: UpdateSyncSettingsArgs = serde_json::from_str(json).unwrap();
        assert!(args.server_url.is_none());
        assert!(args.api_key.is_none());
        assert!(!args.enabled);
    }

    #[test]
    fn update_sync_settings_data_clear_url_writes_empty_row() {
        // The UI sends server_url: None when the user clears the field.
        // The command must write an empty row (Some("")) rather than
        // leaving the stale URL (which would keep auto-provision from ever
        // repairing a broken URL) or deleting the row (which would make a
        // cleared + disabled install look like a fresh one and re-trigger
        // provisioning on the next debug launch). THIS app is where the
        // should_auto_provision discriminator runs, so the pin belongs
        // here, not just on the tablet twin.
        let conn = oz_core::migrations::fresh_db();
        Settings::set_sync_server_url(&conn, "https://sync.example.com").unwrap();
        Settings::set_sync_enabled(&conn, false).unwrap();

        let args = UpdateSyncSettingsArgs {
            server_url: None,
            api_key: None,
            enabled: false,
        };
        update_sync_settings_data(&conn, &args).unwrap();

        assert_eq!(
            Settings::get_sync_server_url(&conn).unwrap(),
            Some("".into())
        );
    }

    #[test]
    fn update_sync_settings_debug() {
        let args = UpdateSyncSettingsArgs {
            server_url: Some("https://sync.example.com".into()),
            api_key: None,
            enabled: true,
        };
        let debug = format!("{args:?}");
        assert!(debug.contains("sync.example.com"));
        assert!(debug.contains("true"));
    }

    #[test]
    fn sync_pull_args_deserialize() {
        let json = r#"{"confirmDestructive":true}"#;
        let args: SyncPullArgs = serde_json::from_str(json).unwrap();
        assert!(args.confirm_destructive);
    }

    #[test]
    fn sync_pull_args_deserialize_false() {
        let json = r#"{"confirmDestructive":false}"#;
        let args: SyncPullArgs = serde_json::from_str(json).unwrap();
        assert!(!args.confirm_destructive);
    }

    #[test]
    fn sync_pull_args_missing_consent_fails() {
        // SYNC-03: a payload with no consent key must not silently
        // default to true — serde errors on the missing field.
        let result = serde_json::from_str::<SyncPullArgs>(r#"{}"#);
        assert!(
            result.is_err(),
            "missing confirm_destructive must fail deserialization"
        );
    }

    #[test]
    fn validate_pull_consent_accepts_true() {
        let args = SyncPullArgs {
            confirm_destructive: true,
        };
        assert!(validate_pull_consent(&args).is_ok());
    }

    #[test]
    fn validate_pull_consent_rejects_false() {
        let args = SyncPullArgs {
            confirm_destructive: false,
        };
        let err = validate_pull_consent(&args).unwrap_err();
        assert!(err.to_string().contains("confirm_destructive"));
    }

    #[test]
    fn pull_result_serialize_no_error() {
        let r = PullResult {
            products_pulled: 10,
            tax_rates_pulled: 2,
            users_pulled: 3,
            error: None,
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["products_pulled"], 10);
        assert_eq!(json["tax_rates_pulled"], 2);
        assert_eq!(json["users_pulled"], 3);
        assert!(json["error"].is_null());
    }

    #[test]
    fn pull_result_serialize_with_error() {
        let r = PullResult {
            products_pulled: 0,
            tax_rates_pulled: 0,
            users_pulled: 0,
            error: Some("network unreachable".into()),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["products_pulled"], 0);
        assert_eq!(json["error"], "network unreachable");
    }

    #[test]
    fn pull_result_deserialize() {
        let json = r#"{"products_pulled":5,"tax_rates_pulled":1,"users_pulled":2,"error":null}"#;
        let r: PullResult = serde_json::from_str(json).unwrap();
        assert_eq!(r.products_pulled, 5);
        assert_eq!(r.tax_rates_pulled, 1);
        assert_eq!(r.users_pulled, 2);
        assert!(r.error.is_none());
    }

    #[tokio::test]
    async fn sync_run_uses_persisted_settings_and_reports_empty_queue_success() {
        // Phase 4 bootstrap contract: once the Tauri settings database has
        // the URL, API key, and enabled flag written by auto-provisioning,
        // the real command must read that persisted state and return an
        // explicit successful result when there is nothing to push.
        let conn = oz_core::migrations::fresh_db();
        update_sync_settings_data(
            &conn,
            &UpdateSyncSettingsArgs {
                server_url: Some("http://localhost:3099".into()),
                api_key: Some("test-jwt".into()),
                enabled: true,
            },
        )
        .unwrap();
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let result = sync_run(app.state()).await.unwrap();

        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 0);
        assert!(result.error.is_none());
    }

    async fn spawn_push_test_server() -> (
        String,
        Arc<tokio::sync::Mutex<Option<String>>>,
        tokio::task::JoinHandle<()>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let captured = Arc::new(tokio::sync::Mutex::new(None));
        let captured_by_server = captured.clone();
        let task = tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut buffer = vec![0_u8; 16 * 1024];
            let bytes_read = socket.read(&mut buffer).await.unwrap_or(0);
            *captured_by_server.lock().await =
                Some(String::from_utf8_lossy(&buffer[..bytes_read]).into_owned());
            let body = r#"{"results":[{"outcome":"accepted"}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = socket.write_all(response.as_bytes()).await;
        });
        (url, captured, task)
    }

    #[tokio::test]
    async fn sync_run_enqueues_one_item_and_observes_server_acceptance() {
        // Full isolated harness: a temporary AppState owns the queue, the
        // real command performs the HTTP push, and the test server captures
        // the authenticated request and returns an accepted outcome.
        let (server_url, captured, server_task) = spawn_push_test_server().await;
        let conn = oz_core::migrations::fresh_db();
        update_sync_settings_data(
            &conn,
            &UpdateSyncSettingsArgs {
                server_url: Some(server_url),
                api_key: Some("test-jwt".into()),
                enabled: true,
            },
        )
        .unwrap();
        {
            let store = Store::new(&conn);
            store
                .enqueue_offline("phase4.e2e", r#"{"probe":true}"#)
                .unwrap();
        }
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let result = sync_run(app.state()).await.unwrap();
        server_task.await.unwrap();

        assert_eq!(result.synced, 1);
        assert_eq!(result.failed, 0);
        assert!(result.error.is_none());
        let request = captured.lock().await.clone().unwrap();
        assert!(request.starts_with("POST /api/sync/push HTTP/1.1"));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-jwt"),
            "request did not carry the configured bearer token: {request}"
        );

        let state = app.state::<AppState>();
        let db = state.db.lock().await;
        let items = Store::new(&db).list_all_offline().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].status,
            oz_core::offline::OfflineQueueStatus::Synced
        );
    }

    #[tokio::test]
    async fn sync_run_refreshes_token_and_retries_once_after_401() {
        // ADR sync-auth-hardening P1: when the server rejects the stored
        // token with 401, the command must mint a fresh token, persist it,
        // and retry the push exactly once — no operator action, no loop.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_url = format!("http://{}", listener.local_addr().unwrap());
        let retry_auth: Arc<tokio::sync::Mutex<Option<String>>> =
            Arc::new(tokio::sync::Mutex::new(None));
        let retry_auth_server = retry_auth.clone();
        let task = tokio::spawn(async move {
            let mut auth_of_retry: Option<String> = None;
            for attempt in 0..3 {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let mut buffer = vec![0_u8; 16 * 1024];
                let n = socket.read(&mut buffer).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
                let path = request
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or_default();
                let response = if path == "/api/sync/push" && attempt == 0 {
                    "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_string()
                } else if path == "/api/v1/tokens" {
                    let body = r#"{"token":{"token":"fresh-jwt-456","expires_at":"2026-08-10T00:00:00Z","token_id":"uuid-1"}}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                } else {
                    auth_of_retry = request
                        .lines()
                        .find(|l| l.to_ascii_lowercase().starts_with("authorization:"))
                        .map(|l| l.to_string());
                    let body = r#"{"results":[{"outcome":"accepted"}]}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                };
                let _ = socket.write_all(response.as_bytes()).await;
            }
            *retry_auth_server.lock().await = auth_of_retry;
        });

        let conn = oz_core::migrations::fresh_db();
        update_sync_settings_data(
            &conn,
            &UpdateSyncSettingsArgs {
                server_url: Some(server_url),
                api_key: Some("stale-jwt".into()),
                enabled: true,
            },
        )
        .unwrap();
        {
            let store = Store::new(&conn);
            store
                .enqueue_offline("phase1.refresh", r#"{"probe":true}"#)
                .unwrap();
        }
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let result = sync_run(app.state()).await.unwrap();
        task.await.unwrap();

        assert_eq!(result.synced, 1);
        assert_eq!(result.failed, 0);
        assert!(result.error.is_none());

        // The retried push must carry the freshly minted token.
        let auth = retry_auth.lock().await.clone().unwrap_or_default();
        assert!(
            auth.to_ascii_lowercase().contains("bearer fresh-jwt-456"),
            "retried push did not carry the fresh token: {auth}"
        );

        // The refreshed key was persisted and the item reached synced.
        let state = app.state::<AppState>();
        let db = state.db.lock().await;
        assert_eq!(
            Settings::get_sync_api_key(&db).unwrap().as_deref(),
            Some("fresh-jwt-456")
        );
        let items = Store::new(&db).list_all_offline().unwrap();
        assert_eq!(
            items[0].status,
            oz_core::offline::OfflineQueueStatus::Synced
        );
    }

    #[tokio::test]
    async fn request_token_sends_admin_key_header_when_provided() {
        // ADR sync-auth-hardening P2: a gated server (OZ_ADMIN_KEY set)
        // only mints tokens when the request carries the matching
        // X-Admin-Key header. Pin the wire contract here.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_url = format!("http://{}", listener.local_addr().unwrap());
        let captured: Arc<tokio::sync::Mutex<Option<String>>> =
            Arc::new(tokio::sync::Mutex::new(None));
        let captured_server = captured.clone();
        let task = tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut buffer = vec![0_u8; 16 * 1024];
            let n = socket.read(&mut buffer).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
            *captured_server.lock().await = Some(request);
            let body = r#"{"token":{"token":"jwt-1","expires_at":null,"token_id":"u1"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = socket.write_all(response.as_bytes()).await;
        });

        let result = sync_client::request_token(&server_url, Some("sekret")).await;
        task.await.unwrap();

        assert!(result.ok, "token request failed: {}", result.status);
        let request = captured.lock().await.clone().unwrap();
        assert!(
            request.to_ascii_lowercase().contains("x-admin-key: sekret"),
            "token request did not carry the admin key: {request}"
        );
    }

    // ── PostgreSQL sync settings & daemon commands ────────────────

    #[test]
    fn pg_sync_settings_dto_serialize_camel_case() {
        let dto = PgSyncSettingsDto {
            enabled: true,
            host: Some("db.example.com".into()),
            port: Some("5432".into()),
            dbname: Some("oz_sync".into()),
            user: Some("sync_user".into()),
            has_password: true,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["enabled"], true);
        assert_eq!(json["host"], "db.example.com");
        assert_eq!(json["port"], "5432");
        assert_eq!(json["dbname"], "oz_sync");
        assert_eq!(json["user"], "sync_user");
        assert_eq!(json["hasPassword"], true);
    }

    #[test]
    fn update_pg_sync_settings_args_deserialize() {
        let json = r#"{"enabled":true,"host":"db.example.com","port":"5432","dbname":"oz_sync","user":"sync_user","password":"secret"}"#;
        let args: UpdatePgSyncSettingsArgs = serde_json::from_str(json).unwrap();
        assert!(args.enabled);
        assert_eq!(args.host.as_deref(), Some("db.example.com"));
        assert_eq!(args.port.as_deref(), Some("5432"));
        assert_eq!(args.dbname.as_deref(), Some("oz_sync"));
        assert_eq!(args.user.as_deref(), Some("sync_user"));
        assert_eq!(args.password.as_deref(), Some("secret"));
    }

    #[test]
    fn update_pg_sync_settings_data_roundtrip() {
        let conn = oz_core::migrations::fresh_db();
        let args = UpdatePgSyncSettingsArgs {
            enabled: true,
            host: Some("db.example.com".into()),
            port: Some("5433".into()),
            dbname: Some("oz_sync".into()),
            user: Some("sync_user".into()),
            password: Some("secret".into()),
        };
        update_pg_sync_settings_data(&conn, &args).unwrap();

        let dto = run_get_pg_sync_settings(&conn).unwrap();
        assert!(dto.enabled);
        assert_eq!(dto.host.as_deref(), Some("db.example.com"));
        assert_eq!(dto.port.as_deref(), Some("5433"));
        assert_eq!(dto.dbname.as_deref(), Some("oz_sync"));
        assert_eq!(dto.user.as_deref(), Some("sync_user"));
        assert!(dto.has_password);
    }

    #[test]
    fn update_pg_sync_settings_data_disabled_default() {
        let conn = oz_core::migrations::fresh_db();
        let dto = run_get_pg_sync_settings(&conn).unwrap();
        assert!(!dto.enabled);
        assert!(dto.host.is_none());
        assert!(dto.port.is_none());
        assert!(dto.dbname.is_none());
        assert!(dto.user.is_none());
        assert!(!dto.has_password);
    }

    #[test]
    fn update_pg_sync_settings_data_none_clears_optional_fields() {
        let conn = oz_core::migrations::fresh_db();
        update_pg_sync_settings_data(
            &conn,
            &UpdatePgSyncSettingsArgs {
                enabled: true,
                host: Some("db.example.com".into()),
                port: Some("5432".into()),
                dbname: Some("oz_sync".into()),
                user: Some("sync_user".into()),
                password: None,
            },
        )
        .unwrap();
        // A later save with None clears the connection fields (same
        // contract as the HTTP sync URL handling).
        update_pg_sync_settings_data(
            &conn,
            &UpdatePgSyncSettingsArgs {
                enabled: false,
                host: None,
                port: None,
                dbname: None,
                user: None,
                password: None,
            },
        )
        .unwrap();

        let dto = run_get_pg_sync_settings(&conn).unwrap();
        assert!(!dto.enabled);
        assert!(dto.host.is_none());
        assert!(dto.port.is_none());
        assert!(dto.dbname.is_none());
        assert!(dto.user.is_none());
    }

    #[test]
    fn update_pg_sync_settings_data_password_preserved_when_none() {
        let conn = oz_core::migrations::fresh_db();
        update_pg_sync_settings_data(
            &conn,
            &UpdatePgSyncSettingsArgs {
                enabled: true,
                host: None,
                port: None,
                dbname: None,
                user: None,
                password: Some("secret".into()),
            },
        )
        .unwrap();
        // A later save without a password must keep the stored secret —
        // the UI sends None for the untouched masked field, mirroring the
        // HTTP sync API-key handling.
        update_pg_sync_settings_data(
            &conn,
            &UpdatePgSyncSettingsArgs {
                enabled: true,
                host: Some("db.example.com".into()),
                port: None,
                dbname: None,
                user: None,
                password: None,
            },
        )
        .unwrap();

        let dto = run_get_pg_sync_settings(&conn).unwrap();
        assert!(dto.has_password);
    }

    #[tokio::test]
    async fn pg_sync_settings_command_roundtrip() {
        let conn = oz_core::migrations::fresh_db();
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        update_pg_sync_settings(
            UpdatePgSyncSettingsArgs {
                enabled: true,
                host: Some("db.example.com".into()),
                port: None,
                dbname: Some("oz_sync".into()),
                user: None,
                password: Some("secret".into()),
            },
            app.state(),
        )
        .await
        .unwrap();

        let dto = get_pg_sync_settings(app.state()).await.unwrap();
        assert!(dto.enabled);
        assert_eq!(dto.host.as_deref(), Some("db.example.com"));
        assert_eq!(dto.dbname.as_deref(), Some("oz_sync"));
        assert!(dto.has_password);
    }

    #[tokio::test]
    async fn pg_sync_status_returns_default_on_fresh_state() {
        let conn = oz_core::migrations::fresh_db();
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let status = pg_sync_status(app.state()).await.unwrap();
        assert!(!status.running);
        assert_eq!(status.last_pushed, 0);
        assert_eq!(status.last_pulled, 0);
        assert_eq!(status.pending_count, 0);
        assert!(status.last_error.is_none());
    }

    #[tokio::test]
    async fn pg_sync_stop_on_stopped_daemon_is_noop() {
        let conn = oz_core::migrations::fresh_db();
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        // Stopping a daemon that was never started must succeed quietly.
        pg_sync_stop(app.state()).await.unwrap();
        let status = pg_sync_status(app.state()).await.unwrap();
        assert!(!status.running);
    }
}
