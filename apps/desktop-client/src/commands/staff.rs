//! Staff management commands — list, create, update staff members and roles.
//!
//! These commands are the IPC surface for the Staff Management UI.

use serde::{Deserialize, Serialize};
use tauri::State;

use oz_core::auth::hash_pin;
use oz_core::db::Store;
use oz_core::db::assignments::{Assignment, AssignmentSpec, ScopeMode};
use oz_core::db::profile::{UserProfile, mask_last4};
use oz_core::permissions;
use oz_core::subscription::TenantSubscription;
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
    /// National id rendered last-4 masked (ADR #35 D6: the full value never
    /// renders without `staff:read_identity`; the list only shows the mask).
    pub national_id_masked: String,
    /// Whether all 8 required profile fields are present — incomplete users
    /// are flagged and management-role assignment is gated on this.
    pub is_profile_complete: bool,
    /// The user's single effective assignment (ADR #35 D5 / spec 0048).
    pub assignment: AssignmentDto,
}

/// The 17 profile fields carried by the staff create/edit IPC args (ADR #35
/// D6 / spec 0049). All optional on the wire — creation-time mandatory-ness
/// is enforced by `create_user_with_profile` with field-specific errors, and
/// the form blocks submission before the command is ever called.
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

/// A staff profile as seen by the caller (ADR #35 D6): full sensitive
/// values only when the explicit grants are held, national id always
/// last-4 masked, reads audited by oz-core.
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
    /// Granted permission keys, verbatim from the role's permissions JSON
    /// (may include `"*"`). Shown in the staff screen so an admin can see
    /// exactly what each role can do.
    pub permissions: Vec<String>,
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
    /// ADR #35 D6 profile fields — creation requires the 9 mandatory fields
    /// (validated by `create_user_with_profile`).
    pub profile: ProfileArgs,
    /// Optional assignment scope (spec 0048). When `Some`, the user is
    /// created with this scope instead of the default global all/all.
    #[serde(default)]
    pub assignment: Option<AssignmentArgs>,
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
/// audited — see [`Store`].
#[tauri::command]
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
        .map(|r| {
            let permissions = r.permission_keys();
            RoleDto {
                id: r.id,
                name: r.name,
                description: r.description,
                permissions,
            }
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
    // C1.1: enforce the subscription tier's staff-user limit (Free 1 / Plus 5 /
    // Pro 20) before creating — the count runs against the global identity DB
    // that also holds the tenant_subscription row.
    let sub = TenantSubscription::load(&db, "default")?
        .ok_or_else(|| AppError::Internal("default tenant subscription not found".into()))?;
    sub.verify_signature()?;
    store.enforce_staff_quota(&sub.effective_tier())?;
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
/// STAFF-05: the profile update, PIN rotation, and (optional) assignment
/// scope run as one command inside a single global-DB transaction — any
/// failure rolls the whole update back atomically. The legacy store-scoped
/// `workspace_keys` write path (which needed cross-DB compensation) is
/// retired; assignments ride the same transaction as the profile.
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
        // oz-core) follow the same atomic update. Single-statement write,
        // safe inside this transaction.
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

    /// A complete ADR #35 D6 profile for create/update fixtures.
    fn complete_profile_args() -> ProfileArgs {
        ProfileArgs {
            date_of_birth: Some("1990-05-14".into()),
            phone: Some("+14155550123".into()),
            national_id_type: Some("ssn".into()),
            national_id: Some("123456789".into()),
            email: Some("fixture@example.com".into()),
            monthly_take_home_minor: Some(5_000_000),
            emergency_contact_name: Some("Bob".into()),
            emergency_contact_phone: Some("+14155550987".into()),
            ..Default::default()
        }
    }

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
        assert_eq!(json["is_active"], false);
    }

    // ── RoleDto ─────────────────────────────────────────────────────────

    #[test]
    fn role_dto_debug() {
        let dto = RoleDto {
            id: "r1".into(),
            name: "Admin".into(),
            description: "Full access".into(),
            permissions: vec![],
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
            permissions: vec![],
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
    /// limited user (no staff permissions — the retired cashier role maps
    /// to a narrow custom role, 0048 sweep). Users/roles are global records
    /// (ADR #4 / ADR #7); store-scoped DBs contain no users.
    fn seed_global_users(conn: &rusqlite::Connection) {
        let store = Store::new(conn);
        store.seed_default_roles().unwrap();
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-lite', 'Lite', 'Limited', '[\"sales:view\"]', '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-owner',   'owner',   'hash', 'Owner',   'role-owner',   1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-lite',    1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
        )
        .unwrap();
    }

    /// Raise the default tenant's subscription tier so C1.1 staff-quota
    /// enforcement has headroom (fresh_db seeds Free, which allows 1 staff).
    fn seed_subscription_tier(conn: &rusqlite::Connection, tier_key: &str) {
        conn.execute(
            "UPDATE tenant_subscription SET tier_key = ?1 WHERE tenant_id = 'default'",
            [tier_key],
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
                role_id: "role-staff".into(),
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
                role_id: "role-staff".into(),
                profile: complete_profile_args(),
                assignment: None,
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
            "role-lite",
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
                role_id: "role-staff".into(),
                profile: complete_profile_args(),
                assignment: None,
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
        // Pro (20 staff) — plenty of headroom past the seeded cashier.
        seed_subscription_tier(&conn, "pro");
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
                role_id: "role-staff".into(),
                profile: complete_profile_args(),
                assignment: None,
            },
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(result.username, "mallory");
        assert_eq!(result.role_name, "Staff");
    }

    #[tokio::test]
    async fn scoped_create_staff_blocked_at_free_tier_staff_limit() {
        // C1.1: fresh_db seeds Free (max 1 staff) and seed_global_users
        // already created the cashier — the next creation must be rejected
        // with the subscription-limit error, not silently inserted.
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
                role_id: "role-staff".into(),
                profile: complete_profile_args(),
                assignment: None,
            },
            app.state(),
        )
        .await;

        match result {
            Err(AppError::Core { sub_kind, message }) => {
                assert!(matches!(
                    sub_kind,
                    oz_core::CoreErrorKind::SubscriptionLimitExceeded
                ));
                assert!(message.contains("allows maximum 1 staff users"));
            }
            other => panic!("expected subscription-limit error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn scoped_create_staff_allowed_with_headroom_tier() {
        // C1.1: with a Plus tier (5 staff) and a single seeded cashier,
        // the owner can add a new staff member.
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        seed_subscription_tier(&conn, "plus");
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
                role_id: "role-staff".into(),
                profile: complete_profile_args(),
                assignment: None,
            },
            app.state(),
        )
        .await
        .unwrap();
        assert_eq!(result.username, "mallory");
    }

    #[tokio::test]
    async fn scoped_update_staff_denies_cashier_session() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state = scoped_state_with_token(
            conn,
            "cashier-token",
            "user-cashier",
            "role-lite",
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
                profile: None,
                assignment: None,
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
                profile: complete_profile_args(),
                assignment: None,
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
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-lite', 'Lite', 'Limited', '[\"sales:view\"]', '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-manager', 'manager', 'hash', 'Manager', 'role-manager', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-lite', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
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
                profile: None,
                assignment: None,
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
                profile: None,
                assignment: None,
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
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-lite', 'Lite', 'Limited', '[\"sales:view\"]', '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z'),
                ('user-cashier', 'cashier', 'hash', 'Cashier', 'role-lite', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z');",
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
                profile: None,
                assignment: None,
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
                role_id: "role-lite".into(),
                is_active: true,
                pin: Some("9876".into()),
                profile: None,
                assignment: None,
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
                "role-lite".into(),
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
                role_id: "role-lite".into(),
                is_active: true,
                pin: Some("9876".into()),
                profile: None,
                assignment: None,
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
                profile: None,
                assignment: None,
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
    async fn scoped_update_staff_writes_assignment_scope_atomically() {
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
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
                role_id: "role-lite".into(),
                is_active: true,
                pin: None,
                profile: None,
                // ADR #35 D5 (spec 0048): scoped assignment with explicit
                // all/list per dimension — `retail-pos` is FK-valid (seeded
                // by migration 128), branch ids are store_profiles ids.
                assignment: Some(AssignmentArgs {
                    scope_mode: "scoped".into(),
                    branches_all: false,
                    branch_ids: vec!["store-a".into()],
                    workspaces_all: false,
                    workspace_keys: vec!["retail-pos".into()],
                }),
            },
            app.state(),
        )
        .await
        .unwrap();

        let st = app.state::<AppState>();
        let db = st.db.lock().await;
        let assignment = Store::new(&db)
            .assignment_for_user("user-cashier")
            .unwrap()
            .expect("assignment");
        assert_eq!(assignment.scope_mode, ScopeMode::Scoped);
        assert!(!assignment.branches_all && !assignment.workspaces_all);
        assert_eq!(assignment.branches, vec!["store-a"]);
        assert_eq!(assignment.workspaces, vec!["retail-pos"]);
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
                role_id: "role-lite".into(),
                is_active: true,
                pin: Some("9876".into()),
                profile: None,
                assignment: None,
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
                "role-lite".into(),
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
                role_id: "role-lite".into(),
                is_active: true,
                pin: Some("9876".into()),
                profile: None,
                assignment: None,
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
                role_id: "role-lite".into(),
                is_active: true,
                pin: Some("12".into()),
                profile: None,
                assignment: None,
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
            "role-lite",
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
            "role-lite",
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
    async fn scoped_list_roles_carries_each_roles_granted_permission_keys() {
        // The staff screen shows what each role can do — the role listing
        // must carry the granted keys verbatim (Owner = global wildcard,
        // a narrow custom role = its exact grants).
        let conn = oz_core::migrations::fresh_db();
        seed_global_users(&conn);
        let state =
            scoped_state_with_token(conn, "owner-token", "user-owner", "role-owner", "store-a");
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::generate_context!())
            .unwrap();

        let roles = list_roles_scoped("owner-token".into(), app.state())
            .await
            .unwrap();
        let owner = roles.iter().find(|r| r.id == "role-owner").unwrap();
        assert_eq!(owner.permissions, vec!["*"]);
        let lite = roles.iter().find(|r| r.id == "role-lite").unwrap();
        assert_eq!(lite.permissions, vec!["sales:view"]);
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
        // Pro tier so the staff-creation quota (C1.1) has headroom.
        seed_subscription_tier(&conn, "pro");
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
                role_id: "role-staff".into(),
                profile: complete_profile_args(),
                assignment: None,
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
                permissions: vec!["*".into()],
            },
            picker_ticket: String::new(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["session"]["user_id"], "u1");
        assert_eq!(json["session"]["role_name"], "Owner");
        assert_eq!(json["session"]["permissions"], serde_json::json!(["*"]));
    }

    #[test]
    fn bootstrap_owner_result_debug() {
        let result = BootstrapOwnerResult {
            session: oz_core::auth::LoginSession {
                user_id: "u2".into(),
                display_name: "Alice".into(),
                role_name: "Owner".into(),
                role_id: "role-owner".into(),
                permissions: vec![],
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
            .create_user("existing", "hash", "Existing", "role-staff")
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
