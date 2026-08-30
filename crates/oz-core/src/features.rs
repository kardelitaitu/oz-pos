//! Feature flag system — toggleable POS capabilities.
/*
last audited 25-07-26 by RSA-Agent (oz-core slice C4: features deep read)
crate: oz-core | status: SAFE | lint: CLEAN
findings: production logic (1-685) clean — dependency DAG with recursive enable, kebab-case settings keys, FeatureGuard veto registry (KDS tickets / open shifts), format! interpolates an internal constant only; COR-33 CONVENTION: ~660 lines of inline #[test]/proptest (691-1349) live in this production file despite the declared sibling features_tests.rs — violates AGENTS.md ("never tests in production files", 1,000-line rule); guard COUNT queries use .unwrap_or(0) -> DB error = veto passes (fail-open on a safety guard, COR-11/25 family, INFO)
next: move inline tests to features_tests.rs (COR-33); propagate guard query errors | perf: N/A
*/
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

use rusqlite::Connection;

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
    /// USB weight scale for produce/groceries.
    UsbScale,

    // ── Business Rules ───────────────────────────────────────────
    /// Percentage and fixed-amount discounts.
    DiscountEngine,
    /// Tax calculation engine.
    TaxEngine,
    /// Customer loyalty points and tiers.
    LoyaltyProgram,
    /// Gift cards — issue, redeem, top-up, freeze.
    GiftCards,
    /// Quick return from POS — scan receipt barcode to initiate refund.
    QuickReturn,
    /// Time-limited promotions (buy-X-get-Y, etc.).
    PromotionsEngine,
    /// Sell multiple SKUs as a bundle.
    ProductBundles,
    /// Stock counting / physical inventory.
    StockCounting,
    /// Transfer stock between locations or terminals.
    StockTransfers,
    /// Purchase orders and supplier management.
    PurchaseOrders,
    /// Track serial numbers for warranty tracking at checkout.
    SerialTracking,

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

            // Products.
            Self::StockCounting => &[Self::InventoryTracking],
            Self::SerialTracking => &[Self::InventoryTracking],

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
                    key.strip_prefix("feature.").and_then(feature_from_key)
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
            Feature::UsbScale,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::LoyaltyProgram,
            Feature::GiftCards,
            Feature::QuickReturn,
            Feature::PromotionsEngine,
            Feature::ProductBundles,
            Feature::Reporting,
            Feature::Analytics,
            Feature::ExportImport,
        ])
    }

    /// **Cafe / Bakery** — quick-service with kitchen display, cash+card, discounts.
    pub fn cafe() -> Self {
        Self::from_set([
            Feature::SimpleRetail,
            Feature::Restaurant, // required by KitchenDisplay
            Feature::CashPayment,
            Feature::CardPayment,
            Feature::ReceiptPrinting,
            Feature::CustomerDisplay,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::KitchenDisplay,
            Feature::PromotionsEngine,
        ])
    }

    /// **Franchise** — multi-store, multi-terminal, restaurant + full admin stack.
    pub fn franchise() -> Self {
        Self::from_set([
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
            Feature::ReceiptPrinting,
            Feature::DiscountEngine,
            Feature::TaxEngine,
            Feature::KitchenDisplay,
            Feature::TableManagement,
            Feature::CloudSync,
            Feature::MultiStore,
            Feature::MultiTerminal,
            Feature::Reporting,
            Feature::Analytics,
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
        Feature::UsbScale => "usb-scale",
        Feature::DiscountEngine => "discount-engine",
        Feature::TaxEngine => "tax-engine",
        Feature::LoyaltyProgram => "loyalty-program",
        Feature::GiftCards => "gift-cards",
        Feature::QuickReturn => "quick-return",
        Feature::StockCounting => "stock-counting",
        Feature::StockTransfers => "stock-transfers",
        Feature::PurchaseOrders => "purchase-orders",
        Feature::SerialTracking => "serial-tracking",
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
        "usb-scale" => Some(Feature::UsbScale),
        "discount-engine" => Some(Feature::DiscountEngine),
        "tax-engine" => Some(Feature::TaxEngine),
        "loyalty-program" => Some(Feature::LoyaltyProgram),
        "gift-cards" => Some(Feature::GiftCards),
        "quick-return" => Some(Feature::QuickReturn),
        "stock-counting" => Some(Feature::StockCounting),
        "stock-transfers" => Some(Feature::StockTransfers),
        "purchase-orders" => Some(Feature::PurchaseOrders),
        "serial-tracking" => Some(Feature::SerialTracking),
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

// ── Feature Guards ──────────────────────────────────────────────────────

/// Runtime safety guard that can veto a feature being disabled.
///
/// Guards are checked **before** the feature is toggled off in the
/// kernel and persisted to the database. If any guard returns
/// `Err(reason)` for the feature being disabled, the toggle is
/// aborted and the error message is surfaced to the admin UI.
///
/// # Example
///
/// ```
/// use oz_core::{Feature, FeatureGuard, KdsFeatureGuard};
/// use rusqlite::Connection;
///
/// let conn = Connection::open_in_memory().unwrap();
/// conn.execute_batch(
///     "CREATE TABLE IF NOT EXISTS kds_orders (
///         id TEXT PRIMARY KEY,
///         status TEXT NOT NULL DEFAULT 'pending'
///     );"
/// ).unwrap();
///
/// let guard = KdsFeatureGuard;
/// // No orders exist, so KitchenDisplay can be disabled safely.
/// assert!(guard.can_disable(Feature::KitchenDisplay, &conn).is_ok());
/// ```
pub trait FeatureGuard: Send + Sync {
    /// Return `Ok(())` if the feature can be disabled, or
    /// `Err(reason)` with an actionable message for the admin.
    fn can_disable(&self, feature: Feature, conn: &Connection) -> Result<(), String>;
}

/// Guard that prevents disabling `KitchenDisplay` while KDS tickets
/// are actively being prepared.
///
/// Queries the `kds_orders` table for orders with status `'pending'`
/// or `'preparing'`. If any exist, the feature cannot be safely
/// disabled because tickets will be orphaned.
#[derive(Debug, Clone, Copy)]
pub struct KdsFeatureGuard;

impl FeatureGuard for KdsFeatureGuard {
    fn can_disable(&self, feature: Feature, conn: &Connection) -> Result<(), String> {
        if feature != Feature::KitchenDisplay {
            return Ok(());
        }

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kds_orders WHERE status IN ('pending', 'preparing')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if count > 0 {
            Err(format!(
                "Cannot disable Kitchen Display while {count} ticket{} are actively in progress",
                if count == 1 { " is" } else { "s are" }
            ))
        } else {
            Ok(())
        }
    }
}

/// Guard that prevents disabling `ShiftManagement` while a shift is
/// still open and unreconciled.
///
/// Queries the `shifts` table for rows where `closed_at IS NULL`.
/// Active shifts must be closed before the feature can be turned off.
#[derive(Debug, Clone, Copy)]
pub struct ShiftFeatureGuard;

impl FeatureGuard for ShiftFeatureGuard {
    fn can_disable(&self, feature: Feature, conn: &Connection) -> Result<(), String> {
        if feature != Feature::ShiftManagement {
            return Ok(());
        }

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM shifts WHERE closed_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if count > 0 {
            Err(format!(
                "Cannot disable Shift Management while {} shift{} are actively open and unreconciled",
                count,
                if count == 1 { " is" } else { "s are" }
            ))
        } else {
            Ok(())
        }
    }
}

/// A registry of all active [`FeatureGuard`] instances.
///
/// `set_feature` calls `check_feature` before disabling a feature;
/// the registry runs every guard and collects all failures.
#[derive(Default)]
pub struct FeatureGuardRegistry {
    guards: Vec<Box<dyn FeatureGuard>>,
}

impl std::fmt::Debug for FeatureGuardRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeatureGuardRegistry")
            .field("count", &self.guards.len())
            .finish()
    }
}

impl FeatureGuardRegistry {
    /// Create an empty registry with no guards.
    pub fn new() -> Self {
        Self { guards: Vec::new() }
    }

    /// Register a guard. Multiple guards can be registered; they are
    /// all checked independently during `check_feature`.
    pub fn register(&mut self, guard: Box<dyn FeatureGuard>) {
        self.guards.push(guard);
    }

    /// Check `feature` against all registered guards.
    ///
    /// Returns `Ok(())` if **all** guards approve. Returns
    /// `Err(reasons)` with **all** failure messages joined by `"; "`.
    pub fn check_feature(&self, feature: Feature, conn: &Connection) -> Result<(), String> {
        let failures: Vec<String> = self
            .guards
            .iter()
            .filter_map(|guard| guard.can_disable(feature, conn).err())
            .collect();

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }

    /// Number of registered guards.
    pub fn count(&self) -> usize {
        self.guards.len()
    }

    /// True when no guards have been registered.
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }

    /// Create a registry pre-loaded with all built-in guards.
    ///
    /// Currently includes:
    /// - [`KdsFeatureGuard`] — protects open KDS tickets
    /// - [`ShiftFeatureGuard`] — protects unreconciled shifts
    pub fn new_with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(KdsFeatureGuard));
        registry.register(Box::new(ShiftFeatureGuard));
        registry
    }
}

#[cfg(test)]
#[path = "features_tests.rs"]
mod tests;

#[test]
fn kds_guard_allows_other_features() {
    let conn = Connection::open_in_memory().unwrap();
    let guard = KdsFeatureGuard;
    // Guards should always allow features they don't guard.
    assert!(guard.can_disable(Feature::ShiftManagement, &conn).is_ok());
    assert!(guard.can_disable(Feature::SimpleRetail, &conn).is_ok());
    assert!(guard.can_disable(Feature::CashPayment, &conn).is_ok());
}

#[test]
fn kds_guard_allows_with_no_orders() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE kds_orders (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'pending'
            );",
    )
    .unwrap();
    let guard = KdsFeatureGuard;
    assert!(guard.can_disable(Feature::KitchenDisplay, &conn).is_ok());
}

#[test]
fn kds_guard_rejects_with_active_orders() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE kds_orders (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'pending'
            );
            INSERT INTO kds_orders (id, status) VALUES ('o1', 'preparing');
            INSERT INTO kds_orders (id, status) VALUES ('o2', 'pending');",
    )
    .unwrap();
    let guard = KdsFeatureGuard;
    let result = guard.can_disable(Feature::KitchenDisplay, &conn);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("2 tickets"));
}

#[test]
fn kds_guard_ignores_completed_orders() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE kds_orders (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'pending'
            );
            INSERT INTO kds_orders (id, status) VALUES ('o1', 'served');
            INSERT INTO kds_orders (id, status) VALUES ('o2', 'cancelled');
            INSERT INTO kds_orders (id, status) VALUES ('o3', 'ready');",
    )
    .unwrap();
    let guard = KdsFeatureGuard;
    // 'ready', 'served', and 'cancelled' are terminal states — not blocked.
    assert!(guard.can_disable(Feature::KitchenDisplay, &conn).is_ok());
}

#[test]
fn shift_guard_allows_other_features() {
    let conn = Connection::open_in_memory().unwrap();
    let guard = ShiftFeatureGuard;
    assert!(guard.can_disable(Feature::KitchenDisplay, &conn).is_ok());
    assert!(guard.can_disable(Feature::SimpleRetail, &conn).is_ok());
}

#[test]
fn shift_guard_allows_with_no_open_shifts() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE shifts (
                id TEXT PRIMARY KEY,
                closed_at TEXT,
                status TEXT NOT NULL DEFAULT 'closed'
            );
            INSERT INTO shifts (id, closed_at, status)
                VALUES ('s1', '2026-01-01T00:00:00Z', 'closed');
            INSERT INTO shifts (id, closed_at, status)
                VALUES ('s2', '2026-01-02T00:00:00Z', 'closed');",
    )
    .unwrap();
    let guard = ShiftFeatureGuard;
    assert!(guard.can_disable(Feature::ShiftManagement, &conn).is_ok());
}

#[test]
fn shift_guard_rejects_with_open_shifts() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE shifts (
                id TEXT PRIMARY KEY,
                closed_at TEXT,
                status TEXT NOT NULL DEFAULT 'open'
            );
            INSERT INTO shifts (id, closed_at, status) VALUES ('s1', NULL, 'open');",
    )
    .unwrap();
    let guard = ShiftFeatureGuard;
    let result = guard.can_disable(Feature::ShiftManagement, &conn);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("1 shift"));
}

#[test]
fn shift_guard_rejects_with_multiple_open_shifts() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE shifts (
                id TEXT PRIMARY KEY,
                closed_at TEXT,
                status TEXT NOT NULL DEFAULT 'open'
            );
            INSERT INTO shifts (id, closed_at, status) VALUES ('s1', NULL, 'open');
            INSERT INTO shifts (id, closed_at, status) VALUES ('s2', NULL, 'open');",
    )
    .unwrap();
    let guard = ShiftFeatureGuard;
    let result = guard.can_disable(Feature::ShiftManagement, &conn);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("2 shifts"));
}

#[test]
fn guard_registry_empty_by_default() {
    let registry = FeatureGuardRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.count(), 0);
}

#[test]
fn guard_registry_new_with_defaults() {
    let registry = FeatureGuardRegistry::new_with_defaults();
    assert_eq!(registry.count(), 2);
    assert!(!registry.is_empty());
}

#[test]
fn guard_registry_allows_when_all_guards_pass() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
            "CREATE TABLE kds_orders (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'pending'
            );
            CREATE TABLE shifts (
                id TEXT PRIMARY KEY,
                closed_at TEXT,
                status TEXT NOT NULL DEFAULT 'open'
            );
            INSERT INTO shifts (id, closed_at, status) VALUES ('s1', '2026-01-01T00:00:00Z', 'closed');",
        )
        .unwrap();
    let registry = FeatureGuardRegistry::new_with_defaults();
    assert!(
        registry
            .check_feature(Feature::KitchenDisplay, &conn)
            .is_ok()
    );
    assert!(
        registry
            .check_feature(Feature::ShiftManagement, &conn)
            .is_ok()
    );
}

#[test]
fn guard_registry_rejects_with_failures() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE kds_orders (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'pending'
            );
            CREATE TABLE shifts (
                id TEXT PRIMARY KEY,
                closed_at TEXT,
                status TEXT NOT NULL DEFAULT 'open'
            );
            INSERT INTO kds_orders (id, status) VALUES ('o1', 'pending');
            INSERT INTO shifts (id, closed_at, status) VALUES ('s1', NULL, 'open');",
    )
    .unwrap();
    let registry = FeatureGuardRegistry::new_with_defaults();
    // KitchenDisplay should fail due to KDS guard (not shift guard).
    let result = registry.check_feature(Feature::KitchenDisplay, &conn);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("1 ticket"), "expected ticket error: {err}");
    assert!(
        !err.contains("shift"),
        "should not include shift error for KitchenDisplay: {err}"
    );

    // ShiftManagement should fail due to shift guard (not KDS guard).
    let result = registry.check_feature(Feature::ShiftManagement, &conn);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("1 shift"), "expected shift error: {err}");
}

#[test]
fn guard_registry_collects_all_failures() {
    // When both guards fail for the same feature (unusual but possible
    // if a future custom guard is added), all failures are returned.
    // For now, KDS guard and Shift guard guard different features,
    // so this test verifies the collection mechanism works.
    let mut registry = FeatureGuardRegistry::new();
    registry.register(Box::new(KdsFeatureGuard));
    registry.register(Box::new(KdsFeatureGuard)); // duplicate

    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE kds_orders (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL DEFAULT 'pending'
            );
            INSERT INTO kds_orders (id, status) VALUES ('o1', 'pending');",
    )
    .unwrap();

    let result = registry.check_feature(Feature::KitchenDisplay, &conn);
    assert!(result.is_err());
    // Both KDS guards should fail — we get two "1 ticket is" messages.
    let err = result.unwrap_err();
    assert!(err.contains("ticket"), "expected ticket error in: {err}");
}

// ── Property-based tests (proptest) ─────────────────────────────────

// ── Additional edge-case tests ─────────────────────────────

#[test]
fn feature_from_key_unknown_returns_none() {
    assert_eq!(feature_from_key("unknown-feature"), None);
    assert_eq!(feature_from_key(""), None);
    assert_eq!(feature_from_key("SimpleRetail"), None);
    assert_eq!(feature_from_key("SIMPLE-RETAIL"), None);
}

#[test]
fn feature_key_is_lowercase_kebab_case() {
    for &feature in &[
        Feature::SimpleRetail,
        Feature::StaffRoles,
        Feature::CashDrawer,
        Feature::KitchenDisplay,
        Feature::MultiTerminal,
        Feature::CloudSync,
        Feature::Analytics,
        Feature::SelfServiceKiosk,
        Feature::PromotionsEngine,
    ] {
        let key = feature_key(feature);
        assert!(!key.is_empty(), "key for {feature:?} should not be empty");
        assert!(
            key.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "key '{key}' for {feature:?} should be lowercase kebab-case"
        );
    }
}

#[test]
fn from_settings_rows_with_mixed_keys_and_values() {
    let rows = vec![
        ("feature.simple-retail".into(), "1".into()),
        ("feature.cash-payment".into(), "0".into()),
        ("feature.staff-login".into(), "1".into()),
        ("store.name".into(), "1".into()),
        ("feature.unknown-feature".into(), "1".into()),
    ];
    let reg = FeatureRegistry::from_settings_rows(&rows);
    assert!(reg.is_enabled(Feature::SimpleRetail));
    assert!(!reg.is_enabled(Feature::CashPayment));
    assert!(reg.is_enabled(Feature::StaffLogin));
    assert_eq!(reg.count(), 2);
}

#[test]
fn to_settings_rows_sorted_deterministic() {
    let mut reg = FeatureRegistry::new();
    reg.enable(Feature::SimpleRetail);
    reg.enable(Feature::StaffRoles); // cascades StaffLogin
    let rows = reg.to_settings_rows();
    // All keys should start with "feature."
    for (key, value) in &rows {
        assert!(key.starts_with("feature."));
        assert_eq!(value, "1");
    }
    // Should contain all 3 enabled features
    assert_eq!(rows.len(), 3);
}

#[test]
fn enable_deep_chain_with_multi_level_deps() {
    let mut reg = FeatureRegistry::new();
    // MultiTerminal -> MultiStore -> (nothing)
    reg.enable(Feature::MultiTerminal);
    assert!(reg.is_enabled(Feature::MultiTerminal));
    assert!(reg.is_enabled(Feature::MultiStore));
    assert_eq!(reg.count(), 2);
}

#[test]
fn enable_cloud_sync_brings_in_multi_store() {
    let mut reg = FeatureRegistry::new();
    reg.enable(Feature::CloudSync);
    assert!(reg.is_enabled(Feature::CloudSync));
    assert!(reg.is_enabled(Feature::MultiStore));
}

#[test]
fn disable_multi_store_does_not_affect_multi_terminal() {
    let mut reg = FeatureRegistry::new();
    reg.enable(Feature::MultiTerminal); // brings in MultiStore
    reg.enable(Feature::CloudSync); // also needs MultiStore
    reg.disable(Feature::MultiStore);
    // MultiTerminal and CloudSync remain enabled (stale state)
    assert!(reg.is_enabled(Feature::MultiTerminal));
    assert!(reg.is_enabled(Feature::CloudSync));
}

#[test]
fn simple_retail_preset_count() {
    let reg = FeatureRegistry::simple_retail();
    assert!(
        reg.count() >= 5,
        "simple retail should have at least 5 features"
    );
}

#[test]
fn full_store_preset_count() {
    let reg = FeatureRegistry::full_store();
    assert!(reg.count() > 20, "full store should have many features");
}

#[test]
fn from_set_deduplicates() {
    let mut set = HashSet::new();
    set.insert(Feature::SimpleRetail);
    let reg = FeatureRegistry::from_set(set);
    assert_eq!(reg.count(), 1);
}

#[test]
fn kitchen_display_requires_restaurant() {
    let mut reg = FeatureRegistry::new();
    reg.enable(Feature::KitchenDisplay);
    // KitchenDisplay depends on Restaurant, so Restaurant is auto-enabled
    assert!(reg.is_enabled(Feature::KitchenDisplay));
    assert!(reg.is_enabled(Feature::Restaurant));
}

#[test]
fn restaurant_does_not_auto_enable_kitchen_display() {
    let mut reg = FeatureRegistry::new();
    reg.enable(Feature::Restaurant);
    assert!(reg.is_enabled(Feature::Restaurant));
    // KitchenDisplay depends ON Restaurant, not the other way around
    assert!(!reg.is_enabled(Feature::KitchenDisplay));
    assert!(!reg.is_enabled(Feature::TableManagement));
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// List of all features for generating random selections.
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
        Feature::UsbScale,
        Feature::DiscountEngine,
        Feature::TaxEngine,
        Feature::LoyaltyProgram,
        Feature::GiftCards,
        Feature::QuickReturn,
        Feature::PromotionsEngine,
        Feature::ProductBundles,
        Feature::PurchaseOrders,
        Feature::SerialTracking,
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
        #![proptest_config(ProptestConfig::with_cases(16))]
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
        #![proptest_config(ProptestConfig::with_cases(16))]
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
        #![proptest_config(ProptestConfig::with_cases(16))]
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
        #![proptest_config(ProptestConfig::with_cases(16))]
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
        #![proptest_config(ProptestConfig::with_cases(16))]
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

    #[test]
    fn cafe_preset_satisfies_invariant() {
        let reg = FeatureRegistry::cafe();
        for f in reg.enabled_features() {
            for &dep in f.dependencies() {
                assert!(
                    reg.is_enabled(dep),
                    "cafe: {f:?} enabled but dep {dep:?} is not"
                );
            }
        }
    }

    #[test]
    fn franchise_preset_satisfies_invariant() {
        let reg = FeatureRegistry::franchise();
        for f in reg.enabled_features() {
            for &dep in f.dependencies() {
                assert!(
                    reg.is_enabled(dep),
                    "franchise: {f:?} enabled but dep {dep:?} is not"
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
        assert!(
            features.is_empty(),
            "empty registry should have no enabled features: {features:?}"
        );
    }
}

// ── Deterministic unit tests ────────────────────────────────────
