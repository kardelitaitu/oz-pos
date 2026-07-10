//! Staff management — User CRUD + Role CRUD.

use rusqlite::params;

use crate::error::CoreError;
use crate::{Role, User};
use platform_core::rbac::ROLE_PRESETS;

use super::Store;

// ── Role CRUD ───────────────────────────────────────────────────

impl Store<'_> {
    /// Seed any built-in roles that do not yet exist in the database.
    ///
    /// Idempotent — uses `INSERT OR IGNORE` so roles that already exist
    /// (by their fixed id) are skipped. Safe to call on every startup
    /// or during the setup wizard.
    ///
    /// Returns the number of roles that were newly inserted.
    pub fn seed_default_roles(&self) -> Result<usize, CoreError> {
        let mut count = 0usize;
        for preset in ROLE_PRESETS {
            let role = preset.into_role();
            let result = self.conn.execute(
                "INSERT OR IGNORE INTO roles (id, name, description, permissions, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![role.id, role.name, role.description, role.permissions, role.created_at, role.updated_at],
            );
            count += result?;
        }
        Ok(count)
    }

    /// List all roles, ordered by name.
    pub fn list_roles(&self) -> Result<Vec<Role>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, permissions, created_at, updated_at FROM roles ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Role {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
                permissions: row.get("permissions")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single role by id.
    pub fn get_role(&self, id: &str) -> Result<Option<Role>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, permissions, created_at, updated_at FROM roles WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(Role {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
                permissions: row.get("permissions")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new role.
    pub fn create_role(
        &self,
        id: &str,
        name: &str,
        description: &str,
        permissions: &str,
    ) -> Result<Role, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let result = self.conn.execute(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name.trim(), description, permissions, now, now],
        );
        match result {
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(CoreError::Conflict {
                    entity: "role",
                    field: "name",
                });
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }
        Ok(Role {
            id: id.to_owned(),
            name: name.trim().to_owned(),
            description: description.to_owned(),
            permissions: permissions.to_owned(),
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

// ── User CRUD ───────────────────────────────────────────────────

impl Store<'_> {
    /// List all users, ordered by display_name.
    pub fn list_users(&self) -> Result<Vec<User>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at
             FROM users ORDER BY display_name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(User {
                id: row.get("id")?,
                username: row.get("username")?,
                pin_hash: row.get("pin_hash")?,
                display_name: row.get("display_name")?,
                role_id: row.get("role_id")?,
                is_active: row.get("is_active")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single user by id.
    pub fn get_user(&self, id: &str) -> Result<Option<User>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at
             FROM users WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(User {
                id: row.get("id")?,
                username: row.get("username")?,
                pin_hash: row.get("pin_hash")?,
                display_name: row.get("display_name")?,
                role_id: row.get("role_id")?,
                is_active: row.get("is_active")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(u) => Ok(Some(u)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Look up a user by username.
    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at
             FROM users WHERE username = ?1",
        )?;
        let result = stmt.query_row(params![username], |row| {
            Ok(User {
                id: row.get("id")?,
                username: row.get("username")?,
                pin_hash: row.get("pin_hash")?,
                display_name: row.get("display_name")?,
                role_id: row.get("role_id")?,
                is_active: row.get("is_active")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(u) => Ok(Some(u)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new user.
    pub fn create_user(
        &self,
        username: &str,
        pin_hash: &str,
        display_name: &str,
        role_id: &str,
    ) -> Result<User, CoreError> {
        if username.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "username",
                message: "username must not be empty".into(),
            });
        }
        if display_name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "display_name",
                message: "display name must not be empty".into(),
            });
        }

        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let result = self.conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, username.trim(), pin_hash, display_name.trim(), role_id, now, now],
        );
        match result {
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(CoreError::Conflict {
                    entity: "user",
                    field: "username",
                });
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        Ok(User {
            id,
            username: username.trim().to_owned(),
            pin_hash: pin_hash.to_owned(),
            display_name: display_name.trim().to_owned(),
            role_id: role_id.to_owned(),
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing user.
    pub fn update_user(
        &self,
        id: &str,
        username: &str,
        display_name: &str,
        role_id: &str,
        is_active: bool,
    ) -> Result<User, CoreError> {
        if display_name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "display_name",
                message: "display name must not be empty".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE users SET username = ?1, display_name = ?2, role_id = ?3, is_active = ?4, updated_at = ?5 WHERE id = ?6",
            params![username.trim(), display_name.trim(), role_id, is_active, now, id],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "user",
                id: id.to_owned(),
            });
        }
        self.get_user(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "user",
            id: id.to_owned(),
        })
    }

    /// Delete a user by id.
    pub fn delete_user(&self, id: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM users WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "user",
                id: id.to_owned(),
            });
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn seed_roles(conn: &Connection) {
        store(conn).seed_default_roles().unwrap();
    }

    fn seed_users(conn: &Connection) {
        store(conn).seed_default_roles().unwrap();
        conn.execute_batch(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-1', 'alice',   'hash_alice',   'Alice',   'role-cashier', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('user-2', 'bob',     'hash_bob',     'Bob',     'role-owner',   1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('user-3', 'carol',   'hash_carol',   'Carol',   'role-cashier', 0, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    // ── Role CRUD ───────────────────────────────────────────────────

    #[test]
    fn list_roles_empty_db() {
        let conn = fresh();
        let roles = store(&conn).list_roles().unwrap();
        assert!(roles.is_empty());
    }

    #[test]
    fn list_roles_seeded() {
        let conn = fresh();
        seed_roles(&conn);
        let roles = store(&conn).list_roles().unwrap();
        assert_eq!(roles.len(), 4);
        // Ordered by name: cashier, kitchen, manager, owner.
        assert_eq!(roles[0].name, "Cashier");
        assert_eq!(roles[0].id, "role-cashier");
        assert_eq!(roles[1].name, "Kitchen");
        assert_eq!(roles[1].id, "role-kitchen");
        assert!(roles[1].permissions.contains("kds:view"));
        assert!(roles[1].permissions.contains("kds:update"));
        assert_eq!(roles[2].name, "Manager");
        assert_eq!(roles[2].id, "role-manager");
        assert_eq!(roles[3].name, "Owner");
        assert_eq!(roles[3].id, "role-owner");
    }

    #[test]
    fn get_role_found() {
        let conn = fresh();
        seed_roles(&conn);
        let r = store(&conn).get_role("role-owner").unwrap().unwrap();
        assert_eq!(r.name, "Owner");
        assert_eq!(r.permissions, "[\"*\"]");
    }

    #[test]
    fn get_role_not_found() {
        let conn = fresh();
        let r = store(&conn).get_role("nope").unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn create_role_basic() {
        let conn = fresh();
        let r = store(&conn)
            .create_role(
                "role-viewer",
                "viewer",
                "Read-only access",
                "[\"sales:view\"]",
            )
            .unwrap();
        assert_eq!(r.name, "viewer");
        assert_eq!(r.description, "Read-only access");
        assert_eq!(r.permissions, "[\"sales:view\"]");
    }

    #[test]
    fn create_role_duplicate_name() {
        let conn = fresh();
        seed_roles(&conn);
        // 'Owner' is already taken by the preset — duplicate name should conflict.
        let err = store(&conn)
            .create_role("role-dup", "Owner", "Dup", "[]")
            .unwrap_err();
        assert!(matches!(err, CoreError::Conflict { entity, .. } if entity == "role"));
    }

    // ── User CRUD ───────────────────────────────────────────────────

    #[test]
    fn list_users_empty_db() {
        let conn = fresh();
        let users = store(&conn).list_users().unwrap();
        assert!(users.is_empty());
    }

    #[test]
    fn list_users_returns_all() {
        let conn = fresh();
        seed_users(&conn);
        let users = store(&conn).list_users().unwrap();
        assert_eq!(users.len(), 3);
        // Ordered by display_name: Alice, Bob, Carol.
        assert_eq!(users[0].username, "alice");
        assert_eq!(users[1].username, "bob");
        assert_eq!(users[2].username, "carol");
    }

    #[test]
    fn get_user_found() {
        let conn = fresh();
        seed_users(&conn);
        let u = store(&conn).get_user("user-1").unwrap().unwrap();
        assert_eq!(u.username, "alice");
        assert_eq!(u.display_name, "Alice");
        assert_eq!(u.role_id, "role-cashier");
        assert!(u.is_active);
    }

    #[test]
    fn get_user_not_found() {
        let conn = fresh();
        let u = store(&conn).get_user("nope").unwrap();
        assert!(u.is_none());
    }

    #[test]
    fn get_user_by_username_found() {
        let conn = fresh();
        seed_users(&conn);
        let u = store(&conn).get_user_by_username("bob").unwrap().unwrap();
        assert_eq!(u.id, "user-2");
        assert_eq!(u.display_name, "Bob");
    }

    #[test]
    fn get_user_by_username_not_found() {
        let conn = fresh();
        let u = store(&conn).get_user_by_username("nobody").unwrap();
        assert!(u.is_none());
    }

    #[test]
    fn get_user_inactive_user() {
        let conn = fresh();
        seed_users(&conn);
        let u = store(&conn).get_user("user-3").unwrap().unwrap();
        assert_eq!(u.username, "carol");
        assert!(!u.is_active);
    }

    #[test]
    fn create_user_minimal() {
        let conn = fresh();
        seed_roles(&conn);
        let u = store(&conn)
            .create_user("diana", "hash_diana", "Diana", "role-cashier")
            .unwrap();
        assert_eq!(u.username, "diana");
        assert_eq!(u.display_name, "Diana");
        assert_eq!(u.role_id, "role-cashier");
        assert!(u.is_active);
        assert!(!u.id.is_empty());
    }

    #[test]
    fn create_user_empty_username() {
        let conn = fresh();
        seed_roles(&conn);
        let err = store(&conn)
            .create_user("", "hash", "Diana", "role-cashier")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "username"));
    }

    #[test]
    fn create_user_empty_display_name() {
        let conn = fresh();
        seed_roles(&conn);
        let err = store(&conn)
            .create_user("diana", "hash", "   ", "role-cashier")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "display_name"));
    }

    #[test]
    fn create_user_duplicate_username() {
        let conn = fresh();
        seed_users(&conn);
        let err = store(&conn)
            .create_user("alice", "hash2", "Alice 2", "role-owner")
            .unwrap_err();
        assert!(matches!(err, CoreError::Conflict { .. }));
    }

    #[test]
    fn update_user_basic() {
        let conn = fresh();
        seed_users(&conn);
        let updated = store(&conn)
            .update_user("user-1", "alice_new", "Alice Updated", "role-owner", true)
            .unwrap();
        assert_eq!(updated.username, "alice_new");
        assert_eq!(updated.display_name, "Alice Updated");
        assert_eq!(updated.role_id, "role-owner");
        assert!(updated.is_active);
        assert!(updated.updated_at.as_str() > "2025-01-01");
    }

    #[test]
    fn update_user_deactivate() {
        let conn = fresh();
        seed_users(&conn);
        let updated = store(&conn)
            .update_user("user-1", "alice", "Alice", "role-cashier", false)
            .unwrap();
        assert!(!updated.is_active);
    }

    #[test]
    fn update_user_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_user("nope", "u", "U", "role-owner", true)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_user_empty_display_name() {
        let conn = fresh();
        seed_users(&conn);
        let err = store(&conn)
            .update_user("user-1", "alice", "", "role-cashier", true)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "display_name"));
    }

    #[test]
    fn delete_user_removes_row() {
        let conn = fresh();
        seed_users(&conn);
        store(&conn).delete_user("user-3").unwrap();
        let u = store(&conn).get_user("user-3").unwrap();
        assert!(u.is_none());
    }

    #[test]
    fn delete_user_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_user("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }
}
