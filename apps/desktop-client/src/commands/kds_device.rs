//! KDS device management commands.
//!
//! IPC surface for registering, listing, updating status, and
//! deactivating Kitchen Display System devices.
//!
//! All commands require `kds:manage` permission for writes and
//! `kds:view` for reads.

use tauri::State;

use oz_core::db::Store;
use oz_core::kds::{KdsConnectionStatus, KdsDevice, RegisterKdsDeviceInput};
use oz_core::permissions;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

/// Register a new KDS device bound to a Restaurant POS.
///
/// The caller supplies a pre-hashed pairing token (SHA-256 of the QR
/// enrollment token) and its expiry timestamp. The device starts in
/// `disconnected` status and `is_active = true`.
#[tauri::command]
pub async fn register_kds_device_scoped(
    session_token: String,
    input: RegisterKdsDeviceInput,
    state: State<'_, AppState>,
) -> Result<KdsDevice, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::KDS_UPDATE).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let device = store.register_kds_device(input)?;
    Ok(device)
}

/// List all KDS devices for the Restaurant POS bound to the current session.
#[tauri::command]
pub async fn list_kds_devices_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<KdsDevice>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::KDS_VIEW).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);

    // Use the restaurant_pos_id from the session, or fall back to terminal_id.
    let resto_id = session
        .restaurant_pos_id
        .as_deref()
        .unwrap_or(&session.terminal_id);
    let devices = store.list_kds_devices_for_restaurant(resto_id)?;
    Ok(devices)
}

/// Get a single KDS device by ID.
#[tauri::command]
pub async fn get_kds_device_scoped(
    session_token: String,
    device_id: String,
    state: State<'_, AppState>,
) -> Result<Option<KdsDevice>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::KDS_VIEW).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let device = store.get_kds_device(&device_id)?;
    Ok(device)
}

/// Update a KDS device's connection status.
///
/// The Restaurant POS calls this when a KDS device connects or
/// disconnects. Setting `Connected` also updates `last_seen_at`.
#[tauri::command]
pub async fn update_kds_device_status_scoped(
    session_token: String,
    device_id: String,
    status: KdsConnectionStatus,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::KDS_UPDATE).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.update_kds_device_status(&device_id, status)?;
    Ok(())
}

/// Deactivate a KDS device (soft-delete).
///
/// Deactivated devices no longer receive routed orders. The device
/// record is retained for audit purposes.
#[tauri::command]
pub async fn deactivate_kds_device_scoped(
    session_token: String,
    device_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::KDS_UPDATE).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.deactivate_kds_device(&device_id)?;
    Ok(())
}

/// Acknowledge a KDS order — atomically transitions from 'pending' to 'ready'.
///
/// Uses optimistic locking: if another device already acknowledged
/// this order, returns `false` instead of erroring.
#[tauri::command]
pub async fn ack_kds_order_scoped(
    session_token: String,
    order_id: String,
    device_id: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::KDS_UPDATE).await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let acked = store.ack_kds_order(&order_id, &device_id)?;
    Ok(acked)
}
