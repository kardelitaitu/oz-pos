//! IPC surface for the loopback local API server (step 2 of
//! `docs/guides/EXTENDING.md`).
//!
//! Device-level settings (`local_api.*` on the global DB, like the
//! `lan_server.*` precedent): the server is one per machine, not per
//! store — but it SERVES the primary store's database (resolved via
//! `store_profiles.is_primary`, see `local_api::open_api_store_connection`).
//! Mutations require `settings:edit`; status reads require
//! `settings:read`. The signing secret never crosses the IPC boundary —
//! the UI gets a minted token, not the secret (the secret doubles as the
//! operator `X-Admin-Key`, so it is also on the settings secret
//! deny-list).
//!
//! Every lifecycle transition holds `AppState::local_api_op` for its
//! full check-then-act sequence, so a toggle can never interleave with
//! the boot auto-start daemon or another toggle.

use rusqlite::Connection;
use tauri::Manager;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::local_api::{self, LocalApiStatus};
use crate::state::AppState;
use oz_core::permissions;

/// Resolve the local image store directory (same layout as
/// `commands::products_images`: `app_cache_dir()/images`).
fn image_dir_for(app: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Internal(format!("resolving app cache dir: {e}")))?;
    Ok(cache_dir.join("images"))
}

/// Persist a `local_api.*` setting on the global DB.
fn persist_setting(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    oz_core::Settings::set(conn, key, value)
        .map_err(|e| AppError::Internal(format!("persisting {key}: {e}")))
}

/// Read port + secret + primary-store API connection from state.
///
/// The global-db guard is dropped before returning —
/// `open_api_store_connection` opens its own handle. The primary store
/// id is resolved on the GLOBAL DB (`store_profiles`), never the store
/// DB.
async fn prepare(
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<
    (
        u16,
        String,
        std::sync::Arc<tokio::sync::Mutex<Connection>>,
        std::path::PathBuf,
        std::path::PathBuf,
    ),
    AppError,
> {
    let (port, secret, store_id) = {
        let db = state.db.lock().await;
        let port = local_api::resolve_port(&db);
        let secret = local_api::load_or_create_secret(&db).map_err(AppError::Internal)?;
        let store_id = local_api::primary_store_id(&db);
        (port, secret, store_id)
    }; // global db guard dropped — open_store below may await internally
    let (api_db, api_db_path) = local_api::open_api_store_connection(&state.db_manager, &store_id)
        .map_err(AppError::Internal)?;
    let image_dir = image_dir_for(app)?;
    Ok((port, secret, api_db, api_db_path, image_dir))
}

/// Build the status snapshot from current settings + running handle.
async fn build_status(state: &AppState) -> Result<LocalApiStatus, AppError> {
    let (enabled, port) = {
        let db = state.db.lock().await;
        (local_api::is_enabled(&db), local_api::resolve_port(&db))
    };
    let running = state.local_api.lock().await;
    Ok(LocalApiStatus {
        enabled,
        running: running.is_some(),
        port,
        base_url: running.as_ref().map(|h| h.base_url.clone()),
    })
}

/// Report whether the local API is enabled/running and on which port.
#[tauri::command]
pub async fn local_api_status_scoped(
    session_token: String,
    state: tauri::State<'_, AppState>,
) -> Result<LocalApiStatus, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::SETTINGS_READ).await?;
    build_status(&state).await
}

/// Enable or disable the local API server (persisted across restarts).
///
/// Enabling binds `127.0.0.1:<port>` against the PRIMARY STORE's
/// database; a port conflict returns an error and leaves the setting
/// off. Loopback-only by design — LAN exposure is not part of this
/// surface (see the guide §10).
#[tauri::command]
pub async fn local_api_set_enabled_scoped(
    session_token: String,
    enabled: bool,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<LocalApiStatus, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::SETTINGS_EDIT).await?;

    // Serialize the whole transition against the boot daemon and other
    // toggles (review HIGH-2).
    let _op = state.local_api_op.lock().await;

    if !enabled {
        if let Some(handle) = state.local_api.lock().await.take() {
            handle.stop_async().await;
        }
        let db = state.db.lock().await;
        persist_setting(&db, local_api::SETTINGS_ENABLED, "0")?;
        drop(db);
        return build_status(&state).await;
    }

    // Already running? Idempotent success.
    if state.local_api.lock().await.is_some() {
        return build_status(&state).await;
    }

    let (port, secret, api_db, api_db_path, image_dir) = prepare(&state, &app).await?;
    let handle = local_api::start(api_db, api_db_path, image_dir, secret, port)
        .await
        .map_err(AppError::Internal)?;
    *state.local_api.lock().await = Some(handle);

    // Persist the enabled intent only AFTER a successful bind, so a
    // port conflict never leaves an "enabled but not running" setting
    // to confuse the next boot.
    let db = state.db.lock().await;
    persist_setting(&db, local_api::SETTINGS_ENABLED, "1")?;
    drop(db);
    build_status(&state).await
}

/// Change the listen port. When the server is running it is restarted
/// on the new port; a failed restart returns an error and leaves the
/// new port persisted (the next enable/boot retries it).
#[tauri::command]
pub async fn local_api_set_port_scoped(
    session_token: String,
    port: u16,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<LocalApiStatus, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::SETTINGS_EDIT).await?;
    if !(1024..=65535).contains(&port) {
        return Err(AppError::Invalid(
            "port must be between 1024 and 65535".into(),
        ));
    }

    let _op = state.local_api_op.lock().await;

    {
        let db = state.db.lock().await;
        persist_setting(&db, local_api::SETTINGS_PORT, &port.to_string())?;
    }

    // Restart if running so the new port takes effect immediately.
    // stop_async guarantees the old listener is gone before re-binding
    // (review MED-3: rapid disable→enable on the same port).
    let was_running = {
        let mut slot = state.local_api.lock().await;
        match slot.take() {
            Some(handle) => {
                drop(slot); // release the slot before start + re-lock
                handle.stop_async().await;
                true
            }
            None => false,
        }
    };
    if was_running {
        let (_old_port, secret, api_db, api_db_path, image_dir) = prepare(&state, &app).await?;
        let handle = local_api::start(api_db, api_db_path, image_dir, secret, port)
            .await
            .map_err(AppError::Internal)?;
        *state.local_api.lock().await = Some(handle);
    }
    build_status(&state).await
}

/// Mint a long-lived API token signed with the per-install secret.
///
/// Returns the JWT + expiry; the secret itself never crosses the IPC
/// boundary. Master-data writes additionally need
/// `X-Admin-Key: <secret>` — the UI explains this and the secret is
/// available to operators via the settings table only (deny-listed from
/// `get_setting`).
#[tauri::command]
pub async fn local_api_mint_token_scoped(
    session_token: String,
    label: String,
    expiry_hours: Option<i64>,
    state: tauri::State<'_, AppState>,
) -> Result<oz_api::auth::TokenResponse, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::SETTINGS_EDIT).await?;
    let secret = {
        let db = state.db.lock().await;
        local_api::load_or_create_secret(&db).map_err(AppError::Internal)?
    };
    local_api::mint_token(&secret, &label, expiry_hours).map_err(AppError::Internal)
}

#[cfg(test)]
#[path = "local_api_command_tests.rs"]
mod tests;
