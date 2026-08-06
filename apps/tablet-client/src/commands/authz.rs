//! Authorization helpers for Tauri commands.
//!
//! Provides `require_permission` and `require_permission_for_user`
//! to verify that the caller has the required permission.

use oz_core::db::Store;

use crate::error::AppError;

/// Load the role by `role_id` and verify it has the given permission.
///
/// # Errors
///
/// Returns [`AppError::PermissionDenied`] if the role is missing or
/// lacks the required permission.
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
/// has the given permission.  This prevents role‑ID forgery.
pub fn require_permission_for_user(
    store: &Store<'_>,
    user_id: &str,
    required: &str,
) -> Result<(), AppError> {
    let user = store
        .get_user(user_id)?
        .ok_or_else(|| AppError::PermissionDenied("user not found".into()))?;
    if !user.is_active {
        return Err(AppError::PermissionDenied("user is inactive".into()));
    }

    let role = store
        .get_role(&user.role_id)?
        .ok_or_else(|| AppError::Internal(format!("role {} not found", user.role_id)))?;

    role.authorize(required)
        .map_err(|e| AppError::PermissionDenied(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use oz_core::permissions;
    use rusqlite::params;

    fn seeded_store() -> rusqlite::Connection {
        let conn = migrations::fresh_db();
        let store = Store::new(&conn);
        store.seed_default_roles().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
            params![
                "user-cashier",
                "cashier",
                "hash",
                "Cashier",
                "role-cashier",
                "2026-07-31T00:00:00.000Z"
            ],
        )
        .unwrap();
        conn
    }

    #[test]
    fn user_permission_check_uses_database_role() {
        let conn = seeded_store();
        let store = Store::new(&conn);

        assert!(
            require_permission_for_user(&store, "user-cashier", permissions::LOYALTY_VIEW).is_ok()
        );
        assert!(matches!(
            require_permission_for_user(&store, "user-cashier", permissions::LOYALTY_MANAGE),
            Err(AppError::PermissionDenied(_))
        ));
    }

    #[test]
    fn missing_user_is_denied_even_when_role_id_is_known() {
        let conn = seeded_store();
        let store = Store::new(&conn);

        assert!(matches!(
            require_permission_for_user(&store, "role-owner", permissions::LOYALTY_MANAGE),
            Err(AppError::PermissionDenied(_))
        ));
    }

    #[test]
    fn inactive_user_is_denied_even_with_granted_permission() {
        let conn = seeded_store();
        conn.execute(
            "UPDATE users SET is_active = 0 WHERE id = 'user-cashier'",
            [],
        )
        .unwrap();
        let store = Store::new(&conn);

        assert!(matches!(
            require_permission_for_user(&store, "user-cashier", permissions::LOYALTY_VIEW),
            Err(AppError::PermissionDenied(message)) if message == "user is inactive"
        ));
    }

    #[test]
    fn owner_role_grants_loyalty_management() {
        let conn = seeded_store();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();
        let store = Store::new(&conn);

        assert!(
            require_permission_for_user(&store, "user-owner", permissions::LOYALTY_MANAGE).is_ok()
        );
    }
}
