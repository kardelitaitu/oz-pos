//! Staff management — User CRUD + Role CRUD.
/*
last audited 25-07-26 by RSA-Agent (oz-core slice B5 closeout)
crate: oz-core | status: SAFE | lint: CLEAN
findings: exemplary — STAFF-07 three-tier rate limiter (per-account/per-device/global + backoff) with settings-driven tuning object; role presets upsert-sync converge model; permission resolution fails closed (unresolvable role = PermissionDenied, never a crash); username normalized + conflict mapped to typed error; default global assignment on create; pin_hash column (user PINs hashed — contrast COR-17 gift-card PINs)
next: none | perf: N/A
*/

use rusqlite::params;

use crate::error::CoreError;
use crate::subscription::{QuotaError, SubscriptionTier};
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
    /// Seed built-in roles from their presets.
    ///
    /// Idempotent and safe to call on every startup or during the setup
    /// wizard. Uses an upsert on the fixed preset id, so roles that already
    /// exist are re-synced to the preset (name, description, permissions) —
    /// existing databases converge on the current preset model instead of
    /// keeping stale grants.
    ///
    /// Returns the number of roles that were created or re-synced.
    pub fn seed_default_roles(&self) -> Result<usize, CoreError> {
        let mut count = 0usize;
        for preset in ROLE_PRESETS {
            let role = preset.into_role();
            let result = self.conn.execute(
                "INSERT INTO roles (id, name, description, permissions, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     description = excluded.description,
                     permissions = excluded.permissions",
                params![
                    role.id,
                    role.name,
                    role.description,
                    role.permissions,
                    role.created_at,
                    role.updated_at
                ],
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

    /// Count active staff users (non-owner) for C1.1 quota enforcement.
    ///
    /// The owner license-holder is not "staff" — the limit applies to team
    /// members (subscription-tiers.md §3). Inactive users don't consume quota.
    pub fn count_staff_users(&self) -> Result<i64, CoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM users WHERE is_active = 1 AND role_id != ?1",
            params![crate::builtin_roles::OWNER],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Enforce the subscription tier's staff-user limit before creating a
    /// new staff member (C1.1 — §9 pre-launch item 1: prevents revenue
    /// leakage from unlimited Free/Plus team accounts).
    ///
    /// When the tier's `max_staff_users()` cap is reached, returns
    /// [`QuotaError::StaffLimit`] (surfaced as `SubscriptionLimitExceeded`,
    /// which the UI maps to an upgrade CTA). Unlimited tiers (`None`) pass.
    pub fn enforce_staff_quota(&self, tier: &SubscriptionTier) -> Result<(), CoreError> {
        if let Some(limit) = tier.max_staff_users() {
            let current = self.count_staff_users()?;
            if current >= limit {
                return Err(QuotaError::StaffLimit {
                    tier: tier.name().into(),
                    limit,
                    current,
                }
                .into());
            }
        }
        Ok(())
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
    /// [`Self::require_permission`], plus the assignment's branch/workspace scope
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
    // STAFF-07 (audit-open-findings): per-account throttling is now combined with
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
#[path = "staff_tests.rs"]
mod tests;
