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
        assert_eq!(cloned.user_id, ctx.user_id);
        assert_eq!(cloned.role_id, ctx.role_id);
    }

    #[test]
    fn session_context_debug_output() {
        let ctx = SessionContext::new(
            "u1".into(),
            "r1".into(),
            "t1".into(),
            "s1".into(),
            "i1".into(),
            "restaurant-pos".into(),
        );
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("u1"));
        assert!(debug.contains("s1"));
        assert!(debug.contains("restaurant-pos"));
    }

    #[test]
    fn session_context_empty_strings_accepted() {
        let ctx = SessionContext::new(
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        assert_eq!(ctx.user_id, "");
        assert_eq!(ctx.store_id, "");
        assert_eq!(ctx.type_key, "");
    }

    #[test]
    fn session_context_different_stores_are_independent() {
        let store_a = SessionContext::new(
            "u1".into(),
            "r1".into(),
            "t1".into(),
            "store-a".into(),
            "i1".into(),
            "pos".into(),
        );
        let store_b = SessionContext::new(
            "u2".into(),
            "r2".into(),
            "t2".into(),
            "store-b".into(),
            "i2".into(),
            "pos".into(),
        );
        assert_ne!(store_a.store_id, store_b.store_id);
        assert_ne!(store_a.user_id, store_b.user_id);
        assert_ne!(store_a.instance_id, store_b.instance_id);
    }

    #[test]
    fn session_context_all_fields_accessible() {
        let ctx = SessionContext::new(
            "user-42".into(),
            "role-admin".into(),
            "term-front".into(),
            "store-main".into(),
            "default-pos".into(),
            "restaurant-pos".into(),
        );
        assert_eq!(ctx.user_id, "user-42");
        assert_eq!(ctx.role_id, "role-admin");
        assert_eq!(ctx.terminal_id, "term-front");
        assert_eq!(ctx.store_id, "store-main");
        assert_eq!(ctx.instance_id, "default-pos");
        assert_eq!(ctx.type_key, "restaurant-pos");
    }
}
