//! Staff management commands — list, create, update staff members and roles.
//!
//! These commands are the IPC surface for the Staff Management UI.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::auth::hash_pin;
use oz_core::db::Store;
use oz_core::permissions;
use oz_core::{Role, User};

use foundation::{validate_min_length, validate_not_empty};

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

// ── Staff member DTO ────────────────────────────────────────────────

/// Staff member as seen by the front-end (no pin_hash exposed).
#[derive(Debug, Serialize)]
pub struct StaffMemberDto {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role_id: String,
    pub role_name: String,
    pub is_active: bool,
}

fn to_staff_dto(user: &User, roles: &[Role]) -> StaffMemberDto {
    let role_name = roles
        .iter()
        .find(|r| r.id == user.role_id)
        .map(|r| r.name.clone())
        .unwrap_or_default();
    StaffMemberDto {
        id: user.id.clone(),
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        role_id: user.role_id.clone(),
        role_name,
        is_active: user.is_active,
    }
}

// ── List staff ─────────────────────────────────────────────────────

#[command]
pub async fn list_staff(state: State<'_, AppState>) -> Result<Vec<StaffMemberDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let users = store.list_users()?;
    let roles = store.list_roles()?;
    drop(db);
    Ok(users.iter().map(|u| to_staff_dto(u, &roles)).collect())
}

// ── List roles ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RoleDto {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[command]
pub async fn list_roles(state: State<'_, AppState>) -> Result<Vec<RoleDto>, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let roles = store.list_roles()?;
    drop(db);
    Ok(roles
        .into_iter()
        .map(|r| RoleDto {
            id: r.id,
            name: r.name,
            description: r.description,
        })
        .collect())
}

// ── Create staff member ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateStaffArgs {
    pub username: String,
    pub pin: String,
    pub display_name: String,
    pub role_id: String,
    /// User ID of the caller (from `LoginSession`). Used for permission check.
    pub caller_user_id: String,
}

#[command]
pub async fn create_staff(
    args: CreateStaffArgs,
    state: State<'_, AppState>,
) -> Result<StaffMemberDto, AppError> {
    let username = args.username.trim().to_lowercase();
    let display_name = args.display_name.trim();

    validate_not_empty("username", &username).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("display_name", display_name)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_min_length("pin", &args.pin, 4).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("role_id", &args.role_id).map_err(|e| AppError::Invalid(e.to_string()))?;

    let pin_hash =
        hash_pin(&args.pin).map_err(|e| AppError::Internal(format!("hashing PIN: {e}")))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Permission check: caller must have staff:create.
    require_permission_for_user(&store, &args.caller_user_id, permissions::STAFF_CREATE)?;

    let user = store.create_user(&username, &pin_hash, display_name, &args.role_id)?;
    let roles = store.list_roles()?;
    drop(db);

    Ok(to_staff_dto(&user, &roles))
}

// ── Update staff member ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateStaffArgs {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role_id: String,
    pub is_active: bool,
    /// User ID of the caller (from `LoginSession`). Used for permission check.
    pub caller_user_id: String,
}

#[command]
pub async fn update_staff(
    args: UpdateStaffArgs,
    state: State<'_, AppState>,
) -> Result<StaffMemberDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Permission check: caller must have staff:update.
    require_permission_for_user(&store, &args.caller_user_id, permissions::STAFF_UPDATE)?;

    let user = store.update_user(
        &args.id,
        &args.username,
        &args.display_name,
        &args.role_id,
        args.is_active,
    )?;
    let roles = store.list_roles()?;
    drop(db);

    Ok(to_staff_dto(&user, &roles))
}
