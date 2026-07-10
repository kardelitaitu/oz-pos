//! Tauri commands for workspace listing, navigation screens, and
//! per-user workspace assignment (admin feature).
//!
//! ADR #4 Phase 1: Now returns `WorkspaceDto` with instance-aware fields
//! and supports instance CRUD. Legacy commands are preserved for
//! backward compatibility and marked as deprecated.
//!
//! ADR #7: All active commands have scoped variants using the session token
//! pattern. Old commands taking raw `user_id` / `store_id` are deprecated.

use serde::Serialize;
use tauri::{State, command};

use hmac::{Hmac, Mac};
use sha2::Sha256;

use oz_core::db::Store;
use oz_core::db::workspaces::WorkspaceDto;
use oz_core::permissions;
use oz_core::subscription::TenantSubscription;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Legacy workspace DTO (pre-ADR #4).
///
/// Kept for backward compatibility with `list_workspace_types` and
/// `list_all_workspaces` commands. New code should use `WorkspaceDto`
/// from `oz_core::db::workspaces` instead.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct WorkspaceTypeDto {
    pub key: String,
    pub name: String,
    pub description: String,
    pub icon: String,
}

/// Screen within a workspace as seen by the front-end.
#[derive(Debug, Serialize)]
pub struct WorkspaceScreenDto {
    pub screen_key: String,
    pub sort_order: i32,
}

/// Request body for creating a workspace instance.
#[derive(Debug, serde::Deserialize)]
pub struct CreateInstanceRequest {
    pub id: String,
    pub type_key: String,
    pub store_id: String,
    pub name: String,
    pub description: Option<String>,
    pub colour: Option<String>,
}

// ── Scoped Commands (ADR #7) ────────────────────────────────────────

/// List workspace instances accessible to the session user within their store. ADR #7.
///
/// ADR #5: Filters results by subscription tier entitlement.
#[command]
pub async fn list_workspaces_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    // ADR #5: Load subscription from global DB for entitlement filtering.
    let tier = {
        let global_db = state.db.lock().await;
        TenantSubscription::load(&global_db, "default")?
            .map(|sub| sub.tier)
            .unwrap_or_else(|| {
                tracing::warn!("no subscription found for tenant 'default', defaulting to Free tier");
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
#[command]
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
#[command]
pub async fn create_workspace_instance_scoped(
    session_token: String,
    req: CreateInstanceRequest,
    state: State<'_, AppState>,
) -> Result<WorkspaceDto, AppError> {
    let session = state.resolve_session(&session_token)?;

    // ADR #5: Load subscription from the GLOBAL database first.
    // This must happen before opening the store DB to avoid holding
    // a std::sync::MutexGuard across an .await boundary.
    let sub = {
        let global_db = state.db.lock().await;
        TenantSubscription::load(&global_db, "default")?
            .ok_or_else(|| AppError::Internal("default tenant subscription not found".into()))?
    };
    sub.verify_signature("")?;

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_UPDATE)?;
    store.enforce_instance_quota(&sub.tier, &req.type_key, &req.store_id)?;
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

/// List screens for a workspace type from the store-scoped database. ADR #7.
#[command]
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
#[command]
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
#[command]
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

/// List workspace instances accessible to the given role and user
/// within a specific store.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_workspaces_scoped`.
#[command]
pub async fn list_workspaces(
    state: State<'_, AppState>,
    role_id: String,
    user_id: Option<String>,
    store_id: String,
) -> Result<Vec<WorkspaceDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.list_workspaces(&role_id, user_id.as_deref(), &store_id)?;
    drop(db);
    Ok(rows)
}

/// Get a single workspace instance by ID.
///
/// **Deprecated for multi-store (ADR #7):** Use `get_workspace_instance_scoped`.
#[command]
pub async fn get_workspace_instance(
    state: State<'_, AppState>,
    instance_id: String,
    user_id: Option<String>,
) -> Result<WorkspaceDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let dto = store.get_workspace_instance(&instance_id, user_id.as_deref())?;
    drop(db);
    Ok(dto)
}

/// Create a new workspace instance (admin).
///
/// **Deprecated for multi-store (ADR #7):** Use `create_workspace_instance_scoped`.
#[command]
pub async fn create_workspace_instance(
    state: State<'_, AppState>,
    req: CreateInstanceRequest,
    caller_user_id: String,
) -> Result<WorkspaceDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &caller_user_id, permissions::STAFF_UPDATE)?;
    let _row = store.create_workspace_instance(
        &req.id,
        &req.type_key,
        &req.store_id,
        &req.name,
        req.description.as_deref().unwrap_or(""),
        req.colour.as_deref(),
    )?;
    let dto = store.get_workspace_instance(&req.id, Some(&caller_user_id))?;
    drop(db);
    tracing::info!(
        instance_id = %req.id,
        type_key = %req.type_key,
        store_id = %req.store_id,
        "workspace instance created"
    );
    Ok(dto)
}

// ── Legacy Commands (backward compatible) ────────────────────────────

/// List all workspace types.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_workspaces_scoped` instead.
#[command]
pub async fn list_workspace_types(
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceTypeDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
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

/// List ALL workspace types (for admin dropdowns).
///
/// **Deprecated for multi-store (ADR #7):** Use `list_workspaces_scoped` instead.
#[command]
pub async fn list_all_workspaces(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<WorkspaceTypeDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::STAFF_READ)?;
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

/// List all workspace types resolved from a session token. ADR #7.
#[command]
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
#[command]
pub async fn set_user_workspaces(
    state: State<'_, AppState>,
    user_id: String,
    workspace_keys: Vec<String>,
    caller_user_id: String,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &caller_user_id, permissions::STAFF_UPDATE)?;
    let keys: Vec<&str> = workspace_keys.iter().map(|s| s.as_str()).collect();
    store.set_user_workspaces_legacy(&user_id, keys)?;
    drop(db);
    tracing::info!(user_id = %user_id, count = %workspace_keys.len(), "user workspace assignments updated (legacy)");
    Ok(())
}

/// Replace all workspace assignments for a user (legacy tables), caller from session. ADR #7.
#[command]
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
#[command]
pub async fn get_user_workspaces(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<String>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::STAFF_READ)?;
    let keys = store.get_user_workspace_keys_legacy(&user_id)?;
    drop(db);
    Ok(keys)
}

/// Get workspace keys for a user (legacy table), caller from session. ADR #7.
#[command]
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

/// List screens (nav items) for a given workspace type.
///
/// **Deprecated for multi-store (ADR #7):** Use `list_workspace_screens_scoped`.
#[command]
pub async fn list_workspace_screens(
    state: State<'_, AppState>,
    type_key: String,
) -> Result<Vec<WorkspaceScreenDto>, AppError> {
    let db = state.db.lock().await;
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

/// Replace all instance assignments for a user (old command).
///
/// **Deprecated for multi-store (ADR #7):** Use `set_user_workspace_instances_scoped`.
#[command]
pub async fn set_user_workspace_instances(
    state: State<'_, AppState>,
    user_id: String,
    instance_ids: Vec<String>,
    default_instance_id: Option<String>,
    caller_user_id: String,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &caller_user_id, permissions::STAFF_UPDATE)?;
    let ids: Vec<&str> = instance_ids.iter().map(|s| s.as_str()).collect();
    store.set_user_workspace_instances(&user_id, ids, default_instance_id.as_deref())?;
    drop(db);
    tracing::info!(user_id = %user_id, count = %instance_ids.len(), "user workspace instance assignments updated");
    Ok(())
}

/// Get the explicit instance IDs assigned to a user (old command).
///
/// **Deprecated for multi-store (ADR #7):** Use `get_user_workspace_instances_scoped`.
#[command]
pub async fn get_user_workspace_instances(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<String>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::STAFF_READ)?;
    let ids = store.get_user_workspace_instance_ids(&user_id)?;
    drop(db);
    Ok(ids)
}

// ── Boot Resolution (ADR #4 Phase 3) ────────────────────────────────

/// DTO returned by `resolve_boot_store`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootResolution {
    pub is_bound: bool,
    pub store_id: String,
    pub instance_id: Option<String>,
}

/// Resolve the active store and instance from device binding.
///
/// This is called once at boot time (before authentication). It does not use
/// a session token because no user is logged in yet.
#[command]
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
                Some(secret) => {
                    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                        .map_err(|e| AppError::Internal(format!("HMAC init failed: {e}")))?;
                    mac.update(terminal_id.as_bytes());
                    mac.update(b":");
                    mac.update(bound_store_id.as_bytes());
                    mac.update(b":");
                    mac.update(bound_instance_id.as_bytes());
                    hex::encode(mac.finalize().into_bytes()) == signature
                }
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
}
