/*
last audited 25-07-26 by RSA-Agent (oz-core slice A)
crate: oz-core | status: SAFE | lint: CLEAN
findings: immutable scope struct, TTL via expires_at sound; COR-5: expires_at None means never-expires and the type cannot enforce dev-only — verify production default TTL during settings.rs slice
next: verify default TTL enforcement in slice C | perf: N/A
*/
//! Session context — immutable scope resolved at login/startup.
//!
//! ADR #4: Every authenticated session carries a `SessionContext` that
//! binds the user to their resolved scope (store + instance + type).
//!
//! ADR #7 (TTL): Sessions now carry an optional `expires_at` unix timestamp.
//! When set, `resolve_session` rejects expired tokens with
//! `AppError::InvalidSession`. A background daemon purges expired entries
//! every 5 minutes.
//!
//! The frontend never passes `store_id` as a command parameter.
//! Instead, commands receive an opaque `session_token` which the backend
//! maps to this context via an in-memory session store.

use std::time::{SystemTime, UNIX_EPOCH};

/// Immutable session scope resolved at authentication time.
///
/// Each field encodes one level of the three-tier resolution hierarchy:
///
/// | Level | Field | Resolved From |
/// |---|---|---|
/// | 1 — Store | `store_id` | Device binding or user's primary store |
/// | 2 — Instance | `instance_id` | Device binding or instance resolution |
/// | 3 — Type | `type_key` | Instance's type (always implicit) |
///
/// The frontend never passes `store_id` as a command parameter.
/// Instead, commands receive an opaque `session_token` which the backend
/// maps to this context via an in-memory session store.
///
/// **Multi-terminal model**: Each POS terminal runs its own process with
/// its own `AppState`, but they all share the same `store_id` database.
/// The `(terminal_id, store_id)` pair uniquely identifies a terminal
/// instance within a store — two terminals bound to the same store have
/// different `terminal_id` values but identical `store_id`. All stores
/// within the same workspace instance share the same `instance_id`.
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// Authenticated user ID.
    pub user_id: String,
    /// User's active role ID.
    pub role_id: String,
    /// Terminal/device ID.
    pub terminal_id: String,
    /// Active store ID — determines which database file to open.
    pub store_id: String,
    /// Active workspace instance ID.
    pub instance_id: String,
    /// Workspace type key — determines which React component to render.
    pub type_key: String,
    /// Unix timestamp (seconds) after which the session is considered
    /// expired. `None` means the session never expires (development mode).
    /// Set at creation time from the `session.ttl_seconds` setting.
    pub expires_at: Option<i64>,
    /// Unix timestamp (seconds) of when this session was created.
    /// Used for deterministic LRU eviction when the session store is full.
    pub created_at: i64,
    /// Optional Restaurant POS identifier for multi-KDS routing.
    ///
    /// When `Some(id)`, the session is scoped to a specific Restaurant POS
    /// and `resolve_store` opens that POS's database. When `None` (the
    /// default for all current sessions), falls back to `store_id` scoping
    /// — identical to today's behavior.
    ///
    /// Added as part of multi-KDS architecture (plan_multi_kds_one_location).
    pub restaurant_pos_id: Option<String>,
}

impl SessionContext {
    /// Create a new session context (legacy 8-field constructor).
    ///
    /// Sets `restaurant_pos_id` to `None` (default for all current sessions).
    /// For multi-KDS sessions, use [`new_with_restaurant_pos`] instead.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: String,
        role_id: String,
        terminal_id: String,
        store_id: String,
        instance_id: String,
        type_key: String,
        expires_at: Option<i64>,
        created_at: i64,
    ) -> Self {
        Self {
            user_id,
            role_id,
            terminal_id,
            store_id,
            instance_id,
            type_key,
            expires_at,
            created_at,
            restaurant_pos_id: None,
        }
    }

    /// Create a session context scoped to a specific Restaurant POS.
    ///
    /// Used by KDS devices that need to be isolated to a single Restaurant POS.
    /// When `restaurant_pos_id` is `None`, behaves identically to [`new`].
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_restaurant_pos(
        user_id: String,
        role_id: String,
        terminal_id: String,
        store_id: String,
        instance_id: String,
        type_key: String,
        expires_at: Option<i64>,
        created_at: i64,
        restaurant_pos_id: Option<String>,
    ) -> Self {
        Self {
            user_id,
            role_id,
            terminal_id,
            store_id,
            instance_id,
            type_key,
            expires_at,
            created_at,
            restaurant_pos_id,
        }
    }

    /// Returns `true` if the session has an expiry timestamp and that
    /// timestamp is in the past (relative to the system clock).
    ///
    /// A session with `expires_at: None` is never considered expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|ts| {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                now >= ts
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
