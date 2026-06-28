//! Feature flag system — toggleable POS capabilities.
//!
//! The [`Feature`] enum defines all 32 toggleable features in the
//! OZ-POS framework. A [`FeatureRegistry`] holds the currently-active
//! set and provides helpers for enabling/disabling flags with automatic
//! dependency resolution.
//!
//! Feature flags are persisted in the `settings` table as
//! `feature.<variant_name>` = `"1"` rows. The bridge between
//! [`FeatureRegistry`] and the settings store lives in `settings.rs`
//! (#6 in the core plan).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Every toggleable feature in the OZ-POS framework.
///
/// Variants are in logical groups: core, payments, products, staff,
/// hardware, business rules, scaling, and advanced. The order is stable;
/// adding new variants at the end preserves the integer discriminants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    // ── Core ─────────────────────────────────────────────────────
    /// Core retail POS: scan barcode, add to cart, sell, print receipt.
    SimpleRetail,
    /// Restaurant mode with table management and KDS.
    Restaurant,

    // ── Payments ─────────────────────────────────────────────────
    /// Cash payment method.
    CashPayment,
    /// Card payment method (debit / credit).
    CardPayment,
    /// Multi-currency support with exchange rates.
    MultiCurrency,

    // ── Products ─────────────────────────────────────────────────
    /// Track stock levels per product.
    InventoryTracking,
    /// Product variants (size, colour, flavour).
    ProductVariants,
    /// Product category grouping.
    CategoriesEnabled,

    // ── Staff ────────────────────────────────────────────────────
    /// Staff PIN / password login.
    StaffLogin,
    /// Role-based permissions (owner, manager, cashier).
    StaffRoles,
    /// Open / close shift with cash reconciliation.
    ShiftManagement,
    /// Immutable append-only audit log.
    AuditLog,

    // ── Hardware ─────────────────────────────────────────────────
    /// Barcode scanner support (USB, serial, Bluetooth).
    BarcodeScanning,
    /// Receipt printer (USB / serial / network).
    ReceiptPrinting,
    /// Cash drawer trigger (via printer GPIO).
    CashDrawer,
    /// Customer-facing secondary display.
    CustomerDisplay,
    /// NFC / contactless reader.
    NfcReader,

    // ── Business Rules ───────────────────────────────────────────
    /// Percentage and fixed-amount discounts.
    DiscountEngine,
    /// Tax calculation engine.
    TaxEngine,
    /// Customer loyalty points and tiers.
    LoyaltyProgram,
    /// Time-limited promotions (buy-X-get-Y, etc.).
    PromotionsEngine,
    /// Sell multiple SKUs as a bundle.
    ProductBundles,

    // ── Restaurant ───────────────────────────────────────────────
    /// Kitchen display system for order routing.
    KitchenDisplay,
    /// Interactive table management (floor plan).
    TableManagement,
    /// Locked-down full-screen self-service mode.
    SelfServiceKiosk,

    // ── Scaling ──────────────────────────────────────────────────
    /// Cloud database synchronisation.
    CloudSync,
    /// Multi-store management.
    MultiStore,
    /// Multiple terminals per store.
    MultiTerminal,

    // ── Reporting ────────────────────────────────────────────────
    /// Sales, inventory, and shift reports.
    Reporting,
    /// Advanced analytics with charts and exports.
    Analytics,

    // ── Advanced ─────────────────────────────────────────────────
    /// Data export / import (.ozpkg format).
    ExportImport,
    /// Third-party plugin system.
    PluginSystem,
}

impl Feature {
    /// Features that must be enabled before this one can be turned on.
    ///
    /// Returns an empty slice if the feature has no dependencies.
    pub fn dependencies(self) -> &'static [Feature] {
        match self {
            // Staff hierarchy.
            Self::StaffRoles => &[Self::StaffLogin],
            Self::ShiftManagement => &[Self::StaffLogin],
            Self::AuditLog => &[Self::StaffLogin],

            // Hardware chains.
            Self::CashDrawer => &[Self::ReceiptPrinting],
            Self::CustomerDisplay => &[Self::ReceiptPrinting],

            // Restaurant.
            Self::KitchenDisplay => &[Self::Restaurant],
            Self::TableManagement => &[Self::Restaurant],
            Self::SelfServiceKiosk => &[Self::SimpleRetail],

            // Business rules that need staff login.
            Self::LoyaltyProgram => &[Self::StaffLogin],
            Self::PromotionsEngine => &[Self::DiscountEngine],

            // Scaling.
            Self::MultiTerminal => &[Self::MultiStore],
            Self::CloudSync => &[Self::MultiStore],

            // Reporting.
            Self::Analytics => &[Self::Reporting],

            // Everything else has no dependencies.
            _ => &[],
        }
    }
}

/// Holds the currently-active feature set.
///
/// Persisted to the `settings` table as `feature.<kebab-case-name>` = `"1"`.
/// Provides preset constructors for common store types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureRegistry {
    enabled: HashSet<Feature>,
}

impl FeatureRegistry {
    /// Create an empty registry with no features enabled.
    pub fn new() -> Self {
        Self {
            enabled: HashSet::new(),
        }
    }

    /// Create a registry from a set of features.
    ///
    /// # Panics
    ///
    /// Panics if any feature's dependencies are not included in the set.
    pub fn from_set(features: impl IntoIterator<Item = Feature>) -> Self {
        let enabled: HashSet<Feature> = features.into_iter().collect();
        for &f in &enabled {
            for &dep in f.dependencies() {
                assert!(
                    enabled.contains(&dep),
                    "feature {f:?} requires {dep:?} but it is not in the set"
                );
            }
        }
        Self { enabled }
    }

    /// True when the given feature is turned on.
    pub fn is_enabled(&self, feature: Feature) -> bool {
        self.enabled.contains(&feature)
    }

    /// Enable a feature and all of its dependencies recursively.
    ///
    /// Returns `true` if the feature was newly enabled (not already on).
    ///
    /// # Example
    ///
    /// ```
    /// use oz_core::{Feature, FeatureRegistry};
    ///
    /// let mut reg = FeatureRegistry::new();
    ///
    /// // Enabling StaffRoles automatically enables its dependency StaffLogin.
    /// assert!(reg.enable(Feature::StaffRoles));
    /// assert!(reg.is_enabled(Feature::StaffRoles));
    /// assert!(reg.is_enabled(Feature::StaffLogin),  "auto-enabled dep");
    /// assert_eq!(reg.count(), 2);
    ///
    /// // Enabling again returns false (already on).
    /// assert!(!reg.enable(Feature::StaffRoles));
    /// ```
    pub fn enable(&mut self, feature: Feature) -> bool {
        if self.enabled.contains(&feature) {
            return false;
        }
        // Enable dependencies first (bottom-up).
        for &dep in feature.dependencies() {
            self.enable(dep);
        }
        self.enabled.insert(feature)
    }

    /// Disable a feature.
    ///
    /// Does **not** cascade to dependents — callers must decide whether
    /// to disable features that depend on this one. Returns `true` if
    /// the feature was actually removed.
    pub fn disable(&mut self, feature: Feature) -> bool {
        self.enabled.remove(&feature)
    }

    /// All currently-enabled features (unordered).
    pub fn enabled_features(&self) -> impl Iterator<Item = Feature> + '_ {
        self.enabled.iter().copied()
    }

    /// Number of enabled features.
    pub fn count(&self) -> usize {
        self.enabled.len()
    }

    /// Serialize to key-value pairs suitable for the `settings` table.
    ///
    /// Each enabled feature becomes `"feature.<kebab-case-name>"` = `"1"`.
    /// Disabled features are omitted (the settings store may carry stale
    /// keys from previous runs; they will be cleaned up by the store).
    pub fn to_settings_rows(&self) -> Vec<(String, String)> {
        self.enabled
            .iter()
            .map(|f| {
                let key = format!("feature.{}", feature_key(*f));
                (key, "1".into())
            })
            .collect()
    }

    /// Reconstruct a registry from key-value rows loaded from the
    /// `settings` table.
    ///
    /// Rows whose key starts with `"feature."` are parsed; all other
    /// keys are silently ignored. Rows with value `"1"` enable the
    /// feature. Dependency validation is NOT performed — the stored
    /// state is assumed to be consistent.
    pub fn from_settings_rows(rows: &[(String, String)]) -> Self {
        let enabled: HashSet<Feature> = rows
            .iter()
            .filter_map(|(key, value)| {
                if value == "1" {
                    key.strip_prefix("feature.")
                        .and_then(feature_from_key)
                } else {
                    None
                }
            })
            .collect();
        // Note: we purposely skip dependency validation here.
        // The settings store should always be internally consistent;
        // if it's not, the UX layer will handle missing deps gracefully.
        Self { enabled }
    }
}

// ── Presets ────────────────────────────────────────────────────────────

impl FeatureRegistry {
    /// **Simple Retail** — barcode, cash, receipt, inventory, tax.
    pub fn simple_retail() -> Self {
        Self::from_set([
            Feature::SimpleRetail,
            Feature::CashPayment,
            Feature::BarcodeScanning,
            Feature::ReceiptPrinting,
            Feature::InventoryTracking,
            Feature::CategoriesEnabled,
            Feature::TaxEngine,
        ])
    }

    /// **Restaurant** — tables, KDS, cash, receipt, discounts.
    pub fn restaurant() -> Self {
        Self::from_set([
            Feature::Restaurant,
            Feature::CashPayment,
            Feature::ReceiptPrinting,
            Feature::InventoryTracking,
            Feature::CategoriesEnabled,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::KitchenDisplay,
            Feature::TableManagement,
            Feature::StaffLogin,
        ])
    }

    /// **Full Store** — everything except cloud, multi-store, and plugins.
    pub fn full_store() -> Self {
        Self::from_set([
            Feature::SimpleRetail,
            Feature::CashPayment,
            Feature::CardPayment,
            Feature::MultiCurrency,
            Feature::InventoryTracking,
            Feature::ProductVariants,
            Feature::CategoriesEnabled,
            Feature::StaffLogin,
            Feature::StaffRoles,
            Feature::ShiftManagement,
            Feature::AuditLog,
            Feature::BarcodeScanning,
            Feature::ReceiptPrinting,
            Feature::CashDrawer,
            Feature::CustomerDisplay,
            Feature::NfcReader,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::LoyaltyProgram,
            Feature::PromotionsEngine,
            Feature::ProductBundles,
            Feature::Reporting,
            Feature::Analytics,
            Feature::ExportImport,
        ])
    }

    /// **Custom** — empty; the Setup Wizard will enable features one by one.
    pub fn custom() -> Self {
        Self::new()
    }
}

// ── Settings-table helpers ──────────────────────────────────────────────

/// Convert a [`Feature`] to its kebab-case settings key suffix.
///
/// The full settings key is `"feature.<suffix>"`. This function returns
/// just the suffix (e.g., `"simple-retail"`).
pub fn feature_key(f: Feature) -> &'static str {
    match f {
        Feature::SimpleRetail => "simple-retail",
        Feature::Restaurant => "restaurant",
        Feature::CashPayment => "cash-payment",
        Feature::CardPayment => "card-payment",
        Feature::MultiCurrency => "multi-currency",
        Feature::InventoryTracking => "inventory-tracking",
        Feature::ProductVariants => "product-variants",
        Feature::CategoriesEnabled => "categories-enabled",
        Feature::StaffLogin => "staff-login",
        Feature::StaffRoles => "staff-roles",
        Feature::ShiftManagement => "shift-management",
        Feature::AuditLog => "audit-log",
        Feature::BarcodeScanning => "barcode-scanning",
        Feature::ReceiptPrinting => "receipt-printing",
        Feature::CashDrawer => "cash-drawer",
        Feature::CustomerDisplay => "customer-display",
        Feature::NfcReader => "nfc-reader",
        Feature::DiscountEngine => "discount-engine",
        Feature::TaxEngine => "tax-engine",
        Feature::LoyaltyProgram => "loyalty-program",
        Feature::PromotionsEngine => "promotions-engine",
        Feature::ProductBundles => "product-bundles",
        Feature::KitchenDisplay => "kitchen-display",
        Feature::TableManagement => "table-management",
        Feature::SelfServiceKiosk => "self-service-kiosk",
        Feature::CloudSync => "cloud-sync",
        Feature::MultiStore => "multi-store",
        Feature::MultiTerminal => "multi-terminal",
        Feature::Reporting => "reporting",
        Feature::Analytics => "analytics",
        Feature::ExportImport => "export-import",
        Feature::PluginSystem => "plugin-system",
    }
}

/// Parse a kebab-case settings key suffix back to a [`Feature`].
///
/// Returns `None` if the suffix doesn't match any known feature.
pub fn feature_from_key(suffix: &str) -> Option<Feature> {
    match suffix {
        "simple-retail" => Some(Feature::SimpleRetail),
        "restaurant" => Some(Feature::Restaurant),
        "cash-payment" => Some(Feature::CashPayment),
        "card-payment" => Some(Feature::CardPayment),
        "multi-currency" => Some(Feature::MultiCurrency),
        "inventory-tracking" => Some(Feature::InventoryTracking),
        "product-variants" => Some(Feature::ProductVariants),
        "categories-enabled" => Some(Feature::CategoriesEnabled),
        "staff-login" => Some(Feature::StaffLogin),
        "staff-roles" => Some(Feature::StaffRoles),
        "shift-management" => Some(Feature::ShiftManagement),
        "audit-log" => Some(Feature::AuditLog),
        "barcode-scanning" => Some(Feature::BarcodeScanning),
        "receipt-printing" => Some(Feature::ReceiptPrinting),
        "cash-drawer" => Some(Feature::CashDrawer),
        "customer-display" => Some(Feature::CustomerDisplay),
        "nfc-reader" => Some(Feature::NfcReader),
        "discount-engine" => Some(Feature::DiscountEngine),
        "tax-engine" => Some(Feature::TaxEngine),
        "loyalty-program" => Some(Feature::LoyaltyProgram),
        "promotions-engine" => Some(Feature::PromotionsEngine),
        "product-bundles" => Some(Feature::ProductBundles),
        "kitchen-display" => Some(Feature::KitchenDisplay),
        "table-management" => Some(Feature::TableManagement),
        "self-service-kiosk" => Some(Feature::SelfServiceKiosk),
        "cloud-sync" => Some(Feature::CloudSync),
        "multi-store" => Some(Feature::MultiStore),
        "multi-terminal" => Some(Feature::MultiTerminal),
        "reporting" => Some(Feature::Reporting),
        "analytics" => Some(Feature::Analytics),
        "export-import" => Some(Feature::ExportImport),
        "plugin-system" => Some(Feature::PluginSystem),
        _ => None,
    }
}

// ── Default ─────────────────────────────────────────────────────────────

impl Default for FeatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── All dependency declarations ──────────────────────────────
    //
    // Every Feature with at least one dependency must be tested below
    // to ensure the dependency graph stays correct as the enum grows.

    #[test]
    fn simple_retail_has_no_deps() {
        assert!(Feature::SimpleRetail.dependencies().is_empty());
    }

    #[test]
    fn staff_roles_depends_on_staff_login() {
        assert_eq!(Feature::StaffRoles.dependencies(), &[Feature::StaffLogin]);
    }

    #[test]
    fn shift_management_depends_on_staff_login() {
        assert_eq!(
            Feature::ShiftManagement.dependencies(),
            &[Feature::StaffLogin]
        );
    }

    #[test]
    fn audit_log_depends_on_staff_login() {
        assert_eq!(Feature::AuditLog.dependencies(), &[Feature::StaffLogin]);
    }

    #[test]
    fn cash_drawer_depends_on_receipt_printing() {
        assert_eq!(
            Feature::CashDrawer.dependencies(),
            &[Feature::ReceiptPrinting]
        );
    }

    #[test]
    fn customer_display_depends_on_receipt_printing() {
        assert_eq!(
            Feature::CustomerDisplay.dependencies(),
            &[Feature::ReceiptPrinting]
        );
    }

    #[test]
    fn kitchen_display_depends_on_restaurant() {
        assert_eq!(
            Feature::KitchenDisplay.dependencies(),
            &[Feature::Restaurant]
        );
    }

    #[test]
    fn table_management_depends_on_restaurant() {
        assert_eq!(
            Feature::TableManagement.dependencies(),
            &[Feature::Restaurant]
        );
    }

    #[test]
    fn self_service_kiosk_depends_on_simple_retail() {
        assert_eq!(
            Feature::SelfServiceKiosk.dependencies(),
            &[Feature::SimpleRetail]
        );
    }

    #[test]
    fn loyalty_program_depends_on_staff_login() {
        assert_eq!(
            Feature::LoyaltyProgram.dependencies(),
            &[Feature::StaffLogin]
        );
    }

    #[test]
    fn promotions_engine_depends_on_discount_engine() {
        assert_eq!(
            Feature::PromotionsEngine.dependencies(),
            &[Feature::DiscountEngine]
        );
    }

    #[test]
    fn analytics_depends_on_reporting() {
        assert_eq!(Feature::Analytics.dependencies(), &[Feature::Reporting]);
    }

    #[test]
    fn multi_terminal_depends_on_multi_store() {
        assert_eq!(
            Feature::MultiTerminal.dependencies(),
            &[Feature::MultiStore]
        );
    }

    #[test]
    fn cloud_sync_depends_on_multi_store() {
        assert_eq!(Feature::CloudSync.dependencies(), &[Feature::MultiStore]);
    }

    /// All features that have **no** dependencies.
    /// If this test fails, a new feature with deps wasn't added to the
    /// dependency tests above, or a feature's deps changed unexpectedly.
    #[test]
    fn features_without_dependencies_have_empty_slice() {
        let no_deps = [
            Feature::SimpleRetail,
            Feature::Restaurant,
            Feature::CashPayment,
            Feature::CardPayment,
            Feature::MultiCurrency,
            Feature::InventoryTracking,
            Feature::ProductVariants,
            Feature::CategoriesEnabled,
            Feature::StaffLogin,
            Feature::BarcodeScanning,
            Feature::ReceiptPrinting,
            Feature::NfcReader,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::ProductBundles,
            Feature::Reporting,
            Feature::MultiStore,
            Feature::ExportImport,
            Feature::PluginSystem,
        ];
        for f in no_deps {
            assert!(
                f.dependencies().is_empty(),
                "expected {f:?} to have no dependencies"
            );
        }
    }

    /// Every feature that HAS dependencies is listed here so we catch
    /// regressions — if a new feature is added with deps but no test is
    /// written, this test will need updating.
    ///
    /// Note: the dependency graph is a static DAG (no cycles) because
    /// [`Feature::dependencies`] returns a fixed slice — there is no
    /// dynamic registration mechanism that could introduce cycles.
    #[test]
    fn all_features_known_dep_or_no_dep() {
        // Features with at least one dependency.
        let with_deps: std::collections::HashSet<Feature> = [
            Feature::StaffRoles,
            Feature::ShiftManagement,
            Feature::AuditLog,
            Feature::CashDrawer,
            Feature::CustomerDisplay,
            Feature::KitchenDisplay,
            Feature::TableManagement,
            Feature::SelfServiceKiosk,
            Feature::LoyaltyProgram,
            Feature::PromotionsEngine,
            Feature::Analytics,
            Feature::MultiTerminal,
            Feature::CloudSync,
        ]
        .into_iter()
        .collect();

        // All 32 features listed explicitly (same pattern as
        // `feature_key_roundtrip`). This avoids unsafe transmute.
        let all_features = [
            Feature::SimpleRetail,
            Feature::Restaurant,
            Feature::CashPayment,
            Feature::CardPayment,
            Feature::MultiCurrency,
            Feature::InventoryTracking,
            Feature::ProductVariants,
            Feature::CategoriesEnabled,
            Feature::StaffLogin,
            Feature::StaffRoles,
            Feature::ShiftManagement,
            Feature::AuditLog,
            Feature::BarcodeScanning,
            Feature::ReceiptPrinting,
            Feature::CashDrawer,
            Feature::CustomerDisplay,
            Feature::NfcReader,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::LoyaltyProgram,
            Feature::PromotionsEngine,
            Feature::ProductBundles,
            Feature::KitchenDisplay,
            Feature::TableManagement,
            Feature::SelfServiceKiosk,
            Feature::CloudSync,
            Feature::MultiStore,
            Feature::MultiTerminal,
            Feature::Reporting,
            Feature::Analytics,
            Feature::ExportImport,
            Feature::PluginSystem,
        ];

        for &f in &all_features {
            if with_deps.contains(&f) {
                assert!(
                    !f.dependencies().is_empty(),
                    "{f:?} tagged as having deps but returned empty"
                );
            } else {
                assert!(
                    f.dependencies().is_empty(),
                    "{f:?} has dependencies but is not listed in the with_deps set: {:?}",
                    f.dependencies(),
                );
            }
        }
    }

    // ── Enable / disable ─────────────────────────────────────────

    #[test]
    fn new_registry_is_empty() {
        let reg = FeatureRegistry::new();
        assert_eq!(reg.count(), 0);
        assert!(!reg.is_enabled(Feature::SimpleRetail));
    }

    #[test]
    fn enable_returns_true_when_new() {
        let mut reg = FeatureRegistry::new();
        assert!(reg.enable(Feature::CashPayment));
        assert!(reg.is_enabled(Feature::CashPayment));
    }

    #[test]
    fn enable_returns_false_when_already_on() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::CashPayment);
        assert!(!reg.enable(Feature::CashPayment));
    }

    #[test]
    fn disable_removes_feature() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::CashPayment);
        assert!(reg.disable(Feature::CashPayment));
        assert!(!reg.is_enabled(Feature::CashPayment));
    }

    #[test]
    fn disable_returns_false_when_not_enabled() {
        let mut reg = FeatureRegistry::new();
        assert!(!reg.disable(Feature::CashPayment));
    }

    #[test]
    fn enable_returns_true_after_disable() {
        let mut reg = FeatureRegistry::new();
        assert!(reg.enable(Feature::CashPayment));
        assert!(reg.disable(Feature::CashPayment));
        assert!(reg.enable(Feature::CashPayment), "re-enable after disable");
    }

    #[test]
    fn enable_auto_enables_dependencies() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::StaffRoles);
        assert!(reg.is_enabled(Feature::StaffLogin), "auto-enabled dep");
        assert!(reg.is_enabled(Feature::StaffRoles));
    }

    #[test]
    fn enable_resolves_deep_dependency_chain() {
        let mut reg = FeatureRegistry::new();
        // MultiTerminal → MultiStore, MultiStore has no deps.
        reg.enable(Feature::MultiTerminal);
        assert!(reg.is_enabled(Feature::MultiStore));
        assert!(reg.is_enabled(Feature::MultiTerminal));
    }

    #[test]
    fn enable_with_dep_already_present() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::StaffLogin);
        assert!(reg.enable(Feature::StaffRoles));
        assert!(reg.is_enabled(Feature::StaffRoles));
        assert!(reg.is_enabled(Feature::StaffLogin));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn enable_dep_unchanged_when_dependent_already_present() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::StaffRoles);
        assert!(!reg.enable(Feature::StaffLogin));
    }

    #[test]
    fn enable_multiple_features_sharing_dependency() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::StaffRoles);
        reg.enable(Feature::ShiftManagement);
        reg.enable(Feature::AuditLog);
        assert!(reg.is_enabled(Feature::StaffLogin));
        assert!(reg.is_enabled(Feature::StaffRoles));
        assert!(reg.is_enabled(Feature::ShiftManagement));
        assert!(reg.is_enabled(Feature::AuditLog));
        assert_eq!(reg.count(), 4);
    }

    #[test]
    fn enable_multiple_features_sharing_two_level_dep() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::CashDrawer);
        reg.enable(Feature::CustomerDisplay);
        assert!(reg.is_enabled(Feature::ReceiptPrinting));
        assert!(reg.is_enabled(Feature::CashDrawer));
        assert!(reg.is_enabled(Feature::CustomerDisplay));
        assert_eq!(reg.count(), 3);
    }

    #[test]
    fn disable_does_not_cascade_to_dependents() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::StaffRoles);
        reg.disable(Feature::StaffLogin);
        assert!(!reg.is_enabled(Feature::StaffLogin));
        assert!(reg.is_enabled(Feature::StaffRoles));
    }

    #[test]
    fn disable_dep_then_re_enable_dependent_restores_dep() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::StaffRoles);
        assert!(reg.is_enabled(Feature::StaffLogin));
        assert!(reg.is_enabled(Feature::StaffRoles));

        reg.disable(Feature::StaffRoles);
        reg.disable(Feature::StaffLogin);
        assert!(!reg.is_enabled(Feature::StaffLogin));
        assert!(!reg.is_enabled(Feature::StaffRoles));

        assert!(reg.enable(Feature::StaffRoles));
        assert!(reg.is_enabled(Feature::StaffRoles));
        assert!(reg.is_enabled(Feature::StaffLogin), "dep restored by enable");
    }

    #[test]
    fn count_correct_after_enable_disable_chain() {
        let mut reg = FeatureRegistry::new();
        assert_eq!(reg.count(), 0);

        reg.enable(Feature::CashPayment);
        assert_eq!(reg.count(), 1);

        reg.enable(Feature::StaffRoles);
        assert_eq!(reg.count(), 3);

        reg.disable(Feature::StaffLogin);
        assert_eq!(reg.count(), 2);

        reg.disable(Feature::CashPayment);
        assert_eq!(reg.count(), 1);

        reg.disable(Feature::StaffRoles);
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn enable_all_features_then_disable_all() {
        let mut reg = FeatureRegistry::new();
        let all_features = [
            Feature::SimpleRetail,
            Feature::Restaurant,
            Feature::CashPayment,
            Feature::CardPayment,
            Feature::MultiCurrency,
            Feature::InventoryTracking,
            Feature::ProductVariants,
            Feature::CategoriesEnabled,
            Feature::StaffLogin,
            Feature::StaffRoles,
            Feature::ShiftManagement,
            Feature::AuditLog,
            Feature::BarcodeScanning,
            Feature::ReceiptPrinting,
            Feature::CashDrawer,
            Feature::CustomerDisplay,
            Feature::NfcReader,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::LoyaltyProgram,
            Feature::PromotionsEngine,
            Feature::ProductBundles,
            Feature::KitchenDisplay,
            Feature::TableManagement,
            Feature::SelfServiceKiosk,
            Feature::CloudSync,
            Feature::MultiStore,
            Feature::MultiTerminal,
            Feature::Reporting,
            Feature::Analytics,
            Feature::ExportImport,
            Feature::PluginSystem,
        ];

        for f in all_features {
            reg.enable(f);
        }
        assert!(reg.count() >= 32);

        for f in all_features {
            reg.disable(f);
        }
        assert_eq!(reg.count(), 0);
        assert!(!reg.is_enabled(Feature::SimpleRetail));
        assert!(!reg.is_enabled(Feature::PluginSystem));
    }

    #[test]
    fn from_set_with_all_deps_present_does_not_panic() {
        let reg = FeatureRegistry::from_set([
            Feature::SimpleRetail,
            Feature::SelfServiceKiosk,
        ]);
        assert!(reg.is_enabled(Feature::SimpleRetail));
        assert!(reg.is_enabled(Feature::SelfServiceKiosk));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn from_set_with_direct_dep_present_does_not_panic() {
        let reg = FeatureRegistry::from_set([
            Feature::Reporting,
            Feature::Analytics,
        ]);
        assert!(reg.is_enabled(Feature::Reporting));
        assert!(reg.is_enabled(Feature::Analytics));
    }

    // ── Presets ──────────────────────────────────────────────────

    #[test]
    fn simple_retail_preset_has_expected_features() {
        let reg = FeatureRegistry::simple_retail();
        assert!(reg.is_enabled(Feature::SimpleRetail));
        assert!(reg.is_enabled(Feature::CashPayment));
        assert!(reg.is_enabled(Feature::BarcodeScanning));
        assert!(reg.is_enabled(Feature::ReceiptPrinting));
        assert!(reg.is_enabled(Feature::InventoryTracking));
        assert!(reg.is_enabled(Feature::CategoriesEnabled));
        assert!(reg.is_enabled(Feature::TaxEngine));
        assert!(!reg.is_enabled(Feature::CardPayment));
        assert!(!reg.is_enabled(Feature::StaffLogin));
    }

    #[test]
    fn restaurant_preset_includes_dependencies() {
        let reg = FeatureRegistry::restaurant();
        assert!(reg.is_enabled(Feature::Restaurant));
        assert!(reg.is_enabled(Feature::KitchenDisplay));
        assert!(reg.is_enabled(Feature::TableManagement));
        assert!(reg.is_enabled(Feature::StaffLogin));
    }

    #[test]
    fn full_store_preset_is_large() {
        let reg = FeatureRegistry::full_store();
        assert!(reg.count() >= 20);
        assert!(reg.is_enabled(Feature::SimpleRetail));
        assert!(reg.is_enabled(Feature::CardPayment));
        assert!(reg.is_enabled(Feature::StaffLogin));
        assert!(reg.is_enabled(Feature::Analytics));
        assert!(!reg.is_enabled(Feature::CloudSync));
        assert!(!reg.is_enabled(Feature::MultiStore));
    }

    #[test]
    fn custom_preset_is_empty() {
        let reg = FeatureRegistry::custom();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    #[should_panic]
    fn from_set_panics_on_missing_dependency() {
        FeatureRegistry::from_set([Feature::StaffRoles]);
    }

    // ── Settings serialization ───────────────────────────────────

    #[test]
    fn to_settings_rows_empty_registry() {
        let reg = FeatureRegistry::new();
        assert!(reg.to_settings_rows().is_empty());
    }

    #[test]
    fn to_settings_rows_produces_expected_keys() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::CashPayment);
        reg.enable(Feature::BarcodeScanning);
        let rows = reg.to_settings_rows();
        assert_eq!(rows.len(), 2);
        assert!(rows.contains(&("feature.cash-payment".into(), "1".into())));
        assert!(rows.contains(&("feature.barcode-scanning".into(), "1".into())));
    }

    #[test]
    fn from_settings_rows_reconstructs_registry() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::CashPayment);
        reg.enable(Feature::BarcodeScanning);
        let rows = reg.to_settings_rows();
        let back = FeatureRegistry::from_settings_rows(&rows);
        assert_eq!(back, reg);
    }

    #[test]
    fn from_settings_rows_ignores_non_feature_keys() {
        let rows: Vec<(String, String)> = vec![
            ("feature.cash-payment".into(), "1".into()),
            ("store.name".into(), "My Store".into()),
            ("feature.barcode-scanning".into(), "1".into()),
            ("random.key".into(), "whatever".into()),
        ];
        let reg = FeatureRegistry::from_settings_rows(&rows);
        assert!(reg.is_enabled(Feature::CashPayment));
        assert!(reg.is_enabled(Feature::BarcodeScanning));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn from_settings_rows_ignores_zero_valued_features() {
        let rows: Vec<(String, String)> = vec![
            ("feature.cash-payment".into(), "0".into()),
            ("feature.tax-engine".into(), "1".into()),
        ];
        let reg = FeatureRegistry::from_settings_rows(&rows);
        assert!(!reg.is_enabled(Feature::CashPayment));
        assert!(reg.is_enabled(Feature::TaxEngine));
    }

    #[test]
    fn feature_key_roundtrip() {
        let features = [
            Feature::SimpleRetail,
            Feature::Restaurant,
            Feature::CashPayment,
            Feature::CardPayment,
            Feature::MultiCurrency,
            Feature::InventoryTracking,
            Feature::ProductVariants,
            Feature::CategoriesEnabled,
            Feature::StaffLogin,
            Feature::StaffRoles,
            Feature::ShiftManagement,
            Feature::AuditLog,
            Feature::BarcodeScanning,
            Feature::ReceiptPrinting,
            Feature::CashDrawer,
            Feature::CustomerDisplay,
            Feature::NfcReader,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::LoyaltyProgram,
            Feature::PromotionsEngine,
            Feature::ProductBundles,
            Feature::KitchenDisplay,
            Feature::TableManagement,
            Feature::SelfServiceKiosk,
            Feature::CloudSync,
            Feature::MultiStore,
            Feature::MultiTerminal,
            Feature::Reporting,
            Feature::Analytics,
            Feature::ExportImport,
            Feature::PluginSystem,
        ];
        for f in features {
            let key = feature_key(f);
            let back = feature_from_key(key).unwrap();
            assert_eq!(back, f, "roundtrip failed for {f:?}");
        }
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let reg = FeatureRegistry::simple_retail();
        let json = serde_json::to_string(&reg).unwrap();
        let back: FeatureRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(back, reg);
    }

    // ── Iterator ─────────────────────────────────────────────────

    #[test]
    fn enabled_features_iterator() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::CashPayment);
        reg.enable(Feature::BarcodeScanning);
        let features: Vec<_> = reg.enabled_features().collect();
        assert_eq!(features.len(), 2);
        assert!(features.contains(&Feature::CashPayment));
    }
}

// ── Property-based tests (proptest) ─────────────────────────────────

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// List of all 32 features for generating random selections.
    const ALL_FEATURES: &[Feature] = &[
        Feature::SimpleRetail,
        Feature::Restaurant,
        Feature::CashPayment,
        Feature::CardPayment,
        Feature::MultiCurrency,
        Feature::InventoryTracking,
        Feature::ProductVariants,
        Feature::CategoriesEnabled,
        Feature::StaffLogin,
        Feature::StaffRoles,
        Feature::ShiftManagement,
        Feature::AuditLog,
        Feature::BarcodeScanning,
        Feature::ReceiptPrinting,
        Feature::CashDrawer,
        Feature::CustomerDisplay,
        Feature::NfcReader,
        Feature::DiscountEngine,
        Feature::TaxEngine,
        Feature::LoyaltyProgram,
        Feature::PromotionsEngine,
        Feature::ProductBundles,
        Feature::KitchenDisplay,
        Feature::TableManagement,
        Feature::SelfServiceKiosk,
        Feature::CloudSync,
        Feature::MultiStore,
        Feature::MultiTerminal,
        Feature::Reporting,
        Feature::Analytics,
        Feature::ExportImport,
        Feature::PluginSystem,
    ];

    /// Strategy: a random sequence of enable/disable operations.
    /// Each step is `(should_enable, feature_index)`. We use the index
    /// into `ALL_FEATURES` rather than the Feature directly to work
    /// around proptest's requirement for `Arbitrary`.
    fn arb_ops() -> impl Strategy<Value = Vec<(bool, usize)>> {
        proptest::collection::vec(
            (proptest::bool::ANY, 0usize..ALL_FEATURES.len()),
            0..200, // sequences from 0 to 200 steps
        )
    }

    // ── Invariant: newly-enabled features satisfy deps ───────────

    proptest! {
        /// After every `enable` call, the **newly-enabled** features
        /// (including auto-enabled dependencies) all have their own
        /// dependencies satisfied.
        ///
        /// We only check newly-added features, not the full set,
        /// because `disable` intentionally does not cascade — a
        /// feature can be left with a missing dependency after a
        /// previous `disable` call, and that is by design.
        #[test]
        fn dependency_invariant_holds_after_enables(ops in arb_ops()) {
            let mut reg = FeatureRegistry::new();

            for (should_enable, idx) in &ops {
                let feature = ALL_FEATURES[*idx];
                if *should_enable {
                    let before: HashSet<Feature> = reg.enabled_features().collect();
                    reg.enable(feature);
                    let after: HashSet<Feature> = reg.enabled_features().collect();
                    let new_features: Vec<&Feature> =
                        after.difference(&before).collect();

                    for &&f in &new_features {
                        for &dep in f.dependencies() {
                            prop_assert!(
                                reg.is_enabled(dep),
                                "after enable({f:?}): dep {dep:?} is missing"
                            );
                        }
                    }
                } else {
                    reg.disable(feature);
                }
            }
        }
    }

    // ── Disable does NOT cascade (design property) ────────────────

    proptest! {
        /// `disable` does NOT cascade to dependents — only the
        /// specific feature is removed from the set. Every other
        /// feature that was enabled remains enabled.
        #[test]
        fn disable_does_not_cascade(ops in arb_ops()) {
            let mut reg = FeatureRegistry::new();

            // Enable features from all operations marked for enable.
            for (should_enable, idx) in &ops {
                if *should_enable {
                    reg.enable(ALL_FEATURES[*idx]);
                }
            }

            // Snapshot the enabled set before any disable calls.
            let before_disable: HashSet<Feature> = reg.enabled_features().collect();

            // Disable features marked for disable.
            let disabled_features: Vec<Feature> = ops
                .iter()
                .filter(|(se, _)| !*se)
                .map(|(_, idx)| ALL_FEATURES[*idx])
                .collect();
            for &f in &disabled_features {
                reg.disable(f);
            }

            // Every disabled feature is no longer in the set.
            for &f in &disabled_features {
                prop_assert!(!reg.is_enabled(f), "disable({f:?}) should have removed it");
            }

            // Every feature that WAS in the set and was NOT disabled
            // is still enabled (disable does not cascade).
            for &f in &before_disable {
                if !disabled_features.contains(&f) {
                    prop_assert!(reg.is_enabled(f), "{f:?} was removed despite not being disabled");
                }
            }
        }
    }

    // ── Enable return value ───────────────────────────────────────

    proptest! {
        /// `enable(f)` returns `true` iff `f` was NOT already in the set.
        #[test]
        fn enable_return_value_matches_precondition(ops in arb_ops()) {
            let mut reg = FeatureRegistry::new();

            for (should_enable, idx) in &ops {
                let feature = ALL_FEATURES[*idx];
                if *should_enable {
                    let was_enabled = reg.is_enabled(feature);
                    prop_assert_eq!(reg.enable(feature), !was_enabled);
                    prop_assert!(reg.is_enabled(feature));
                }
            }
        }
    }

    // ── Disable return value ──────────────────────────────────────

    proptest! {
        /// `disable(f)` returns `true` iff `f` WAS already in the set.
        #[test]
        fn disable_return_value_matches_precondition(ops in arb_ops()) {
            let mut reg = FeatureRegistry::new();

            // First, enable a bunch of features.
            for (should_enable, idx) in &ops {
                if *should_enable {
                    reg.enable(ALL_FEATURES[*idx]);
                }
            }

            // Then disable the same features.
            for (_, idx) in &ops {
                let feature = ALL_FEATURES[*idx];
                let was_enabled = reg.is_enabled(feature);
                prop_assert_eq!(reg.disable(feature), was_enabled);
            }
        }
    }

    // ── Serialization roundtrip ───────────────────────────────────

    proptest! {
        /// A registry survives `to_settings_rows` → `from_settings_rows`
        /// losslessly (modulo unknown keys, which we don't supply).
        #[test]
        fn serialization_roundtrip(ops in arb_ops()) {
            let mut reg = FeatureRegistry::new();

            for (should_enable, idx) in &ops {
                if *should_enable {
                    reg.enable(ALL_FEATURES[*idx]);
                } else {
                    reg.disable(ALL_FEATURES[*idx]);
                }
            }

            let rows = reg.to_settings_rows();
            let back = FeatureRegistry::from_settings_rows(&rows);
            prop_assert_eq!(back, reg);
        }
    }

    // ── Presets satisfy invariant ─────────────────────────────────

    #[test]
    fn simple_retail_preset_satisfies_invariant() {
        let reg = FeatureRegistry::simple_retail();
        for f in reg.enabled_features() {
            for &dep in f.dependencies() {
                assert!(
                    reg.is_enabled(dep),
                    "simple_retail: {f:?} enabled but dep {dep:?} is not"
                );
            }
        }
    }

    #[test]
    fn restaurant_preset_satisfies_invariant() {
        let reg = FeatureRegistry::restaurant();
        for f in reg.enabled_features() {
            for &dep in f.dependencies() {
                assert!(
                    reg.is_enabled(dep),
                    "restaurant: {f:?} enabled but dep {dep:?} is not"
                );
            }
        }
    }

    #[test]
    fn full_store_preset_satisfies_invariant() {
        let reg = FeatureRegistry::full_store();
        for f in reg.enabled_features() {
            for &dep in f.dependencies() {
                assert!(
                    reg.is_enabled(dep),
                    "full_store: {f:?} enabled but dep {dep:?} is not"
                );
            }
        }
    }

    // ── Empty registry invariant ──────────────────────────────────

    #[test]
    fn empty_registry_satisfies_invariant() {
        let reg = FeatureRegistry::new();
        assert_eq!(reg.count(), 0);
        let features: Vec<Feature> = reg.enabled_features().collect();
        assert!(features.is_empty(), "empty registry should have no enabled features: {features:?}");
    }
}
