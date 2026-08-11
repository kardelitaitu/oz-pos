//! Code-resident permission registry — the single source of truth for what a
//! permission key means, which family it belongs to, and whether it is
//! sensitive (ADR #35 D3 / spec 0046).
//!
//! Growing the system means adding keys here — never editing roles. Sensitive
//! keys (voids, refunds, settlement, role management, bulk export) are never
//! grantable through a family wildcard; only explicit grants may carry them.

/// One registered permission key.
pub struct PermissionEntry {
    /// The `domain:action` key — byte-identical to the enforced string.
    pub key: &'static str,
    /// The family (domain) this key belongs to; `family:*` wildcards grant
    /// every *operational* key in the family.
    pub family: &'static str,
    /// Whether the key is sensitive — never grantable via a family wildcard.
    pub sensitive: bool,
    /// One-line description of what the key grants.
    pub description: &'static str,
}

/// Errors from [`validate_grant`] / [`validate_grants`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// The key (or family wildcard) is not registered.
    #[error("unregistered permission '{0}' — add it to the registry first")]
    UnknownKey(String),
    /// A family wildcard would implicitly grant sensitive key(s).
    #[error("wildcard '{0}' would grant sensitive key(s): {1}")]
    SensitiveUnderWildcard(String, String),
    /// The global `*` wildcard is reserved for the Owner seed.
    #[error("global wildcard '*' is reserved for the Owner seed")]
    GlobalWildcardDenied,
}

/// The registry — every enforced key, classified by family and sensitivity.
///
/// Sensitive per ADR #35 D2: voids, refunds, payment settlement, role
/// management, staff deletion, and bulk export of reports/audit data.
pub const REGISTRY: &[PermissionEntry] = &[
    // ── sales ────────────────────────────────────────────────────
    PermissionEntry {
        key: "sales:process",
        family: "sales",
        sensitive: false,
        description: "Process a new sale (add items, accept payment, complete).",
    },
    PermissionEntry {
        key: "sales:view",
        family: "sales",
        sensitive: false,
        description: "View sales history and transaction details.",
    },
    PermissionEntry {
        key: "sales:discount",
        family: "sales",
        sensitive: false,
        description: "Apply a discount to a sale.",
    },
    PermissionEntry {
        key: "sales:split",
        family: "sales",
        sensitive: false,
        description: "Split a sale across multiple payments or tickets.",
    },
    PermissionEntry {
        key: "sales:override_price",
        family: "sales",
        sensitive: false,
        description: "Override the unit price of a line in an active cart.",
    },
    PermissionEntry {
        key: "sales:void",
        family: "sales",
        sensitive: true,
        description: "Void a completed sale.",
    },
    PermissionEntry {
        key: "sales:refund",
        family: "sales",
        sensitive: true,
        description: "Process a full or partial refund.",
    },
    // ── products ─────────────────────────────────────────────────
    PermissionEntry {
        key: "products:create",
        family: "products",
        sensitive: false,
        description: "Create a new product.",
    },
    PermissionEntry {
        key: "products:read",
        family: "products",
        sensitive: false,
        description: "View product catalog and details.",
    },
    PermissionEntry {
        key: "products:update",
        family: "products",
        sensitive: false,
        description: "Update existing product details.",
    },
    PermissionEntry {
        key: "products:delete",
        family: "products",
        sensitive: false,
        description: "Delete a product from the catalog.",
    },
    PermissionEntry {
        key: "products:import",
        family: "products",
        sensitive: false,
        description: "Bulk-import products from a file.",
    },
    PermissionEntry {
        key: "products:export",
        family: "products",
        sensitive: false,
        description: "Export the product catalog.",
    },
    PermissionEntry {
        key: "products:crud",
        family: "products",
        sensitive: false,
        description: "Legacy composite seed key — product create/read/update/delete. Kept byte-identical.",
    },
    // ── inventory ────────────────────────────────────────────────
    PermissionEntry {
        key: "inventory:view",
        family: "inventory",
        sensitive: false,
        description: "View stock levels.",
    },
    PermissionEntry {
        key: "inventory:adjust",
        family: "inventory",
        sensitive: false,
        description: "Adjust stock quantities (add / remove).",
    },
    PermissionEntry {
        key: "inventory:transfer",
        family: "inventory",
        sensitive: false,
        description: "Transfer stock between stores or locations.",
    },
    PermissionEntry {
        key: "inventory:count",
        family: "inventory",
        sensitive: false,
        description: "Perform a physical inventory count.",
    },
    PermissionEntry {
        key: "inventory:locations_manage",
        family: "inventory",
        sensitive: false,
        description: "Create, rename, deactivate, or rebind inventory locations.",
    },
    // ── staff ────────────────────────────────────────────────────
    PermissionEntry {
        key: "staff:create",
        family: "staff",
        sensitive: false,
        description: "Create a new staff user.",
    },
    PermissionEntry {
        key: "staff:read",
        family: "staff",
        sensitive: false,
        description: "View staff members and their details.",
    },
    PermissionEntry {
        key: "staff:update",
        family: "staff",
        sensitive: false,
        description: "Update an existing staff member.",
    },
    PermissionEntry {
        key: "staff:delete",
        family: "staff",
        sensitive: true,
        description: "Delete / deactivate a staff member.",
    },
    PermissionEntry {
        key: "staff:manage_roles",
        family: "staff",
        sensitive: true,
        description: "Create, edit, or delete roles and their permission sets.",
    },
    // ── settings ─────────────────────────────────────────────────
    PermissionEntry {
        key: "settings:read",
        family: "settings",
        sensitive: false,
        description: "View store and system settings.",
    },
    PermissionEntry {
        key: "settings:edit",
        family: "settings",
        sensitive: false,
        description: "Modify store and system settings.",
    },
    // ── reports ──────────────────────────────────────────────────
    PermissionEntry {
        key: "reports:view",
        family: "reports",
        sensitive: false,
        description: "View sales, inventory, and shift reports.",
    },
    PermissionEntry {
        key: "reports:schedule",
        family: "reports",
        sensitive: false,
        description: "Schedule automated report generation.",
    },
    PermissionEntry {
        key: "reports:export",
        family: "reports",
        sensitive: true,
        description: "Export reports to file (PDF, CSV, etc.).",
    },
    // ── shifts ───────────────────────────────────────────────────
    PermissionEntry {
        key: "shifts:open",
        family: "shifts",
        sensitive: false,
        description: "Open a new cashier shift.",
    },
    PermissionEntry {
        key: "shifts:close",
        family: "shifts",
        sensitive: false,
        description: "Close the current shift.",
    },
    PermissionEntry {
        key: "shifts:view_any",
        family: "shifts",
        sensitive: false,
        description: "View shifts belonging to other cashiers.",
    },
    // ── audit ────────────────────────────────────────────────────
    PermissionEntry {
        key: "audit:view",
        family: "audit",
        sensitive: false,
        description: "View the audit log.",
    },
    PermissionEntry {
        key: "audit:export",
        family: "audit",
        sensitive: true,
        description: "Export the audit log.",
    },
    // ── payments ─────────────────────────────────────────────────
    PermissionEntry {
        key: "payments:cash",
        family: "payments",
        sensitive: false,
        description: "Handle cash payments (open drawer, count change).",
    },
    PermissionEntry {
        key: "payments:card",
        family: "payments",
        sensitive: false,
        description: "Process card / contactless payments.",
    },
    PermissionEntry {
        key: "payments:refund",
        family: "payments",
        sensitive: true,
        description: "Process a payment refund.",
    },
    PermissionEntry {
        key: "payments:settle",
        family: "payments",
        sensitive: true,
        description: "Settle / reconcile payment batches.",
    },
    // ── customers ────────────────────────────────────────────────
    PermissionEntry {
        key: "customers:create",
        family: "customers",
        sensitive: false,
        description: "Create a new customer record.",
    },
    PermissionEntry {
        key: "customers:view",
        family: "customers",
        sensitive: false,
        description: "View customer details and history.",
    },
    PermissionEntry {
        key: "customers:edit",
        family: "customers",
        sensitive: false,
        description: "Edit an existing customer record.",
    },
    PermissionEntry {
        key: "customers:delete",
        family: "customers",
        sensitive: false,
        description: "Delete a customer record.",
    },
    // ── loyalty ──────────────────────────────────────────────────
    PermissionEntry {
        key: "loyalty:view",
        family: "loyalty",
        sensitive: false,
        description: "View loyalty accounts, balances, and tiers.",
    },
    PermissionEntry {
        key: "loyalty:earn",
        family: "loyalty",
        sensitive: false,
        description: "Earn loyalty points for completed sales.",
    },
    PermissionEntry {
        key: "loyalty:redeem",
        family: "loyalty",
        sensitive: false,
        description: "Redeem loyalty points during checkout.",
    },
    PermissionEntry {
        key: "loyalty:manage",
        family: "loyalty",
        sensitive: false,
        description: "Manage loyalty tier configuration.",
    },
    // ── tables ───────────────────────────────────────────────────
    PermissionEntry {
        key: "tables:assign",
        family: "tables",
        sensitive: false,
        description: "Assign a table to a customer or server.",
    },
    PermissionEntry {
        key: "tables:merge",
        family: "tables",
        sensitive: false,
        description: "Merge two or more tables.",
    },
    PermissionEntry {
        key: "tables:split",
        family: "tables",
        sensitive: false,
        description: "Split a table into separate checks.",
    },
    PermissionEntry {
        key: "tables:close",
        family: "tables",
        sensitive: false,
        description: "Close / clear a table.",
    },
    PermissionEntry {
        key: "tables:create",
        family: "tables",
        sensitive: false,
        description: "Create a new table.",
    },
    PermissionEntry {
        key: "tables:edit",
        family: "tables",
        sensitive: false,
        description: "Edit table properties (name, capacity, position, shape, section).",
    },
    PermissionEntry {
        key: "tables:delete",
        family: "tables",
        sensitive: false,
        description: "Delete a table from the floor plan.",
    },
    // ── discounts ────────────────────────────────────────────────
    PermissionEntry {
        key: "discounts:apply",
        family: "discounts",
        sensitive: false,
        description: "Apply an existing discount to a sale.",
    },
    PermissionEntry {
        key: "discounts:create",
        family: "discounts",
        sensitive: false,
        description: "Create a new discount rule.",
    },
    PermissionEntry {
        key: "discounts:manage",
        family: "discounts",
        sensitive: false,
        description: "Manage all discount rules (edit, delete, enable/disable).",
    },
    // ── workspaces ───────────────────────────────────────────────
    PermissionEntry {
        key: "workspaces:switch",
        family: "workspaces",
        sensitive: false,
        description: "Switch between workspaces / stores.",
    },
    // ── kds ──────────────────────────────────────────────────────
    PermissionEntry {
        key: "kds:view",
        family: "kds",
        sensitive: false,
        description: "View the KDS order queue.",
    },
    PermissionEntry {
        key: "kds:update",
        family: "kds",
        sensitive: false,
        description: "Update KDS order status (advance tickets).",
    },
    // ── promotions ───────────────────────────────────────────────
    PermissionEntry {
        key: "promotions:create",
        family: "promotions",
        sensitive: false,
        description: "Create a new promotion rule.",
    },
    PermissionEntry {
        key: "promotions:edit",
        family: "promotions",
        sensitive: false,
        description: "Edit an existing promotion rule.",
    },
    PermissionEntry {
        key: "promotions:delete",
        family: "promotions",
        sensitive: false,
        description: "Delete a promotion rule.",
    },
    PermissionEntry {
        key: "promotions:apply",
        family: "promotions",
        sensitive: false,
        description: "Apply a promotion to a sale.",
    },
    // ── terminals ────────────────────────────────────────────────
    PermissionEntry {
        key: "terminals:register",
        family: "terminals",
        sensitive: false,
        description: "Register a new POS terminal.",
    },
    PermissionEntry {
        key: "terminals:edit",
        family: "terminals",
        sensitive: false,
        description: "Edit terminal configuration.",
    },
    PermissionEntry {
        key: "terminals:delete",
        family: "terminals",
        sensitive: false,
        description: "Delete / unregister a terminal.",
    },
    // ── categories ───────────────────────────────────────────────
    PermissionEntry {
        key: "categories:manage",
        family: "categories",
        sensitive: false,
        description: "Legacy seed key — category create/update/delete. Kept byte-identical.",
    },
    // ── plugins ──────────────────────────────────────────────────
    PermissionEntry {
        key: "plugins:manage",
        family: "plugins",
        sensitive: false,
        description: "Manage plugins (install, enable, disable, remove).",
    },
];

/// Look up a registered key.
pub fn lookup(key: &str) -> Option<&'static PermissionEntry> {
    REGISTRY.iter().find(|e| e.key == key)
}

/// Whether the key is registered.
pub fn is_registered(key: &str) -> bool {
    lookup(key).is_some()
}

/// Whether the key is registered and sensitive.
pub fn is_sensitive(key: &str) -> bool {
    lookup(key).is_some_and(|e| e.sensitive)
}

/// Validate a single grant string.
///
/// `allow_global` permits `"*"` — the documented Owner-seed exception
/// (ADR #35 D4).
pub fn validate_grant(grant: &str, allow_global: bool) -> Result<(), RegistryError> {
    if grant == "*" {
        return if allow_global {
            Ok(())
        } else {
            Err(RegistryError::GlobalWildcardDenied)
        };
    }
    if let Some(family) = grant.strip_suffix(":*") {
        let sensitive: Vec<&str> = REGISTRY
            .iter()
            .filter(|e| e.family == family && e.sensitive)
            .map(|e| e.key)
            .collect();
        if !sensitive.is_empty() {
            return Err(RegistryError::SensitiveUnderWildcard(
                grant.to_owned(),
                sensitive.join(", "),
            ));
        }
        if REGISTRY.iter().any(|e| e.family == family) {
            Ok(())
        } else {
            Err(RegistryError::UnknownKey(grant.to_owned()))
        }
    } else if is_registered(grant) {
        Ok(())
    } else {
        Err(RegistryError::UnknownKey(grant.to_owned()))
    }
}

/// Validate a grant set, collecting every error.
pub fn validate_grants(grants: &[String], allow_global: bool) -> Result<(), Vec<RegistryError>> {
    let errors: Vec<RegistryError> = grants
        .iter()
        .filter_map(|g| validate_grant(g, allow_global).err())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rbac::permissions;

    /// The complete enforced key inventory — every `rbac::permissions`
    /// constant plus the two legacy seed keys (`products:crud`,
    /// `categories:manage`).
    const ALL: &[&str] = &[
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
        permissions::PRODUCTS_CRUD,
        permissions::INVENTORY_VIEW,
        permissions::INVENTORY_ADJUST,
        permissions::INVENTORY_TRANSFER,
        permissions::INVENTORY_COUNT,
        permissions::INVENTORY_LOCATIONS_MANAGE,
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
        permissions::LOYALTY_VIEW,
        permissions::LOYALTY_EARN,
        permissions::LOYALTY_REDEEM,
        permissions::LOYALTY_MANAGE,
        permissions::TABLES_ASSIGN,
        permissions::TABLES_MERGE,
        permissions::TABLES_SPLIT,
        permissions::TABLES_CLOSE,
        permissions::DISCOUNTS_APPLY,
        permissions::DISCOUNTS_CREATE,
        permissions::DISCOUNTS_MANAGE,
        permissions::WORKSPACES_SWITCH,
        permissions::KDS_VIEW,
        permissions::KDS_UPDATE,
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
        permissions::CATEGORIES_MANAGE,
        permissions::PLUGINS_MANAGE,
    ];

    /// The sensitive keys per ADR #35 D2: voids, refunds, settlement,
    /// role management, and bulk export are never wildcard-eligible.
    fn is_expected_sensitive(key: &str) -> bool {
        matches!(
            key,
            permissions::SALES_VOID
                | permissions::SALES_REFUND
                | permissions::PAYMENTS_REFUND
                | permissions::PAYMENTS_SETTLE
                | permissions::STAFF_MANAGE_ROLES
                | permissions::STAFF_DELETE
                | permissions::REPORTS_EXPORT
                | permissions::AUDIT_EXPORT
        )
    }

    #[test]
    fn every_permission_constant_is_registered() {
        for &p in ALL {
            assert!(is_registered(p), "unregistered enforced key: {p}");
        }
    }

    #[test]
    fn every_registry_key_is_a_known_constant() {
        for e in REGISTRY {
            assert!(
                ALL.contains(&e.key),
                "registry key not in the constant inventory: {}",
                e.key
            );
        }
    }

    #[test]
    fn sensitive_classification_is_explicit() {
        for &p in ALL {
            let entry = lookup(p).expect("every enforced key must be registered");
            assert_eq!(
                entry.sensitive,
                is_expected_sensitive(p),
                "sensitive flag mismatch for {p}"
            );
        }
    }

    #[test]
    fn sensitive_families_are_not_wildcardable() {
        for w in ["sales:*", "payments:*", "staff:*", "reports:*", "audit:*"] {
            assert!(
                validate_grant(w, false).is_err(),
                "wildcard {w} must be rejected: it would grant sensitive keys"
            );
        }
    }

    #[test]
    fn operational_families_are_wildcardable() {
        for w in [
            "products:*",
            "inventory:*",
            "settings:*",
            "customers:*",
            "tables:*",
            "discounts:*",
            "shifts:*",
            "loyalty:*",
            "kds:*",
        ] {
            assert!(
                validate_grant(w, false).is_ok(),
                "operational wildcard {w} must be allowed"
            );
        }
    }

    #[test]
    fn unknown_key_is_rejected() {
        assert_eq!(
            validate_grant("sales:typo", false),
            Err(RegistryError::UnknownKey("sales:typo".into()))
        );
        assert_eq!(
            validate_grant("unknown:*", false),
            Err(RegistryError::UnknownKey("unknown:*".into()))
        );
    }

    #[test]
    fn exact_sensitive_key_is_allowed() {
        // Explicit grants are the sanctioned way to carry sensitive keys.
        assert!(validate_grant(permissions::SALES_VOID, false).is_ok());
        assert!(validate_grant(permissions::SALES_REFUND, false).is_ok());
        assert!(validate_grant(permissions::STAFF_MANAGE_ROLES, false).is_ok());
    }

    #[test]
    fn global_wildcard_denied_unless_explicit() {
        assert_eq!(
            validate_grant("*", false),
            Err(RegistryError::GlobalWildcardDenied)
        );
        assert!(validate_grant("*", true).is_ok());
    }

    #[test]
    fn registry_has_no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for e in REGISTRY {
            assert!(seen.insert(e.key), "duplicate registry key: {}", e.key);
        }
    }
}
