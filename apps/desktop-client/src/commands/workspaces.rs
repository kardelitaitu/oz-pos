//! Tauri commands for workspace listing, navigation screens, and
//! per-user workspace assignment (admin feature).

use serde::Serialize;
use tauri::{State, command};

use oz_core::db::Store;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

/// Workspace as seen by the front-end.
#[derive(Debug, Serialize)]
pub struct WorkspaceDto {
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

/// List workspaces accessible to the given role and user.
///
/// When `user_id` is provided and the user has explicit workspace
/// assignments, those replace the role-level defaults.
#[command]
pub async fn list_workspaces(
    state: State<'_, AppState>,
    role_id: String,
    user_id: Option<String>,
) -> Result<Vec<WorkspaceDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.list_workspaces(&role_id, user_id.as_deref())?;
    drop(db);
    Ok(rows
        .into_iter()
        .map(|r| WorkspaceDto {
            key: r.key,
            name: r.name,
            description: r.description,
            icon: r.icon,
        })
        .collect())
}

/// List ALL workspaces in the system (for admin dropdowns).
///
/// Requires `staff:read` permission.
#[command]
pub async fn list_all_workspaces(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<WorkspaceDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::STAFF_READ)?;
    let rows = store.list_all_workspaces()?;
    drop(db);
    Ok(rows
        .into_iter()
        .map(|r| WorkspaceDto {
            key: r.key,
            name: r.name,
            description: r.description,
            icon: r.icon,
        })
        .collect())
}

/// Replace all workspace assignments for a user.
///
/// Passing an empty list clears all assignments (user falls back to
/// role defaults). Requires `staff:update` permission.
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
    store.set_user_workspaces(&user_id, keys)?;
    drop(db);
    tracing::info!(user_id = %user_id, count = %workspace_keys.len(), "user workspace assignments updated");
    Ok(())
}

/// Get the explicit workspace keys assigned to a user.
///
/// Returns an empty list when the user has no custom assignments
/// (they use role-based defaults). Requires `staff:read` permission.
#[command]
pub async fn get_user_workspaces(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<String>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, permissions::STAFF_READ)?;
    let keys = store.get_user_workspace_keys(&user_id)?;
    drop(db);
    Ok(keys)
}

/// List screens (nav items) for a given workspace.
#[command]
pub async fn list_workspace_screens(
    state: State<'_, AppState>,
    workspace_key: String,
) -> Result<Vec<WorkspaceScreenDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let rows = store.list_workspace_screens(&workspace_key)?;
    drop(db);
    Ok(rows
        .into_iter()
        .map(|r| WorkspaceScreenDto {
            screen_key: r.screen_key,
            sort_order: r.sort_order,
        })
        .collect())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── WorkspaceDto ────────────────────────────────────────────────────

    #[test]
    fn workspace_dto_debug() {
        let dto = WorkspaceDto {
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
    fn workspace_dto_serialize() {
        let dto = WorkspaceDto {
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
}
