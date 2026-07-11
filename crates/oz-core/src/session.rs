//! Session context — immutable scope resolved at login/startup.
//!
//! ADR #4: Every authenticated session carries a `SessionContext` that
//! binds the user to their resolved scope (store + instance + type).
//!
//! This context is created once during session resolution and never
//! mutated. All Tauri commands read `store_id` from this context, not
//! from frontend parameters.

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
}

impl SessionContext {
    /// Create a new session context.
    pub fn new(
        user_id: String,
        role_id: String,
        terminal_id: String,
        store_id: String,
        instance_id: String,
        type_key: String,
    ) -> Self {
        Self {
            user_id,
            role_id,
            terminal_id,
            store_id,
            instance_id,
            type_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_context_creation() {
        let ctx = SessionContext::new(
            "user-1".into(),
            "role-cashier".into(),
            "term-1".into(),
            "store-downtown".into(),
            "default-restaurant-pos".into(),
            "restaurant-pos".into(),
        );
        assert_eq!(ctx.user_id, "user-1");
        assert_eq!(ctx.role_id, "role-cashier");
        assert_eq!(ctx.terminal_id, "term-1");
        assert_eq!(ctx.store_id, "store-downtown");
        assert_eq!(ctx.instance_id, "default-restaurant-pos");
        assert_eq!(ctx.type_key, "restaurant-pos");
    }

    #[test]
    fn session_context_clone() {
        let ctx = SessionContext::new(
            "u1".into(),
            "r1".into(),
            "t1".into(),
            "s1".into(),
            "i1".into(),
            "type1".into(),
        );
        let cloned = ctx.clone();
        assert_eq!(cloned.store_id, ctx.store_id);
    }
}
