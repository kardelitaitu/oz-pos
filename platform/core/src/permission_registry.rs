//! Code-resident permission registry — the single source of truth for what a
//! permission key means, which family it belongs to, and whether it is
//! sensitive (ADR #35 D3 / spec 0046).
/*
last audited 25-07-26 by RSA-Agent (platform-core slice B: permission_registry deep read)
crate: platform-core | status: SAFE | lint: CLEAN
findings: exemplary — single source of truth with family/sensitivity classification; sensitive keys (voids, refunds, settlement, role mgmt, staff deletion, bulk export) never grantable through family wildcards (validate_grants enforces + tests); global * reserved for the Owner seed; duplicate-key invariant pinned
next: none | perf: linear registry scan
*/
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
    PermissionEntry {
        key: "products:edit_cost",
        family: "products",
        sensitive: false,
        description: "Set or override a product's cost (HPP). Granted to manager/admin presets only — cost is local-only (ADR #36 D7).",
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
    PermissionEntry {
        key: "staff:read_identity",
        family: "staff",
        sensitive: true,
        description: "Read a staff member's identity fields (national id, tax id) unmasked.",
    },
    PermissionEntry {
        key: "staff:read_payroll",
        family: "staff",
        sensitive: true,
        description: "Read a staff member's payroll fields (monthly take-home pay).",
    },
    PermissionEntry {
        key: "staff:edit_notes",
        family: "staff",
        sensitive: true,
        description: "Edit a staff member's free-text notes.",
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
    // ── analytics ────────────────────────────────────────────────
    PermissionEntry {
        key: "analytics:view",
        family: "analytics",
        sensitive: false,
        description: "View per-staff shift and sales analytics (owner / admin / manager).",
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
    use crate::rbac::{ALL_ENFORCED, permissions};

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
                | permissions::STAFF_READ_IDENTITY
                | permissions::STAFF_READ_PAYROLL
                | permissions::STAFF_EDIT_NOTES
                | permissions::REPORTS_EXPORT
                | permissions::AUDIT_EXPORT
        )
    }

    #[test]
    fn every_permission_constant_is_registered() {
        for &p in ALL_ENFORCED {
            assert!(is_registered(p), "unregistered enforced key: {p}");
        }
    }

    #[test]
    fn every_registry_key_is_a_known_constant() {
        for e in REGISTRY {
            assert!(
                ALL_ENFORCED.contains(&e.key),
                "registry key not in the constant inventory: {}",
                e.key
            );
        }
    }

    #[test]
    fn sensitive_classification_is_explicit() {
        for &p in ALL_ENFORCED {
            let entry = lookup(p).expect("every enforced key must be registered");
            assert_eq!(
                entry.sensitive,
                is_expected_sensitive(p),
                "sensitive flag mismatch for {p}"
            );
        }
    }

    /// ADR #35 D6 / spec 0049: the profile sensitive keys are registered,
    /// classified sensitive (never wildcard-eligible), granted to every
    /// management preset, and deliberately withheld from Auditor.
    #[test]
    fn profile_sensitive_keys_are_registered_and_granted() {
        for key in [
            permissions::STAFF_READ_IDENTITY,
            permissions::STAFF_READ_PAYROLL,
            permissions::STAFF_EDIT_NOTES,
        ] {
            assert!(is_registered(key), "{key} must be registered");
            assert!(ALL_ENFORCED.contains(&key), "{key} must be in ALL_ENFORCED");
            let entry = lookup(key).expect("registered");
            assert!(entry.sensitive, "{key} must be classified sensitive");
            // Sensitive keys can never ride a family wildcard.
            assert!(
                validate_grant("staff:*", false).is_err(),
                "staff:* must reject {key}"
            );
        }

        // Management presets grant them; Staff (checkout-only) and Auditor
        // (read-only) are excluded.
        for preset in crate::rbac::ROLE_PRESETS {
            let grants = preset.permissions;
            match preset.id {
                crate::rbac::builtin_roles::MANAGER | crate::rbac::builtin_roles::ADMIN => {
                    for key in [
                        permissions::STAFF_READ_IDENTITY,
                        permissions::STAFF_READ_PAYROLL,
                        permissions::STAFF_EDIT_NOTES,
                    ] {
                        assert!(
                            grants.contains(&key),
                            "{} preset must grant {key}",
                            preset.id
                        );
                    }
                }
                crate::rbac::builtin_roles::STAFF => {
                    for key in [
                        permissions::STAFF_READ_IDENTITY,
                        permissions::STAFF_READ_PAYROLL,
                        permissions::STAFF_EDIT_NOTES,
                    ] {
                        assert!(!grants.contains(&key), "Staff must NOT grant {key}");
                    }
                }
                crate::rbac::builtin_roles::AUDITOR => {
                    for key in [
                        permissions::STAFF_READ_IDENTITY,
                        permissions::STAFF_READ_PAYROLL,
                        permissions::STAFF_EDIT_NOTES,
                    ] {
                        assert!(!grants.contains(&key), "Auditor must NOT grant {key}");
                    }
                }
                _ => {}
            }
        }
    }

    /// Every family present in the registry, sorted for stable output.
    fn families() -> Vec<&'static str> {
        let mut families: Vec<&'static str> = REGISTRY.iter().map(|e| e.family).collect();
        families.sort_unstable();
        families.dedup();
        families
    }

    #[test]
    fn wildcard_is_rejected_for_every_family_with_a_sensitive_key() {
        for family in families() {
            let wildcard = format!("{family}:*");
            let has_sensitive = REGISTRY.iter().any(|e| e.family == family && e.sensitive);
            if has_sensitive {
                assert!(
                    validate_grant(&wildcard, false).is_err(),
                    "wildcard {wildcard} must be rejected: the family contains sensitive keys"
                );
            }
        }
    }

    #[test]
    fn wildcard_is_accepted_for_every_family_without_sensitive_keys() {
        for family in families() {
            let wildcard = format!("{family}:*");
            let has_sensitive = REGISTRY.iter().any(|e| e.family == family && e.sensitive);
            if !has_sensitive {
                assert!(
                    validate_grant(&wildcard, false).is_ok(),
                    "operational wildcard {wildcard} must be allowed"
                );
            }
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
