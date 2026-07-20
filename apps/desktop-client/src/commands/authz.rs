//! Authorization helpers for Tauri commands.
//!
//! Provides [`require_permission`] and [`require_permission_for_user`]
//! to verify that the caller has the required permission.
//!
//! `require_permission` trusts the caller-supplied `role_id` and is
//! kept for backward compatibility.  All new code should use
//! `require_permission_for_user` which looks up the user's actual role
//! from the database, preventing role‑ID forgery.

use oz_core::db::Store;

use crate::error::AppError;

/// Load the role by `role_id` and verify it has the given permission.
///
/// # Errors
///
/// Returns [`AppError::PermissionDenied`] if the role is missing or
/// lacks the required permission. Returns [`AppError::Core`] on DB
/// errors.
///
/// # Example
///
/// ```ignore
/// # use crate::commands::authz::require_permission;
/// # use oz_core::permissions;
/// # fn example(store: &oz_core::db::Store, role_id: &str) -> Result<(), crate::error::AppError> {
/// require_permission(store, role_id, permissions::SALES_VOID)?;
/// # Ok(())
/// # }
/// ```
pub fn require_permission(
    store: &Store<'_>,
    role_id: &str,
    required: &str,
) -> Result<(), AppError> {
    let role = store
        .get_role(role_id)?
        .ok_or_else(|| AppError::Internal(format!("role {role_id} not found")))?;

    role.authorize(required)
        .map_err(|e| AppError::PermissionDenied(e.to_string()))
}

/// Look up the user by `user_id`, load their role, and verify the role
/// has the given permission.
///
/// This is the recommended helper for all Tauri commands because the
/// backend always derives the role from the user — a compromised or
/// tampered frontend cannot forge a different role_id.
///
/// # Errors
///
/// Returns [`AppError::PermissionDenied`] if the user is not found,
/// the role is missing, or the permission is not granted.  Returns
/// [`AppError::Core`] on DB errors.
pub fn require_permission_for_user(
    store: &Store<'_>,
    user_id: &str,
    required: &str,
) -> Result<(), AppError> {
    let user = store
        .get_user(user_id)?
        .ok_or_else(|| AppError::PermissionDenied("user not found".into()))?;

    let role = store
        .get_role(&user.role_id)?
        .ok_or_else(|| AppError::Internal(format!("role {} not found", user.role_id)))?;

    role.authorize(required)
        .map_err(|e| AppError::PermissionDenied(e.to_string()))
}
