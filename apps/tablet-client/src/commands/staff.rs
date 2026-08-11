//! Staff management commands — list, create, update staff members and roles.
//!
//! These commands are the IPC surface for the Staff Management UI.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::auth::hash_pin;
use oz_core::db::Store;
use oz_core::db::assignments::{Assignment, AssignmentSpec, ScopeMode};
use oz_core::db::profile::{UserProfile, mask_last4};
use oz_core::permissions;
use oz_core::{Role, User};

use foundation::{validate_min_length, validate_not_empty};

use crate::commands::authz::require_permission_for_user;
use crate::commands::picker_ticket;
use crate::error::AppError;
use crate::state::AppState;

// ── Staff member DTO ────────────────────────────────────────────────

/// A user's single effective assignment as seen by the front-end (ADR #35
/// D5 / spec 0048): scope mode plus the per-dimension explicit-all flag and
/// list. Legacy users without an assignment row resolve as global all/all.
#[derive(Debug, Serialize)]
pub struct AssignmentDto {
    /// `"global"` or `"scoped"`.
    pub scope_mode: String,
    /// Branch dimension is explicit `all`.
    pub branches_all: bool,
    /// Branch ids in scope when `branches_all` is false.
    pub branch_ids: Vec<String>,
    /// Workspace dimension is explicit `all`.
    pub workspaces_all: bool,
    /// Workspace keys in scope when `workspaces_all` is false.
    pub workspace_keys: Vec<String>,
}

/// The assignment scope carried by the staff create/edit IPC args (ADR #35
/// D5 / spec 0048): `scope_mode` plus the per-dimension explicit-all flag
/// and list. Empty lists never mean "all" — the `*_all` flags are the
/// explicit marker, so `list` with no ids is a deny.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AssignmentArgs {
    /// `"global"` or `"scoped"`.
    pub scope_mode: String,
    /// Branch dimension is explicit `all`.
    pub branches_all: bool,
    /// Branch ids in scope when `branches_all` is false.
    pub branch_ids: Vec<String>,
    /// Workspace dimension is explicit `all`.
    pub workspaces_all: bool,
    /// Workspace keys in scope when `workspaces_all` is false.
    pub workspace_keys: Vec<String>,
}

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
    /// National id rendered last-4 masked (ADR #35 D6).
    pub national_id_masked: String,
    /// Whether all 8 required profile fields are present.
    pub is_profile_complete: bool,
    /// The user's single effective assignment (ADR #35 D5 / spec 0048).
    pub assignment: AssignmentDto,
}

/// The 17 profile fields carried by the staff create/edit IPC args (ADR #35
/// D6 / spec 0049). All optional on the wire — creation-time mandatory-ness
/// is enforced by `create_user_with_profile`, and the form blocks submission
/// before the command is ever called.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ProfileArgs {
    /// ISO date (`YYYY-MM-DD`), never in the future.
    pub date_of_birth: Option<String>,
    /// Phone in E.164 form.
    pub phone: Option<String>,
    /// `"ssn"` or `"nik"`.
    pub national_id_type: Option<String>,
    /// National id (ssn 9 / nik 16 digits) — encrypted at rest.
    pub national_id: Option<String>,
    /// Lowercase email, unique when present.
    pub email: Option<String>,
    /// Monthly take-home pay in i64 minor units — encrypted at rest.
    pub monthly_take_home_minor: Option<i64>,
    /// Emergency contact name (required at creation).
    pub emergency_contact_name: Option<String>,
    /// Emergency contact phone (required at creation).
    pub emergency_contact_phone: Option<String>,
    /// Job title (free text).
    pub job_title: Option<String>,
    /// Free-text notes.
    pub notes: Option<String>,
    /// Street address.
    pub address: Option<String>,
    /// UI language preference.
    pub language: Option<String>,
    /// Avatar reference.
    pub avatar: Option<String>,
    /// Tax identification number.
    pub tax_id: Option<String>,
    /// Expiry of the national id document (`YYYY-MM-DD`).
    pub national_id_expires_at: Option<String>,
    /// Relationship of the emergency contact (e.g. "spouse").
    pub emergency_contact_relationship: Option<String>,
    /// Hire date (`YYYY-MM-DD`).
    pub hire_date: Option<String>,
}

impl ProfileArgs {
    /// Build the domain [`UserProfile`] (empty strings for the stable slots).
    pub fn into_profile(self) -> UserProfile {
        UserProfile {
            date_of_birth: self.date_of_birth,
            phone: self.phone,
            national_id_type: self.national_id_type,
            national_id: self.national_id,
            email: self.email,
            monthly_take_home_minor: self.monthly_take_home_minor,
            emergency_contact_name: self.emergency_contact_name,
            emergency_contact_phone: self.emergency_contact_phone,
            job_title: self.job_title.unwrap_or_default(),
            notes: self.notes.unwrap_or_default(),
            address: self.address,
            language: self.language,
            avatar: self.avatar,
            tax_id: self.tax_id,
            national_id_expires_at: self.national_id_expires_at,
            emergency_contact_relationship: self.emergency_contact_relationship,
            hire_date: self.hire_date,
        }
    }
}

/// A staff profile as seen by the caller (ADR #35 D6).
#[derive(Debug, Serialize)]
pub struct ProfileViewDto {
    /// Target user id.
    pub user_id: String,
    /// Login username.
    pub username: String,
    /// Display name.
    pub display_name: String,
    /// ISO date of birth.
    pub date_of_birth: Option<String>,
    /// Phone in E.164 form.
    pub phone: Option<String>,
    /// `"ssn"` or `"nik"`.
    pub national_id_type: Option<String>,
    /// Full national id — present only with `staff:read_identity`.
    pub national_id: Option<String>,
    /// Last-4 masked national id — always present.
    pub national_id_masked: String,
    /// Lowercase email.
    pub email: Option<String>,
    /// Monthly take-home pay — present only with `staff:read_payroll`.
    pub monthly_take_home_minor: Option<i64>,
    /// Emergency contact name.
    pub emergency_contact_name: Option<String>,
    /// Emergency contact phone.
    pub emergency_contact_phone: Option<String>,
    /// Job title.
    pub job_title: String,
    /// Free-text notes.
    pub notes: String,
    /// Street address.
    pub address: Option<String>,
    /// UI language preference.
    pub language: Option<String>,
    /// Avatar reference.
    pub avatar: Option<String>,
    /// Tax id — present only with `staff:read_identity`.
    pub tax_id: Option<String>,
    /// National id document expiry.
    pub national_id_expires_at: Option<String>,
    /// Emergency contact relationship.
    pub emergency_contact_relationship: Option<String>,
    /// Hire date.
    pub hire_date: Option<String>,
    /// Whether all 8 required profile fields are present.
    pub is_complete: bool,
}

impl From<oz_core::db::profile::ProfileView> for ProfileViewDto {
    fn from(view: oz_core::db::profile::ProfileView) -> Self {
        Self {
            user_id: String::new(),
            username: view.username,
            display_name: view.display_name,
            date_of_birth: view.date_of_birth,
            phone: view.phone,
            national_id_type: view.national_id_type,
            national_id: view.national_id,
            national_id_masked: view.national_id_masked,
            email: view.email,
            monthly_take_home_minor: view.monthly_take_home_minor,
            emergency_contact_name: view.emergency_contact_name,
            emergency_contact_phone: view.emergency_contact_phone,
            job_title: view.job_title,
            notes: view.notes,
            address: view.address,
            language: view.language,
            avatar: view.avatar,
            tax_id: view.tax_id,
            national_id_expires_at: view.national_id_expires_at,
            emergency_contact_relationship: view.emergency_contact_relationship,
            hire_date: view.hire_date,
            is_complete: view.is_complete,
        }
    }
}

fn to_staff_dto(
    user: &User,
    roles: &[Role],
    profile: Option<&UserProfile>,
    assignment: Option<&Assignment>,
) -> StaffMemberDto {
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
        national_id_masked: profile
            .and_then(|p| p.national_id.as_deref())
            .map(mask_last4)
            .unwrap_or_else(|| "****".to_string()),
        is_profile_complete: profile.map(|p| p.is_complete()).unwrap_or(false),
        assignment: assignment_dto(assignment),
    }
}

/// Render an assignment for the wire. Legacy users without an assignment
/// row (pre-0048 databases) resolve as global all/all — the same effective
/// semantics as `users.role_id` alone.
fn assignment_dto(assignment: Option<&Assignment>) -> AssignmentDto {
    match assignment {
        Some(a) => AssignmentDto {
            scope_mode: a.scope_mode.as_str().to_string(),
            branches_all: a.branches_all,
            branch_ids: a.branches.clone(),
            workspaces_all: a.workspaces_all,
            workspace_keys: a.workspaces.clone(),
        },
        None => AssignmentDto {
            scope_mode: ScopeMode::Global.as_str().to_string(),
            branches_all: true,
            branch_ids: vec![],
            workspaces_all: true,
            workspace_keys: vec![],
        },
    }
}

/// Parse the wire `scope_mode` string, rejecting anything else.
fn parse_scope_mode(s: &str) -> Result<ScopeMode, AppError> {
    ScopeMode::parse(s).ok_or_else(|| AppError::Invalid(format!("invalid scope_mode: {s}")))
}

/// Map the wire args to an oz-core assignment spec.
fn assignment_spec(args: &AssignmentArgs) -> Result<AssignmentSpec, AppError> {
    Ok(AssignmentSpec {
        scope_mode: parse_scope_mode(&args.scope_mode)?,
        branches_all: args.branches_all,
        branches: args.branch_ids.clone(),
        workspaces_all: args.workspaces_all,
        workspaces: args.workspace_keys.clone(),
    })
}

// ── List staff ─────────────────────────────────────────────────────

#[command]
/// List staff.
///
/// **Deprecated for multi-store (ADR #7):** Use [`list_staff_scoped`] so the
/// caller identity is resolved from the session token instead of a
/// client-supplied `caller_user_id`.
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

#[command]
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

#[command]
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

#[command]
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
// Replacement for the legacy staff commands. Caller identity is resolved
// from the opaque `session_token`; the commands NEVER accept a
// caller-supplied `caller_user_id`. Users/roles are GLOBAL identity
// records (ADR #4 / ADR #7) — the permission check and CRUD run against
// the global identity DB.

/// Arguments for creating a staff member from a session token.
///
/// Deliberately carries NO caller identity field.
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
    /// ADR #35 D6 profile fields — creation requires the 9 mandatory fields.
    pub profile: ProfileArgs,
    /// Optional assignment scope (spec 0048). When `Some`, the user is
    /// created with this scope instead of the default global all/all.
    #[serde(default)]
    pub assignment: Option<AssignmentArgs>,
}

/// Arguments for updating a staff member from a session token.
///
/// Deliberately carries NO caller identity field.
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
    /// ADR #35 D6 profile fields (validated + encrypted at rest). When
    /// `Some`, they are written atomically with the user update.
    #[serde(default)]
    pub profile: Option<ProfileArgs>,
    /// Optional assignment scope (ADR #35 D5 / spec 0048). When `Some`, it
    /// is written atomically with the user + profile update inside the same
    /// transaction (replaces the legacy store-scoped workspace write for
    /// callers using the new model).
    #[serde(default)]
    pub assignment: Option<AssignmentArgs>,
}

/// List staff members. Caller identity is resolved from the session token.
#[command]
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
    let dtos = users
        .iter()
        .map(|u| {
            let profile = store.get_user_profile(&u.id).ok().flatten();
            let assignment = store.assignment_for_user(&u.id).ok().flatten();
            to_staff_dto(u, &roles, profile.as_ref(), assignment.as_ref())
        })
        .collect();
    drop(db);
    Ok(dtos)
}

/// Load a staff member's full profile as the session user sees it (ADR #35
/// D6). Sensitive fields are withheld or masked unless the caller holds
/// `staff:read_identity` / `staff:read_payroll`, and every sensitive read is
/// audited.
#[command]
pub async fn get_staff_profile_scoped(
    session_token: String,
    user_id: String,
    state: State<'_, AppState>,
) -> Result<ProfileViewDto, AppError> {
    let session = state.resolve_session(&session_token)?;
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, permissions::STAFF_READ)?;
    let view = store
        .get_user_profile_viewed_by(&session.user_id, &user_id)?
        .ok_or_else(|| AppError::Invalid(format!("no such user: {user_id}")))?;
    drop(db);
    let mut dto: ProfileViewDto = view.into();
    dto.user_id = user_id;
    Ok(dto)
}

/// List roles. Caller identity is resolved from the session token.
#[command]
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

/// Create a staff member. Caller identity is resolved from the session token.
///
/// STAFF-02: enforces the role-assignment hierarchy (only Owner-level
/// callers may create an Owner account).
#[command]
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
    let profile = args.profile.into_profile();
    let assignment = args.assignment.as_ref().map(assignment_spec).transpose()?;
    let user = store.create_user_with_profile(
        &username,
        &pin_hash,
        display_name,
        &args.role_id,
        &profile,
        assignment.as_ref(),
    )?;
    let roles = store.list_roles()?;
    let assignment = store.assignment_for_user(&user.id)?;
    drop(db);

    Ok(to_staff_dto(
        &user,
        &roles,
        Some(&profile),
        assignment.as_ref(),
    ))
}

/// Update a staff member. Caller identity is resolved from the session token.
///
/// STAFF-02: enforces the role-assignment hierarchy.
/// STAFF-03: optionally rotates the PIN when `args.pin` is a non-empty value.
/// STAFF-05: the profile update, PIN rotation, and (optional) assignment
/// scope run as one command inside a single global-DB transaction — any
/// failure rolls the whole update back atomically. The legacy store-scoped
/// `workspace_keys` write path (which needed cross-DB compensation) is
/// retired; assignments ride the same transaction as the profile.
#[command]
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
        let user = store.get_user(&args.id)?;
        let profile = store.get_user_profile(&args.id)?;
        user.map(|u| {
            (
                u.username,
                u.display_name,
                u.role_id,
                u.is_active,
                u.pin_hash,
                profile,
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
        // ADR #35 D6 incomplete-profile semantics: assigning a role that
        // grants sensitive permissions requires a complete profile.
        store.require_role_assignable(&args.id, &args.role_id)?;
        store.update_user(
            &args.id,
            &args.username,
            &args.display_name,
            &args.role_id,
            args.is_active,
        )?;
        // ADR #35 D6: the profile columns (validated, encrypted at rest by
        // oz-core) follow the same atomic update.
        if let Some(profile) = &args.profile {
            store.write_user_profile(&args.id, &profile.clone().into_profile())?;
        }

        // ADR #35 D5 (spec 0048): the assignment scope rides the same
        // transaction — in-tx writer, no nested BEGIN. `update_user` above
        // already synced the assignment role; this replaces only the scope.
        if let Some(spec) = &args.assignment {
            let spec = assignment_spec(spec)?;
            store.write_assignment_scope(&args.id, &args.role_id, &spec)?;
        }

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

    if pin_rotated {
        // STAFF-03: a rotated PIN invalidates every OTHER session issued
        // under the old PIN. The caller's own session is preserved — they
        // authenticated moments ago and the UI reloads with the same token.
        state.invalidate_user_sessions_except(&args.id, &session_token);
    }

    let profile = match &args.profile {
        Some(p) => Some(p.clone().into_profile()),
        None => previous_profile.and_then(|(_, _, _, _, _, p)| p),
    };
    let assignment = {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        store.assignment_for_user(&args.id)?
    };
    Ok(to_staff_dto(
        &user,
        &roles,
        profile.as_ref(),
        assignment.as_ref(),
    ))
}

// ── Bootstrap first owner (no authentication required) ────────────────
//
// Parity with the desktop client (audit/06 residual): the tablet needs the
// same first-owner path so a fresh installation can be provisioned from the
// tablet itself. Like `staff_login`, the command mints a short-lived picker
// ticket so the pre-session workspace picker stays bound to the real user.

/// Arguments for the `bootstrap_owner` command.
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
/// Bootstrapownerresult.
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
/// Returns `Invalid` if any users already exist, preventing accidental
/// re-bootstrapping after staff accounts have been created.
/// Returns `Invalid` if validation fails (empty username, short PIN, etc.).
#[command]
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
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
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

    let permissions = role.permission_keys();

    Ok(BootstrapOwnerResult {
        session: oz_core::auth::LoginSession {
            user_id: user.id,
            display_name: user.display_name,
            role_name: role.name,
            role_id: role.id,
            permissions,
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
            national_id_masked: "*****6789".into(),
            is_profile_complete: true,
            assignment: assignment_dto(None),
        };
        let d = format!("{dto:?}");
        assert!(d.contains("jdoe"));
        assert!(d.contains("John Doe"));
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
            national_id_masked: "****".into(),
            is_profile_complete: false,
            assignment: assignment_dto(None),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["username"], "asmith");
        assert_eq!(json["role_name"], "Cashier");
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
        assert!(d.contains("Full access"));
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
        assert_eq!(args.pin, "1234");
        assert_eq!(args.display_name, "John Doe");
        assert_eq!(args.role_id, "r1");
        assert_eq!(args.caller_user_id, "admin1");
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
        assert_eq!(args.caller_user_id, "admin1");
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

    // ── BootstrapOwnerArgs / BootstrapOwnerResult ────────────────────────

    #[test]
    fn bootstrap_owner_args_deserialize() {
        let json = r##"{"username":"owner","pin":"1234","display_name":"Store Owner"}"##;
        let args: BootstrapOwnerArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.username, "owner");
        assert_eq!(args.pin, "1234");
        assert_eq!(args.display_name, "Store Owner");
    }

    #[test]
    fn bootstrap_owner_args_debug() {
        let args = BootstrapOwnerArgs {
            username: "owner".into(),
            pin: "0000".into(),
            display_name: "Owner".into(),
        };
        let d = format!("{args:?}");
        assert!(d.contains("owner"));
        assert!(d.contains("Owner"));
    }

    #[test]
    fn bootstrap_owner_result_serialize() {
        let result = BootstrapOwnerResult {
            session: oz_core::auth::LoginSession {
                user_id: "u1".into(),
                display_name: "Owner".into(),
                role_name: "Owner".into(),
                role_id: "role-owner".into(),
                permissions: vec!["*".into()],
            },
            picker_ticket: "ticket".into(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["session"]["role_id"], "role-owner");
        assert_eq!(json["picker_ticket"], "ticket");
    }

    #[test]
    fn bootstrap_owner_result_debug() {
        let result = BootstrapOwnerResult {
            session: oz_core::auth::LoginSession {
                user_id: "u2".into(),
                display_name: "Boss".into(),
                role_name: "Owner".into(),
                role_id: "role-owner".into(),
                permissions: vec![],
            },
            picker_ticket: String::new(),
        };
        let d = format!("{result:?}");
        assert!(d.contains("Boss"));
    }

    // ── run_bootstrap_owner logic ───────────────────────────────────────

    use oz_core::migrations;
    use tauri::Manager as _;

    #[test]
    fn run_bootstrap_owner_creates_owner_role_user() {
        let conn = migrations::fresh_db();
        let result = run_bootstrap_owner(
            &conn,
            &BootstrapOwnerArgs {
                username: "owner".into(),
                pin: "1234".into(),
                display_name: "Store Owner".into(),
            },
        )
        .unwrap();

        assert_eq!(result.session.role_id, "role-owner");
        assert_eq!(result.session.role_name, "Owner");
        assert_eq!(result.session.display_name, "Store Owner");

        let store = Store::new(&conn);
        let user = store.get_user(&result.session.user_id).unwrap().unwrap();
        assert_eq!(user.role_id, oz_core::builtin_roles::OWNER);
        assert!(user.is_active);
    }

    #[test]
    fn run_bootstrap_owner_lowercases_username() {
        let conn = migrations::fresh_db();
        let result = run_bootstrap_owner(
            &conn,
            &BootstrapOwnerArgs {
                username: "  OWNER  ".into(),
                pin: "1234".into(),
                display_name: "  Store Owner  ".into(),
            },
        )
        .unwrap();
        // `create_user` assigns a UUID id; the USERNAME must be normalized.
        assert_eq!(result.session.display_name, "Store Owner");
        let user = Store::new(&conn)
            .get_user_by_username("owner")
            .unwrap()
            .unwrap();
        assert_eq!(user.id, result.session.user_id);
    }

    #[test]
    fn run_bootstrap_owner_rejects_when_users_exist() {
        let conn = migrations::fresh_db();
        // Bootstrap once, then try again — the second call must fail closed.
        run_bootstrap_owner(
            &conn,
            &BootstrapOwnerArgs {
                username: "owner".into(),
                pin: "1234".into(),
                display_name: "Owner".into(),
            },
        )
        .unwrap();
        let err = run_bootstrap_owner(
            &conn,
            &BootstrapOwnerArgs {
                username: "owner2".into(),
                pin: "1234".into(),
                display_name: "Owner 2".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
        // No second user may exist.
        assert_eq!(Store::new(&conn).list_users().unwrap().len(), 1);
    }

    #[test]
    fn run_bootstrap_owner_rejects_empty_username() {
        let conn = migrations::fresh_db();
        let err = run_bootstrap_owner(
            &conn,
            &BootstrapOwnerArgs {
                username: "  ".into(),
                pin: "1234".into(),
                display_name: "Owner".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
    }

    #[test]
    fn run_bootstrap_owner_rejects_empty_display_name() {
        let conn = migrations::fresh_db();
        let err = run_bootstrap_owner(
            &conn,
            &BootstrapOwnerArgs {
                username: "owner".into(),
                pin: "1234".into(),
                display_name: "  ".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
    }

    #[test]
    fn run_bootstrap_owner_rejects_short_pin() {
        let conn = migrations::fresh_db();
        let err = run_bootstrap_owner(
            &conn,
            &BootstrapOwnerArgs {
                username: "owner".into(),
                pin: "123".into(),
                display_name: "Owner".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
    }

    #[tokio::test]
    async fn bootstrap_owner_mints_verifiable_picker_ticket() {
        // audit/06 (parity with the desktop client): the command-level
        // bootstrap must mint a ticket bound to the NEW owner so the
        // pre-session workspace picker works immediately after setup.
        let conn = migrations::fresh_db();
        let app = tauri::test::mock_builder()
            .manage(AppState::for_test_with_conn(conn))
            .build(tauri::generate_context!())
            .unwrap();

        let result = bootstrap_owner(
            BootstrapOwnerArgs {
                username: "owner".into(),
                pin: "1234".into(),
                display_name: "Store Owner".into(),
            },
            app.state(),
        )
        .await
        .unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let state = app.state::<AppState>();
        assert_eq!(
            picker_ticket::verify_picker_ticket(
                &state.picker_ticket_secret,
                &result.picker_ticket,
                now
            )
            .as_deref(),
            Some(result.session.user_id.as_str()),
            "bootstrap must mint a ticket bound to the new owner"
        );
        assert!(!result.picker_ticket.is_empty());
        assert_eq!(result.session.role_id, "role-owner");
    }
}
