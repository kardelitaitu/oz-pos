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
        // Every grant must be registered, and sensitive keys must never ride
        // a family wildcard (ADR #35 D3 / spec 0046). The global `*` wildcard
        // is reserved for the Owner seed, which uses a direct insert and is
        // never validated here.
        let grants: Vec<String> =
            serde_json::from_str(permissions).map_err(|e| CoreError::Validation {
                field: "permissions",
                message: format!("permissions must be a JSON array of strings: {e}"),
            })?;
        platform_core::permission_registry::validate_grants(&grants, false).map_err(|errors| {
            let message = errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            CoreError::Validation {
                field: "permissions",
                message,
            }
        })?;
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

    /// The centralized fail-closed authorization gate (ADR #35 D3 / spec
    /// 0047): resolve `user_id` to their role and verify the role grants
    /// `required`.
    ///
    /// Denies by default: an unregistered permission key is rejected even
    /// for the `"*"` Owner grant, and an unresolvable user or role denies
    /// rather than erroring internally. The role resolves through the
    /// user's assignment when one exists (0048); legacy users without an
    /// assignment fall back to `users.role_id`.
    pub fn require_permission(&self, user_id: &str, required: &str) -> Result<(), CoreError> {
        let assignment = self.assignment_for_user(user_id)?;
        self.authorize_with(user_id, required, &assignment)
    }

    /// The scope-aware gate (ADR #35 D5 / spec 0048): same as
    /// [`require_permission`], plus the assignment's branch/workspace scope
    /// is evaluated for scoped assignments. Global assignments and legacy
    /// users without an assignment are not scope-restricted.
    pub fn require_permission_scoped(
        &self,
        user_id: &str,
        required: &str,
        branch: Option<&str>,
        workspace: Option<&str>,
    ) -> Result<(), CoreError> {
        let assignment = self.assignment_for_user(user_id)?;
        // Scoped assignments deny when the requested branch/workspace is out
        // of scope; global assignments and legacy users (no assignment) are
        // not scope-restricted (ADR #35 D5).
        if assignment
            .as_ref()
            .is_some_and(|a| !a.matches_scope(branch, workspace))
        {
            return Err(CoreError::PermissionDenied(format!(
                "branch/workspace out of scope for user {user_id}"
            )));
        }
        self.authorize_with(user_id, required, &assignment)
    }

    /// Shared gate body: registry deny-by-default, user resolution + active
    /// check, role resolution (assignment first, `users.role_id` fallback),
    /// then `role.authorize`.
    fn authorize_with(
        &self,
        user_id: &str,
        required: &str,
        assignment: &Option<crate::db::assignments::Assignment>,
    ) -> Result<(), CoreError> {
        // Deny by default: an unregistered permission key is rejected even
        // for the global `"*"` Owner grant — the registry is the only
        // vocabulary authorization speaks (ADR #35 D3 / spec 0046 + 0047).
        if !platform_core::permission_registry::is_registered(required) {
            return Err(CoreError::PermissionDenied(format!(
                "unknown permission: {required}"
            )));
        }
        let user = self
            .get_user(user_id)?
            .ok_or_else(|| CoreError::PermissionDenied("user not found".into()))?;
        if !user.is_active {
            return Err(CoreError::PermissionDenied("user is inactive".into()));
        }
        // The role resolves through the assignment when one exists; legacy
        // users without an assignment fall back to `users.role_id`.
        let role_id = assignment
            .as_ref()
            .map(|a| a.role_id.as_str())
            .unwrap_or(user.role_id.as_str());
        // Fail closed: an unresolvable role is a denial, never an internal
        // error (a role row deleted out from under a user must not surface
        // as a crash-adjacent 500 to the frontend).
        let role = self
            .get_role(role_id)?
            .ok_or_else(|| CoreError::PermissionDenied(format!("role {role_id} not found")))?;
        role.authorize(required)
            .map_err(|e| CoreError::PermissionDenied(e.to_string()))
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
        if username.len() > 100 {
            return Err(CoreError::Validation {
                field: "username",
                message: format!(
                    "username must not exceed 100 characters, got {}",
                    username.len()
                ),
            });
        }
        if display_name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "display_name",
                message: "display name must not be empty".into(),
            });
        }
        if display_name.len() > 255 {
            return Err(CoreError::Validation {
                field: "display_name",
                message: format!(
                    "display name must not exceed 255 characters, got {}",
                    display_name.len()
                ),
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

        // Every user gets their single effective assignment (ADR #35 D5 /
        // spec 0048): a default global-mode assignment mirroring the role.
        self.conn.execute(
            "INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope)
             VALUES (?1, ?2, 'global', 'all', 'all')",
            params![id, role_id],
        )?;

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
        // Keep the assignment role in sync — the scope columns and scope rows
        // of an existing assignment are preserved (only the role follows).
        self.conn.execute(
            "INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope, updated_at)
             VALUES (?1, ?2, 'global', 'all', 'all', ?3)
             ON CONFLICT(user_id) DO UPDATE SET role_id = excluded.role_id, updated_at = excluded.updated_at",
            params![id, role_id, now],
        )?;
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
        // role-lite: a narrow custom role (sales:view only) standing in for
        // the retired cashier role � the gate tests below pin its LIMITED
        // grants (view yes, void no), which role-staff no longer provides.
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-lite', 'Lite', 'Limited sales view', '[\"sales:view\"]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at) VALUES
                ('user-1', 'alice',   'hash_alice',   'Alice',   'role-lite',   1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('user-2', 'bob',     'hash_bob',     'Bob',     'role-owner',  1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('user-3', 'carol',   'hash_carol',   'Carol',   'role-lite',   0, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    // ── Authorization gate (0047) ──────────────────────────────────

    #[test]
    fn gate_denies_unregistered_permission_even_for_owner() {
        let conn = fresh();
        seed_users(&conn);
        // bob is role-owner with the global `"*"` grant — a typo'd or
        // future key must STILL deny: unregistered means deny-by-default.
        let err = store(&conn)
            .require_permission("user-2", "sales:typo")
            .unwrap_err();
        assert!(matches!(err, CoreError::PermissionDenied(_)), "got {err:?}");
    }

    #[test]
    fn gate_allows_registered_permission_for_owner() {
        let conn = fresh();
        seed_users(&conn);
        // The global `*` grant covers every registered key.
        assert!(
            store(&conn)
                .require_permission("user-2", "sales:void")
                .is_ok()
        );
        assert!(
            store(&conn)
                .require_permission("user-2", "settings:edit")
                .is_ok()
        );
        assert!(
            store(&conn)
                .require_permission("user-2", "kds:update")
                .is_ok()
        );
    }

    #[test]
    fn gate_denies_unknown_user() {
        let conn = fresh();
        seed_users(&conn);
        let err = store(&conn)
            .require_permission("no-such-user", "sales:view")
            .unwrap_err();
        assert!(matches!(err, CoreError::PermissionDenied(_)), "got {err:?}");
    }

    #[test]
    fn gate_denies_inactive_user() {
        let conn = fresh();
        seed_users(&conn);
        let err = store(&conn)
            .require_permission("user-3", "sales:view")
            .unwrap_err();
        assert!(matches!(err, CoreError::PermissionDenied(_)), "got {err:?}");
    }

    #[test]
    fn gate_denies_user_with_unresolvable_role() {
        let conn = fresh();
        seed_roles(&conn);
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-ghost', 'ghost', 'h', 'Ghost', 'role-staff', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        // The role row disappears out from under the user. FK enforcement
        // normally prevents this, but defense-in-depth demands the gate fail
        // closed even when it happens (FKs off, partial migration, tampered
        // DB) — so the test simulates it with FK enforcement off.
        conn.execute_batch("PRAGMA foreign_keys = OFF; DELETE FROM roles WHERE id = 'role-staff';")
            .unwrap();
        // Fail-closed: an unresolvable role is a denial, not an internal error.
        let err = store(&conn)
            .require_permission("user-ghost", "sales:view")
            .unwrap_err();
        assert!(matches!(err, CoreError::PermissionDenied(_)), "got {err:?}");
    }

    #[test]
    fn gate_denies_permission_not_granted_to_role() {
        let conn = fresh();
        seed_users(&conn);
        // alice is cashier: has sales:view but not sales:void.
        assert!(
            store(&conn)
                .require_permission("user-1", "sales:view")
                .is_ok()
        );
        let err = store(&conn)
            .require_permission("user-1", "sales:void")
            .unwrap_err();
        assert!(matches!(err, CoreError::PermissionDenied(_)), "got {err:?}");
    }

    #[test]
    fn gate_resolves_wildcard_grants_via_registry() {
        let conn = fresh();
        seed_roles(&conn);
        store(&conn)
            .create_role("role-wild", "Wild", "tables wildcard", "[\"tables:*\"]")
            .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-op', 'op', 'h', 'Op', 'role-wild', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        // tables:* has no sensitive keys, so the wildcard is a valid grant
        // and the gate resolves every operational tables action through it.
        assert!(
            store(&conn)
                .require_permission("user-op", "tables:assign")
                .is_ok()
        );
        assert!(
            store(&conn)
                .require_permission("user-op", "tables:close")
                .is_ok()
        );
        assert!(
            store(&conn)
                .require_permission("user-op", "sales:view")
                .is_err()
        );
    }

    #[test]
    fn gate_resolves_sensitive_keys_only_by_explicit_grant() {
        let conn = fresh();
        seed_roles(&conn);
        // Explicit sensitive grant passes; a role without it is denied.
        store(&conn)
            .create_role("role-v", "Void", "explicit void", "[\"sales:void\"]")
            .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-v', 'v', 'h', 'V', 'role-v', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        assert!(
            store(&conn)
                .require_permission("user-v", "sales:void")
                .is_ok()
        );
        assert!(
            store(&conn)
                .require_permission("user-v", "sales:refund")
                .is_err()
        );
    }

    // ── Assignment-aware gate (0048 cycle 2) ──────────────────────────

    /// Seed a user whose legacy `role_id` and assignment role disagree, so
    /// the tests prove the gate resolves through the ASSIGNMENT.
    fn seed_assigned_user(conn: &rusqlite::Connection, user_id: &str, role_id: &str) {
        seed_roles(conn);
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'h', ?3, ?4, 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            params![user_id, user_id, user_id, role_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope)
             VALUES (?1, 'role-staff', 'global', 'all', 'all')",
            params![user_id],
        )
        .unwrap();
    }

    #[test]
    fn gate_resolves_role_through_assignment_not_role_id() {
        let conn = fresh();
        // Legacy column says role-manager; the assignment says role-staff.
        // role-manager grants settings:edit; role-staff does NOT — so the
        // denial proves the gate authorized through the ASSIGNMENT.
        seed_assigned_user(&conn, "user-a", "role-manager");
        let store = store(&conn);
        assert!(store.require_permission("user-a", "sales:void").is_ok());
        assert!(
            store.require_permission("user-a", "settings:edit").is_err(),
            "assignment role (role-staff) must win over the legacy role_id (role-manager)"
        );
    }

    #[test]
    fn gate_falls_back_to_role_id_when_no_assignment() {
        let conn = fresh();
        seed_users(&conn);
        // alice (cashier) has no assignment row — legacy fallback applies.
        assert!(
            store(&conn)
                .require_permission("user-1", "sales:view")
                .is_ok()
        );
        assert!(
            store(&conn)
                .require_permission("user-1", "sales:void")
                .is_err()
        );
    }

    #[test]
    fn gate_scoped_denies_workspace_out_of_scope() {
        let conn = fresh();
        seed_roles(&conn);
        // scoped to the kds workspace only.
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-k', 'k', 'h', 'K', 'role-staff', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        conn.execute_batch(
            "INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope)
                 VALUES ('user-k', 'role-staff', 'scoped', 'all', 'list');
             INSERT INTO assignment_workspaces (assignment_user_id, workspace_key)
                 VALUES ('user-k', 'kds');",
        )
        .unwrap();
        let store = store(&conn);
        // In the kds workspace the grant holds; anywhere else it denies.
        assert!(
            store
                .require_permission_scoped("user-k", "sales:view", None, Some("kds"))
                .is_ok()
        );
        assert!(
            store
                .require_permission_scoped("user-k", "sales:view", None, Some("retail-pos"))
                .is_err()
        );
        assert!(
            store
                .require_permission_scoped("user-k", "sales:view", None, None)
                .is_err()
        );
    }

    #[test]
    fn gate_scoped_denies_branch_out_of_scope() {
        let conn = fresh();
        seed_roles(&conn);
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
             VALUES ('user-b', 'b', 'h', 'B', 'role-staff', 1, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();
        conn.execute_batch(
            "INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope)
                 VALUES ('user-b', 'role-staff', 'scoped', 'list', 'all');
             INSERT INTO assignment_branches (assignment_user_id, branch_id)
                 VALUES ('user-b', 'store-a');",
        )
        .unwrap();
        let store = store(&conn);
        assert!(
            store
                .require_permission_scoped("user-b", "sales:view", Some("store-a"), None)
                .is_ok()
        );
        assert!(
            store
                .require_permission_scoped("user-b", "sales:view", Some("store-b"), None)
                .is_err()
        );
    }

    #[test]
    fn gate_scoped_global_assignment_ignores_scope() {
        let conn = fresh();
        seed_assigned_user(&conn, "user-g", "role-staff");
        let store = store(&conn);
        // Global assignment: scope context is ignored, like the plain gate.
        assert!(
            store
                .require_permission_scoped("user-g", "sales:view", None, Some("retail-pos"))
                .is_ok()
        );
        assert!(
            store
                .require_permission_scoped("user-g", "sales:view", Some("store-z"), None)
                .is_ok()
        );
    }

    #[test]
    fn gate_scoped_legacy_user_without_assignment_has_no_scope_restriction() {
        let conn = fresh();
        seed_users(&conn);
        // user-1 (cashier) predates assignments — scope is not evaluated.
        assert!(
            store(&conn)
                .require_permission_scoped("user-1", "sales:view", None, Some("retail-pos"))
                .is_ok()
        );
    }

    #[test]
    fn create_user_writes_default_global_assignment() {
        let conn = fresh();
        seed_roles(&conn);
        let store = store(&conn);
        let user = store
            .create_user("newbie", "hash", "Newbie", "role-staff")
            .unwrap();
        let a = store
            .assignment_for_user(&user.id)
            .unwrap()
            .expect("create_user must write an assignment");
        assert_eq!(a.role_id, "role-staff");
        assert_eq!(a.scope_mode, crate::db::assignments::ScopeMode::Global);
    }

    #[test]
    fn update_user_syncs_assignment_role() {
        let conn = fresh();
        seed_roles(&conn);
        let store = store(&conn);
        let user = store
            .create_user("switcher", "hash", "Switcher", "role-staff")
            .unwrap();
        store
            .update_user(&user.id, "switcher", "Switcher", "role-manager", true)
            .unwrap();
        let a = store.assignment_for_user(&user.id).unwrap().unwrap();
        assert_eq!(
            a.role_id, "role-manager",
            "assignment role must follow the update"
        );
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
        // Ordered by name: admin, auditor, custom, manager, owner, staff.
        assert_eq!(roles[0].name, "Admin");
        assert_eq!(roles[0].id, "role-admin");
        assert_eq!(roles[1].name, "Auditor");
        assert_eq!(roles[1].id, "role-auditor");
        assert_eq!(roles[2].name, "Custom");
        assert_eq!(roles[2].id, "role-custom");
        assert_eq!(roles[2].permissions, "[]");
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

    #[test]
    fn create_role_rejects_unregistered_permission() {
        let conn = fresh();
        let err = store(&conn)
            .create_role("role-x", "X", "x", "[\"sales:typo\"]")
            .unwrap_err();
        assert!(
            matches!(err, CoreError::Validation { field, .. } if field == "permissions"),
            "unregistered key must fail validation: {err}"
        );
    }

    #[test]
    fn create_role_rejects_sensitive_family_wildcard() {
        let conn = fresh();
        let err = store(&conn)
            .create_role("role-x", "X", "x", "[\"sales:*\"]")
            .unwrap_err();
        assert!(
            matches!(err, CoreError::Validation { field, .. } if field == "permissions"),
            "a wildcard covering sensitive keys must fail validation: {err}"
        );
    }

    #[test]
    fn create_role_accepts_valid_permission_set() {
        let conn = fresh();
        let r = store(&conn)
            .create_role(
                "role-x",
                "X",
                "x",
                "[\"sales:process\", \"products:*\", \"sales:void\"]",
            )
            .unwrap();
        assert_eq!(
            r.permissions,
            "[\"sales:process\", \"products:*\", \"sales:void\"]"
        );
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
        assert_eq!(u.role_id, "role-lite");
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
            .create_user("diana", "hash_diana", "Diana", "role-staff")
            .unwrap();
        assert_eq!(u.username, "diana");
        assert_eq!(u.display_name, "Diana");
        assert_eq!(u.role_id, "role-staff");
        assert!(u.is_active);
        assert!(!u.id.is_empty());
    }

    #[test]
    fn create_user_empty_username() {
        let conn = fresh();
        seed_roles(&conn);
        let err = store(&conn)
            .create_user("", "hash", "Diana", "role-staff")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "username"));
    }

    #[test]
    fn create_user_empty_display_name() {
        let conn = fresh();
        seed_roles(&conn);
        let err = store(&conn)
            .create_user("diana", "hash", "   ", "role-staff")
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
            .update_user("user-1", "alice", "Alice", "role-staff", false)
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
            .update_user("user-1", "alice", "", "role-staff", true)
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
            .create_user("ALICE_UPPER", "hash", "Alice Upper", "role-staff")
            .unwrap();
        assert_eq!(u.username, "alice_upper", "username should be lowercased");
    }

    #[test]
    fn create_user_mixed_case_normalized_to_lowercase() {
        let conn = fresh();
        seed_roles(&conn);
        let u = store(&conn)
            .create_user("MiXeDcAsE", "hash", "Mixed", "role-staff")
            .unwrap();
        assert_eq!(u.username, "mixedcase");
    }

    #[test]
    fn get_user_by_username_case_insensitive_after_normalization() {
        let conn = fresh();
        seed_roles(&conn);
        store(&conn)
            .create_user("CASE_USER", "hash", "Case User", "role-staff")
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
