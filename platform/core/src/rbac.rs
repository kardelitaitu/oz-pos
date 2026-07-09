//! Role-Based Access Control primitives.
//!
//! Provides the [`Role`] and [`Permission`] types, the [`permissions`]
//! constant catalog, and the [`has_permission`] resolver that handles
//! wildcard permission strings (`"*"`, `"sales:*"`, `"sales:void"`).
//!
//! # Permission format
//!
//! Permissions follow `<domain>:<action>`. The wildcard `*` matches
//! everything. Domain-level wildcards like `sales:*` match every action
//! within that domain.
//!
//! # Examples
//!
//! ```
//! use platform_core::rbac::{has_permission, permissions};
//!
//! // Exact match
//! assert!(has_permission(&["sales:void".into()], permissions::SALES_VOID));
//!
//! // Domain wildcard
//! assert!(has_permission(&["sales:*".into()], "sales:process"));
//!
//! // Global wildcard
//! assert!(has_permission(&["*".into()], "settings:edit"));
//!
//! // Deny
//! assert!(!has_permission(&["products:read".into()], "sales:void"));
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Error returned when a role does not have the required permission.
///
/// Contains both the permission that was checked and the role name,
/// so callers can produce actionable error messages.
#[derive(Debug, Clone, thiserror::Error)]
#[error("permission denied: '{required}' — role '{role_name}' lacks this permission")]
pub struct AuthorizationError {
    /// The permission string that was required but not granted.
    pub required: String,
    /// The name of the role that was checked.
    pub role_name: String,
}

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

    /// Parse the JSON `permissions` field and check whether this role
    /// grants a specific action.
    ///
    /// Malformed JSON is treated as an empty permission set (deny all).
    ///
    /// # Examples
    ///
    /// ```
    /// use platform_core::rbac::Role;
    ///
    /// let role = Role::new("role-test", "Test")
    ///     .with_permissions_json("[\"sales:void\"]");
    /// assert!(role.has_permission("sales:void"));
    /// assert!(!role.has_permission("settings:edit"));
    ///
    /// // Wildcard
    /// let admin = Role::new("role-admin", "Admin")
    ///     .with_permissions_json("[\"*\"]");
    /// assert!(admin.has_permission("any:thing"));
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
    ///
    /// # Examples
    ///
    /// ```
    /// use platform_core::rbac::Role;
    ///
    /// let role = Role::new("role-test", "Cashier")
    ///     .with_permissions_json("[\"sales:process\"]");
    ///
    /// assert!(role.authorize("sales:process").is_ok());
    /// assert!(role.authorize("sales:void").is_err());
    /// ```
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

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

// ── Resolver ────────────────────────────────────────────────────────

/// Check whether a set of granted permission strings authorises a
/// required action.
///
/// Supports three levels of wildcard:
/// - `"*"` — grants **every** action
/// - `"<domain>:*"` — grants every action within that domain
///   (e.g. `"sales:*"` matches `"sales:void"`, `"sales:process"`, etc.)
/// - `"<domain>:<action>"` — grants only that exact action
///
/// The empty set grants nothing. Malformed permission strings (those
/// without a `:`) are matched exactly — they never match a wildcard
/// and are only matched by `"*"` or an identical string.
///
/// # Examples
///
/// ```
/// use platform_core::rbac::has_permission;
///
/// // Global wildcard
/// assert!(has_permission(&["*".into()], "sales:void"));
/// assert!(has_permission(&["*".into()], "anything:here"));
///
/// // Domain wildcard
/// assert!(has_permission(&["sales:*".into()], "sales:void"));
/// assert!(has_permission(&["sales:*".into()], "sales:process"));
/// assert!(!has_permission(&["sales:*".into()], "products:read"));
///
/// // Exact match
/// assert!(has_permission(&["sales:void".into()], "sales:void"));
/// assert!(!has_permission(&["sales:void".into()], "sales:process"));
///
/// // Multiple granted permissions (OR logic)
/// assert!(has_permission(
///     &["products:read".into(), "sales:process".into()],
///     "sales:process",
/// ));
/// assert!(!has_permission(
///     &["products:read".into(), "sales:process".into()],
///     "settings:edit",
/// ));
///
/// // Empty set denies everything
/// assert!(!has_permission(&[] as &[String], "sales:void"));
/// ```
#[must_use]
pub fn has_permission(granted: &[String], required: &str) -> bool {
    let (domain, _action) = required.split_once(':').unwrap_or((required, ""));
    let wildcard_domain = format!("{domain}:*");

    granted
        .iter()
        .any(|p| p == "*" || p == required || p == &wildcard_domain)
}

// ── Built-in role ids ───────────────────────────────────────────────

/// Well-known built-in role ids.
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

// ── Role presets ─────────────────────────────────────────────────────

/// A preset role definition with a fixed id, name, description, and set
/// of permission strings. Use [`RolePreset::permissions_json`] to get the
/// JSON array for storage.
pub struct RolePreset {
    /// Role id constant (e.g. `"role-owner"`).
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Short description.
    pub description: &'static str,
    /// Permission strings to grant.
    pub permissions: &'static [&'static str],
}

impl RolePreset {
    /// Serialise the permission list to a JSON string.
    ///
    /// Returns e.g. `"[\"sales:process\",\"sales:view\"]"`.
    pub fn permissions_json(&self) -> String {
        let items: Vec<String> = self
            .permissions
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect();
        format!("[{}]", items.join(","))
    }

    /// Build a [`Role`] from this preset, filling timestamps with the
    /// current UTC time.
    pub fn into_role(&self) -> Role {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Role {
            id: self.id.to_owned(),
            name: self.name.to_owned(),
            description: self.description.to_owned(),
            permissions: self.permissions_json(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// All built-in role presets bundled together for bulk seeding.
pub const ROLE_PRESETS: &[RolePreset] = &[
    RolePreset {
        id: builtin_roles::OWNER,
        name: "Owner",
        description: "Full access to all features and settings.",
        permissions: &["*"],
    },
    RolePreset {
        id: builtin_roles::MANAGER,
        name: "Manager",
        description: "Can manage products, inventory, sales, staff, and settings.",
        permissions: &[
            permissions::SALES_PROCESS,
            permissions::SALES_VOID,
            permissions::SALES_REFUND,
            permissions::SALES_VIEW,
            permissions::SALES_DISCOUNT,
            permissions::SALES_SPLIT,
            permissions::SALES_OVERRIDE_PRICE,
            permissions::PRODUCTS_CREATE,
            permissions::PRODUCTS_READ,
            permissions::PRODUCTS_UPDATE,
            permissions::PRODUCTS_DELETE,
            permissions::PRODUCTS_IMPORT,
            permissions::PRODUCTS_EXPORT,
            permissions::INVENTORY_VIEW,
            permissions::INVENTORY_ADJUST,
            permissions::INVENTORY_TRANSFER,
            permissions::INVENTORY_COUNT,
            permissions::STAFF_READ,
            permissions::STAFF_CREATE,
            permissions::STAFF_UPDATE,
            permissions::SETTINGS_READ,
            permissions::SETTINGS_EDIT,
            permissions::REPORTS_VIEW,
            permissions::REPORTS_EXPORT,
            permissions::REPORTS_SCHEDULE,
            permissions::SHIFTS_OPEN,
            permissions::SHIFTS_CLOSE,
            permissions::SHIFTS_VIEW_ANY,
            permissions::AUDIT_VIEW,
            permissions::AUDIT_EXPORT,
            permissions::PAYMENTS_CASH,
            permissions::PAYMENTS_CARD,
            permissions::PAYMENTS_REFUND,
            permissions::PAYMENTS_SETTLE,
            permissions::CUSTOMERS_CREATE,
            permissions::CUSTOMERS_VIEW,
            permissions::CUSTOMERS_EDIT,
            permissions::CUSTOMERS_DELETE,
            permissions::TABLES_ASSIGN,
            permissions::TABLES_MERGE,
            permissions::TABLES_SPLIT,
            permissions::TABLES_CLOSE,
            permissions::TABLES_CREATE,
            permissions::TABLES_EDIT,
            permissions::TABLES_DELETE,
            permissions::DISCOUNTS_APPLY,
            permissions::DISCOUNTS_CREATE,
            permissions::DISCOUNTS_MANAGE,
            permissions::WORKSPACES_SWITCH,
            permissions::PROMOTIONS_CREATE,
            permissions::PROMOTIONS_EDIT,
            permissions::PROMOTIONS_DELETE,
            permissions::PROMOTIONS_APPLY,
            permissions::TERMINALS_REGISTER,
            permissions::TERMINALS_EDIT,
            permissions::TERMINALS_DELETE,
        ],
    },
    RolePreset {
        id: builtin_roles::CASHIER,
        name: "Cashier",
        description: "Can process sales and manage the daily register.",
        permissions: &[
            permissions::SALES_PROCESS,
            permissions::SALES_VIEW,
            permissions::SALES_DISCOUNT,
            permissions::SALES_SPLIT,
            permissions::PAYMENTS_CASH,
            permissions::PAYMENTS_CARD,
            permissions::CUSTOMERS_CREATE,
            permissions::CUSTOMERS_VIEW,
            permissions::DISCOUNTS_APPLY,
            permissions::INVENTORY_VIEW,
            permissions::SHIFTS_OPEN,
            permissions::SHIFTS_CLOSE,
            permissions::WORKSPACES_SWITCH,
        ],
    },
    RolePreset {
        id: builtin_roles::KITCHEN,
        name: "Kitchen",
        description: "Can view and update KDS orders and manage the order queue.",
        permissions: &[
            permissions::KDS_VIEW,
            permissions::KDS_UPDATE,
            permissions::SALES_VIEW,
            permissions::WORKSPACES_SWITCH,
        ],
    },
];

#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn owner_preset_contains_global_wildcard() {
        let preset = &ROLE_PRESETS[0];
        assert_eq!(preset.id, builtin_roles::OWNER);
        assert_eq!(preset.permissions, &["*"]);
    }

    #[test]
    fn manager_preset_excludes_sensitive_permissions() {
        let preset = &ROLE_PRESETS[1];
        assert_eq!(preset.id, builtin_roles::MANAGER);
        assert!(!preset.permissions.contains(&permissions::STAFF_DELETE));
        assert!(
            !preset
                .permissions
                .contains(&permissions::STAFF_MANAGE_ROLES)
        );
        assert!(!preset.permissions.contains(&permissions::PLUGINS_MANAGE));
    }

    #[test]
    fn cashier_preset_includes_basic_sales() {
        let preset = &ROLE_PRESETS[2];
        assert_eq!(preset.id, builtin_roles::CASHIER);
        assert!(preset.permissions.contains(&permissions::SALES_PROCESS));
        assert!(preset.permissions.contains(&permissions::PAYMENTS_CASH));
        assert!(preset.permissions.contains(&permissions::SHIFTS_OPEN));
    }

    #[test]
    fn cashier_lacks_management_permissions() {
        let preset = &ROLE_PRESETS[2];
        assert!(!preset.permissions.contains(&permissions::SETTINGS_EDIT));
        assert!(!preset.permissions.contains(&permissions::STAFF_CREATE));
        assert!(!preset.permissions.contains(&permissions::SALES_VOID));
        assert!(!preset.permissions.contains(&permissions::REPORTS_VIEW));
        assert!(!preset.permissions.contains(&permissions::PRODUCTS_CREATE));
    }

    #[test]
    fn permissions_json_is_valid() {
        for preset in ROLE_PRESETS {
            let json = preset.permissions_json();
            let parsed: Vec<String> =
                serde_json::from_str(&json).expect("permissions_json should produce valid JSON");
            assert_eq!(parsed.len(), preset.permissions.len());
        }
    }

    #[test]
    fn into_role_has_correct_id_and_name() {
        let role = ROLE_PRESETS[0].into_role();
        assert_eq!(role.id, builtin_roles::OWNER);
        assert_eq!(role.name, "Owner");
    }
}

// ── Permission constants ────────────────────────────────────────────

/// Well-known permission strings organised by domain.
///
/// Every constant follows the `<domain>:<action>` format. Use these
/// constants instead of raw strings to get compile-time checking and
/// IDE autocompletion.
pub mod permissions {
    // ── Sales ─────────────────────────────────────────────────────
    /// Process a new sale (add items, accept payment, complete).
    pub const SALES_PROCESS: &str = "sales:process";
    /// Void a completed sale.
    pub const SALES_VOID: &str = "sales:void";
    /// Process a full or partial refund.
    pub const SALES_REFUND: &str = "sales:refund";
    /// View sales history and transaction details.
    pub const SALES_VIEW: &str = "sales:view";
    /// Apply a discount to a sale.
    pub const SALES_DISCOUNT: &str = "sales:discount";
    /// Split a sale across multiple payments or tickets.
    pub const SALES_SPLIT: &str = "sales:split";
    /// Override the unit price of a line in an active cart.
    pub const SALES_OVERRIDE_PRICE: &str = "sales:override_price";

    // ── Products ──────────────────────────────────────────────────
    /// Create a new product.
    pub const PRODUCTS_CREATE: &str = "products:create";
    /// View product catalog and details.
    pub const PRODUCTS_READ: &str = "products:read";
    /// Update existing product details.
    pub const PRODUCTS_UPDATE: &str = "products:update";
    /// Delete a product from the catalog.
    pub const PRODUCTS_DELETE: &str = "products:delete";
    /// Bulk-import products from a file.
    pub const PRODUCTS_IMPORT: &str = "products:import";
    /// Export the product catalog.
    pub const PRODUCTS_EXPORT: &str = "products:export";

    // ── Inventory ─────────────────────────────────────────────────
    /// View stock levels.
    pub const INVENTORY_VIEW: &str = "inventory:view";
    /// Adjust stock quantities (add / remove).
    pub const INVENTORY_ADJUST: &str = "inventory:adjust";
    /// Transfer stock between stores or locations.
    pub const INVENTORY_TRANSFER: &str = "inventory:transfer";
    /// Perform a physical inventory count.
    pub const INVENTORY_COUNT: &str = "inventory:count";

    // ── Staff ─────────────────────────────────────────────────────
    /// Create a new staff user.
    pub const STAFF_CREATE: &str = "staff:create";
    /// View staff members and their details.
    pub const STAFF_READ: &str = "staff:read";
    /// Update an existing staff member.
    pub const STAFF_UPDATE: &str = "staff:update";
    /// Delete / deactivate a staff member.
    pub const STAFF_DELETE: &str = "staff:delete";
    /// Create, edit, or delete roles and their permission sets.
    pub const STAFF_MANAGE_ROLES: &str = "staff:manage_roles";

    // ── Settings ──────────────────────────────────────────────────
    /// View store and system settings.
    pub const SETTINGS_READ: &str = "settings:read";
    /// Modify store and system settings.
    pub const SETTINGS_EDIT: &str = "settings:edit";

    // ── Reports ───────────────────────────────────────────────────
    /// View sales, inventory, and shift reports.
    pub const REPORTS_VIEW: &str = "reports:view";
    /// Export reports to file (PDF, CSV, etc.).
    pub const REPORTS_EXPORT: &str = "reports:export";
    /// Schedule automated report generation.
    pub const REPORTS_SCHEDULE: &str = "reports:schedule";

    // ── Shifts ────────────────────────────────────────────────────
    /// Open a new cashier shift.
    pub const SHIFTS_OPEN: &str = "shifts:open";
    /// Close the current shift.
    pub const SHIFTS_CLOSE: &str = "shifts:close";
    /// View shifts belonging to other cashiers.
    pub const SHIFTS_VIEW_ANY: &str = "shifts:view_any";

    // ── Audit ─────────────────────────────────────────────────────
    /// View the audit log.
    pub const AUDIT_VIEW: &str = "audit:view";
    /// Export the audit log.
    pub const AUDIT_EXPORT: &str = "audit:export";

    // ── Payments ──────────────────────────────────────────────────
    /// Handle cash payments (open drawer, count change).
    pub const PAYMENTS_CASH: &str = "payments:cash";
    /// Process card / contactless payments.
    pub const PAYMENTS_CARD: &str = "payments:card";
    /// Process a payment refund.
    pub const PAYMENTS_REFUND: &str = "payments:refund";
    /// Settle / reconcile payment batches.
    pub const PAYMENTS_SETTLE: &str = "payments:settle";

    // ── Customers ─────────────────────────────────────────────────
    /// Create a new customer record.
    pub const CUSTOMERS_CREATE: &str = "customers:create";
    /// View customer details and history.
    pub const CUSTOMERS_VIEW: &str = "customers:view";
    /// Edit an existing customer record.
    pub const CUSTOMERS_EDIT: &str = "customers:edit";
    /// Delete a customer record.
    pub const CUSTOMERS_DELETE: &str = "customers:delete";

    // ── Tables ────────────────────────────────────────────────────
    /// Assign a table to a customer or server.
    pub const TABLES_ASSIGN: &str = "tables:assign";
    /// Merge two or more tables.
    pub const TABLES_MERGE: &str = "tables:merge";
    /// Split a table into separate checks.
    pub const TABLES_SPLIT: &str = "tables:split";
    /// Close / clear a table.
    pub const TABLES_CLOSE: &str = "tables:close";

    // ── Discounts ─────────────────────────────────────────────────
    /// Apply an existing discount to a sale.
    pub const DISCOUNTS_APPLY: &str = "discounts:apply";
    /// Create a new discount rule.
    pub const DISCOUNTS_CREATE: &str = "discounts:create";
    /// Manage all discount rules (edit, delete, enable/disable).
    pub const DISCOUNTS_MANAGE: &str = "discounts:manage";

    // ── Workspaces ────────────────────────────────────────────────
    /// Switch between workspaces / stores.
    pub const WORKSPACES_SWITCH: &str = "workspaces:switch";

    // ── KDS ─────────────────────────────────────────────────────
    /// View the KDS order queue.
    pub const KDS_VIEW: &str = "kds:view";
    /// Update KDS order status (advance tickets).
    pub const KDS_UPDATE: &str = "kds:update";

    // ── Promotions ──────────────────────────────────────────────
    /// Create a new promotion rule.
    pub const PROMOTIONS_CREATE: &str = "promotions:create";
    /// Edit an existing promotion rule.
    pub const PROMOTIONS_EDIT: &str = "promotions:edit";
    /// Delete a promotion rule.
    pub const PROMOTIONS_DELETE: &str = "promotions:delete";
    /// Apply a promotion to a sale.
    pub const PROMOTIONS_APPLY: &str = "promotions:apply";

    // ── Tables (CRUD) ────────────────────────────────────────────
    /// Create a new table.
    pub const TABLES_CREATE: &str = "tables:create";
    /// Edit table properties (name, capacity, position, shape, section).
    pub const TABLES_EDIT: &str = "tables:edit";
    /// Delete a table from the floor plan.
    pub const TABLES_DELETE: &str = "tables:delete";

    // ── Terminals ────────────────────────────────────────────────
    /// Register a new POS terminal.
    pub const TERMINALS_REGISTER: &str = "terminals:register";
    /// Edit terminal configuration.
    pub const TERMINALS_EDIT: &str = "terminals:edit";
    /// Delete / unregister a terminal.
    pub const TERMINALS_DELETE: &str = "terminals:delete";

    // ── Plugins ───────────────────────────────────────────────────
    /// Manage plugins (install, enable, disable, remove).
    pub const PLUGINS_MANAGE: &str = "plugins:manage";
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Role basics ───────────────────────────────────────────────

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
    fn role_serde_roundtrip() {
        let r = Role::new("role-owner", "Owner").with_description("Full access");
        let json = serde_json::to_string(&r).unwrap();
        let back: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    // ── Permission basics ─────────────────────────────────────────

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
    fn permission_serde_roundtrip() {
        let p = Permission::new("sales:void").with_description("Void a sale");
        let json = serde_json::to_string(&p).unwrap();
        let back: Permission = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "sales:void");
        assert_eq!(back.description, "Void a sale");
    }

    #[test]
    fn permission_clone_eq() {
        let p = Permission::new("test:action");
        assert_eq!(p, p.clone());
    }

    // ── Built-in constants sanity ─────────────────────────────────

    #[test]
    fn builtin_role_constants_are_distinct() {
        assert_ne!(builtin_roles::OWNER, builtin_roles::MANAGER);
        assert_ne!(builtin_roles::MANAGER, builtin_roles::CASHIER);
    }

    // ── has_permission — wildcard resolution ──────────────────────

    #[test]
    fn global_wildcard_grants_everything() {
        assert!(has_permission(&["*".into()], "sales:void"));
        assert!(has_permission(&["*".into()], "anything:here"));
        assert!(has_permission(&["*".into()], "settings:edit"));
    }

    #[test]
    fn domain_wildcard_grants_domain_actions() {
        let granted = &["sales:*".into()];
        assert!(has_permission(granted, "sales:void"));
        assert!(has_permission(granted, "sales:process"));
        assert!(has_permission(granted, "sales:refund"));
        assert!(has_permission(granted, "sales:discount"));
        assert!(!has_permission(granted, "products:read"));
        assert!(!has_permission(granted, "settings:edit"));
    }

    #[test]
    fn exact_match_works() {
        assert!(has_permission(&["sales:void".into()], "sales:void"));
        assert!(!has_permission(&["sales:void".into()], "sales:process"));
        assert!(!has_permission(&["sales:void".into()], "products:read"));
    }

    #[test]
    fn empty_set_denies_everything() {
        let empty: &[String] = &[];
        assert!(!has_permission(empty, "sales:void"));
        assert!(!has_permission(empty, "*"));
    }

    #[test]
    fn multiple_permissions_or_logic() {
        let granted = &["products:read".into(), "sales:process".into()];
        assert!(has_permission(granted, "sales:process"));
        assert!(has_permission(granted, "products:read"));
        assert!(!has_permission(granted, "settings:edit"));
        assert!(!has_permission(granted, "sales:void"));
    }

    #[test]
    fn global_wildcard_among_other_permissions() {
        let granted = &["products:read".into(), "*".into(), "sales:process".into()];
        assert!(has_permission(granted, "settings:edit"));
        assert!(has_permission(granted, "anything"));
    }

    #[test]
    fn domain_wildcard_does_not_leak_to_other_domains() {
        let granted = &["sales:*".into(), "products:*".into()];
        assert!(has_permission(granted, "sales:void"));
        assert!(has_permission(granted, "products:read"));
        assert!(!has_permission(granted, "settings:edit"));
    }

    // ── Role::has_permission and ::authorize ──────────────────────

    #[test]
    fn role_has_permission_from_json() {
        let role = Role::new("role-test", "Test")
            .with_permissions_json("[\"sales:void\", \"products:read\"]");
        assert!(role.has_permission("sales:void"));
        assert!(role.has_permission("products:read"));
        assert!(!role.has_permission("settings:edit"));
    }

    #[test]
    fn role_authorize_returns_ok_for_granted() {
        let role = Role::new("role-test", "Test").with_permissions_json("[\"sales:void\"]");
        assert!(role.authorize("sales:void").is_ok());
    }

    #[test]
    fn role_authorize_returns_error_for_denied() {
        let role = Role::new("role-test", "Cashier").with_permissions_json("[\"sales:process\"]");
        let err = role.authorize("sales:void").unwrap_err();
        assert_eq!(err.required, "sales:void");
        assert_eq!(err.role_name, "Cashier");
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn role_authorize_error_display() {
        let err = AuthorizationError {
            required: "sales:void".into(),
            role_name: "Cashier".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("sales:void"));
        assert!(msg.contains("Cashier"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn role_with_malformed_json_denies_all() {
        let role = Role::new("role-test", "Test").with_permissions_json("not json");
        assert!(!role.has_permission("sales:void"));
    }

    #[test]
    fn role_with_global_wildcard() {
        let role = Role::new("role-admin", "Admin").with_permissions_json("[\"*\"]");
        assert!(role.has_permission("sales:void"));
        assert!(role.has_permission("settings:edit"));
        assert!(role.has_permission("staff:manage"));
        assert!(role.authorize("anything").is_ok());
    }

    #[test]
    fn role_has_permission_empty_json() {
        let role = Role::new("role-test", "Test").with_permissions_json("[]");
        assert!(!role.has_permission("sales:void"));
        assert!(!role.has_permission("*"));
    }

    #[test]
    fn role_authorize_with_domain_wildcard() {
        let role = Role::new("role-test", "Test").with_permissions_json("[\"sales:*\"]");
        assert!(role.authorize("sales:void").is_ok());
        assert!(role.authorize("sales:process").is_ok());
        assert!(role.authorize("products:read").is_err());
    }

    #[test]
    fn authorization_error_debug() {
        let err = AuthorizationError {
            required: "sales:void".into(),
            role_name: "Cashier".into(),
        };
        let debug = format!("{err:?}");
        assert!(debug.contains("sales:void"));
        assert!(debug.contains("Cashier"));
    }

    // ── Permission constants well-formedness ──────────────────────

    #[test]
    fn all_permission_constants_contain_colon() {
        let all = [
            permissions::SALES_PROCESS,
            permissions::SALES_VOID,
            permissions::SALES_REFUND,
            permissions::SALES_VIEW,
            permissions::SALES_DISCOUNT,
            permissions::SALES_SPLIT,
            permissions::PRODUCTS_CREATE,
            permissions::PRODUCTS_READ,
            permissions::PRODUCTS_UPDATE,
            permissions::PRODUCTS_DELETE,
            permissions::PRODUCTS_IMPORT,
            permissions::PRODUCTS_EXPORT,
            permissions::INVENTORY_VIEW,
            permissions::INVENTORY_ADJUST,
            permissions::INVENTORY_TRANSFER,
            permissions::INVENTORY_COUNT,
            permissions::STAFF_CREATE,
            permissions::STAFF_READ,
            permissions::STAFF_UPDATE,
            permissions::STAFF_DELETE,
            permissions::STAFF_MANAGE_ROLES,
            permissions::SETTINGS_READ,
            permissions::SETTINGS_EDIT,
            permissions::REPORTS_VIEW,
            permissions::REPORTS_EXPORT,
            permissions::REPORTS_SCHEDULE,
            permissions::SHIFTS_OPEN,
            permissions::SHIFTS_CLOSE,
            permissions::SHIFTS_VIEW_ANY,
            permissions::AUDIT_VIEW,
            permissions::AUDIT_EXPORT,
            permissions::PAYMENTS_CASH,
            permissions::PAYMENTS_CARD,
            permissions::PAYMENTS_REFUND,
            permissions::PAYMENTS_SETTLE,
            permissions::CUSTOMERS_CREATE,
            permissions::CUSTOMERS_VIEW,
            permissions::CUSTOMERS_EDIT,
            permissions::CUSTOMERS_DELETE,
            permissions::TABLES_ASSIGN,
            permissions::TABLES_MERGE,
            permissions::TABLES_SPLIT,
            permissions::TABLES_CLOSE,
            permissions::DISCOUNTS_APPLY,
            permissions::DISCOUNTS_CREATE,
            permissions::DISCOUNTS_MANAGE,
            permissions::WORKSPACES_SWITCH,
            permissions::PROMOTIONS_CREATE,
            permissions::PROMOTIONS_EDIT,
            permissions::PROMOTIONS_DELETE,
            permissions::PROMOTIONS_APPLY,
            permissions::TABLES_CREATE,
            permissions::TABLES_EDIT,
            permissions::TABLES_DELETE,
            permissions::TERMINALS_REGISTER,
            permissions::TERMINALS_EDIT,
            permissions::TERMINALS_DELETE,
            permissions::PLUGINS_MANAGE,
        ];
        for &p in &all {
            assert!(p.contains(':'), "constant {p} is missing ':' separator");
        }
    }

    #[test]
    fn permission_constants_are_unique() {
        use std::collections::HashSet;
        let all = [
            permissions::SALES_PROCESS,
            permissions::SALES_VOID,
            permissions::SALES_REFUND,
            permissions::SALES_VIEW,
            permissions::SALES_DISCOUNT,
            permissions::SALES_SPLIT,
            permissions::PRODUCTS_CREATE,
            permissions::PRODUCTS_READ,
            permissions::PRODUCTS_UPDATE,
            permissions::PRODUCTS_DELETE,
            permissions::PRODUCTS_IMPORT,
            permissions::PRODUCTS_EXPORT,
            permissions::INVENTORY_VIEW,
            permissions::INVENTORY_ADJUST,
            permissions::INVENTORY_TRANSFER,
            permissions::INVENTORY_COUNT,
            permissions::STAFF_CREATE,
            permissions::STAFF_READ,
            permissions::STAFF_UPDATE,
            permissions::STAFF_DELETE,
            permissions::STAFF_MANAGE_ROLES,
            permissions::SETTINGS_READ,
            permissions::SETTINGS_EDIT,
            permissions::REPORTS_VIEW,
            permissions::REPORTS_EXPORT,
            permissions::REPORTS_SCHEDULE,
            permissions::SHIFTS_OPEN,
            permissions::SHIFTS_CLOSE,
            permissions::SHIFTS_VIEW_ANY,
            permissions::AUDIT_VIEW,
            permissions::AUDIT_EXPORT,
            permissions::PAYMENTS_CASH,
            permissions::PAYMENTS_CARD,
            permissions::PAYMENTS_REFUND,
            permissions::PAYMENTS_SETTLE,
            permissions::CUSTOMERS_CREATE,
            permissions::CUSTOMERS_VIEW,
            permissions::CUSTOMERS_EDIT,
            permissions::CUSTOMERS_DELETE,
            permissions::TABLES_ASSIGN,
            permissions::TABLES_MERGE,
            permissions::TABLES_SPLIT,
            permissions::TABLES_CLOSE,
            permissions::DISCOUNTS_APPLY,
            permissions::DISCOUNTS_CREATE,
            permissions::DISCOUNTS_MANAGE,
            permissions::WORKSPACES_SWITCH,
            permissions::PROMOTIONS_CREATE,
            permissions::PROMOTIONS_EDIT,
            permissions::PROMOTIONS_DELETE,
            permissions::PROMOTIONS_APPLY,
            permissions::TABLES_CREATE,
            permissions::TABLES_EDIT,
            permissions::TABLES_DELETE,
            permissions::TERMINALS_REGISTER,
            permissions::TERMINALS_EDIT,
            permissions::TERMINALS_DELETE,
            permissions::PLUGINS_MANAGE,
        ];
        let mut seen = HashSet::new();
        for &p in &all {
            assert!(seen.insert(p), "duplicate permission constant: {p}");
        }
    }

    // ── has_permission edge cases ────────────────────────────────

    #[test]
    fn no_colon_permission_exact_match() {
        // Permissions without a colon are matched exactly.
        assert!(has_permission(&["admin".into()], "admin"));
        assert!(!has_permission(&["admin".into()], "user"));
        // A no-colon granted permission is treated as its own domain,
        // so a domain wildcard can match it.
        assert!(has_permission(&["admin:*".into()], "admin"));
    }

    #[test]
    fn no_colon_required_matches_global_wildcard() {
        // A required permission without a colon should match the global wildcard.
        assert!(has_permission(&["*".into()], "admin"));
    }

    #[test]
    fn role_preset_empty_permissions_json() {
        let preset = RolePreset {
            id: "role-empty",
            name: "Empty",
            description: "No permissions",
            permissions: &[],
        };
        assert_eq!(preset.permissions_json(), "[]");
    }

    #[test]
    fn role_preset_into_role_timestamps() {
        let role = ROLE_PRESETS[0].into_role();
        assert!(!role.created_at.is_empty());
        assert!(!role.updated_at.is_empty());
        assert!(role.created_at.contains('T'));
    }
}
