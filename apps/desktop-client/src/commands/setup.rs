//! Setup Wizard commands.
//!
//! `complete_setup` persists the chosen preset and enabled features to
//! the settings table and marks the wizard as complete.
//! `get_setup_status` lets the front-end decide whether to show the
//! wizard or go straight to the main app.

use oz_core::{FeatureRegistry, Settings, Store, features};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

// ── Args ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
/// Completesetupargs.
pub struct CompleteSetupArgs {
    /// Store preset name (e.g. `"simple-retail"`, `"restaurant"`).
    pub preset: String,
    /// Enabled feature keys (kebab-case, e.g. `"cash-payment"`).
    pub features: Vec<String>,
    /// ISO-4217 default currency code (e.g. `"IDR"`, `"USD"`).
    #[serde(default = "default_currency")]
    pub default_currency: String,
}

fn default_currency() -> String {
    "IDR".to_string()
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
/// Setupstatus.
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
#[tauri::command]
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
#[tauri::command]
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

        // 6. Set default currency.
        Settings::set_default_currency(&tx, &args.default_currency)?;

        // 7. Dismiss the wizard so it doesn't show on next launch.
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
#[tauri::command]
pub async fn get_setup_status(state: State<'_, AppState>) -> Result<SetupStatus, AppError> {
    let db = state.db.lock().await;

    let completed = Settings::get(&db, oz_core::settings::keys::SHOW_SETUP_WIZARD)?
        .map(|v| v == "false")
        .unwrap_or(false);

    let preset = Settings::get(&db, oz_core::settings::keys::STORE_PRESET)?;

    Ok(SetupStatus { completed, preset })
}

/// Requires the `staff:manage_roles` permission.
#[tauri::command]
pub async fn seed_default_roles_scoped(
    session_token: String,
    state: State<'_, AppState>,
) -> Result<usize, AppError> {
    let session = state.resolve_session(&session_token)?;
    // Authorize against the GLOBAL identity DB: users + roles live there,
    // never in the store DB (which this command is about to seed).
    require_permission_for_session(&state, &session, oz_core::permissions::STAFF_MANAGE_ROLES)
        .await?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let count = store.seed_default_roles()?;
    drop(db);
    tracing::info!(count, "default roles seeded (scoped)");
    Ok(count)
}

/// Dismiss the setup wizard without enabling any features.
///
/// Called when the user clicks "Skip setup". Only writes the
/// `show_setup_wizard = false` flag — no preset or features are saved.
#[tauri::command]
pub async fn dismiss_setup_wizard(state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().await;
    Settings::set(&db, oz_core::settings::keys::SHOW_SETUP_WIZARD, "false")?;
    tracing::info!("setup wizard dismissed (skip)");
    Ok(())
}

#[cfg(test)] #[path = "setup_tests.rs"] mod tests;
