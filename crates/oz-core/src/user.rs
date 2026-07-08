//! User and Role domain types — staff management for the POS.
//!
//! A [`User`] represents a staff member who can log in to the POS.
//! Each user has a [`Role`] that determines their permissions
//! (owner, manager, cashier).

use serde::{Deserialize, Serialize};

use crate::has_permission;
use platform_core::rbac::AuthorizationError;

/// A staff role with a set of permissions.
///
/// # Schema mapping
///
/// Maps 1:1 to the `roles` table (migration `007_customers.sql`).
/// The `permissions` field is stored as a JSON array of permission
/// strings (e.g., `["sales:void", "settings:edit"]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Internal row id.
    pub id: String,

    /// Unique role name (e.g. "owner", "manager", "cashier").
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// JSON array of permission strings.
    pub permissions: String,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl Role {
    /// Create a new role.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty after trimming.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let name = name.into().trim().to_owned();
        assert!(!name.is_empty(), "role name must not be empty");
        Self {
            id: id.into(),
            name,
            description: String::new(),
            permissions: "[]".into(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// Set the description (builder-style).
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Parse the JSON `permissions` field and check whether this role
    /// grants a specific action.
    ///
    /// Delegates to [`platform_core::rbac::has_permission`].
    /// Malformed JSON is treated as an empty permission set (deny all).
    ///
    /// # Examples
    ///
    /// ```
    /// use oz_core::Role;
    ///
    /// let role = Role::new("role-test", "Test")
    ///     .with_permissions_json("[\"sales:void\"]");
    /// assert!(role.has_permission("sales:void"));
    /// assert!(!role.has_permission("settings:edit"));
    /// ```
    #[must_use]
    pub fn has_permission(&self, required: &str) -> bool {
        let granted: Vec<String> = serde_json::from_str(&self.permissions).unwrap_or_default();
        has_permission(&granted, required)
    }

    /// Convenience: same as [`has_permission`] but returns
    /// [`AuthorizationError`] on failure for use with `?`.
    ///
    /// # Errors
    ///
    /// Returns [`AuthorizationError`] when the required permission
    /// is not granted by this role.
    pub fn authorize(&self, required: &str) -> Result<(), AuthorizationError> {
        if self.has_permission(required) {
            Ok(())
        } else {
            Err(AuthorizationError {
                required: required.to_owned(),
                role_name: self.name.clone(),
            })
        }
    }

    /// Builder-style: replace the `permissions` JSON string.
    ///
    /// Does **not** validate the JSON — the caller is responsible for
    /// supplying a valid JSON array of strings.
    #[must_use]
    pub fn with_permissions_json(mut self, json: &str) -> Self {
        self.permissions = json.to_owned();
        self
    }
}

/// A staff member who can log in to the POS.
///
/// # Schema mapping
///
/// Maps 1:1 to the `users` table (migration `007_customers.sql`).
/// The `pin_hash` stores a hashed PIN/password — the actual hash
/// algorithm is chosen by the authentication layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Internal row id (UUID v4).
    pub id: String,

    /// Unique login username.
    pub username: String,

    /// Hashed PIN/password (algorithm chosen by auth layer).
    pub pin_hash: String,

    /// Display name shown on the POS UI.
    pub display_name: String,

    /// FK to `roles.id`.
    pub role_id: String,

    /// Whether this user can log in.
    pub is_active: bool,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl User {
    /// Create a new user.
    ///
    /// Generates a fresh UUID for `id`.
    ///
    /// # Panics
    ///
    /// Panics if `username` or `display_name` is empty after trimming.
    pub fn new(
        username: impl Into<String>,
        pin_hash: impl Into<String>,
        display_name: impl Into<String>,
        role_id: impl Into<String>,
    ) -> Self {
        let username = username.into().trim().to_owned();
        assert!(!username.is_empty(), "username must not be empty");
        let display_name = display_name.into().trim().to_owned();
        assert!(!display_name.is_empty(), "display name must not be empty");

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            pin_hash: pin_hash.into(),
            display_name,
            role_id: role_id.into(),
            is_active: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

/// Well-known role ids used by the seed data.
pub mod builtin_roles {
    /// Owner — full access to all features and settings.
    pub const OWNER: &str = "role-owner";
    /// Manager — can manage products, categories, and view reports.
    pub const MANAGER: &str = "role-manager";
    /// Cashier — can process sales and manage the daily register.
    pub const CASHIER: &str = "role-cashier";
    /// Kitchen — can view and update KDS orders.
    pub const KITCHEN: &str = "role-kitchen";
}

/// Well-known seed user ids.
pub mod seed_users {
    /// Default admin user created by `oz init-db`.
    pub const ADMIN: &str = "user-admin";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_role() {
        let r = Role::new("role-cashier", "Cashier");
        assert_eq!(r.id, "role-cashier");
        assert_eq!(r.name, "Cashier");
        assert_eq!(r.description, "");
        assert_eq!(r.permissions, "[]");
    }

    #[test]
    #[should_panic(expected = "role name must not be empty")]
    fn role_panics_on_empty_name() {
        Role::new("r", "   ");
    }

    #[test]
    fn role_with_description() {
        let r = Role::new("r", "Role").with_description("A test role");
        assert_eq!(r.description, "A test role");
    }

    #[test]
    fn new_user() {
        let u = User::new("alice", "hashed_pin", "Alice", "role-cashier");
        assert_eq!(u.username, "alice");
        assert_eq!(u.display_name, "Alice");
        assert_eq!(u.role_id, "role-cashier");
        assert!(u.is_active);
        assert!(!u.id.is_empty());
    }

    #[test]
    #[should_panic(expected = "username must not be empty")]
    fn user_panics_on_empty_username() {
        User::new("", "pin", "Alice", "role-cashier");
    }

    #[test]
    #[should_panic(expected = "display name must not be empty")]
    fn user_panics_on_empty_display_name() {
        User::new("alice", "pin", "   ", "role-cashier");
    }

    #[test]
    fn serde_roundtrip() {
        let u = User::new("alice", "pin123", "Alice", "role-cashier");
        let json = serde_json::to_string(&u).unwrap();
        let back: User = serde_json::from_str(&json).unwrap();
        assert_eq!(back, u);
    }

    #[test]
    fn role_serde_roundtrip() {
        let r = Role::new("role-manager", "Manager")
            .with_description("Manages staff")
            .with_permissions_json("[\"sales:void\",\"staff:read\"]");
        let json = serde_json::to_string(&r).unwrap();
        let back: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn user_debug_output() {
        let u = User::new("bob", "hash", "Bob", "role-cashier");
        let debug = format!("{u:?}");
        assert!(debug.contains("bob"));
        assert!(debug.contains("Bob"));
    }

    #[test]
    fn role_debug_output() {
        let r = Role::new("role-owner", "Owner");
        let debug = format!("{r:?}");
        assert!(debug.contains("role-owner"));
        assert!(debug.contains("Owner"));
    }
}
