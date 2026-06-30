//! Role-Based Access Control primitives.
//!
//! Provides the [`Role`] and [`Permission`] types used by the
//! platform's authorisation layer. These are stub data structures
//! that will be fleshed out with enforcement logic in a later phase.

use serde::{Deserialize, Serialize};

/// A named role with an optional set of permissions.
///
/// Roles are the primary mechanism for grouping permissions —
/// staff members are assigned a role, and the role determines
/// what actions they can perform in the POS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Unique identifier (UUID or short slug like "role-cashier").
    pub id: String,
    /// Human-readable name (e.g. "Owner", "Manager", "Cashier").
    pub name: String,
    /// Optional description of what this role covers.
    pub description: String,
    /// JSON-encoded array of permission strings (e.g. `["sales:void"]`).
    pub permissions: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}

impl Role {
    /// Create a new role with no permissions.
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
}

/// A single permission string representing an action in the POS.
///
/// Permissions follow the format `<domain>:<action>` where domain is
/// the feature area (sales, products, settings, staff, etc.) and
/// action is the specific operation (create, read, update, delete,
/// void, export, etc.).
///
/// # Examples
///
/// - `"sales:void"` — void a completed sale
/// - `"products:edit"` — create/update/delete products
/// - `"settings:edit"` — modify store settings
/// - `"staff:manage"` — create/update/delete staff users
/// - `"reports:view"` — view sales reports
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permission {
    /// The permission string (e.g. `"sales:void"`).
    pub name: String,
    /// Human-readable description of what this permission grants.
    pub description: String,
}

impl Permission {
    /// Create a new permission.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
        }
    }

    /// Set the description (builder-style).
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

/// Well-known built-in role ids.
pub mod builtin_roles {
    /// Owner — full access to all features and settings.
    pub const OWNER: &str = "role-owner";
    /// Manager — can manage products, categories, and view reports.
    pub const MANAGER: &str = "role-manager";
    /// Cashier — can process sales and manage the daily register.
    pub const CASHIER: &str = "role-cashier";
}

/// Well-known permission strings.
pub mod permissions {
    /// Void a completed sale.
    pub const SALES_VOID: &str = "sales:void";
    /// Create, update, or delete products.
    pub const PRODUCTS_EDIT: &str = "products:edit";
    /// Modify store settings.
    pub const SETTINGS_EDIT: &str = "settings:edit";
    /// Manage staff users.
    pub const STAFF_MANAGE: &str = "staff:manage";
    /// View sales reports.
    pub const REPORTS_VIEW: &str = "reports:view";
    /// View audit log.
    pub const AUDIT_VIEW: &str = "audit:view";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_role() {
        let r = Role::new("role-cashier", "Cashier");
        assert_eq!(r.id, "role-cashier");
        assert_eq!(r.name, "Cashier");
        assert!(r.description.is_empty());
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
    fn new_permission() {
        let p = Permission::new("sales:void");
        assert_eq!(p.name, "sales:void");
        assert!(p.description.is_empty());
    }

    #[test]
    fn permission_with_description() {
        let p = Permission::new("sales:void").with_description("Void a sale");
        assert_eq!(p.description, "Void a sale");
    }

    #[test]
    fn permission_display() {
        let p = Permission::new("products:edit");
        assert_eq!(p.to_string(), "products:edit");
    }

    #[test]
    fn role_serde_roundtrip() {
        let r = Role::new("role-owner", "Owner").with_description("Full access");
        let json = serde_json::to_string(&r).unwrap();
        let back: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn builtin_role_constants_are_distinct() {
        assert_ne!(builtin_roles::OWNER, builtin_roles::MANAGER);
        assert_ne!(builtin_roles::MANAGER, builtin_roles::CASHIER);
    }

    #[test]
    fn permission_constants_are_well_formed() {
        assert!(permissions::SALES_VOID.contains(':'));
        assert!(permissions::PRODUCTS_EDIT.contains(':'));
        assert!(permissions::SETTINGS_EDIT.contains(':'));
        assert!(permissions::STAFF_MANAGE.contains(':'));
        assert!(permissions::REPORTS_VIEW.contains(':'));
        assert!(permissions::AUDIT_VIEW.contains(':'));
    }
}
