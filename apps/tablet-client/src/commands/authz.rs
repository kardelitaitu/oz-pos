//! Authorization helpers for Tauri commands.
//!
//! Provides `require_permission` and `require_permission_for_user`
//! to verify that the caller has the required permission.

use oz_core::CoreError;
use oz_core::db::Store;
use oz_core::session::SessionContext;

use crate::error::AppError;
use crate::state::AppState;

/// Map a gate denial to the client's `permissionDenied` wire shape.
///
/// `Store::require_permission` returns `CoreError::PermissionDenied` for
/// every fail-closed case; anything else (DB errors) becomes a `Core` error
/// as usual. The frontend sees `kind: "permissionDenied"` unchanged.
fn map_gate_error(e: CoreError) -> AppError {
    match e {
        CoreError::PermissionDenied(message) => AppError::PermissionDenied(message),
        other => AppError::from(other),
    }
}

/// Look up the user by `user_id`, load their role, and verify the role
/// has the given permission.  This prevents role‑ID forgery.
pub fn require_permission_for_user(
    store: &Store<'_>,
    user_id: &str,
    required: &str,
) -> Result<(), AppError> {
    store
        .require_permission(user_id, required)
        .map_err(map_gate_error)
}

/// The scope-aware variant (ADR #35 D5 / spec 0048): for commands that run
/// inside a branch/workspace context, this enforces the caller's scoped
/// assignment in addition to the permission. Global assignments and legacy
/// users without an assignment are not scope-restricted.
pub fn require_permission_for_user_scoped(
    store: &Store<'_>,
    user_id: &str,
    required: &str,
    branch: Option<&str>,
    workspace: Option<&str>,
) -> Result<(), AppError> {
    store
        .require_permission_scoped(user_id, required, branch, workspace)
        .map_err(map_gate_error)
}

/// Authorize the session user against the GLOBAL identity database,
/// scope-aware (ADR #35 D5 / spec 0048): the session's resolved store
/// (branch) and workspace `type_key` are evaluated against the caller's
/// assignment in addition to the permission. Global assignments and legacy
/// users without an assignment row are not scope-restricted.
pub async fn require_permission_for_session(
    state: &AppState,
    session: &SessionContext,
    required: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user_scoped(
        &store,
        &session.user_id,
        required,
        Some(&session.store_id),
        Some(&session.type_key),
    )
}

#[cfg(test)] #[path = "authz_tests.rs"] mod tests;
