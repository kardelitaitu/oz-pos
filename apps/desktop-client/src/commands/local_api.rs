//! IPC surface for the loopback local API server (step 2 of
//! `docs/guides/EXTENDING.md`).
//!
//! Device-level settings (`local_api.*` on the global DB, like the
//! `lan_server.*` precedent): the server is one per machine, not per
//! store. Mutations require `settings:edit`; status reads require
//! `settings:read`. The signing secret never crosses the IPC boundary —
//! the UI gets a minted token, not the secret (the secret doubles as the
//! operator `X-Admin-Key`, so it is also on the settings secret
//! deny-list).

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
/// Enabling binds `127.0.0.1:<port>`; a port conflict returns an error
/// and leaves the setting off. Loopback-only by design — LAN exposure
/// is not part of this surface (see the guide §10).
#[tauri::command]
pub async fn local_api_set_enabled_scoped(
    session_token: String,
    enabled: bool,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<LocalApiStatus, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::SETTINGS_EDIT).await?;

    if !enabled {
        if let Some(handle) = state.local_api.lock().await.take() {
            handle.stop();
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

    // Read config + secret, then DROP the db guard before any await —
    // `start` hands a clone of the same Arc<Mutex<Connection>> to the
    // API server, which must be able to lock it.
    let (port, secret) = {
        let db = state.db.lock().await;
        let port = local_api::resolve_port(&db);
        let secret = local_api::load_or_create_secret(&db).map_err(AppError::Internal)?;
        (port, secret)
    };
    let handle = local_api::start(
        state.db.clone(),
        state.db_path.clone(),
        image_dir_for(&app)?,
        secret,
        port,
    )
    .await
    .map_err(AppError::Internal)?;
    *state.local_api.lock().await = Some(handle);

    // Persist the enabled intent only AFTER a successful bind, so a
    // port conflict never leaves a "enabled but not running" setting
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

    {
        let db = state.db.lock().await;
        persist_setting(&db, local_api::SETTINGS_PORT, &port.to_string())?;
    }

    // Restart if running so the new port takes effect immediately.
    let was_running = {
        let mut slot = state.local_api.lock().await;
        match slot.take() {
            Some(handle) => {
                drop(slot); // release the slot before start + re-lock
                handle.stop();
                true
            }
            None => false,
        }
    };
    if was_running {
        let (secret, image_dir) = {
            let db = state.db.lock().await;
            (
                local_api::load_or_create_secret(&db).map_err(AppError::Internal)?,
                image_dir_for(&app)?,
            )
        };
        let handle = local_api::start(
            state.db.clone(),
            state.db_path.clone(),
            image_dir,
            secret,
            port,
        )
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
