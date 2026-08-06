//! Staff management — User CRUD + Role CRUD.

use rusqlite::params;

use crate::error::CoreError;
use crate::{Role, User};
use platform_core::rbac::ROLE_PRESETS;

use super::Store;

/// Tuning knobs for the STAFF-07 login rate limiter.
///
/// Kept as a single value object so the limiter signature stays small
/// (clippy `too_many_arguments` clean) while callers can express per
/// account, per device, and global abuse policies independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginLimits {
    /// Max failed attempts per account within `window_secs`.
    pub max_attempts: usize,
    /// Sliding window length in seconds.
    pub window_secs: u64,
    /// Max failed attempts per device (across all usernames).
    pub device_max_attempts: usize,
    /// Max failed attempts globally (across all accounts/devices).
    pub global_max_attempts: usize,
    /// Upper bound for the exponential backoff, in seconds.
    pub max_backoff_secs: u64,
}

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
    ///
    /// Username is normalized to lowercase trimmed.
    pub fn create_user(
        &self,
        username: &str,
        pin_hash: &str,
        display_name: &str,
        role_id: &str,
    ) -> Result<User, CoreError> {
        let username = username.trim().to_lowercase();
        if username.is_empty() {
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
            params![id, username, pin_hash, display_name.trim(), role_id, now, now],
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
            username,
            pin_hash: pin_hash.to_owned(),
            display_name: display_name.trim().to_owned(),
            role_id: role_id.to_owned(),
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing user.
    ///
    /// Username is normalized to lowercase trimmed.
    pub fn update_user(
        &self,
        id: &str,
        username: &str,
        display_name: &str,
        role_id: &str,
        is_active: bool,
    ) -> Result<User, CoreError> {
        let username = username.trim().to_lowercase();
        if display_name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "display_name",
                message: "display name must not be empty".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE users SET username = ?1, display_name = ?2, role_id = ?3, is_active = ?4, updated_at = ?5 WHERE id = ?6",
            params![username, display_name.trim(), role_id, is_active, now, id],
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

    /// Update only the `pin_hash` for a user (STAFF-03 PIN rotation).
    ///
    /// Used by the staff-management PIN-reset path. The caller must already
    /// have validated and hashed the new PIN; this method never accepts a
    /// plaintext PIN.
    pub fn update_user_pin(&self, id: &str, pin_hash: &str) -> Result<User, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE users SET pin_hash = ?1, updated_at = ?2 WHERE id = ?3",
            params![pin_hash, now, id],
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

    // ── Login attempt rate limiting (persistent) ───────────────────
    //
    // STAFF-07 (audit/06): per-account throttling is now combined with
    // per-device and global abuse controls, and uses exponential backoff
    // instead of a fixed short lock. All rows are persisted in
    // `login_attempts` so lockouts survive app restarts.

    /// Exponential backoff for a lockout: `base * 2^(strikes-1)` capped at
    /// `max_secs`. `strikes` is how many times the limit has been breached
    /// (1 = first lockout).
    fn login_backoff_secs(base_secs: u64, strikes: usize, max_secs: u64) -> u64 {
        let shift = strikes.saturating_sub(1).min(16);
        base_secs.saturating_mul(1u64 << shift).min(max_secs).max(1)
    }

    /// Record a failed login attempt, enforcing per-account, per-device,
    /// and global limits within a sliding window (STAFF-07).
    ///
    /// Returns `Ok(remaining)` when the attempt is recorded and the caller
    /// may keep trying, or `Err(retry_after_secs)` when a limit is breached
    /// and the caller must wait. Expired attempts are pruned on every call;
    /// the data survives app restarts.
    pub fn record_login_attempt_scoped(
        &self,
        username: &str,
        device_id: Option<&str>,
        limits: LoginLimits,
    ) -> Result<Result<usize, u64>, CoreError> {
        let max_attempts = limits.max_attempts;
        let window_secs = limits.window_secs;
        let device_max_attempts = limits.device_max_attempts;
        let global_max_attempts = limits.global_max_attempts;
        let max_backoff_secs = limits.max_backoff_secs;
        let now = chrono::Utc::now().timestamp();
        let window_start = now - window_secs as i64;

        // Prune expired entries for the whole table (account + device +
        // global counters all share the same window).
        self.conn.execute(
            "DELETE FROM login_attempts WHERE attempted_at < ?1",
            params![window_start],
        )?;

        // ── Per-account limit ────────────────────────────────────
        let account_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM login_attempts WHERE username = ?1",
            params![username],
            |row| row.get(0),
        )?;
        if account_count >= max_attempts as i64 {
            let strikes = (account_count as usize / max_attempts).max(1);
            return Ok(Err(Self::login_backoff_secs(
                window_secs,
                strikes,
                max_backoff_secs,
            )));
        }

        // ── Per-device limit (across all usernames) ───────────────
        if let Some(device) = device_id.filter(|d| !d.is_empty()) {
            let device_count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM login_attempts WHERE device_id = ?1",
                params![device],
                |row| row.get(0),
            )?;
            if device_count >= device_max_attempts as i64 {
                let strikes = (device_count as usize / device_max_attempts).max(1);
                return Ok(Err(Self::login_backoff_secs(
                    window_secs,
                    strikes,
                    max_backoff_secs,
                )));
            }
        }

        // ── Global abuse limit ────────────────────────────────────
        let global_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM login_attempts", [], |row| row.get(0))?;
        if global_count >= global_max_attempts as i64 {
            let strikes = (global_count as usize / global_max_attempts).max(1);
            return Ok(Err(Self::login_backoff_secs(
                window_secs,
                strikes,
                max_backoff_secs,
            )));
        }

        // Record this attempt.
        self.conn.execute(
            "INSERT INTO login_attempts (id, username, device_id, attempted_at) VALUES (?1, ?2, ?3, ?4)",
            params![uuid::Uuid::now_v7().to_string(), username, device_id, now],
        )?;

        // Re-check after recording to catch the push-over-the-limit case.
        let new_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM login_attempts WHERE username = ?1",
            params![username],
            |row| row.get(0),
        )?;
        if new_count >= max_attempts as i64 {
            let strikes = (new_count as usize / max_attempts).max(1);
            return Ok(Err(Self::login_backoff_secs(
                window_secs,
                strikes,
                max_backoff_secs,
            )));
        }

        let remaining = max_attempts.saturating_sub(new_count as usize);
        Ok(Ok(remaining))
    }

    /// Record a failed login attempt with the legacy username-only
    /// signature. Kept for callers that have no device context; delegates
    /// to [`Self::record_login_attempt_scoped`] with device-independent
    /// defaults so every path gets at least per-account protection.
    pub fn record_login_attempt(
        &self,
        username: &str,
        max_attempts: usize,
        window_secs: u64,
    ) -> Result<Result<usize, u64>, CoreError> {
        self.record_login_attempt_scoped(
            username,
            None,
            LoginLimits {
                max_attempts,
                window_secs,
                device_max_attempts: max_attempts.saturating_mul(4),
                global_max_attempts: max_attempts.saturating_mul(20),
                max_backoff_secs: window_secs.saturating_mul(8),
            },
        )
    }

    /// Clear all recorded login attempts for `username` (call on
    /// successful login or admin reset).
    pub fn clear_login_attempts(&self, username: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM login_attempts WHERE username = ?1",
            params![username],
        )?;
        Ok(())
    }

    /// Clear all recorded login attempts for a device (STAFF-07). Called on
    /// successful login so a legitimate terminal is not held at a per-device
    /// limit; does not touch other devices or global history.
    pub fn clear_login_attempts_by_device(&self, device_id: &str) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM login_attempts WHERE device_id = ?1",
            params![device_id],
        )?;
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
        assert_eq!(roles.len(), 6);
        // Ordered by name: cashier, custom, kitchen, manager, owner, staff.
        assert_eq!(roles[0].name, "Cashier");
        assert_eq!(roles[0].id, "role-cashier");
        assert_eq!(roles[1].name, "Custom");
        assert_eq!(roles[1].id, "role-custom");
        assert_eq!(roles[1].permissions, "[]");
        assert_eq!(roles[2].name, "Kitchen");
        assert_eq!(roles[2].id, "role-kitchen");
        assert!(roles[2].permissions.contains("kds:view"));
        assert!(roles[2].permissions.contains("kds:update"));
        assert_eq!(roles[3].name, "Manager");
        assert_eq!(roles[3].id, "role-manager");
        assert_eq!(roles[4].name, "Owner");
        assert_eq!(roles[4].id, "role-owner");
        assert_eq!(roles[5].name, "Staff");
        assert_eq!(roles[5].id, "role-staff");
        assert!(!roles[5].permissions.contains("settings:read"));
        assert!(!roles[5].permissions.contains("settings:edit"));
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

    // ── Username normalization ────────────────────────────────────

    #[test]
    fn create_user_uppercase_normalized_to_lowercase() {
        let conn = fresh();
        seed_roles(&conn);
        let u = store(&conn)
            .create_user("ALICE_UPPER", "hash", "Alice Upper", "role-cashier")
            .unwrap();
        assert_eq!(u.username, "alice_upper", "username should be lowercased");
    }

    #[test]
    fn create_user_mixed_case_normalized_to_lowercase() {
        let conn = fresh();
        seed_roles(&conn);
        let u = store(&conn)
            .create_user("MiXeDcAsE", "hash", "Mixed", "role-cashier")
            .unwrap();
        assert_eq!(u.username, "mixedcase");
    }

    #[test]
    fn get_user_by_username_case_insensitive_after_normalization() {
        let conn = fresh();
        seed_roles(&conn);
        store(&conn)
            .create_user("CASE_USER", "hash", "Case User", "role-cashier")
            .unwrap();
        // Lookup with the normalized (lowercase) form should find it.
        let u = store(&conn)
            .get_user_by_username("case_user")
            .unwrap()
            .expect("user should be found by normalized username");
        assert_eq!(u.username, "case_user");
    }

    #[test]
    fn update_user_normalizes_username_to_lowercase() {
        let conn = fresh();
        seed_users(&conn);
        let updated = store(&conn)
            .update_user("user-1", "ALICE_NEW", "Alice Updated", "role-owner", true)
            .unwrap();
        assert_eq!(updated.username, "alice_new");
    }

    // ── PIN rotation (STAFF-03) ───────────────────────────────────

    #[test]
    fn update_user_pin_rotates_hash() {
        let conn = fresh();
        seed_users(&conn);
        let updated = store(&conn).update_user_pin("user-1", "new_hash").unwrap();
        assert_eq!(updated.pin_hash, "new_hash");
        // Verify persistence via a fresh read.
        let user = store(&conn).get_user("user-1").unwrap().unwrap();
        assert_eq!(user.pin_hash, "new_hash");
    }

    #[test]
    fn update_user_pin_not_found() {
        let conn = fresh();
        let err = store(&conn).update_user_pin("nope", "hash").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "user"));
    }

    // ── Login attempt rate limiting (STAFF-07) ──────────────────────

    #[test]
    fn login_backoff_first_lockout_is_window() {
        // strikes=1 → base * 2^0 = base.
        assert_eq!(Store::login_backoff_secs(60, 1, 3600), 60);
    }

    #[test]
    fn login_backoff_doubles_per_strike() {
        assert_eq!(Store::login_backoff_secs(60, 2, 3600), 120);
        assert_eq!(Store::login_backoff_secs(60, 3, 3600), 240);
    }

    #[test]
    fn login_backoff_caps_at_max() {
        assert_eq!(Store::login_backoff_secs(60, 100, 3600), 3600);
    }

    /// Test limits: 3/account, 20/device, 100/global within 60s.
    const LIMITS: LoginLimits = LoginLimits {
        max_attempts: 3,
        window_secs: 60,
        device_max_attempts: 20,
        global_max_attempts: 100,
        max_backoff_secs: 3600,
    };

    #[test]
    fn record_login_attempt_locks_after_max() {
        let conn = fresh();
        let s = store(&conn);
        // 3 max attempts within a 60s window.
        assert!(
            s.record_login_attempt_scoped("alice", None, LIMITS)
                .unwrap()
                .is_ok()
        );
        assert!(
            s.record_login_attempt_scoped("alice", None, LIMITS)
                .unwrap()
                .is_ok()
        );
        let third = s
            .record_login_attempt_scoped("alice", None, LIMITS)
            .unwrap();
        assert!(matches!(third, Err(retry) if retry >= 1));
        // Still locked on the next attempt.
        let fourth = s
            .record_login_attempt_scoped("alice", None, LIMITS)
            .unwrap();
        assert!(fourth.is_err());
    }

    #[test]
    fn record_login_attempt_returns_remaining() {
        let conn = fresh();
        let s = store(&conn);
        let first = s
            .record_login_attempt_scoped("alice", None, LIMITS)
            .unwrap()
            .unwrap();
        assert_eq!(first, 2);
        let second = s
            .record_login_attempt_scoped("alice", None, LIMITS)
            .unwrap()
            .unwrap();
        assert_eq!(second, 1);
    }

    #[test]
    fn device_limit_applies_across_usernames() {
        // device_max=2 → two usernames sharing a device exhaust the device
        // cap even though each account individually is below its limit.
        let conn = fresh();
        let s = store(&conn);
        let device_limits = LoginLimits {
            device_max_attempts: 2,
            ..LIMITS
        };
        assert!(
            s.record_login_attempt_scoped("alice", Some("term-1"), device_limits)
                .unwrap()
                .is_ok()
        );
        assert!(
            s.record_login_attempt_scoped("bob", Some("term-1"), device_limits)
                .unwrap()
                .is_ok()
        );
        let third = s
            .record_login_attempt_scoped("carol", Some("term-1"), device_limits)
            .unwrap();
        assert!(third.is_err(), "device cap must lock out across usernames");
    }

    #[test]
    fn different_devices_do_not_interfere() {
        let conn = fresh();
        let s = store(&conn);
        // Alice exhausts her own per-account limit (max_attempts: 1) on term-1.
        let account_limits = LoginLimits {
            max_attempts: 1,
            ..LIMITS
        };
        assert!(
            s.record_login_attempt_scoped("alice", Some("term-1"), account_limits)
                .unwrap()
                .is_err()
        );
        // term-2 is unaffected by term-1's failures.
        let ok_limits = LoginLimits {
            max_attempts: 5,
            ..LIMITS
        };
        assert!(
            s.record_login_attempt_scoped("alice", Some("term-2"), ok_limits)
                .unwrap()
                .is_ok()
        );
    }

    #[test]
    fn global_limit_locks_everyone() {
        // global_max=3 → a fourth attempt from any account is rejected even
        // though that account has never tried before.
        let conn = fresh();
        let s = store(&conn);
        let global_limits = LoginLimits {
            global_max_attempts: 3,
            ..LIMITS
        };
        assert!(
            s.record_login_attempt_scoped("a", Some("d1"), global_limits)
                .unwrap()
                .is_ok()
        );
        assert!(
            s.record_login_attempt_scoped("b", Some("d2"), global_limits)
                .unwrap()
                .is_ok()
        );
        assert!(
            s.record_login_attempt_scoped("c", Some("d3"), global_limits)
                .unwrap()
                .is_ok()
        );
        let fourth = s
            .record_login_attempt_scoped("d", Some("d4"), global_limits)
            .unwrap();
        assert!(fourth.is_err(), "global cap must lock out new accounts");
    }

    #[test]
    fn clear_login_attempts_by_device_only_clears_that_device() {
        let conn = fresh();
        let s = store(&conn);
        let device_limits = LoginLimits {
            device_max_attempts: 5,
            ..LIMITS
        };
        let _ = s.record_login_attempt_scoped("alice", Some("term-1"), device_limits);
        let _ = s.record_login_attempt_scoped("bob", Some("term-2"), device_limits);

        s.clear_login_attempts_by_device("term-1").unwrap();

        // term-1 is now free for its next attempt.
        assert!(
            s.record_login_attempt_scoped("alice", Some("term-1"), device_limits)
                .unwrap()
                .is_ok()
        );
        // term-2's history is untouched.
        let remaining = s
            .record_login_attempt_scoped("bob", Some("term-2"), device_limits)
            .unwrap();
        assert!(remaining.is_ok(), "other device history must survive");
    }

    #[test]
    fn legacy_record_login_attempt_delegates_to_scoped() {
        let conn = fresh();
        let s = store(&conn);
        assert!(s.record_login_attempt("alice", 3, 60).unwrap().is_ok());
        assert!(s.record_login_attempt("alice", 3, 60).unwrap().is_ok());
        assert!(s.record_login_attempt("alice", 3, 60).unwrap().is_err());
    }
}
