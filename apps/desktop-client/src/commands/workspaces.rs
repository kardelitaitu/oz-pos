//! Tauri commands for workspace listing, navigation screens, and
//! per-user workspace assignment (admin feature).
//!
//! ADR #4 Phase 1: Now returns `WorkspaceDto` with instance-aware fields
//! and supports instance CRUD. Legacy commands are preserved for
//! backward compatibility and marked as deprecated.

use serde::Serialize;
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::db::workspaces::WorkspaceDto;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

/// Legacy workspace DTO (pre-ADR #4).
/// Kept for backward compatibility with existing frontend code.
#[deprecated(
    since = "0.0.4",
    note = "Use WorkspaceDto from oz_core::db::workspaces instead"
)]
#[derive(Debug, Serialize)]
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

// ── New Commands (ADR #4 Phase 1) ────────────────────────────────────

/// List workspace instances accessible to the given role and user
/// within a specific store.
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
/// When `user_id` is provided, `is_default` reflects whether this
/// instance is the user's default.
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
/// Requires `staff:update` permission.
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

/// List all workspace types (the old `list_workspaces`).
/// Deprecated — use `list_workspaces` with `store_id` instead.
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
/// Requires `staff:read` permission.
/// Deprecated — use `list_workspace_types`.
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

/// Replace all workspace assignments for a user (legacy tables).
/// Requires `staff:update` permission.
/// Deprecated — use `set_user_workspace_instances` with instance IDs.
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

/// Get the explicit workspace keys assigned to a user (legacy table).
/// Requires `staff:read` permission.
/// Deprecated — use `get_user_workspace_instance_ids`.
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

/// List screens (nav items) for a given workspace type.
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

// ── New Instance Assignment Commands ─────────────────────────────────

/// Replace all instance assignments for a user.
/// Passing empty `instance_ids` clears all assignments.
/// Requires `staff:update` permission.
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

/// Get the explicit instance IDs assigned to a user.
/// Requires `staff:read` permission.
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
