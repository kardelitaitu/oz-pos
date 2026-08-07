//! Tauri commands for workspace listing, navigation screens, and
//! per-user workspace assignment (admin feature).
//!
//! ADR #4 Phase 1: Now returns `WorkspaceDto` with instance-aware fields
//! and supports instance CRUD.
//!
//! ADR #7: Session-scoped commands are used for authenticated operations.
//! Only the pre-session workspace picker retains narrowly scoped discovery
//! commands; legacy mutation and user-targeted assignment commands are not
//! registered with Tauri.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::State;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use oz_core::db::Store;
use oz_core::db::workspaces::WorkspaceDto;
use oz_core::permissions;
use oz_core::subscription::TenantSubscription;

use crate::commands::authz::require_permission_for_user;
use crate::commands::picker_ticket;
use crate::error::AppError;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Legacy workspace DTO (pre-ADR #4).
///
/// Kept for the session-scoped workspace-type listing command. New code
/// should use `WorkspaceDto` from `oz_core::db::workspaces` when it needs
/// instance-aware data.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct WorkspaceTypeDto {
    /// Key.
    pub key: String,
    /// Display name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Icon.
    pub icon: String,
}

/// Screen within a workspace as seen by the front-end.
#[derive(Debug, Serialize)]
pub struct WorkspaceScreenDto {
    /// Screen Key.
    pub screen_key: String,
    /// Display sort order.
    pub sort_order: i32,
}

/// Request body for creating a workspace instance.
#[derive(Debug, serde::Deserialize)]
pub struct CreateInstanceRequest {
    /// Unique identifier.
    pub id: String,
    /// Type Key.
    pub type_key: String,
    /// ID of the associated store.
    pub store_id: String,
    /// Display name.
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Colour.
    pub colour: Option<String>,
}

// ── Scoped Commands (ADR #7) ────────────────────────────────────────

/// List workspace instances accessible to the session user within their store. ADR #7.
///
/// ADR #5: Filters results by subscription tier entitlement.
#[tauri::command]
pub async fn list_workspaces_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceDto>, AppError> {
    let session = state.resolve_session(&session_token)?; // ADR #5: Load subscription from global DB for entitlement filtering.
    // Also validates the system clock has not been rolled back.
    let tier = {
        let global_db = state.db.lock().await;
        TenantSubscription::validate_clock_rollback(&global_db)?;
        TenantSubscription::load(&global_db, "default")?
            .map(|sub| sub.effective_tier())
            .unwrap_or_else(|| {
                tracing::warn!(
                    "no subscription found for tenant 'default', defaulting to Free tier"
                );
                oz_core::SubscriptionTier::Free
            })
    };
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.list_workspaces_with_entitlement(
        &session.role_id,
        Some(&session.user_id),
        &session.store_id,
        &tier,
    )?;
    drop(db);
    Ok(rows)
}

/// Get a single workspace instance. `is_default` reflects the session user. ADR #7.
#[tauri::command]
pub async fn get_workspace_instance_scoped(
    session_token: String,
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let dto = store.get_workspace_instance(&instance_id, Some(&session.user_id))?;
    drop(db);
    Ok(dto)
}

/// Create a new workspace instance (admin). Permission from session. ADR #7.
///
/// ADR #5: Enforces subscription tier quota before creating.
#[tauri::command]
pub async fn create_workspace_instance_scoped(
    session_token: String,
    req: CreateInstanceRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceDto, AppError> {
    let session = state.resolve_session(&session_token)?;

    // ADR #5: Load subscription from the GLOBAL database first.
    // Also validates the system clock has not been rolled back.
    // This must happen before opening the store DB to avoid holding
    // a std::sync::MutexGuard across an .await boundary.
    let sub = {
        let global_db = state.db.lock().await;
        TenantSubscription::validate_clock_rollback(&global_db)?;
        TenantSubscription::load(&global_db, "default")?
            .ok_or_else(|| AppError::Internal("default tenant subscription not found".into()))?
    };
    sub.verify_signature()?;

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_UPDATE)?;
    let effective = sub.effective_tier();
    store.enforce_instance_quota(&effective, &req.type_key, &req.store_id)?;
    let _row = store.create_workspace_instance(
        &req.id,
        &req.type_key,
        &req.store_id,
        &req.name,
        req.description.as_deref().unwrap_or(""),
        req.colour.as_deref(),
    )?;
    let dto = store.get_workspace_instance(&req.id, Some(&session.user_id))?;
    drop(db);
    tracing::info!(
        instance_id = %req.id,
        type_key = %req.type_key,
        store_id = %req.store_id,
        "workspace instance created (scoped)"
    );
    Ok(dto)
}

/// Update the editable fields of a workspace instance (admin). ADR #7.
///
/// Renames the instance and updates its description / accent colour.
/// The `type_key` and `store_id` are immutable and cannot be changed.
/// Requires `STAFF_UPDATE` permission from the session user.
#[tauri::command]
pub async fn update_workspace_instance_scoped(
    session_token: String,
    instance_id: String,
    name: String,
    description: Option<String>,
    colour: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_UPDATE)?;
    store.update_workspace_instance(
        &instance_id,
        &name,
        description.as_deref(),
        colour.as_deref(),
    )?;
    drop(db);
    tracing::info!(instance_id = %instance_id, "workspace instance updated (scoped)");
    Ok(())
}

/// Archive (soft-delete) a workspace instance (admin). ADR #7.
///
/// Sets the instance status to `archived`, preserving referential
/// integrity with historical sales. Requires `STAFF_UPDATE` permission.
#[tauri::command]
pub async fn archive_workspace_instance_scoped(
    session_token: String,
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_UPDATE)?;
    store.archive_instance(&instance_id)?;
    drop(db);
    tracing::info!(instance_id = %instance_id, "workspace instance archived (scoped)");
    Ok(())
}

/// Recover `QuotaSuspended` workspace instances after a tier upgrade. ADR #5 Phase 3b.
///
/// Iterates the target store's database, restores suspended instances up to
/// the tier's per-store register limit, and returns the count of restored instances.
#[tauri::command]
pub async fn recover_workspace_instances_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<u32, AppError> {
    let session = state.resolve_session(&session_token)?;

    // Load subscription from the GLOBAL database.
    let sub = {
        let global_db = state.db.lock().await;
        TenantSubscription::validate_clock_rollback(&global_db)?;
        TenantSubscription::load(&global_db, "default")?
            .ok_or_else(|| AppError::Internal("default tenant subscription not found".into()))?
    };
    sub.verify_signature()?;

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let effective = sub.effective_tier();
    let restored = store.auto_recover_instances(&session.store_id, &effective)?;
    drop(db);
    tracing::info!(
        store_id = %session.store_id,
        restored = %restored,
        tier = %effective.name(),
        "workspace instances recovered after tier upgrade"
    );
    Ok(restored as u32)
}

/// Suspend surplus workspace instances after a tier downgrade. ADR #5 Phase 3c.
///
/// If the store has more active instances than the tier allows, the
/// least-recently-used instances are transitioned to `QuotaSuspended`.
/// Returns the count of suspended instances.
#[tauri::command]
pub async fn suspend_surplus_workspace_instances_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<u32, AppError> {
    let session = state.resolve_session(&session_token)?;

    // Load subscription from the GLOBAL database.
    let sub = {
        let global_db = state.db.lock().await;
        TenantSubscription::validate_clock_rollback(&global_db)?;
        TenantSubscription::load(&global_db, "default")?
            .ok_or_else(|| AppError::Internal("default tenant subscription not found".into()))?
    };
    sub.verify_signature()?;

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let effective = sub.effective_tier();
    let suspended = store.suspend_surplus_instances(&session.store_id, &effective)?;
    drop(db);
    tracing::info!(
        store_id = %session.store_id,
        suspended = %suspended,
        tier = %effective.name(),
        "surplus workspace instances suspended after tier downgrade"
    );
    Ok(suspended as u32)
}

/// List screens for a workspace type from the store-scoped database. ADR #7.
#[tauri::command]
pub async fn list_workspace_screens_scoped(
    session_token: String,
    type_key: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceScreenDto>, AppError> {
    let conn = state.resolve_store(&session_token)?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.list_workspace_type_screens(&type_key)?;
    drop(db);
    Ok(rows
        .into_iter()
        .map(|r| WorkspaceScreenDto {
            screen_key: r.screen_key,
            sort_order: r.sort_order,
        })
        .collect())
}

/// Replace all instance assignments for a user. Caller permission from session. ADR #7.
#[tauri::command]
pub async fn set_user_workspace_instances_scoped(
    session_token: String,
    user_id: String,
    instance_ids: Vec<String>,
    default_instance_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_UPDATE)?;
    let ids: Vec<&str> = instance_ids.iter().map(|s| s.as_str()).collect();
    store.set_user_workspace_instances(&user_id, ids, default_instance_id.as_deref())?;
    drop(db);
    tracing::info!(user_id = %user_id, count = %instance_ids.len(), "user workspace instance assignments updated (scoped)");
    Ok(())
}

/// Get instance IDs assigned to a user. Permission check from session. ADR #7.
#[tauri::command]
pub async fn get_user_workspace_instances_scoped(
    session_token: String,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_READ)?;
    let ids = store.get_user_workspace_instance_ids(&user_id)?;
    drop(db);
    Ok(ids)
}

// ── Original Commands (deprecated for multi-store — ADR #7) ─────────

/// List workspace instances for the pre-session workspace picker.
///
/// This narrow bootstrap command runs after username/PIN authentication but
/// before an opaque session token exists. The caller presents the short-lived
/// picker ticket minted by `staff_login` / `bootstrap_owner` (audit/06
/// residual): the REAL user is resolved from the global identity database and
/// the REAL role is used for the listing. A caller-supplied `role_id` /
/// `user_id` can no longer enumerate instances in stores the caller has no
/// access to. The requested store is opened through `StoreDatabaseManager` so
/// this read cannot accidentally query the global identity database or
/// another store's connection.
#[tauri::command]
pub async fn list_workspaces(
    state: State<'_, AppState>,
    ticket: String,
    store_id: String,
) -> Result<Vec<WorkspaceDto>, AppError> {
    // 1. Verify the ticket — uniform denial for forged/expired/malformed.
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let user_id = picker_ticket::verify_picker_ticket(&state.picker_ticket_secret, &ticket, now_ts)
        .ok_or_else(|| AppError::PermissionDenied("invalid or expired picker session".into()))?;

    // 2. Resolve the REAL user + role from the global identity DB. The ticket
    //    binds the user; the role is derived from the DB, never the claim.
    let (real_role_id, real_user_id) = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let user = store.get_user(&user_id)?.ok_or_else(|| {
            AppError::PermissionDenied("picker session user no longer exists".into())
        })?;
        if !user.is_active {
            return Err(AppError::PermissionDenied(
                "picker session user is inactive".into(),
            ));
        }
        let role = store
            .get_role(&user.role_id)?
            .ok_or_else(|| AppError::Internal(format!("role {} not found", user.role_id)))?;
        (role.id, user.id)
    };

    // 3. List instances in the requested store using the REAL role + user.
    //    `list_workspaces` applies the owner bypass, `user_store_access`
    //    (multi-store), explicit instance assignment, and role workspace types.
    let conn = state
        .db_manager
        .open_store(&store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.list_workspaces(&real_role_id, Some(&real_user_id), &store_id)?;
    drop(db);
    Ok(rows)
}

/// List workspace instances in an explicitly named store for the session user.
///
/// Authenticated replacement for the terminal-management screen's use of the
/// pre-session picker command (which hardcoded `role-owner`). The session
/// token binds the caller; the requested store is opened through
/// `StoreDatabaseManager` and `list_workspaces` still enforces the caller's
/// store access, so a session can only enumerate stores it may see.
#[tauri::command]
pub async fn list_workspaces_for_store_scoped(
    session_token: String,
    store_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.list_workspaces(&session.role_id, Some(&session.user_id), &store_id)?;
    drop(db);
    Ok(rows)
}

// ── Legacy Commands (backward compatible) ────────────────────────────

/// List ALL workspace types (for admin dropdowns).
///
/// **Deprecated for multi-store (ADR #7):** Use `list_workspaces_scoped` instead.
#[tauri::command]
pub async fn list_all_workspaces(
    _state: State<'_, AppState>,
    _user_id: String,
) -> Result<Vec<WorkspaceTypeDto>, AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped workspace commands are disabled; use list_all_workspaces_scoped".into(),
    ))
}

/// List all workspace types resolved from a session token. ADR #7.
#[tauri::command]
pub async fn list_all_workspaces_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceTypeDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_READ)?;
    let rows = store.list_all_workspace_types()?;
    drop(db);
    Ok(rows
        .into_iter()
        .map(|r| WorkspaceTypeDto {
            key: r.key,
            name: r.name,
            description: r.description,
            icon: r.icon,
        })
        .collect())
}

/// Replace all workspace assignments for a user (legacy tables).
///
/// **Deprecated for multi-store (ADR #7):** Use `set_user_workspace_instances_scoped`.
#[tauri::command]
pub async fn set_user_workspaces(
    _state: State<'_, AppState>,
    _user_id: String,
    _workspace_keys: Vec<String>,
    _caller_user_id: String,
) -> Result<(), AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped workspace commands are disabled; use set_user_workspaces_scoped".into(),
    ))
}

/// Replace all workspace assignments for a user (legacy tables), caller from session. ADR #7.
#[tauri::command]
pub async fn set_user_workspaces_scoped(
    session_token: String,
    user_id: String,
    workspace_keys: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_UPDATE)?;
    let keys: Vec<&str> = workspace_keys.iter().map(|s| s.as_str()).collect();
    store.set_user_workspaces_legacy(&user_id, keys)?;
    drop(db);
    tracing::info!(user_id = %user_id, count = %workspace_keys.len(), "user workspace assignments updated (legacy, scoped)");
    Ok(())
}

/// Get the explicit workspace keys assigned to a user (legacy table).
///
/// **Deprecated for multi-store (ADR #7):** Use `get_user_workspace_instances_scoped`.
#[tauri::command]
pub async fn get_user_workspaces(
    _state: State<'_, AppState>,
    _user_id: String,
) -> Result<Vec<String>, AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped workspace commands are disabled; use get_user_workspaces_scoped".into(),
    ))
}

/// Get workspace keys for a user (legacy table), caller from session. ADR #7.
#[tauri::command]
pub async fn get_user_workspaces_scoped(
    session_token: String,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_READ)?;
    let keys = store.get_user_workspace_keys_legacy(&user_id)?;
    drop(db);
    Ok(keys)
}

/// List screens (nav items) for a workspace type during boot/workspace
/// selection. The store ID is explicit so the read is routed to the correct
/// store database; authenticated callers should prefer the scoped variant.
///
/// The picker ticket (audit/06 residual) proves the caller completed a real
/// login before this bootstrap read can touch any store database.
#[tauri::command]
pub async fn list_workspace_screens(
    state: State<'_, AppState>,
    ticket: String,
    type_key: String,
    store_id: String,
) -> Result<Vec<WorkspaceScreenDto>, AppError> {
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    picker_ticket::verify_picker_ticket(&state.picker_ticket_secret, &ticket, now_ts)
        .ok_or_else(|| AppError::PermissionDenied("invalid or expired picker session".into()))?;
    let conn = state
        .db_manager
        .open_store(&store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let rows = store.list_workspace_type_screens(&type_key)?;
    drop(db);
    Ok(rows
        .into_iter()
        .map(|r| WorkspaceScreenDto {
            screen_key: r.screen_key,
            sort_order: r.sort_order,
        })
        .collect())
}

/// Replace all instance assignments for a user through the session-scoped API.
///
/// The former unscoped command accepted a forgeable `caller_user_id` and is
/// intentionally retained only as a non-callable Rust symbol for source
/// compatibility. It is not registered with Tauri; callers must use
/// `set_user_workspace_instances_scoped`.
#[allow(dead_code)]
pub async fn set_user_workspace_instances(
    _state: State<'_, AppState>,
    _user_id: String,
    _instance_ids: Vec<String>,
    _default_instance_id: Option<String>,
    _caller_user_id: String,
) -> Result<(), AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped workspace commands are disabled; use set_user_workspace_instances_scoped"
            .into(),
    ))
}

/// Get instance IDs through the session-scoped API.
///
/// The former unscoped command is not registered with Tauri because it had no
/// authenticated caller context. Callers must use
/// `get_user_workspace_instances_scoped`.
#[allow(dead_code)]
pub async fn get_user_workspace_instances(
    _state: State<'_, AppState>,
    _user_id: String,
) -> Result<Vec<String>, AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped workspace commands are disabled; use get_user_workspace_instances_scoped"
            .into(),
    ))
}

// ── Boot Resolution (ADR #4 Phase 3) ────────────────────────────────

/// DTO returned by `resolve_boot_store`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootResolution {
    /// Whether this is bound.
    pub is_bound: bool,
    /// ID of the associated store.
    pub store_id: String,
    /// ID of the associated instance.
    pub instance_id: Option<String>,
}

/// Verify a device-binding HMAC signature using constant-time comparison.
///
/// Uses `mac.verify_slice()` which internally uses `subtle::ConstantTimeEq`
/// to prevent timing side-channel attacks. The previous implementation
/// used `hex::encode(mac.finalize().into_bytes()) == signature`, which
/// short-circuits on the first differing byte — leaking the position
/// of the mismatch to an attacker.
fn verify_binding_hmac(
    secret: &str,
    terminal_id: &str,
    store_id: &str,
    instance_id: &str,
    hex_signature: &str,
) -> bool {
    let expected_bytes = match hex::decode(hex_signature) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(terminal_id.as_bytes());
    mac.update(b":");
    mac.update(store_id.as_bytes());
    mac.update(b":");
    mac.update(instance_id.as_bytes());
    mac.verify_slice(&expected_bytes).is_ok()
}

/// Resolve the active store and instance from device binding.
///
/// This is called once at boot time (before authentication). It does not use
/// a session token because no user is logged in yet.
#[tauri::command]
pub async fn resolve_boot_store(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<BootResolution, AppError> {
    let device_id = device_id
        .filter(|d| !d.is_empty())
        .or_else(|| {
            std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .ok()
        })
        .unwrap_or_default();

    if device_id.is_empty() {
        let primary_id = {
            let db = state.db.lock().await;
            let store = Store::new(&db);
            let primary = store
                .get_primary_store()?
                .ok_or_else(|| AppError::Internal("no primary store found".into()))?;
            primary.id
        };
        tracing::info!(
            store_id = %primary_id,
            "boot resolution: no device_id available, using primary store"
        );
        return Ok(BootResolution {
            is_bound: false,
            store_id: primary_id,
            instance_id: None,
        });
    }

    let binding_info: Option<(String, String, String, String)> = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        store
            .get_terminal_by_device_id(&device_id)?
            .and_then(|terminal| {
                let tid = terminal.id;
                store
                    .get_terminal_binding(&tid)
                    .ok()
                    .flatten()
                    .map(|(s, i, sig)| (tid, s, i, sig))
            })
    };

    if let Some((terminal_id, bound_store_id, bound_instance_id, signature)) = binding_info {
        let signature_valid = {
            let keyring = oz_security::default_keyring()
                .map_err(|e| AppError::Internal(format!("keyring unavailable: {e}")))?;
            let secret = keyring
                .get_secret(crate::commands::terminals::DEVICE_BINDING_KEYRING_NAME)
                .map_err(|e| AppError::Internal(format!("keyring read failed: {e}")))?;

            match secret {
                Some(secret) => verify_binding_hmac(
                    &secret,
                    &terminal_id,
                    &bound_store_id,
                    &bound_instance_id,
                    &signature,
                ),
                None => false,
            }
        };

        if !signature_valid {
            tracing::warn!(
                terminal_id = %terminal_id,
                bound_store_id = %bound_store_id,
                "device binding HMAC validation failed — falling back to primary store"
            );
        } else {
            let instance_exists = {
                state
                    .db_manager
                    .open_store(&bound_store_id)
                    .ok()
                    .and_then(|db_arc| {
                        let db = db_arc.lock().ok()?;
                        let store = Store::new(&db);
                        store
                            .get_workspace_instance(&bound_instance_id, None)
                            .ok()
                            .map(|_| true)
                    })
                    .unwrap_or(false)
            };

            if !instance_exists {
                tracing::warn!(
                    terminal_id = %terminal_id,
                    bound_store_id = %bound_store_id,
                    bound_instance_id = %bound_instance_id,
                    "bound instance not found or not active — falling back to primary store"
                );
            } else {
                tracing::info!(
                    terminal_id = %terminal_id,
                    store_id = %bound_store_id,
                    instance_id = %bound_instance_id,
                    "device binding resolved — auto-booting into bound workspace"
                );
                return Ok(BootResolution {
                    is_bound: true,
                    store_id: bound_store_id,
                    instance_id: Some(bound_instance_id),
                });
            }
        }
    }

    let primary_id = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        let primary = store
            .get_primary_store()?
            .ok_or_else(|| AppError::Internal("no primary store found".into()))?;
        primary.id
    };

    tracing::info!(
        store_id = %primary_id,
        "boot resolution fell back to primary store"
    );
    Ok(BootResolution {
        is_bound: false,
        store_id: primary_id,
        instance_id: None,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Token Rejection ─────────────────────────────────────────────────

    #[test]
    fn workspaces_scoped_rejects_invalid_token() {
        let state = AppState::for_test();
        let result = state.resolve_session("nonexistent-token");
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    // ── WorkspaceTypeDto ─────────────────────────────────────────────────

    #[test]
    fn workspace_type_dto_debug() {
        let dto = WorkspaceTypeDto {
            key: "retail".into(),
            name: "Retail".into(),
            description: "Retail POS".into(),
            icon: "store".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("retail"));
        assert!(d.contains("Retail POS"));
    }

    #[test]
    fn workspace_type_dto_serialize() {
        let dto = WorkspaceTypeDto {
            key: "restaurant".into(),
            name: "Restaurant".into(),
            description: String::new(),
            icon: "utensils".into(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["key"], "restaurant");
        assert_eq!(json["description"], "");
    }

    // ── WorkspaceScreenDto ──────────────────────────────────────────────

    #[test]
    fn workspace_screen_dto_debug() {
        let dto = WorkspaceScreenDto {
            screen_key: "pos".into(),
            sort_order: 1,
        };
        let d = format!("{dto:?}");
        assert!(d.contains("pos"));
        assert!(d.contains("1"));
    }

    #[test]
    fn workspace_screen_dto_serialize() {
        let dto = WorkspaceScreenDto {
            screen_key: "history".into(),
            sort_order: 5,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["screen_key"], "history");
        assert_eq!(json["sort_order"], 5);
    }

    // ── CreateInstanceRequest ───────────────────────────────────────────

    #[test]
    fn create_instance_request_deserializes() {
        let json = r#"{
            "id": "ws-dt-1",
            "type_key": "restaurant-pos",
            "store_id": "store-downtown",
            "name": "Downtown - Cashier 1"
        }"#;
        let req: CreateInstanceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "ws-dt-1");
        assert_eq!(req.type_key, "restaurant-pos");
        assert_eq!(req.name, "Downtown - Cashier 1");
        assert!(req.description.is_none());
        assert!(req.colour.is_none());
    }

    // ── BootResolution (ADR #4 Phase 3) ─────────────────────────────────

    #[test]
    fn boot_resolution_dto_serialize_bound() {
        let res = BootResolution {
            is_bound: true,
            store_id: "store-downtown".into(),
            instance_id: Some("ws-dt-cashier-1".into()),
        };
        let json = serde_json::to_value(&res).unwrap();
        assert_eq!(json["isBound"], true);
        assert_eq!(json["storeId"], "store-downtown");
        assert_eq!(json["instanceId"], "ws-dt-cashier-1");
    }

    #[test]
    fn boot_resolution_dto_serialize_unbound() {
        let res = BootResolution {
            is_bound: false,
            store_id: "default".into(),
            instance_id: None,
        };
        let json = serde_json::to_value(&res).unwrap();
        assert_eq!(json["isBound"], false);
        assert_eq!(json["storeId"], "default");
        assert!(json["instanceId"].is_null());
    }

    #[test]
    fn boot_resolution_dto_debug() {
        let res = BootResolution {
            is_bound: false,
            store_id: "default".into(),
            instance_id: None,
        };
        let d = format!("{res:?}");
        assert!(d.contains("default"));
        assert!(d.contains("false"));
    }

    // ── Pre-session picker ticket binding (audit/06 residual) ──────────
    //
    // TDD red: `list_workspaces` / `list_workspace_screens` must bind the
    // listing to the authenticated user server-side. Previously the commands
    // trusted the caller-supplied `role_id` / `user_id`, so any caller who
    // knew an owner's id could enumerate instances in any store as
    // `role-owner`. Now the caller presents a short-lived HMAC ticket and the
    // REAL role is resolved from the global identity DB.

    use oz_core::StoreProfile;
    use oz_core::migrations;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

    /// Seed the GLOBAL identity DB with an owner and a cashier.
    fn seed_global_users(conn: &rusqlite::Connection) {
        let store = Store::new(conn);
        store.seed_default_roles().unwrap();
        conn.execute_batch(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-owner',   'owner',   'hash', 'Owner',   'role-owner',   1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-cashier', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
        )
        .unwrap();
    }

    fn make_profile(id: &str, name: &str) -> StoreProfile {
        StoreProfile {
            id: id.to_owned(),
            name: name.to_owned(),
            address: String::new(),
            tax_id: String::new(),
            currency: "USD".to_owned(),
            timezone: "UTC".to_owned(),
            is_primary: false,
            created_at: "2026-07-01T10:00:00Z".to_owned(),
            updated_at: "2026-07-01T10:00:00Z".to_owned(),
        }
    }

    /// Build an AppState with a global identity DB (users seeded) and a
    /// temp-dir store manager with store-a (1 instance) and store-b (1
    /// instance) so cross-store isolation can be exercised.
    fn picker_state() -> (AppState, tempfile::TempDir) {
        let conn = migrations::fresh_db();
        seed_global_users(&conn);
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), migrations::ALL);

        for (store_id, instance_id) in [("store-a", "ws-a-1"), ("store-b", "ws-b-1")] {
            let conn = state.db_manager.open_store(store_id).unwrap();
            let db = conn.lock().unwrap();
            let store = Store::new(&db);
            store
                .create_store_profile(&make_profile(store_id, store_id))
                .unwrap();
            store
                .create_workspace_instance(instance_id, "store-pos", store_id, "POS", "", None)
                .unwrap();
        }
        (state, temp_dir)
    }

    // ── Repair migration 120 end-to-end (store DB) ────────────────
    //
    // Simulates the migration 066 regression window at the AppState level:
    // the store DB's workspace_instances is emptied (066 dropped rows whose
    // store_id was not yet in store_profiles, and is recorded as applied so
    // it never re-runs). Re-opening the store DB must run repair migration
    // 120 and re-seed the default instances, so the owner picker lists them.
    #[tokio::test]
    async fn list_workspaces_repairs_empty_store_db_after_066_window() {
        let (state, _dir) = picker_state();
        // Wipe store-a's instances to mimic the broken-window empty table.
        {
            let conn = state.db_manager.open_store("store-a").unwrap();
            let db = conn.lock().unwrap();
            db.execute("DELETE FROM workspace_instances", []).unwrap();
            // Mark 120 and 121 as not-yet-applied (both were recorded during
            // setup) so the next open re-runs the repair, exactly like a real
            // upgrade from before the multi-store re-point (120 reseeds, 121
            // re-points the canonical instances to the store's own profile).
            db.execute(
                "DELETE FROM schema_migrations WHERE id = '120_reseed_default_workspace_instances.sql'",
                [],
            )
            .unwrap();
            db.execute(
                "DELETE FROM schema_migrations WHERE id = '121_workspace_instances_store_own_profile.sql'",
                [],
            )
            .unwrap();
            // Verify the wipe while the connection is still cached (no migration
            // re-run), so this asserts the broken-window empty state.
            let wiped: i64 = db
                .query_row("SELECT COUNT(*) FROM workspace_instances", [], |r| r.get(0))
                .unwrap();
            assert_eq!(wiped, 0, "precondition: store DB instances wiped");
        }
        // Evict the cached connection so open_store re-runs migrations
        // (the runner only applies unapplied migrations on a fresh open).
        state.db_manager.close_store("store-a");

        // Re-open the store DB — migrations (incl. 120) run again and repair it.
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let owner_ticket = sign_ticket_for(&app.state(), "user-owner", 300);
        let rows = list_workspaces(app.state(), owner_ticket, "store-a".into())
            .await
            .unwrap();
        // Diagnostics: inspect the store DB state after the repair.
        {
            let conn = app
                .state::<AppState>()
                .db_manager
                .open_store("store-a")
                .unwrap();
            let db = conn.lock().unwrap();
            let wt: i64 = db
                .query_row("SELECT COUNT(*) FROM workspace_types", [], |r| r.get(0))
                .unwrap();
            let wi: i64 = db
                .query_row("SELECT COUNT(*) FROM workspace_instances", [], |r| r.get(0))
                .unwrap();
            let sp: i64 = db
                .query_row("SELECT COUNT(*) FROM store_profiles", [], |r| r.get(0))
                .unwrap();
            eprintln!(
                "DIAG store-a: workspace_types={wt} workspace_instances={wi} store_profiles={sp}"
            );
            let ids: Vec<String> = db
                .prepare("SELECT id FROM workspace_instances")
                .unwrap()
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            eprintln!("DIAG store-a instance ids: {ids:?}");
        }
        assert!(
            rows.iter()
                .any(|d| d.instance_id == "default-store-pos" && d.store_id == "store-a"),
            "repair migration 120 must re-seed the store DB's default instances, got {rows:?}"
        );
    }

    fn sign_ticket_for(state: &AppState, user_id: &str, ttl_offset: i64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        picker_ticket::sign_picker_ticket(&state.picker_ticket_secret, user_id, now + ttl_offset)
    }

    #[tokio::test]
    async fn list_workspaces_rejects_forged_or_missing_ticket() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // Empty ticket, garbage ticket, and a ticket forged with the wrong
        // secret must all be denied uniformly (no enumeration oracle).
        for ticket in [
            String::new(),
            "garbage-not-a-ticket".into(),
            picker_ticket::sign_picker_ticket(
                b"attacker-secret",
                "user-owner",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
                    + 300,
            ),
        ] {
            let result = list_workspaces(app.state(), ticket.clone(), "store-a".into()).await;
            assert!(
                matches!(result, Err(AppError::PermissionDenied(_))),
                "ticket {ticket:?} must be denied"
            );
        }
    }

    #[tokio::test]
    async fn list_workspaces_rejects_expired_ticket() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let expired = sign_ticket_for(&app.state(), "user-owner", -1);
        let result = list_workspaces(app.state(), expired, "store-a".into()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn list_workspaces_rejects_unknown_user_even_with_valid_signature() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // A correctly-signed ticket for a user that no longer exists must
        // still be denied — identity must resolve, not just the HMAC.
        let ghost = sign_ticket_for(&app.state(), "user-deleted", 300);
        let result = list_workspaces(app.state(), ghost, "store-a".into()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn list_workspaces_uses_real_role_not_claimed_role() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // Owner's ticket → owner bypass lists the instance.
        let owner_ticket = sign_ticket_for(&app.state(), "user-owner", 300);
        let owner_rows = list_workspaces(app.state(), owner_ticket, "store-a".into())
            .await
            .unwrap();
        assert!(owner_rows.iter().any(|d| d.instance_id == "ws-a-1"));

        // Cashier's ticket → cashier role has no role_workspace_types in the
        // fresh store DB, so the listing is empty. The old command let a
        // caller CLAIM role-owner to see the instance; the new one derives
        // the role from the DB, so the cashier cannot escalate.
        let cashier_ticket = sign_ticket_for(&app.state(), "user-cashier", 300);
        let cashier_rows = list_workspaces(app.state(), cashier_ticket, "store-a".into())
            .await
            .unwrap();
        assert!(
            cashier_rows.is_empty(),
            "cashier role must not see owner-level instances, got {cashier_rows:?}"
        );
    }

    #[tokio::test]
    async fn list_workspaces_inactive_user_is_denied() {
        let (state, _dir) = picker_state();
        {
            let db = state.db.lock().await;
            db.execute("UPDATE users SET is_active = 0 WHERE id = 'user-owner'", [])
                .unwrap();
        }
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let ticket = sign_ticket_for(&app.state(), "user-owner", 300);
        let result = list_workspaces(app.state(), ticket, "store-a".into()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn list_workspace_screens_requires_valid_ticket() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let denied = list_workspace_screens(
            app.state(),
            "bogus".into(),
            "store-pos".into(),
            "store-a".into(),
        )
        .await;
        assert!(matches!(denied, Err(AppError::PermissionDenied(_))));

        let ticket = sign_ticket_for(&app.state(), "user-owner", 300);
        let allowed =
            list_workspace_screens(app.state(), ticket, "store-pos".into(), "store-a".into()).await;
        assert!(allowed.is_ok());
    }

    #[tokio::test]
    async fn list_workspaces_for_store_scoped_rejects_invalid_session() {
        let (state, _dir) = picker_state();
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result =
            list_workspaces_for_store_scoped("missing-token".into(), "store-a".into(), app.state())
                .await;
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[tokio::test]
    async fn list_workspaces_for_store_scoped_uses_session_role() {
        let (state, _dir) = picker_state();
        state.session_store.write().unwrap().insert(
            "cashier-token".into(),
            oz_core::session::SessionContext::new(
                "user-cashier".into(),
                "role-cashier".into(),
                "terminal-1".into(),
                "store-a".into(),
                "ws-a-1".into(),
                "store-pos".into(),
                None,
                0,
            ),
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // The session token binds the real role — a cashier session listing
        // store-a must not see owner-level instances (same as the ticket path).
        let rows =
            list_workspaces_for_store_scoped("cashier-token".into(), "store-a".into(), app.state())
                .await
                .unwrap();
        assert!(
            rows.is_empty(),
            "cashier session must not enumerate store-a instances, got {rows:?}"
        );
    }
}
