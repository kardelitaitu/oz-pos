//! Staff management commands — list, create, update staff members and roles.
//!
//! These commands are the IPC surface for the Staff Management UI.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::auth::hash_pin;
use oz_core::db::Store;
use oz_core::{Role, User};

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
}

#[command]
pub async fn create_staff(
    args: CreateStaffArgs,
    state: State<'_, AppState>,
) -> Result<StaffMemberDto, AppError> {
    let username = args.username.trim().to_lowercase();
    let display_name = args.display_name.trim();

    if username.is_empty() {
        return Err(AppError::Invalid("username must not be empty".into()));
    }
    if display_name.is_empty() {
        return Err(AppError::Invalid("display name must not be empty".into()));
    }
    if args.pin.len() < 4 {
        return Err(AppError::Invalid(
            "PIN must be at least 4 characters".into(),
        ));
    }
    if args.role_id.is_empty() {
        return Err(AppError::Invalid("role must be selected".into()));
    }

    let pin_hash =
        hash_pin(&args.pin).map_err(|e| AppError::Internal(format!("hashing PIN: {e}")))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

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
}

#[command]
pub async fn update_staff(
    args: UpdateStaffArgs,
    state: State<'_, AppState>,
) -> Result<StaffMemberDto, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);

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
