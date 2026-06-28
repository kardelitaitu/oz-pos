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
fn feature_key(f: Feature) -> &'static str {
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

    // ── Dependencies ─────────────────────────────────────────────

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
    fn cash_drawer_depends_on_receipt_printing() {
        assert_eq!(
            Feature::CashDrawer.dependencies(),
            &[Feature::ReceiptPrinting]
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
    fn disable_does_not_cascade_to_dependents() {
        let mut reg = FeatureRegistry::new();
        reg.enable(Feature::StaffRoles);
        reg.disable(Feature::StaffLogin);
        assert!(!reg.is_enabled(Feature::StaffLogin));
        // StaffRoles stays on (caller handles cascade).
        assert!(reg.is_enabled(Feature::StaffRoles));
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
        // Cloud features NOT included.
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
        FeatureRegistry::from_set([Feature::StaffRoles]); // StaffLogin missing
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
        // Every feature should serialize and deserialize correctly.
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
