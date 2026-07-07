//! Setup Wizard commands.
//!
//! `complete_setup` persists the chosen preset and enabled features to
//! the settings table and marks the wizard as complete.
//! `get_setup_status` lets the front-end decide whether to show the
//! wizard or go straight to the main app.

use oz_core::{FeatureRegistry, Settings, Store, features};
use serde::{Deserialize, Serialize};
use tauri::{State, command};

use crate::commands::authz::require_permission_for_user;
use crate::error::AppError;
use crate::state::AppState;

// ── Args ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CompleteSetupArgs {
    /// Store preset name (e.g. `"simple-retail"`, `"restaurant"`).
    pub preset: String,
    /// Enabled feature keys (kebab-case, e.g. `"cash-payment"`).
    pub features: Vec<String>,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SetupStatus {
    /// Whether the setup wizard has been completed.
    pub completed: bool,
    /// The store preset name, if set.
    pub preset: Option<String>,
}

// ── Response types ───────────────────────────────────────────────────

/// The enabled feature keys returned by `get_enabled_features`.
#[derive(Debug, Serialize)]
pub struct EnabledFeaturesResult {
    /// Kebab-case feature keys (e.g. `"cash-payment"`, `"barcode-scanning"`).
    pub features: Vec<String>,
}

// ── Commands ─────────────────────────────────────────────────────────

/// Return the list of currently-enabled feature keys.
///
/// The front-end calls this once on mount to decide which nav items
/// and UI elements to show/hide.
#[command]
pub async fn get_enabled_features(
    state: State<'_, AppState>,
) -> Result<EnabledFeaturesResult, AppError> {
    let conn = state.db.lock().await;
    let registry = Settings::load_features(&conn)?;

    let features: Vec<String> = registry
        .enabled_features()
        .map(|f| oz_core::features::feature_key(f).to_string())
        .collect();

    Ok(EnabledFeaturesResult { features })
}

/// Persist the chosen preset and features, then mark setup as complete.
///
/// Called by the front-end when the user clicks "Complete Setup" on
/// the last step of the wizard.
#[command]
pub async fn complete_setup(
    state: State<'_, AppState>,
    args: CompleteSetupArgs,
) -> Result<(), AppError> {
    let db = state.db.lock().await;

    // Convert feature key strings → Feature enum variants.
    let mut registry = FeatureRegistry::new();
    for key in &args.features {
        if let Some(feat) = features::feature_from_key(key) {
            registry.enable(feat);
        } else {
            tracing::warn!(feature = %key, "unknown feature key in setup, skipping");
        }
    }

    // Save features + preset + completed flag in a single transaction.
    let tx = db.unchecked_transaction()?;
    {
        let store = Store::new(&tx);

        // 1. Seed built-in roles (idempotent — skips existing).
        store.seed_default_roles()?;

        // 2. Persist features.
        store.save_features(&registry)?;

        // 3. Prune stale feature rows that are no longer enabled.
        Settings::prune_stale_features(&tx, &registry)?;

        // 4. Save the preset name.
        Settings::set(&tx, oz_core::settings::keys::STORE_PRESET, &args.preset)?;

        // 5. Mark setup as complete.
        Settings::set(&tx, oz_core::settings::keys::SETUP_COMPLETE, "1")?;

        // 6. Dismiss the wizard so it doesn't show on next launch.
        Settings::set(&tx, oz_core::settings::keys::SHOW_SETUP_WIZARD, "false")?;
    }
    tx.commit()?;

    tracing::info!(
        preset = %args.preset,
        feature_count = %args.features.len(),
        "setup wizard completed"
    );

    Ok(())
}

/// Returns whether the setup wizard has been completed.
///
/// The front-end calls this on mount to decide whether to render
/// the wizard or the main application.
#[command]
pub async fn get_setup_status(state: State<'_, AppState>) -> Result<SetupStatus, AppError> {
    let db = state.db.lock().await;

    let completed = Settings::get(&db, oz_core::settings::keys::SHOW_SETUP_WIZARD)?
        .map(|v| v == "false")
        .unwrap_or(false);

    let preset = Settings::get(&db, oz_core::settings::keys::STORE_PRESET)?;

    Ok(SetupStatus { completed, preset })
}

/// Seed the three built-in roles (Owner, Manager, Cashier) with their
/// preset permission sets. Idempotent — existing roles are left unchanged.
///
/// Requires the `staff:manage_roles` permission.
#[command]
pub async fn seed_default_roles(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<usize, AppError> {
    let db = state.db.lock().await;
    let store = Store::new(&db);
    require_permission_for_user(&store, &user_id, oz_core::permissions::STAFF_MANAGE_ROLES)?;
    let count = store.seed_default_roles()?;
    drop(db);
    tracing::info!(count, "default roles seeded");
    Ok(count)
}

/// Dismiss the setup wizard without enabling any features.
///
/// Called when the user clicks "Skip setup". Only writes the
/// `show_setup_wizard = false` flag — no preset or features are saved.
#[command]
pub async fn dismiss_setup_wizard(state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    Settings::set(&db, oz_core::settings::keys::SHOW_SETUP_WIZARD, "false")?;
    tracing::info!("setup wizard dismissed (skip)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    /// Create a fresh in-memory connection with migrations applied.
    fn fresh_conn() -> Connection {
        migrations::fresh_db()
    }

    /// Run the same logic as `complete_setup` but with a plain
    /// `&Connection` so tests don't need a Tauri runtime.
    ///
    /// Each individual operation (`save_features`, `prune_stale_features`,
    /// `set`) handles its own transaction internally. The production
    /// `complete_setup` command wraps them in a single outer transaction
    /// for atomicity; tests verify the operations individually.
    fn run_complete_setup(
        conn: &Connection,
        preset: &str,
        features: &[&str],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = FeatureRegistry::new();
        for &key in features {
            if let Some(feat) = features::feature_from_key(key) {
                registry.enable(feat);
            }
        }

        let store = Store::new(conn);
        store.save_features(&registry)?;
        Settings::prune_stale_features(conn, &registry)?;
        Settings::set(conn, oz_core::settings::keys::STORE_PRESET, preset)?;
        Settings::set(conn, oz_core::settings::keys::SETUP_COMPLETE, "1")?;
        Settings::set(conn, oz_core::settings::keys::SHOW_SETUP_WIZARD, "false")?;
        Ok(())
    }

    #[test]
    fn complete_setup_persists_features() {
        let conn = fresh_conn();

        run_complete_setup(
            &conn,
            "simple-retail",
            &[
                "cash-payment",
                "barcode-scanning",
                "receipt-printing",
                "inventory-tracking",
                "categories-enabled",
                "tax-engine",
            ],
        )
        .unwrap();

        // Verify setup is marked complete.
        let completed = Settings::get(&conn, oz_core::settings::keys::SETUP_COMPLETE)
            .unwrap()
            .unwrap();
        assert_eq!(completed, "1");

        let preset = Settings::get(&conn, oz_core::settings::keys::STORE_PRESET)
            .unwrap()
            .unwrap();
        assert_eq!(preset, "simple-retail");
    }

    #[test]
    fn get_setup_status_defaults_to_not_completed() {
        let conn = fresh_conn();

        let completed = Settings::get(&conn, oz_core::settings::keys::SETUP_COMPLETE).unwrap();
        assert_eq!(completed, None);

        let preset = Settings::get(&conn, oz_core::settings::keys::STORE_PRESET).unwrap();
        assert_eq!(preset, None);
    }

    #[test]
    fn complete_setup_skips_unknown_features() {
        let conn = fresh_conn();

        run_complete_setup(
            &conn,
            "custom",
            &["cash-payment", "made-up-feature"], // unknown, should be skipped
        )
        .unwrap();

        // Should still succeed.
        let completed = Settings::get(&conn, oz_core::settings::keys::SETUP_COMPLETE)
            .unwrap()
            .unwrap();
        assert_eq!(completed, "1");

        // Only cash-payment should be enabled.
        let store = Store::new(&conn);
        let loaded = store.load_features().unwrap();
        assert!(loaded.is_enabled(oz_core::Feature::CashPayment));
        assert!(!loaded.is_enabled(oz_core::Feature::BarcodeScanning));
    }

    #[test]
    fn complete_setup_empty_features() {
        let conn = fresh_conn();

        run_complete_setup(&conn, "empty-store", &[]).unwrap();

        let completed = Settings::get(&conn, oz_core::settings::keys::SETUP_COMPLETE)
            .unwrap()
            .unwrap();
        assert_eq!(completed, "1");

        let preset = Settings::get(&conn, oz_core::settings::keys::STORE_PRESET)
            .unwrap()
            .unwrap();
        assert_eq!(preset, "empty-store");

        // No features should be enabled.
        let store = Store::new(&conn);
        let loaded = store.load_features().unwrap();
        assert_eq!(loaded.count(), 0);
    }

    #[test]
    fn complete_setup_with_different_presets() {
        let conn = fresh_conn();

        // Test restaurant preset.
        run_complete_setup(
            &conn,
            "restaurant",
            &[
                "restaurant",
                "cash-payment",
                "receipt-printing",
                "inventory-tracking",
                "categories-enabled",
                "discount-engine",
                "tax-engine",
                "kitchen-display",
                "table-management",
                "staff-login",
            ],
        )
        .unwrap();

        let completed = Settings::get(&conn, oz_core::settings::keys::SETUP_COMPLETE)
            .unwrap()
            .unwrap();
        assert_eq!(completed, "1");

        let preset = Settings::get(&conn, oz_core::settings::keys::STORE_PRESET)
            .unwrap()
            .unwrap();
        assert_eq!(preset, "restaurant");

        // Verify restaurant-specific features.
        let store = Store::new(&conn);
        let loaded = store.load_features().unwrap();
        assert!(loaded.is_enabled(oz_core::Feature::Restaurant));
        assert!(loaded.is_enabled(oz_core::Feature::KitchenDisplay));
        assert!(loaded.is_enabled(oz_core::Feature::TableManagement));
        assert!(loaded.is_enabled(oz_core::Feature::StaffLogin));
        assert!(!loaded.is_enabled(oz_core::Feature::SimpleRetail));
        assert!(!loaded.is_enabled(oz_core::Feature::CardPayment));
    }

    #[test]
    fn complete_setup_all_features_single_preset() {
        let conn = fresh_conn();

        // Full-store preset: 24 feature keys.
        run_complete_setup(
            &conn,
            "full-store",
            &[
                "simple-retail",
                "cash-payment",
                "card-payment",
                "multi-currency",
                "inventory-tracking",
                "product-variants",
                "categories-enabled",
                "staff-login",
                "staff-roles",
                "shift-management",
                "audit-log",
                "barcode-scanning",
                "receipt-printing",
                "cash-drawer",
                "customer-display",
                "nfc-reader",
                "discount-engine",
                "tax-engine",
                "loyalty-program",
                "promotions-engine",
                "product-bundles",
                "reporting",
                "analytics",
                "export-import",
            ],
        )
        .unwrap();

        let store = Store::new(&conn);
        let loaded = store.load_features().unwrap();
        assert!(loaded.count() >= 20);
        assert!(loaded.is_enabled(oz_core::Feature::SimpleRetail));
        assert!(loaded.is_enabled(oz_core::Feature::Analytics));

        // Prune should be a no-op since all features match.
        let removed = Settings::prune_stale_features(&conn, &loaded).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn complete_setup_allows_multiple_calls() {
        let conn = fresh_conn();

        // First call with simple-retail.
        run_complete_setup(
            &conn,
            "simple-retail",
            &["cash-payment", "barcode-scanning", "receipt-printing"],
        )
        .unwrap();

        // Second call overwrites with restaurant (pruning handles cleanup).
        run_complete_setup(
            &conn,
            "restaurant",
            &[
                "restaurant",
                "cash-payment",
                "kitchen-display",
                "table-management",
                "staff-login",
            ],
        )
        .unwrap();

        // Preset was overwritten.
        let preset = Settings::get(&conn, oz_core::settings::keys::STORE_PRESET)
            .unwrap()
            .unwrap();
        assert_eq!(preset, "restaurant");

        // Features should be from restaurant, not simple-retail.
        let store = Store::new(&conn);
        let loaded = store.load_features().unwrap();
        assert!(loaded.is_enabled(oz_core::Feature::Restaurant));
        assert!(!loaded.is_enabled(oz_core::Feature::SimpleRetail));
    }

    #[test]
    fn complete_setup_persists_all_settings() {
        let conn = fresh_conn();

        run_complete_setup(
            &conn,
            "simple-retail",
            &["cash-payment", "receipt-printing"],
        )
        .unwrap();

        // Verify DB state directly.
        let complete = Settings::get(&conn, oz_core::settings::keys::SETUP_COMPLETE)
            .unwrap()
            .unwrap();
        assert_eq!(complete, "1");

        let preset = Settings::get(&conn, oz_core::settings::keys::STORE_PRESET)
            .unwrap()
            .unwrap();
        assert_eq!(preset, "simple-retail");

        // Feature flags.
        let cash = Settings::get(&conn, "feature.cash-payment")
            .unwrap()
            .unwrap();
        assert_eq!(cash, "1");
        let receipt = Settings::get(&conn, "feature.receipt-printing")
            .unwrap()
            .unwrap();
        assert_eq!(receipt, "1");

        // Unknown feature should NOT be present.
        assert_eq!(Settings::get(&conn, "feature.card-payment").unwrap(), None);
    }

    #[test]
    fn complete_setup_without_transaction_leaves_partial_state() {
        let conn = fresh_conn();

        // Run a successful setup first.
        run_complete_setup(&conn, "simple-retail", &["cash-payment"]).unwrap();

        // Write feature rows, preset (but NOT setup_complete) outside a
        // transaction, simulating a crash halfway through.
        {
            let mut registry = FeatureRegistry::new();
            registry.enable(oz_core::Feature::CardPayment);

            let store = Store::new(&conn);
            store.save_features(&registry).unwrap();
            Settings::prune_stale_features(&conn, &registry).unwrap();
            Settings::set(&conn, oz_core::settings::keys::STORE_PRESET, "broken").unwrap();
            // Crashing here — setup_complete is NOT written.
        }

        // setup_complete is still "1" from the first call because the
        // second attempt crashed before writing it.
        let complete = Settings::get(&conn, oz_core::settings::keys::SETUP_COMPLETE)
            .unwrap()
            .unwrap();
        assert_eq!(complete, "1");

        // preset was written (outside a transaction, so visible despite crash).
        let preset = Settings::get(&conn, oz_core::settings::keys::STORE_PRESET)
            .unwrap()
            .unwrap();
        assert_eq!(preset, "broken");
    }

    #[test]
    fn complete_setup_twice_preserves_latest() {
        let conn = fresh_conn();

        // Run setup twice with different presets.
        run_complete_setup(&conn, "first", &["cash-payment", "barcode-scanning"]).unwrap();

        run_complete_setup(
            &conn,
            "second",
            &["restaurant", "cash-payment", "kitchen-display"],
        )
        .unwrap();

        // Second setup's results are in effect.
        let preset = Settings::get(&conn, oz_core::settings::keys::STORE_PRESET)
            .unwrap()
            .unwrap();
        assert_eq!(preset, "second");

        let store = Store::new(&conn);
        let loaded = store.load_features().unwrap();
        assert!(loaded.is_enabled(oz_core::Feature::Restaurant));
        assert!(loaded.is_enabled(oz_core::Feature::KitchenDisplay));
        assert!(!loaded.is_enabled(oz_core::Feature::BarcodeScanning));
        assert!(!loaded.is_enabled(oz_core::Feature::SimpleRetail));
    }

    // ── show_setup_wizard tests ─────────────────────────────────────

    #[test]
    fn show_setup_wizard_defaults_to_true() {
        let conn = fresh_conn();
        // No setup ran → key should be absent (defaults to true/show).
        let val = Settings::get(&conn, oz_core::settings::keys::SHOW_SETUP_WIZARD).unwrap();
        assert_eq!(val, None, "absent means show wizard");
    }

    #[test]
    fn show_setup_wizard_is_false_after_complete_setup() {
        let conn = fresh_conn();

        run_complete_setup(&conn, "restaurant", &["cash-payment"]).unwrap();

        let val = Settings::get(&conn, oz_core::settings::keys::SHOW_SETUP_WIZARD)
            .unwrap()
            .unwrap();
        assert_eq!(val, "false");
    }

    #[test]
    fn show_setup_wizard_is_false_after_dismiss() {
        let conn = fresh_conn();

        // Simulate dismiss_setup_wizard logic.
        Settings::set(&conn, oz_core::settings::keys::SHOW_SETUP_WIZARD, "false").unwrap();

        let val = Settings::get(&conn, oz_core::settings::keys::SHOW_SETUP_WIZARD)
            .unwrap()
            .unwrap();
        assert_eq!(val, "false");
    }

    #[test]
    fn get_setup_status_returns_completed_when_wizard_dismissed() {
        let conn = fresh_conn();

        // dismiss_setup_wizard
        Settings::set(&conn, oz_core::settings::keys::SHOW_SETUP_WIZARD, "false").unwrap();

        let raw = Settings::get(&conn, oz_core::settings::keys::SHOW_SETUP_WIZARD)
            .unwrap()
            .unwrap();
        assert_eq!(raw, "false");

        // Simulate the get_setup_status logic:
        let completed = Settings::get(&conn, oz_core::settings::keys::SHOW_SETUP_WIZARD)
            .unwrap()
            .map(|v| v == "false")
            .unwrap_or(false);
        assert!(completed);
    }

    #[test]
    fn get_setup_status_returns_not_completed_when_key_absent() {
        let conn = fresh_conn();

        let completed = Settings::get(&conn, oz_core::settings::keys::SHOW_SETUP_WIZARD)
            .unwrap()
            .map(|v| v == "false")
            .unwrap_or(false);
        assert!(!completed, "absent key means not completed");
    }
}
