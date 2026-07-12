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
    /// Unique identifier.
    pub id: String,
    /// Username.
    pub username: String,
    /// Display Name.
    pub display_name: String,
    /// ID of the associated role.
    pub role_id: String,
    /// Role Name.
    pub role_name: String,
    /// Whether this is active.
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
/// List staff.
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
/// Roledto.
pub struct RoleDto {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
}

#[command]
/// List roles.
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
/// Createstaffargs.
pub struct CreateStaffArgs {
    /// Username.
    pub username: String,
    /// Pin.
    pub pin: String,
    /// Display Name.
    pub display_name: String,
    /// ID of the associated role.
    pub role_id: String,
    /// User ID of the caller (from `LoginSession`). Used for permission check.
    pub caller_user_id: String,
}

#[command]
/// Create staff.
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
/// Updatestaffargs.
pub struct UpdateStaffArgs {
    /// Unique identifier.
    pub id: String,
    /// Username.
    pub username: String,
    /// Display Name.
    pub display_name: String,
    /// ID of the associated role.
    pub role_id: String,
    /// Whether this is active.
    pub is_active: bool,
    /// User ID of the caller (from `LoginSession`). Used for permission check.
    pub caller_user_id: String,
}

#[command]
/// Update staff.
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

// ── Bootstrap first owner (no authentication required) ────────────────

#[derive(Debug, Deserialize)]
/// Bootstrapownerargs.
pub struct BootstrapOwnerArgs {
    /// Username for the first owner account.
    pub username: String,
    /// Plain-text PIN (minimum 4 characters).
    pub pin: String,
    /// Display name for the first owner.
    pub display_name: String,
}

/// Result of a successful owner bootstrap — returns a login session
/// so the front-end can auto-login immediately.
#[derive(Debug, Serialize)]
pub struct BootstrapOwnerResult {
    /// LoginSession dto.
    pub session: oz_core::auth::LoginSession,
}

/// Create the first owner user in a fresh installation.
///
/// This is the only command that does NOT require an existing session,
/// because there are no users yet. It seeds the default roles first,
/// then creates a user with the `role-owner` role.
///
/// # Errors
///
/// Returns `Conflict` if any users already exist, preventing accidental
/// re-bootstrapping after staff accounts have been created.
/// Returns `Invalid` if validation fails (empty username, short PIN, etc.).
#[command]
pub async fn bootstrap_owner(
    args: BootstrapOwnerArgs,
    state: State<'_, AppState>,
) -> Result<BootstrapOwnerResult, AppError> {
    let username = args.username.trim().to_lowercase();
    let display_name = args.display_name.trim();

    validate_not_empty("username", &username).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("display_name", display_name)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_min_length("pin", &args.pin, 4).map_err(|e| AppError::Invalid(e.to_string()))?;

    let pin_hash =
        hash_pin(&args.pin).map_err(|e| AppError::Internal(format!("hashing PIN: {e}")))?;

    let db = state.db.lock().await;
    let store = Store::new(&db);

    // Guard: refuse to bootstrap if users already exist.
    let existing = store.list_users()?;
    if !existing.is_empty() {
        return Err(AppError::Invalid(
            "cannot bootstrap: staff accounts already exist".into(),
        ));
    }

    // Seed roles first so role-owner exists.
    store.seed_default_roles()?;

    let user = store.create_user(
        &username,
        &pin_hash,
        display_name,
        oz_core::builtin_roles::OWNER,
    )?;
    let role = store
        .get_role(oz_core::builtin_roles::OWNER)?
        .ok_or_else(|| AppError::Internal("owner role not found after seeding".into()))?;
    drop(db);

    tracing::info!(username = %username, "owner account bootstrapped");

    Ok(BootstrapOwnerResult {
        session: oz_core::auth::LoginSession {
            user_id: user.id,
            display_name: user.display_name,
            role_name: role.name,
            role_id: role.id,
        },
    })
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── StaffMemberDto ──────────────────────────────────────────────────

    #[test]
    fn staff_member_dto_debug() {
        let dto = StaffMemberDto {
            id: "u1".into(),
            username: "jdoe".into(),
            display_name: "John Doe".into(),
            role_id: "r1".into(),
            role_name: "Manager".into(),
            is_active: true,
        };
        let d = format!("{dto:?}");
        assert!(d.contains("jdoe"));
        assert!(d.contains("Manager"));
    }

    #[test]
    fn staff_member_dto_serialize() {
        let dto = StaffMemberDto {
            id: "u2".into(),
            username: "asmith".into(),
            display_name: "Alice Smith".into(),
            role_id: "r2".into(),
            role_name: "Cashier".into(),
            is_active: false,
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["username"], "asmith");
        assert_eq!(json["is_active"], false);
    }

    // ── RoleDto ─────────────────────────────────────────────────────────

    #[test]
    fn role_dto_debug() {
        let dto = RoleDto {
            id: "r1".into(),
            name: "Admin".into(),
            description: "Full access".into(),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("Admin"));
    }

    #[test]
    fn role_dto_serialize() {
        let dto = RoleDto {
            id: "r2".into(),
            name: "Viewer".into(),
            description: String::new(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["name"], "Viewer");
        assert_eq!(json["description"], "");
    }

    // ── CreateStaffArgs ─────────────────────────────────────────────────

    #[test]
    fn create_staff_args_deserialize() {
        let json = r##"{"username":"jdoe","pin":"1234","display_name":"John Doe","role_id":"r1","caller_user_id":"admin1"}"##;
        let args: CreateStaffArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.username, "jdoe");
        assert_eq!(args.role_id, "r1");
    }

    #[test]
    fn create_staff_args_debug() {
        let args = CreateStaffArgs {
            username: "u".into(),
            pin: "0000".into(),
            display_name: "D".into(),
            role_id: "r".into(),
            caller_user_id: "c".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("u"));
        assert!(d.contains("r"));
    }

    // ── UpdateStaffArgs ─────────────────────────────────────────────────

    #[test]
    fn update_staff_args_deserialize() {
        let json = r##"{"id":"u1","username":"jdoe2","display_name":"John D","role_id":"r2","is_active":false,"caller_user_id":"admin1"}"##;
        let args: UpdateStaffArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.id, "u1");
        assert!(!args.is_active);
    }

    #[test]
    fn update_staff_args_debug() {
        let args = UpdateStaffArgs {
            id: "x".into(),
            username: "y".into(),
            display_name: "z".into(),
            role_id: "r".into(),
            is_active: true,
            caller_user_id: "c".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("z"));
    }

    // ── BootstrapOwnerArgs ──────────────────────────────────────────────

    #[test]
    fn bootstrap_owner_args_deserialize() {
        let json = r##"{"username":"owner1","pin":"1234","display_name":"Store Owner"}"##;
        let args: BootstrapOwnerArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.username, "owner1");
        assert_eq!(args.pin, "1234");
        assert_eq!(args.display_name, "Store Owner");
    }

    #[test]
    fn bootstrap_owner_args_debug() {
        let args = BootstrapOwnerArgs {
            username: "adm".into(),
            pin: "0000".into(),
            display_name: "Admin".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("adm"));
        assert!(d.contains("Admin"));
    }

    #[test]
    fn bootstrap_owner_result_serialize() {
        let result = BootstrapOwnerResult {
            session: oz_core::auth::LoginSession {
                user_id: "u1".into(),
                display_name: "Owner".into(),
                role_name: "Owner".into(),
                role_id: "role-owner".into(),
            },
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["session"]["user_id"], "u1");
        assert_eq!(json["session"]["role_name"], "Owner");
    }

    #[test]
    fn bootstrap_owner_result_debug() {
        let result = BootstrapOwnerResult {
            session: oz_core::auth::LoginSession {
                user_id: "u2".into(),
                display_name: "Alice".into(),
                role_name: "Owner".into(),
                role_id: "role-owner".into(),
            },
        };
        let d = format!("{result:?}");
        assert!(d.contains("Alice"));
    }
}
