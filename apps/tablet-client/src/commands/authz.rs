//! Authorization helpers for Tauri commands.
//!
//! Provides `require_permission` and `require_permission_for_user`
//! to verify that the caller has the required permission.

use oz_core::CoreError;
use oz_core::db::Store;

use crate::error::AppError;

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
    fn unregistered_permission_denies_owner_through_the_gate() {
        let conn = seeded_store();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
            [],
        )
        .unwrap();
        let store = Store::new(&conn);

        // The `*` Owner grant covers every registered key but must NOT cover
        // an unregistered one — deny-by-default survives the client wrapper
        // and still surfaces as `permissionDenied`, not a core error.
        assert!(matches!(
            require_permission_for_user(&store, "user-owner", "sales:typo"),
            Err(AppError::PermissionDenied(_))
        ));
        assert!(matches!(
            require_permission_for_user(&store, "user-owner", "sales:void"),
            Ok(())
        ));
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
