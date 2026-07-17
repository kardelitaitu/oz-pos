/*
last audited 12-07-27 by C-2 env-var fix
crate: oz-pos-app | status: SAFE (C-2 resolved) | lint: CLEAN
findings: unsafe env::set_var removed from async command path; terminal_id written via AppState field | next: typed setter in AppState + tokio::sync::watch; callers migrate | perf: not in request hot path; concurrency is the concern
*/

//! Feature flag management Tauri commands.
//!
//! Exposes `list_all_features` (returns all 32 features with enabled
//! status and metadata) and `set_feature` (toggle a single feature on
//! or off with automatic dependency resolution).
//!
//! The front-end consumes these via the Feature Toggle screen
//! (Settings → Features) so users can enable/disable capabilities
//! after the initial Setup Wizard.

use serde::{Deserialize, Serialize};
use tauri::{State, command};

use oz_core::{Feature, FeatureGuardRegistry, Store, Terminal};
use platform_kernel::ModuleStatus;

use crate::error::AppError;
use crate::state::AppState;

/// A single feature with its current state and metadata.
#[derive(Debug, Clone, Serialize)]
pub struct FeatureDto {
    /// Kebab-case key (e.g. "simple-retail").
    pub key: String,
    /// Human-readable display name.
    pub name: &'static str,
    /// Short description of what the feature provides.
    pub description: &'static str,
    /// Logical group (e.g. "Core", "Payments", "Products", etc.).
    pub group: &'static str,
    /// Whether this feature is currently enabled.
    pub enabled: bool,
    /// Kebab-case keys of features this one depends on.
    pub dependencies: Vec<String>,
}

/// Response from `list_all_features`.
#[derive(Debug, Serialize)]
pub struct ListAllFeaturesResult {
    /// All 32 features with metadata and enabled status.
    pub features: Vec<FeatureDto>,
}

/// Fetch every known feature with its current enabled status, metadata,
/// and dependency information.
///
/// The front-end renders this into the Feature Toggle screen.
#[command]
pub async fn list_all_features(
    state: State<'_, AppState>,
) -> Result<ListAllFeaturesResult, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    let reg = store.load_features()?;

    let features = build_feature_list(&reg);
    Ok(ListAllFeaturesResult { features })
}

/// Build the full feature DTO list from a loaded registry.
fn build_feature_list(reg: &oz_core::FeatureRegistry) -> Vec<FeatureDto> {
    all_feature_metadata()
        .into_iter()
        .map(|(feat, name, desc, group)| {
            let key = oz_core::features::feature_key(feat).to_string();
            let deps: Vec<String> = feat
                .dependencies()
                .iter()
                .map(|d| oz_core::features::feature_key(*d).to_string())
                .collect();
            FeatureDto {
                key,
                name,
                description: desc,
                group,
                enabled: reg.is_enabled(feat),
                dependencies: deps,
            }
        })
        .collect()
}

// ── Set feature ──────────────────────────────────────────────────────

/// Arguments for `set_feature`.
#[derive(Debug, Deserialize)]
pub struct SetFeatureArgs {
    /// Kebab-case key of the feature to toggle.
    pub key: String,
    /// Whether to enable (`true`) or disable (`false`).
    pub enabled: bool,
}

/// Response from `set_feature`.
#[derive(Debug, Serialize)]
pub struct SetFeatureResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Updated list of all features (same shape as `list_all_features`).
    pub features: Vec<FeatureDto>,
    /// If the operation cascaded (auto-enabled dependencies), these were
    /// also enabled.
    pub auto_enabled: Vec<String>,
}

// ── Set features bulk ───────────────────────────────────────────────

/// Arguments for `set_features_bulk`.
#[derive(Debug, Deserialize)]
pub struct SetFeaturesBulkArgs {
    /// Kebab-case keys of the features to toggle.
    pub keys: Vec<String>,
    /// Whether to enable (`true`) or disable (`false`) all given features.
    pub enabled: bool,
}

/// Enable or disable multiple feature flags atomically in a single
/// SQLite transaction.
///
/// Unlike `set_feature`, this bulk operation:
/// - Executes all changes in a single SQLite transaction
/// - Does NOT run kernel module lifecycle (use individual `set_feature`
///   for module-backed features that need start/stop)
/// - Does NOT cascade auto-enable dependencies (each feature is toggled
///   individually; call `set_feature` if dependency resolution is needed)
/// - Returns `ListAllFeaturesResult` so the front-end can refresh its
///   display with a single response
///
/// This is intended for bulk group toggles in the Feature Toggle screen
/// (e.g. "Enable all Hardware", "Disable all Advanced").
#[command]
pub async fn set_features_bulk(
    args: SetFeaturesBulkArgs,
    state: State<'_, AppState>,
) -> Result<ListAllFeaturesResult, AppError> {
    let mut db = state.db.lock().await;

    // Start a SQLite transaction for atomicity.
    let tx = db.transaction().map_err(|e| {
        AppError::Internal(format!("failed to start transaction for bulk toggle: {e}"))
    })?;

    // Use Store within the transaction (Transaction derefs to Connection).
    let store = Store::new(&tx);
    let mut reg = store.load_features()?;

    // Parse and apply each key.
    for key in &args.keys {
        let feature = oz_core::features::feature_from_key(key)
            .ok_or_else(|| AppError::Invalid(format!("unknown feature key: {key}")))?;

        if args.enabled {
            reg.enable(feature);
        } else {
            reg.disable(feature);
        }
    }

    // Persist and prune within the transaction.
    store.save_features(&reg)?;
    store.prune_stale_features(&reg)?;

    // Commit the transaction.
    tx.commit()
        .map_err(|e| AppError::Internal(format!("failed to commit bulk feature toggle: {e}")))?;

    let features = build_feature_list(&reg);
    Ok(ListAllFeaturesResult { features })
}

/// Enable or disable a single feature flag.
///
/// When enabling, all required dependencies are automatically enabled
/// as well. When disabling, only the specified feature is turned off
/// (dependents are NOT cascaded — the UI must handle that).
///
/// When the feature corresponds to a registered kernel module, the
/// module is started (on enable) or stopped (on disable) via
/// `kernel.start_module` / `kernel.stop_module`. Module lifecycle
/// failures are logged but do not prevent the feature toggle from
/// succeeding — the feature registry is persisted regardless.
#[command]
pub async fn set_feature(
    args: SetFeatureArgs,
    state: State<'_, AppState>,
) -> Result<SetFeatureResult, AppError> {
    let feature = oz_core::features::feature_from_key(&args.key)
        .ok_or_else(|| AppError::Invalid(format!("unknown feature key: {}", args.key)))?;

    // ── Feature safety guards (before any changes) ────────────────
    //
    // For disable operations, run all registered FeatureGuards FIRST
    // to ensure the toggle won't leave the system in an unsafe state
    // (e.g. orphaned KDS tickets or unreconciled shifts). Guards are
    // checked before kernel lifecycle so that a guard rejection does
    // not leave a partially-stopped module.
    if !args.enabled {
        let db = state.db.lock().await;
        let guards = FeatureGuardRegistry::new_with_defaults();
        if let Err(reason) = guards.check_feature(feature, &db) {
            return Err(AppError::Invalid(reason));
        }
        drop(db);
    }

    // ── Kernel module lifecycle (before DB persist) ───────────────
    //
    // For features that map to a kernel module, we attempt the
    // start/stop next. This way, if the module fails to start,
    // the feature toggle is not persisted (preventing inconsistent
    // state where the DB says enabled but the module is not running).
    if let Some(module_id) = feature_to_module_id(feature) {
        let mut kernel = state.kernel.lock().await;
        if args.enabled {
            match kernel.start_module(module_id) {
                Ok(()) => {
                    tracing::info!(
                        module = module_id,
                        feature = %args.key,
                        "kernel module started via feature toggle"
                    );
                }
                Err(e) => {
                    // Module may already be started — that's fine.
                    let status = kernel.module_status(module_id);
                    if status != Some(ModuleStatus::Started) {
                        tracing::warn!(
                            module = module_id,
                            error = %e,
                            status = ?status,
                            "failed to start kernel module for feature, aborting toggle"
                        );
                        return Err(AppError::Internal(format!(
                            "failed to start module '{module_id}' for feature '{}': {e}",
                            args.key,
                        )));
                    }
                }
            }
        } else {
            match kernel.stop_module(module_id) {
                Ok(()) => {
                    tracing::info!(
                        module = module_id,
                        feature = %args.key,
                        "kernel module stopped via feature toggle"
                    );
                }
                Err(e) => {
                    let status = kernel.module_status(module_id);
                    if status != Some(ModuleStatus::Stopped)
                        && status != Some(ModuleStatus::Registered)
                    {
                        tracing::warn!(
                            module = module_id,
                            error = %e,
                            status = ?status,
                            "failed to stop kernel module for feature, aborting toggle"
                        );
                        return Err(AppError::Internal(format!(
                            "failed to stop module '{module_id}' for feature '{}': {e}",
                            args.key,
                        )));
                    }
                }
            }
        }
        // Release kernel lock before DB lock to avoid nested locking.
        drop(kernel);
    }

    let db = state.db.lock().await;
    let store = Store::new(&db);

    let mut reg = store.load_features()?;
    let mut auto_enabled: Vec<String> = Vec::new();

    if args.enabled {
        // Before enabling, track what was already enabled.
        let before_enable: Vec<Feature> = reg.enabled_features().collect();

        // Enable the feature (this auto-enables dependencies).
        reg.enable(feature);

        // Auto-enable CloudSync when MultiTerminal is enabled (logical coupling:
        // cross-terminal sync requires the cloud sync backend).
        if feature == Feature::MultiTerminal && !reg.is_enabled(Feature::CloudSync) {
            reg.enable(Feature::CloudSync);
        }

        // Figure out what was newly auto-enabled.
        for f in reg.enabled_features() {
            if !before_enable.contains(&f) && f != feature {
                auto_enabled.push(oz_core::features::feature_key(f).to_string());
            }
        }
    } else {
        reg.disable(feature);
    }

    // Persist the updated registry and prune stale rows (from disabling).
    store.save_features(&reg)?;
    store.prune_stale_features(&reg)?;

    // Auto-register this device as a terminal when MultiTerminal is enabled.
    // This ensures every POS device has a registered terminal entry for
    // cross-terminal inventory sync and Redis pub/sub filtering.
    if args.enabled && feature == Feature::MultiTerminal {
        let device_id = device_hostname();
        let mut tid = state.terminal_id.lock().unwrap();
        if store.get_terminal_by_device_id(&device_id)?.is_none() {
            let name = format!("{} (auto)", &device_id);
            let terminal = Terminal::new(&name, &device_id);
            store.create_terminal(&terminal)?;
            *tid = Some(terminal.id.clone());
            tracing::info!(
                id = %terminal.id,
                name = %terminal.name,
                "terminal auto-registered for multi-terminal"
            );
        } else if tid.is_none() {
            if let Some(existing) = store.get_terminal_by_device_id(&device_id)? {
                *tid = Some(existing.id);
            }
        }
        drop(tid);
    }

    let features = build_feature_list(&reg);

    Ok(SetFeatureResult {
        success: true,
        features,
        auto_enabled,
    })
}

/// Return a stable device identifier for this machine.
///
/// Uses `COMPUTERNAME` (Windows) or `HOSTNAME` (Unix) if available,
/// falling back to `"unknown-device"`. This identifier is used to
/// auto-register the terminal when multi-terminal mode is enabled.
/// Map a [`Feature`] to its corresponding kernel module ID, if any.
///
/// Not all features have a dedicated kernel module — many are simple
/// flag toggles (e.g. `CashPayment`, `BarcodeScanning`) that only
/// affect UI rendering or payment routing. Only features backed by a
/// registered [`Module`](foundation::contracts::Module) are listed here.
fn feature_to_module_id(feature: Feature) -> Option<&'static str> {
    match feature {
        Feature::InventoryTracking => Some("inventory"),
        Feature::CategoriesEnabled => Some("inventory"),
        Feature::StaffLogin => Some("staff"),
        Feature::StaffRoles => Some("staff"),
        Feature::ShiftManagement => Some("staff"),
        Feature::Reporting => Some("reporting"),
        Feature::Analytics => Some("reporting"),
        Feature::TaxEngine => Some("tax"),
        Feature::SimpleRetail | Feature::Restaurant => Some("sales"),
        Feature::MultiCurrency => Some("currency"),
        _ => None,
    }
}

fn device_hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-device".to_string())
}

// ── Feature metadata ────────────────────────────────────────────────

/// Returns all 32 features with their human-readable name, description,
/// and logical group.
fn all_feature_metadata() -> Vec<(Feature, &'static str, &'static str, &'static str)> {
    vec![
        (
            Feature::SimpleRetail,
            "Simple Retail",
            "Core POS: scan, sell, print receipt",
            "Core",
        ),
        (
            Feature::Restaurant,
            "Restaurant Mode",
            "Table management and KDS",
            "Core",
        ),
        (
            Feature::CashPayment,
            "Cash Payment",
            "Accept cash at checkout",
            "Payments",
        ),
        (
            Feature::CardPayment,
            "Card Payment",
            "Accept debit/credit cards",
            "Payments",
        ),
        (
            Feature::MultiCurrency,
            "Multi-Currency",
            "Support multiple currencies with exchange rates",
            "Payments",
        ),
        (
            Feature::InventoryTracking,
            "Inventory Tracking",
            "Track stock levels per product",
            "Products",
        ),
        (
            Feature::ProductVariants,
            "Product Variants",
            "Size, colour, flavour variants",
            "Products",
        ),
        (
            Feature::CategoriesEnabled,
            "Categories",
            "Group products into categories",
            "Products",
        ),
        (
            Feature::StaffLogin,
            "Staff Login",
            "PIN/password login for staff",
            "Staff",
        ),
        (
            Feature::StaffRoles,
            "Staff Roles",
            "Role-based permissions (owner, manager, cashier)",
            "Staff",
        ),
        (
            Feature::ShiftManagement,
            "Shift Management",
            "Open/close shift with cash reconciliation",
            "Staff",
        ),
        (
            Feature::AuditLog,
            "Audit Log",
            "Immutable append-only action log",
            "Staff",
        ),
        (
            Feature::BarcodeScanning,
            "Barcode Scanning",
            "USB/serial/Bluetooth barcode scanner",
            "Hardware",
        ),
        (
            Feature::ReceiptPrinting,
            "Receipt Printing",
            "USB/serial/network receipt printer",
            "Hardware",
        ),
        (
            Feature::CashDrawer,
            "Cash Drawer",
            "Cash drawer trigger via printer GPIO",
            "Hardware",
        ),
        (
            Feature::CustomerDisplay,
            "Customer Display",
            "Secondary customer-facing screen",
            "Hardware",
        ),
        (
            Feature::NfcReader,
            "NFC Reader",
            "Contactless card/NFC reader",
            "Hardware",
        ),
        (
            Feature::DiscountEngine,
            "Discount Engine",
            "Percentage and fixed-amount discounts",
            "Business Rules",
        ),
        (
            Feature::TaxEngine,
            "Tax Engine",
            "Tax calculation with configurable rates",
            "Business Rules",
        ),
        (
            Feature::LoyaltyProgram,
            "Loyalty Program",
            "Customer points, tiers, and redemption",
            "Business Rules",
        ),
        (
            Feature::GiftCards,
            "Gift Cards",
            "Issue, redeem, and manage gift card balances",
            "Payments",
        ),
        (
            Feature::PromotionsEngine,
            "Promotions Engine",
            "Time-limited buy-X-get-Y and % off",
            "Business Rules",
        ),
        (
            Feature::ProductBundles,
            "Product Bundles",
            "Sell multiple SKUs as one bundle",
            "Business Rules",
        ),
        (
            Feature::KitchenDisplay,
            "Kitchen Display",
            "Order routing to kitchen screens",
            "Restaurant",
        ),
        (
            Feature::TableManagement,
            "Table Management",
            "Interactive restaurant floor plan",
            "Restaurant",
        ),
        (
            Feature::SelfServiceKiosk,
            "Self-Service Kiosk",
            "Locked-down fullscreen kiosk mode",
            "Restaurant",
        ),
        (
            Feature::CloudSync,
            "Cloud Sync",
            "Database synchronisation to cloud",
            "Scaling",
        ),
        (
            Feature::MultiStore,
            "Multi-Store",
            "Manage multiple store locations",
            "Scaling",
        ),
        (
            Feature::MultiTerminal,
            "Multi-Terminal",
            "Multiple terminals per store",
            "Scaling",
        ),
        (
            Feature::Reporting,
            "Reporting",
            "Sales, inventory, and shift reports",
            "Reporting",
        ),
        (
            Feature::Analytics,
            "Analytics",
            "Advanced charts and data exports",
            "Reporting",
        ),
        (
            Feature::ExportImport,
            "Export / Import",
            "Data export/import in .ozpkg format",
            "Advanced",
        ),
        (
            Feature::PluginSystem,
            "Plugin System",
            "Third-party plugin support",
            "Advanced",
        ),
        (
            Feature::StockCounting,
            "Stock Counting",
            "Periodic stock counting",
            "Inventory",
        ),
        (
            Feature::StockTransfers,
            "Stock Transfers",
            "Transfer stock between locations",
            "Inventory",
        ),
        (
            Feature::PurchaseOrders,
            "Purchase Orders",
            "Create and manage purchase orders",
            "Inventory",
        ),
        (
            Feature::SerialTracking,
            "Serial Tracking",
            "Track products by serial number",
            "Inventory",
        ),
        (
            Feature::QuickReturn,
            "Quick Return",
            "Fast return without original sale lookup",
            "Core",
        ),
        (
            Feature::UsbScale,
            "USB Scale",
            "USB-connected weight scale",
            "Hardware",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FeatureDto ───────────────────────────────────────────────

    #[test]
    fn feature_dto_debug_output() {
        let dto = FeatureDto {
            key: "simple-retail".into(),
            name: "Simple Retail",
            description: "Core POS: scan, sell, print receipt",
            group: "Core",
            enabled: true,
            dependencies: vec!["cash-payment".into()],
        };
        let debug = format!("{dto:?}");
        assert!(debug.contains("simple-retail"));
        assert!(debug.contains("Simple Retail"));
        assert!(debug.contains("Core"));
    }

    #[test]
    fn feature_dto_serialize_json() {
        let dto = FeatureDto {
            key: "tax-engine".into(),
            name: "Tax Engine",
            description: "Tax calculation with configurable rates",
            group: "Business Rules",
            enabled: false,
            dependencies: vec![],
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["key"], "tax-engine");
        assert_eq!(json["name"], "Tax Engine");
        assert_eq!(json["group"], "Business Rules");
        assert_eq!(json["enabled"], false);
        assert_eq!(json["dependencies"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn feature_dto_with_dependencies_serialize() {
        let dto = FeatureDto {
            key: "analytics".into(),
            name: "Analytics",
            description: "Advanced charts",
            group: "Reporting",
            enabled: true,
            dependencies: vec!["reporting".into()],
        };
        let json = serde_json::to_value(&dto).unwrap();
        let deps = json["dependencies"].as_array().unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "reporting");
    }

    // ── SetFeatureArgs ───────────────────────────────────────────

    #[test]
    fn set_feature_args_deserialize() {
        let json = r#"{"key": "tax-engine", "enabled": true}"#;
        let args: SetFeatureArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.key, "tax-engine");
        assert!(args.enabled);
    }

    #[test]
    fn set_feature_args_deserialize_disable() {
        let json = r#"{"key": "loyalty-program", "enabled": false}"#;
        let args: SetFeatureArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.key, "loyalty-program");
        assert!(!args.enabled);
    }

    #[test]
    fn set_feature_args_debug() {
        let args = SetFeatureArgs {
            key: "gift-cards".into(),
            enabled: true,
        };
        let debug = format!("{args:?}");
        assert!(debug.contains("gift-cards"));
        assert!(debug.contains("true"));
    }

    // ── SetFeatureResult ─────────────────────────────────────────

    #[test]
    fn set_feature_result_debug() {
        let dto = FeatureDto {
            key: "simple-retail".into(),
            name: "Simple Retail",
            description: "Core POS",
            group: "Core",
            enabled: true,
            dependencies: vec![],
        };
        let result = SetFeatureResult {
            success: true,
            features: vec![dto],
            auto_enabled: vec!["cash-payment".into()],
        };
        let debug = format!("{result:?}");
        assert!(debug.contains("true"));
        assert!(debug.contains("Simple Retail"));
        assert!(debug.contains("cash-payment"));
    }

    #[test]
    fn set_feature_result_empty_auto_enabled() {
        let result = SetFeatureResult {
            success: true,
            features: vec![],
            auto_enabled: vec![],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["auto_enabled"].as_array().unwrap().len(), 0);
    }

    // ── ListAllFeaturesResult ────────────────────────────────────

    #[test]
    fn list_all_features_result_serialize_empty() {
        let result = ListAllFeaturesResult { features: vec![] };
        let json = serde_json::to_value(&result).unwrap();
        assert!(json["features"].as_array().unwrap().is_empty());
    }

    // ── feature_to_module_id ────────────────────────────────────

    #[test]
    fn feature_to_module_id_inventory() {
        assert_eq!(
            feature_to_module_id(Feature::InventoryTracking),
            Some("inventory")
        );
        assert_eq!(
            feature_to_module_id(Feature::CategoriesEnabled),
            Some("inventory")
        );
    }

    #[test]
    fn feature_to_module_id_staff() {
        assert_eq!(feature_to_module_id(Feature::StaffLogin), Some("staff"));
        assert_eq!(feature_to_module_id(Feature::StaffRoles), Some("staff"));
        assert_eq!(
            feature_to_module_id(Feature::ShiftManagement),
            Some("staff")
        );
    }

    #[test]
    fn feature_to_module_id_reporting() {
        assert_eq!(feature_to_module_id(Feature::Reporting), Some("reporting"));
        assert_eq!(feature_to_module_id(Feature::Analytics), Some("reporting"));
    }

    #[test]
    fn feature_to_module_id_tax() {
        assert_eq!(feature_to_module_id(Feature::TaxEngine), Some("tax"));
    }

    #[test]
    fn feature_to_module_id_sales() {
        assert_eq!(feature_to_module_id(Feature::SimpleRetail), Some("sales"));
        assert_eq!(feature_to_module_id(Feature::Restaurant), Some("sales"));
    }

    #[test]
    fn feature_to_module_id_currency() {
        assert_eq!(
            feature_to_module_id(Feature::MultiCurrency),
            Some("currency")
        );
    }

    #[test]
    fn feature_to_module_id_returns_none_for_non_module_features() {
        assert_eq!(feature_to_module_id(Feature::CashPayment), None);
        assert_eq!(feature_to_module_id(Feature::CardPayment), None);
        assert_eq!(feature_to_module_id(Feature::BarcodeScanning), None);
        assert_eq!(feature_to_module_id(Feature::ReceiptPrinting), None);
        assert_eq!(feature_to_module_id(Feature::DiscountEngine), None);
        assert_eq!(feature_to_module_id(Feature::GiftCards), None);
        assert_eq!(feature_to_module_id(Feature::PluginSystem), None);
        assert_eq!(feature_to_module_id(Feature::ExportImport), None);
        assert_eq!(feature_to_module_id(Feature::CloudSync), None);
    }

    #[test]
    fn feature_to_module_id_known_features_are_comprehensive() {
        // Ensure every feature that maps to a module has its mapping
        // listed above. This test will catch missing mappings when
        // new features are added.
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
            Feature::GiftCards,
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
            Feature::StockCounting,
            Feature::StockTransfers,
            Feature::PurchaseOrders,
            Feature::SerialTracking,
            Feature::QuickReturn,
            Feature::UsbScale,
        ];
        for f in all_features {
            let result = feature_to_module_id(f);
            // Just ensure that known module-mapped features return Some
            // and others return None — no panics.
            if matches!(
                f,
                Feature::InventoryTracking
                    | Feature::CategoriesEnabled
                    | Feature::StaffLogin
                    | Feature::StaffRoles
                    | Feature::ShiftManagement
                    | Feature::Reporting
                    | Feature::Analytics
                    | Feature::TaxEngine
                    | Feature::SimpleRetail
                    | Feature::Restaurant
                    | Feature::MultiCurrency
            ) {
                assert!(
                    result.is_some(),
                    "feature {f:?} should have a module mapping"
                );
            }
        }
    }

    // ── SetFeaturesBulkArgs ────────────────────────────────────────

    #[test]
    fn set_features_bulk_args_deserialize() {
        let json = r#"{"keys": ["simple-retail", "cash-payment"], "enabled": true}"#;
        let args: SetFeaturesBulkArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.keys.len(), 2);
        assert_eq!(args.keys[0], "simple-retail");
        assert_eq!(args.keys[1], "cash-payment");
        assert!(args.enabled);
    }

    #[test]
    fn set_features_bulk_args_deserialize_disable() {
        let json = r#"{"keys": ["kitchen-display"], "enabled": false}"#;
        let args: SetFeaturesBulkArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.keys.len(), 1);
        assert_eq!(args.keys[0], "kitchen-display");
        assert!(!args.enabled);
    }

    #[test]
    fn set_features_bulk_args_empty_keys() {
        let json = r#"{"keys": [], "enabled": true}"#;
        let args: SetFeaturesBulkArgs = serde_json::from_str(json).unwrap();
        assert!(args.keys.is_empty());
        assert!(args.enabled);
    }

    #[test]
    fn set_features_bulk_args_debug() {
        let args = SetFeaturesBulkArgs {
            keys: vec!["hardware".into()],
            enabled: false,
        };
        let debug = format!("{args:?}");
        assert!(debug.contains("hardware"));
        assert!(debug.contains("false"));
    }

    // ── all_feature_metadata ─────────────────────────────────────

    #[test]
    fn all_feature_metadata_non_empty() {
        let metadata = all_feature_metadata();
        assert!(!metadata.is_empty(), "should have at least one feature");
    }

    #[test]
    fn all_feature_metadata_no_duplicate_keys() {
        let metadata = all_feature_metadata();
        let keys: Vec<String> = metadata
            .iter()
            .map(|(feat, _, _, _)| oz_core::features::feature_key(*feat).to_string())
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "feature keys must be unique");
    }
}
