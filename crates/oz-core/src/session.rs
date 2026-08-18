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
}

impl SessionContext {
    /// Create a new session context.
    ///
    /// Eight positional parameters is justified because this is a struct
    /// constructor that sets all 8 fields at once. A builder would add
    /// ceremony without preventing misuse.
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
