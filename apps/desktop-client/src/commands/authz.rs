//! Authorization helpers for Tauri commands.
//!
//! Provides [`require_permission_for_user`] to verify that the caller
//! has the required permission: it looks up the user's actual role from
//! the database, preventing role‑ID forgery.

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
    store
        .require_permission(user_id, required)
        .map_err(map_gate_error)
}

/// Authorize the session user against the GLOBAL identity database.
///
/// Users + roles live ONLY in the global identity DB: staff CRUD
/// (`bootstrap_owner`, `create_staff`, `update_staff_scoped`) writes there,
/// and per-store databases never receive user rows — they run the same
/// migrations but the `users` table stays empty by design.
///
/// Commands that open a store-scoped DB (`open_store`) MUST authorize with
/// this helper before touching the store connection. Running
/// `require_permission_for_user` against the store connection always fails
/// with "user not found" for every caller — owner included — because the
/// lookup queries the store DB's empty `users` table (this was the topology
/// Apply denial: "You don't have permission to do this." for everyone).
pub async fn require_permission_for_session(
    state: &AppState,
    session: &SessionContext,
    required: &str,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &session.user_id, required)
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

    /// Pins the identity-DB authorization design: scoped commands must
    /// authorize the session user against the GLOBAL identity DB, never the
    /// store-scoped DB. The global DB holds the owner row (STAFF_UPDATE
    /// granted); the store DB runs the same migrations but has NO users, so
    /// the old pattern (`require_permission_for_user` on the store
    /// connection) denied every caller — the topology Apply "You don't have
    /// permission to do this." bug.
    #[tokio::test]
    async fn session_permission_checks_global_identity_db_not_store_db() {
        use oz_core::session::SessionContext;

        use crate::state::AppState;

        // Global identity DB: migrated, roles seeded, owner created with a
        // FIXED id (create_user mints a UUID). This mirrors `bootstrap_owner`
        // (staff lives ONLY here).
        let conn = oz_core::migrations::fresh_db();
        {
            let store = Store::new(&conn);
            store.seed_default_roles().unwrap();
            conn.execute(
                "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
                 VALUES ('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, '2026-07-31T00:00:00.000Z', '2026-07-31T00:00:00.000Z')",
                [],
            )
            .unwrap();
        }

        let mut state = AppState::for_test_with_conn(conn);
        // Isolate the store DBs in a temp dir so the store-scoped file is
        // created fresh (empty `users` table) instead of reusing temp-dir
        // leftovers from other tests.
        let dir = std::env::temp_dir().join(format!("oz-authz-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        state.db_manager =
            platform_core::StoreDatabaseManager::new(dir.clone(), oz_core::migrations::ALL);

        let session = SessionContext::new(
            "user-owner".into(),
            "role-owner".into(),
            "t1".into(),
            "store-a".into(),
            "i1".into(),
            "store-pos".into(),
            None,
            0,
        );

        // The new gate: resolves the owner from the global DB -> allowed.
        require_permission_for_session(&state, &session, permissions::STAFF_UPDATE)
            .await
            .expect("owner must be authorized from the global identity DB");

        // The old gate: same user looked up on the store DB -> denied,
        // exactly the reported "You don't have permission to do this.".
        let store_conn = state
            .db_manager
            .open_store("store-a")
            .expect("open store db");
        let store_db = store_conn.lock().unwrap();
        let store = Store::new(&store_db);
        assert!(
            store.get_user("user-owner").unwrap().is_none(),
            "store DB must not contain global identity rows"
        );
        assert!(matches!(
            require_permission_for_user(&store, "user-owner", permissions::STAFF_UPDATE),
            Err(AppError::PermissionDenied(_))
        ));

        drop(store_db);
        drop(state);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
