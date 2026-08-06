//! Staff management commands — list, create, update staff members and roles.
//!
//! These commands are the IPC surface for the Staff Management UI.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::auth::hash_pin;
use oz_core::db::Store;
use oz_core::permissions;
use oz_core::{Role, User};

use foundation::{validate_min_length, validate_not_empty};

use crate::commands::authz::require_permission_for_user;
use crate::commands::picker_ticket;
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

/// List staff.
///
/// **Deprecated for multi-store (ADR #7):** Use [`list_staff_scoped`] so the
/// caller identity is resolved from the session token instead of a
/// client-supplied `caller_user_id`.
#[tauri::command]
pub async fn list_staff(_state: State<'_, AppState>) -> Result<Vec<StaffMemberDto>, AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped staff commands are disabled; use list_staff_scoped".into(),
    ))
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

#[tauri::command]
/// List roles.
///
/// **Deprecated for multi-store (ADR #7):** Use [`list_roles_scoped`].
pub async fn list_roles(_state: State<'_, AppState>) -> Result<Vec<RoleDto>, AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped staff commands are disabled; use list_roles_scoped".into(),
    ))
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

#[tauri::command]
/// Create staff.
///
/// **Deprecated for multi-store (ADR #7):** Use [`create_staff_scoped`]. The
/// legacy `caller_user_id` argument is forgeable — never call this from a
/// session-bound UI path.
pub async fn create_staff(
    _args: CreateStaffArgs,
    _state: State<'_, AppState>,
) -> Result<StaffMemberDto, AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped staff commands are disabled; use create_staff_scoped".into(),
    ))
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

#[tauri::command]
/// Update staff.
///
/// **Deprecated for multi-store (ADR #7):** Use [`update_staff_scoped`]. The
/// legacy `caller_user_id` argument is forgeable — never call this from a
/// session-bound UI path.
pub async fn update_staff(
    _args: UpdateStaffArgs,
    _state: State<'_, AppState>,
) -> Result<StaffMemberDto, AppError> {
    Err(AppError::PermissionDenied(
        "legacy unscoped staff commands are disabled; use update_staff_scoped".into(),
    ))
}

// ── Session-scoped staff commands (ADR #7 · audit/06 STAFF-01) ────────
//
// These are the replacement for the legacy staff commands. They resolve the
// caller identity from the opaque `session_token` and NEVER accept a
// caller-supplied `caller_user_id`. Users/roles are GLOBAL identity records
// (ADR #4 / ADR #7); the store-scoped DBs contain no users, so the
// permission check and the CRUD both run against the global identity DB.

/// Arguments for creating a staff member from a session token.
///
/// Deliberately carries NO caller identity field — the caller is resolved
/// from the session token by the command.
#[derive(Debug, Deserialize)]
/// Createstaffscopedargs.
pub struct CreateStaffScopedArgs {
    /// Username.
    pub username: String,
    /// Pin.
    pub pin: String,
    /// Display Name.
    pub display_name: String,
    /// ID of the associated role.
    pub role_id: String,
}

/// Arguments for updating a staff member from a session token.
///
/// Deliberately carries NO caller identity field — the caller is resolved
/// from the session token by the command.
#[derive(Debug, Deserialize)]
/// Updatestaffscopedargs.
pub struct UpdateStaffScopedArgs {
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
    /// Optional new PIN (STAFF-03). When `Some(non-empty)` the PIN is
    /// validated, hashed server-side, and persisted via `update_user_pin`.
    /// `None`/empty leaves the current PIN unchanged.
    pub pin: Option<String>,
    /// Optional workspace key assignment (STAFF-05). When `Some`, the
    /// profile update and the workspace assignment are applied by this one
    /// command — the front-end no longer issues two separate IPC calls. If
    /// the store-scoped workspace write fails after the profile commits,
    /// the profile change is rolled back (compensating update) and a clear
    /// partial-failure error is returned.
    #[serde(default)]
    pub workspace_keys: Option<Vec<String>>,
}

/// List staff members. Caller identity is resolved from the session token.
#[tauri::command]
pub async fn list_staff_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<StaffMemberDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_READ)?;
    let users = store.list_users()?;
    let roles = store.list_roles()?;
    drop(db);
    Ok(users.iter().map(|u| to_staff_dto(u, &roles)).collect())
}

/// List roles. Caller identity is resolved from the session token.
#[tauri::command]
pub async fn list_roles_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<Vec<RoleDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_READ)?;
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

/// Create a staff member. Caller identity is resolved from the session token.
///
/// STAFF-02: enforces the role-assignment hierarchy (only Owner-level
/// callers may create an Owner account).
#[tauri::command]
pub async fn create_staff_scoped(
    session_token: String,
    args: CreateStaffScopedArgs,
    state: State<'_, AppState>,
) -> Result<StaffMemberDto, AppError> {
    let session = state.resolve_session(&session_token)?;
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
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_CREATE)?;
    enforce_role_assignment_policy(&store, &session.user_id, None, &args.role_id, true)?;
    let user = store.create_user(&username, &pin_hash, display_name, &args.role_id)?;
    let roles = store.list_roles()?;
    drop(db);

    Ok(to_staff_dto(&user, &roles))
}

/// Enforce role-assignment policy (STAFF-02).
///
/// - Only a caller with `staff:manage_roles` (i.e. the Owner preset, which
///   carries `*`) may create or promote an account to the Owner role.
/// - A caller may not change their own role (no self-promotion).
/// - The last active Owner may not be deactivated, demoted, or edited away.
fn enforce_role_assignment_policy(
    store: &Store<'_>,
    caller_user_id: &str,
    target_user_id: Option<&str>,
    target_role_id: &str,
    target_is_active: bool,
) -> Result<(), AppError> {
    // Only Owner-level roles may assign the Owner role.
    if target_role_id == oz_core::builtin_roles::OWNER {
        require_permission_for_user(store, caller_user_id, permissions::STAFF_MANAGE_ROLES)?;
    }

    if let Some(target_id) = target_user_id {
        // No self-promotion / self-deactivation: a user cannot change their
        // own role and cannot deactivate their own account (STAFF-10).
        if target_id == caller_user_id {
            let caller = store
                .get_user(caller_user_id)?
                .ok_or_else(|| AppError::PermissionDenied("user not found".into()))?;
            if caller.role_id != target_role_id {
                return Err(AppError::PermissionDenied(
                    "you cannot change your own role".into(),
                ));
            }
            if !target_is_active {
                return Err(AppError::PermissionDenied(
                    "you cannot deactivate your own account".into(),
                ));
            }
        }

        // Last-owner protection: cannot deactivate/demote the last active Owner.
        if let Some(target) = store.get_user(target_id)?
            && target.role_id == oz_core::builtin_roles::OWNER
            && (target_role_id != oz_core::builtin_roles::OWNER || !target_is_active)
        {
            let active_owners = store
                .list_users()?
                .iter()
                .filter(|u| u.role_id == oz_core::builtin_roles::OWNER && u.is_active)
                .count();
            if active_owners <= 1 {
                return Err(AppError::PermissionDenied(
                    "cannot deactivate or demote the last active Owner".into(),
                ));
            }
        }
    }

    Ok(())
}

/// Update a staff member. Caller identity is resolved from the session token.
///
/// STAFF-02: enforces the role-assignment hierarchy (Owner-only promotion,
/// no self-promotion, last-owner protection).
/// STAFF-03: optionally rotates the PIN when `args.pin` is a non-empty value.
/// STAFF-05: the profile update and (optional) workspace assignment run as
/// one command. The profile lives in the GLOBAL identity DB while workspace
/// assignments live in the STORE-scoped DB, so a single SQLite transaction
/// across both is impossible; instead we apply the profile first and, if the
/// store-scoped workspace write then fails, compensate by restoring the
/// previous profile values and returning a clear partial-failure error.
#[tauri::command]
pub async fn update_staff_scoped(
    session_token: String,
    args: UpdateStaffScopedArgs,
    state: State<'_, AppState>,
) -> Result<StaffMemberDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;

    // Permission + role-hierarchy checks run against the global identity DB.
    // The `Store` borrows the (non-Sync) `Connection`, so it must be scoped in
    // a block and dropped BEFORE any further `.await` — otherwise the command
    // future is not `Send` and Tauri rejects it at compile time.
    {
        let store = Store::new(&db);
        require_permission_for_user(&store, &session.user_id, permissions::STAFF_UPDATE)?;
        enforce_role_assignment_policy(
            &store,
            &session.user_id,
            Some(&args.id),
            &args.role_id,
            args.is_active,
        )?;
    }

    // STAFF-05 compensation: snapshot the profile BEFORE the update so we can
    // restore it if the store-scoped workspace write fails afterwards.
    let previous_profile = {
        let store = Store::new(&db);
        store.get_user(&args.id)?.map(|u| {
            (
                u.username,
                u.display_name,
                u.role_id,
                u.is_active,
                u.pin_hash,
            )
        })
    };

    // STAFF-03: profile + PIN rotate atomically inside one transaction so a
    // failed PIN hash never leaves the profile half-updated (STAFF-05). The
    // transaction also borrows the non-Sync Connection, so it stays scoped in
    // its own block too.
    let (user, roles, pin_rotated) = {
        let tx = db.unchecked_transaction()?;
        let store = Store::new(&tx);
        store.update_user(
            &args.id,
            &args.username,
            &args.display_name,
            &args.role_id,
            args.is_active,
        )?;

        // Hash server-side; never accept plaintext beyond the command boundary.
        let pin_rotated = if let Some(pin) = args.pin.as_deref().filter(|p| !p.is_empty()) {
            validate_min_length("pin", pin, 4).map_err(|e| AppError::Invalid(e.to_string()))?;
            let pin_hash =
                hash_pin(pin).map_err(|e| AppError::Internal(format!("hashing PIN: {e}")))?;
            store.update_user_pin(&args.id, &pin_hash)?;
            // A successful rotation also clears any accumulated failed-login
            // lockout for this account (atomic with the rotation).
            store.clear_login_attempts(&args.username.trim().to_lowercase())?;
            true
        } else {
            false
        };

        let user = store
            .get_user(&args.id)?
            .ok_or_else(|| AppError::Internal(format!("updated user {} vanished", args.id)))?;
        let roles = store.list_roles()?;
        tx.commit()?;
        (user, roles, pin_rotated)
    };
    drop(db);

    // STAFF-05: apply the workspace assignment as part of this same command.
    // If it fails, compensate by restoring the previous profile and surface a
    // clear partial-failure error instead of leaving a half-updated account.
    if let Some(keys) = &args.workspace_keys {
        // Keep database-open and mutex failures inside the same compensation
        // path as assignment failures. Returning early here would otherwise
        // leave the committed profile update without reporting or attempting
        // a rollback (STAFF-05).
        let result: Result<(), String> = match state.db_manager.open_store(&session.store_id) {
            Err(error) => Err(format!("opening store db: {error}")),
            Ok(conn) => match conn.lock() {
                Err(error) => Err(format!("store db lock: {error}")),
                Ok(sdb) => {
                    let store = Store::new(&sdb);
                    let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
                    store
                        .set_user_workspaces_legacy(&args.id, key_refs)
                        .map_err(|error| error.to_string())
                }
            },
        };
        if let Err(e) = result {
            // Compensate: roll the profile back to its previous values.
            // Do not use `?` while compensating: a rollback failure must be
            // reported together with the original workspace error, otherwise
            // operators cannot tell that the account may still be inconsistent.
            let rollback_result: Result<(), String> =
                if let Some((username, display_name, role_id, is_active, pin_hash)) =
                    &previous_profile
                {
                    let db = state.db.lock().await;
                    match db.unchecked_transaction() {
                        Ok(tx) => {
                            let store = Store::new(&tx);
                            match store.update_user(
                                &args.id,
                                username,
                                display_name,
                                role_id,
                                *is_active,
                            ) {
                                Ok(_) => match store.update_user_pin(&args.id, pin_hash) {
                                    Ok(_) => tx.commit().map_err(|error| error.to_string()),
                                    Err(error) => Err(error.to_string()),
                                },
                                Err(error) => Err(error.to_string()),
                            }
                        }
                        Err(error) => Err(error.to_string()),
                    }
                } else {
                    Err(format!(
                        "staff profile {} was not found before update",
                        args.id
                    ))
                };
            let rollback_detail = match rollback_result {
                Ok(_) => "profile rollback succeeded".to_owned(),
                Err(rollback_error) => format!("profile rollback failed: {rollback_error}"),
            };
            return Err(AppError::Internal(format!(
                "profile updated but workspace assignment failed: {e}; {rollback_detail}"
            )));
        }
    }

    if pin_rotated {
        // STAFF-03: a rotated PIN invalidates every OTHER session issued
        // under the old PIN. The caller's own session is preserved — they
        // authenticated moments ago and the UI reloads with the same token.
        state.invalidate_user_sessions_except(&args.id, &session_token);
    }

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
    /// Short-lived picker ticket (audit/06 residual).
    ///
    /// The pre-session `list_workspaces` / `list_workspace_screens`
    /// commands verify this ticket and resolve the caller's REAL role
    /// from the database — caller-supplied `role_id` / `user_id` are
    /// never trusted for the workspace picker.
    pub picker_ticket: String,
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
#[tauri::command]
pub async fn bootstrap_owner(
    args: BootstrapOwnerArgs,
    state: State<'_, AppState>,
) -> Result<BootstrapOwnerResult, AppError> {
    let db = state.db.lock().await;
    let mut result = run_bootstrap_owner(&db, &args)?;
    drop(db);

    // Mint the short-lived picker ticket bound to the new owner. It is
    // only valid for the pre-session workspace picker; `create_session`
    // hands out the opaque session token afterwards.
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    result.picker_ticket = picker_ticket::sign_picker_ticket(
        &state.picker_ticket_secret,
        &result.session.user_id,
        now_ts + picker_ticket::PICKER_TICKET_TTL_SECS,
    );
    Ok(result)
}

/// Business logic for `bootstrap_owner` (extracted for testing).
fn run_bootstrap_owner(
    conn: &rusqlite::Connection,
    args: &BootstrapOwnerArgs,
) -> Result<BootstrapOwnerResult, AppError> {
    let username = args.username.trim().to_lowercase();
    let display_name = args.display_name.trim();

    validate_not_empty("username", &username).map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_not_empty("display_name", display_name)
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    validate_min_length("pin", &args.pin, 4).map_err(|e| AppError::Invalid(e.to_string()))?;

    let pin_hash =
        hash_pin(&args.pin).map_err(|e| AppError::Internal(format!("hashing PIN: {e}")))?;

    let store = Store::new(conn);

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

    tracing::info!(username = %username, "owner account bootstrapped");

    Ok(BootstrapOwnerResult {
        session: oz_core::auth::LoginSession {
            user_id: user.id,
            display_name: user.display_name,
            role_name: role.name,
            role_id: role.id,
        },
        // The command wrapper attaches the picker ticket after the pure
        // function returns (it needs the per-process secret).
        picker_ticket: String::new(),
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

    // ── STAFF-01 / STAFF-04 — session-scoped authorization (audit/06) ───
    //
    // TDD red: these tests pin the NEW scoped-command contract. They fail to
    // compile until `list_staff_scoped` / `list_roles_scoped` /
    // `create_staff_scoped` / `update_staff_scoped` and their arg structs
    // (which carry NO caller-supplied identity) exist.

    use oz_core::session::SessionContext;
    use platform_core::StoreDatabaseManager;
    use tauri::Manager as _;

    /// Seed the GLOBAL identity DB with an owner (all permissions) and a
    /// cashier (no staff permissions). Users/roles are global records
    /// (ADR #4 / ADR #7); store-scoped DBs contain no users.
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

    fn scoped_state_with_token(
        conn: rusqlite::Connection,
        token: &str,
        user_id: &str,
        role_id: &str,
        store_id: &str,
    ) -> AppState {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        state.session_store.write().unwrap().insert(
            token.into(),
            SessionContext::new(
                user_id.into(),
                role_id.into(),
                "terminal-1".into(),
                store_id.into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
        state
    }

    // ── STAFF-01 — legacy command trusts client-supplied caller ID ────

    #[tokio::test]
    async fn legacy_create_staff_accepts_forged_caller_user_id() {
        // The legacy command must reject caller-supplied identity rather than
        // allowing the STAFF-01 forged-caller vulnerability.
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state = AppState::for_test_with_conn(conn);
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = create_staff(
            CreateStaffArgs {
                username: "mallory".into(),
                pin: "1234".into(),
                display_name: "Mallory".into(),
                role_id: "role-cashier".into(),
                caller_user_id: "user-owner".into(), // forged
            },
            app.state(),
        )
        .await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn legacy_update_staff_accepts_forged_caller_user_id() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state = AppState::for_test_with_conn(conn);
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = update_staff(
            UpdateStaffArgs {
                id: "user-cashier".into(),
                username: "cashier".into(),
                display_name: "Cashier Updated".into(),
                role_id: "role-owner".into(), // privilege escalation via forged id
                is_active: true,
                caller_user_id: "user-owner".into(), // forged
            },
            app.state(),
        )
        .await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    // ── STAFF-01 fix — scoped commands bind identity to the session ────

    #[tokio::test]
    async fn scoped_create_staff_rejects_invalid_session() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state = AppState::for_test_with_conn(conn);
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = create_staff_scoped(
            "missing-token".into(),
            CreateStaffScopedArgs {
                username: "mallory".into(),
                pin: "1234".into(),
                display_name: "Mallory".into(),
                role_id: "role-cashier".into(),
            },
            app.state(),
        )
        .await;
        assert!(matches!(result, Err(AppError::InvalidSession)));
    }

    #[tokio::test]
    async fn scoped_create_staff_denies_cashier_session() {
        // The caller identity is bound to the session token. A cashier
        // session (no staff:create) must be denied — there is no request
        // field left to forge.
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state = scoped_state_with_token(
            conn,
            "cashier-token",
            "user-cashier",
            "role-cashier",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = create_staff_scoped(
            "cashier-token".into(),
            CreateStaffScopedArgs {
                username: "mallory".into(),
                pin: "1234".into(),
                display_name: "Mallory".into(),
                role_id: "role-cashier".into(),
            },
            app.state(),
        )
        .await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn scoped_create_staff_allows_owner_session() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = create_staff_scoped(
            "owner-token".into(),
            CreateStaffScopedArgs {
                username: "mallory".into(),
                pin: "1234".into(),
                display_name: "Mallory".into(),
                role_id: "role-cashier".into(),
            },
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(result.username, "mallory");
        assert_eq!(result.role_name, "Cashier");
    }

    #[tokio::test]
    async fn scoped_update_staff_denies_cashier_session() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state = scoped_state_with_token(
            conn,
            "cashier-token",
            "user-cashier",
            "role-cashier",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = update_staff_scoped(
            "cashier-token".into(),
            UpdateStaffScopedArgs {
                id: "user-owner".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                role_id: "role-owner".into(),
                is_active: true,
                pin: None,
                workspace_keys: None,
            },
            app.state(),
        )
        .await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    // ── STAFF-02 — role hierarchy ─────────────────────────────────────

    #[tokio::test]
    async fn scoped_create_staff_denies_cashier_creating_owner() {
        // Even though the cashier has no staff:create at all, the hierarchy
        // guard must also block a role that DOES have staff:create but not
        // staff:manage_roles (Manager/Staff presets) from assigning Owner.
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute_batch(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-manager', 'manager', 'hash', 'Manager', 'role-manager', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
        )
        .unwrap();
        let state = scoped_state_with_token(
            conn,
            "manager-token",
            "user-manager",
            "role-manager",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = create_staff_scoped(
            "manager-token".into(),
            CreateStaffScopedArgs {
                username: "newowner".into(),
                pin: "1234".into(),
                display_name: "New Owner".into(),
                role_id: "role-owner".into(),
            },
            app.state(),
        )
        .await;
        assert!(
            matches!(result, Err(AppError::PermissionDenied(_))),
            "Manager must not create an Owner account"
        );
    }

    #[tokio::test]
    async fn scoped_update_staff_denies_manager_promoting_to_owner() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute_batch(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-manager', 'manager', 'hash', 'Manager', 'role-manager', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-cashier', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
        )
        .unwrap();
        let state = scoped_state_with_token(
            conn,
            "manager-token",
            "user-manager",
            "role-manager",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = update_staff_scoped(
            "manager-token".into(),
            UpdateStaffScopedArgs {
                id: "user-cashier".into(),
                username: "cashier".into(),
                display_name: "Cashier".into(),
                role_id: "role-owner".into(),
                is_active: true,
                pin: None,
                workspace_keys: None,
            },
            app.state(),
        )
        .await;
        assert!(
            matches!(result, Err(AppError::PermissionDenied(_))),
            "Manager must not promote a user to Owner"
        );
    }

    #[tokio::test]
    async fn scoped_update_staff_denies_self_promotion() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute_batch(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-manager', 'manager', 'hash', 'Manager', 'role-manager', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
        )
        .unwrap();
        let state = scoped_state_with_token(
            conn,
            "manager-token",
            "user-manager",
            "role-manager",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // Manager edits their OWN role → denied (no self-promotion), even
        // though the assignment to role-manager is itself harmless.
        let result = update_staff_scoped(
            "manager-token".into(),
            UpdateStaffScopedArgs {
                id: "user-manager".into(),
                username: "manager".into(),
                display_name: "Manager".into(),
                role_id: "role-owner".into(),
                is_active: true,
                pin: None,
                workspace_keys: None,
            },
            app.state(),
        )
        .await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn scoped_update_staff_protects_last_active_owner() {
        let conn = oz_core::migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute_batch(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-cashier', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
        )
        .unwrap();
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // Owner is the only active owner → cannot demote or deactivate self.
        let result = update_staff_scoped(
            "owner-token".into(),
            UpdateStaffScopedArgs {
                id: "user-owner".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                role_id: "role-owner".into(),
                is_active: false,
                pin: None,
                workspace_keys: None,
            },
            app.state(),
        )
        .await;
        assert!(
            matches!(result, Err(AppError::PermissionDenied(_))),
            "last active Owner must not be deactivated"
        );
    }

    // ── STAFF-03 — PIN rotation ───────────────────────────────────────

    #[tokio::test]
    async fn scoped_update_staff_rotates_pin_when_provided() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = update_staff_scoped(
            "owner-token".into(),
            UpdateStaffScopedArgs {
                id: "user-cashier".into(),
                username: "cashier".into(),
                display_name: "Cashier".into(),
                role_id: "role-cashier".into(),
                is_active: true,
                pin: Some("9876".into()),
                workspace_keys: None,
            },
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(result.username, "cashier");

        // The PIN hash must have changed from the seeded 'hash'.
        let st = app.state::<AppState>();
        let db = st.db.lock().await;
        let user = Store::new(&db).get_user("user-cashier").unwrap().unwrap();
        assert_ne!(user.pin_hash, "hash");
    }

    #[tokio::test]
    async fn scoped_update_staff_pin_rotation_invalidates_sessions() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        // A stale session for the cashier whose PIN we rotate.
        state.session_store.write().unwrap().insert(
            "cashier-old-session".into(),
            SessionContext::new(
                "user-cashier".into(),
                "role-cashier".into(),
                "terminal-1".into(),
                "store-a".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        update_staff_scoped(
            "owner-token".into(),
            UpdateStaffScopedArgs {
                id: "user-cashier".into(),
                username: "cashier".into(),
                display_name: "Cashier".into(),
                role_id: "role-cashier".into(),
                is_active: true,
                pin: Some("9876".into()),
                workspace_keys: None,
            },
            app.state(),
        )
        .await
        .unwrap();

        // The old cashier session must be gone (invalidated by the rotation).
        let st = app.state::<AppState>();
        assert!(matches!(
            st.resolve_session("cashier-old-session"),
            Err(AppError::InvalidSession)
        ));
        // The owner session survives (different user).
        assert!(st.resolve_session("owner-token").is_ok());
    }

    #[tokio::test]
    async fn scoped_update_staff_self_rotation_preserves_callers_session() {
        // An Owner rotating their OWN PIN must keep their current session:
        // the UI immediately reloads with the same token after the update.
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        // Another terminal session for the same owner (issued under the old
        // PIN) SHOULD be invalidated.
        state.session_store.write().unwrap().insert(
            "owner-stale-terminal".into(),
            SessionContext::new(
                "user-owner".into(),
                "role-owner".into(),
                "terminal-2".into(),
                "store-a".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        update_staff_scoped(
            "owner-token".into(),
            UpdateStaffScopedArgs {
                id: "user-owner".into(),
                username: "owner".into(),
                display_name: "Owner".into(),
                role_id: "role-owner".into(),
                is_active: true,
                pin: Some("4321".into()),
                workspace_keys: None,
            },
            app.state(),
        )
        .await
        .unwrap();

        let st = app.state::<AppState>();
        // Current session survives so the UI can continue working.
        assert!(st.resolve_session("owner-token").is_ok());
        // Stale terminal session is gone.
        assert!(matches!(
            st.resolve_session("owner-stale-terminal"),
            Err(AppError::InvalidSession)
        ));
    }

    #[tokio::test]
    async fn scoped_update_staff_pin_rotation_clears_login_attempts() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        // Simulate an accumulated lockout for the cashier.
        let _ = Store::new(&conn).record_login_attempt("cashier", 3, 60);
        let _ = Store::new(&conn).record_login_attempt("cashier", 3, 60);
        let _ = Store::new(&conn).record_login_attempt("cashier", 3, 60);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        update_staff_scoped(
            "owner-token".into(),
            UpdateStaffScopedArgs {
                id: "user-cashier".into(),
                username: "cashier".into(),
                display_name: "Cashier".into(),
                role_id: "role-cashier".into(),
                is_active: true,
                pin: Some("9876".into()),
                workspace_keys: None,
            },
            app.state(),
        )
        .await
        .unwrap();

        // The lockout must be cleared — a fresh attempt should succeed.
        let st = app.state::<AppState>();
        let db = st.db.lock().await;
        let remaining = Store::new(&db)
            .record_login_attempt("cashier", 3, 60)
            .unwrap();
        assert!(remaining.is_ok(), "lockout should be cleared");
    }

    #[tokio::test]
    async fn scoped_update_staff_pin_rotation_never_touches_other_users_sessions() {
        // Isolation guard: rotating one user's PIN must only invalidate that
        // user's own stale sessions — never a different user's active session.
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        // A third user (manager) with an active session on another terminal.
        // DB row id is a generated UUID — the session below keys off
        // "user-manager" in the in-memory session store, which is what
        // resolve_session validates (mirrors the self-rotation test).
        Store::new(&conn)
            .create_user("manager", "hash", "Manager", "role-owner")
            .unwrap();
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        // Target user's stale terminal (issued under the old PIN).
        state.session_store.write().unwrap().insert(
            "cashier-stale-terminal".into(),
            SessionContext::new(
                "user-cashier".into(),
                "role-cashier".into(),
                "terminal-2".into(),
                "store-a".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
        // A DIFFERENT user's active session — must survive the rotation.
        state.session_store.write().unwrap().insert(
            "manager-token".into(),
            SessionContext::new(
                "user-manager".into(),
                "role-owner".into(),
                "terminal-3".into(),
                "store-a".into(),
                "instance-1".into(),
                "pos".into(),
                None,
                0,
            ),
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        update_staff_scoped(
            "owner-token".into(),
            UpdateStaffScopedArgs {
                id: "user-cashier".into(),
                username: "cashier".into(),
                display_name: "Cashier".into(),
                role_id: "role-cashier".into(),
                is_active: true,
                pin: Some("9876".into()),
                workspace_keys: None,
            },
            app.state(),
        )
        .await
        .unwrap();

        let st = app.state::<AppState>();
        // Target's stale session is gone.
        assert!(matches!(
            st.resolve_session("cashier-stale-terminal"),
            Err(AppError::InvalidSession)
        ));
        // Caller's session survives (UI reload path).
        assert!(st.resolve_session("owner-token").is_ok());
        // The other user's session is completely untouched.
        assert!(st.resolve_session("manager-token").is_ok());
    }

    #[tokio::test]
    async fn scoped_update_staff_rolls_back_profile_when_workspace_assignment_fails() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = update_staff_scoped(
            "owner-token".into(),
            UpdateStaffScopedArgs {
                id: "user-cashier".into(),
                username: "cashier-updated".into(),
                display_name: "Cashier Updated".into(),
                role_id: "role-cashier".into(),
                is_active: true,
                pin: None,
                // This key violates the workspace FK and forces the second
                // database write to fail after the profile transaction.
                workspace_keys: Some(vec!["missing-workspace".into()]),
            },
            app.state(),
        )
        .await;

        let error = result.expect_err("invalid workspace assignment must fail");
        assert!(
            matches!(error, AppError::Internal(message) if message.contains("profile rollback succeeded"))
        );

        let state = app.state::<AppState>();
        let db = state.db.lock().await;
        let user = Store::new(&db).get_user("user-cashier").unwrap().unwrap();
        assert_eq!(user.username, "cashier");
        assert_eq!(user.display_name, "Cashier");
    }

    #[tokio::test]
    async fn scoped_update_staff_rejects_short_pin() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = update_staff_scoped(
            "owner-token".into(),
            UpdateStaffScopedArgs {
                id: "user-cashier".into(),
                username: "cashier".into(),
                display_name: "Cashier".into(),
                role_id: "role-cashier".into(),
                is_active: true,
                pin: Some("12".into()),
                workspace_keys: None,
            },
            app.state(),
        )
        .await;
        assert!(matches!(result, Err(AppError::Invalid(_))));
    }

    #[tokio::test]
    async fn scoped_list_staff_requires_staff_read() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state = scoped_state_with_token(
            conn,
            "cashier-token",
            "user-cashier",
            "role-cashier",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = list_staff_scoped("cashier-token".into(), app.state()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn scoped_list_roles_requires_staff_read() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state = scoped_state_with_token(
            conn,
            "cashier-token",
            "user-cashier",
            "role-cashier",
            "store-a",
        );
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let result = list_roles_scoped("cashier-token".into(), app.state()).await;
        assert!(matches!(result, Err(AppError::PermissionDenied(_))));
    }

    #[tokio::test]
    async fn scoped_list_staff_lists_global_identity_db() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let staff = list_staff_scoped("owner-token".into(), app.state())
            .await
            .unwrap();
        let names: Vec<&str> = staff.iter().map(|s| s.username.as_str()).collect();
        assert!(names.contains(&"owner"));
        assert!(names.contains(&"cashier"));
    }

    // ── STAFF-04 — two-store isolation ────────────────────────────────

    #[tokio::test]
    async fn scoped_staff_commands_use_global_identity_db_for_any_store() {
        // Users/roles are global; store-scoped DBs have no users. A session
        // bound to store B must still resolve the caller from the GLOBAL
        // identity DB (not fail with "user not found" from an empty store
        // DB), and must not observe store A's business data.
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::for_test_with_conn(conn);
        state.db_manager =
            StoreDatabaseManager::new(temp_dir.path().to_path_buf(), oz_core::migrations::ALL);
        for (token, store_id) in [("owner-token-a", "store-a"), ("owner-token-b", "store-b")] {
            state.session_store.write().unwrap().insert(
                token.into(),
                SessionContext::new(
                    "user-owner".into(),
                    "role-owner".into(),
                    "terminal-1".into(),
                    store_id.into(),
                    "instance-1".into(),
                    "pos".into(),
                    None,
                    0,
                ),
            );
        }
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        // Store B's session can create staff (identity + roles are global).
        let created = create_staff_scoped(
            "owner-token-b".into(),
            CreateStaffScopedArgs {
                username: "storeb-cashier".into(),
                pin: "1234".into(),
                display_name: "Store B Cashier".into(),
                role_id: "role-cashier".into(),
            },
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(created.username, "storeb-cashier");

        // Store A's session sees the same global identity set (no cross-store
        // leakage of business data — staff identity is intentionally shared).
        let staff = list_staff_scoped("owner-token-a".into(), app.state())
            .await
            .unwrap();
        let names: Vec<&str> = staff.iter().map(|s| s.username.as_str()).collect();
        assert!(names.contains(&"storeb-cashier"));
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
            picker_ticket: String::new(),
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
            picker_ticket: String::new(),
        };
        let d = format!("{result:?}");
        assert!(d.contains("Alice"));
    }

    // ── BootstrapOwner logic tests ─────────────────────────────────────

    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    #[test]
    fn bootstrap_owner_creates_user_with_owner_role() {
        let conn = fresh_conn();
        let args = BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "1234".into(),
            display_name: "Store Owner".into(),
        };

        let result = run_bootstrap_owner(&conn, &args).unwrap();

        assert_eq!(result.session.display_name, "Store Owner");
        assert_eq!(result.session.role_name, "Owner");
        assert_eq!(result.session.role_id, "role-owner");
        assert!(!result.session.user_id.is_empty());

        // Verify directly via Store.
        let store = Store::new(&conn);
        let users = store.list_users().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, "owner");
        assert_eq!(users[0].display_name, "Store Owner");
        assert_eq!(users[0].role_id, "role-owner");
        assert!(users[0].is_active);
    }

    #[test]
    fn bootstrap_owner_rejects_when_users_exist() {
        let conn = fresh_conn();
        // Seed a user directly to simulate existing staff.
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        store
            .create_user("existing", "hash", "Existing", "role-cashier")
            .unwrap();

        let args = BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "1234".into(),
            display_name: "Owner".into(),
        };

        let err = run_bootstrap_owner(&conn, &args).unwrap_err();
        assert!(matches!(err, AppError::Invalid(msg) if msg.contains("already exist")));
    }

    #[test]
    fn bootstrap_owner_rejects_empty_username() {
        let conn = fresh_conn();
        let args = BootstrapOwnerArgs {
            username: "  ".into(),
            pin: "1234".into(),
            display_name: "Owner".into(),
        };

        let err = run_bootstrap_owner(&conn, &args).unwrap_err();
        assert!(matches!(err, AppError::Invalid(msg) if msg.contains("username")));
    }

    #[test]
    fn bootstrap_owner_rejects_empty_display_name() {
        let conn = fresh_conn();
        let args = BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "1234".into(),
            display_name: "  ".into(),
        };

        let err = run_bootstrap_owner(&conn, &args).unwrap_err();
        assert!(matches!(err, AppError::Invalid(msg) if msg.contains("display_name")));
    }

    #[test]
    fn bootstrap_owner_rejects_short_pin() {
        let conn = fresh_conn();
        let args = BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "12".into(),
            display_name: "Owner".into(),
        };

        let err = run_bootstrap_owner(&conn, &args).unwrap_err();
        assert!(matches!(err, AppError::Invalid(msg) if msg.contains("pin")));
    }

    #[test]
    fn bootstrap_owner_lowercases_username() {
        let conn = fresh_conn();
        let args = BootstrapOwnerArgs {
            username: "StoreOwner".into(),
            pin: "1234".into(),
            display_name: "Store Owner".into(),
        };

        let result = run_bootstrap_owner(&conn, &args).unwrap();
        assert_eq!(result.session.display_name, "Store Owner");

        // Username should be lowercased.
        let store = Store::new(&conn);
        let user = store.get_user_by_username("storeowner").unwrap().unwrap();
        assert_eq!(user.display_name, "Store Owner");
    }

    #[test]
    fn bootstrap_owner_session_matches_user() {
        let conn = fresh_conn();
        let args = BootstrapOwnerArgs {
            username: "admin".into(),
            pin: "9999".into(),
            display_name: "Admin".into(),
        };

        let result = run_bootstrap_owner(&conn, &args).unwrap();

        // The returned session user_id should match the created user.
        let store = Store::new(&conn);
        let user = store.get_user(&result.session.user_id).unwrap().unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.display_name, "Admin");

        // The role name should be resolved from the DB.
        let role = store.get_role("role-owner").unwrap().unwrap();
        assert_eq!(result.session.role_id, role.id);
        assert_eq!(result.session.role_name, role.name);
    }
}
