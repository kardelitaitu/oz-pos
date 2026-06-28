//! Setup Wizard commands.
//!
//! `complete_setup` persists the chosen preset and enabled features to
//! the settings table and marks the wizard as complete.
//! `get_setup_status` lets the front-end decide whether to show the
//! wizard or go straight to the main app.

use serde::{Deserialize, Serialize};
use tauri::{command, State};
use oz_core::{features, FeatureRegistry, Store, Settings};

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

// ── Commands ─────────────────────────────────────────────────────────

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

        // 1. Persist features.
        store.save_features(&registry)?;

        // 2. Prune stale feature rows that are no longer enabled.
        Settings::prune_stale_features(&tx, &registry)?;

        // 3. Save the preset name.
        Settings::set(&tx, oz_core::settings::keys::STORE_PRESET, &args.preset)?;

        // 4. Mark setup as complete.
        Settings::set(&tx, oz_core::settings::keys::SETUP_COMPLETE, "1")?;
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

    let completed = Settings::get(&db, oz_core::settings::keys::SETUP_COMPLETE)?
        .map(|v| v == "1")
        .unwrap_or(false);

    let preset = Settings::get(&db, oz_core::settings::keys::STORE_PRESET)?;

    Ok(SetupStatus { completed, preset })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use rusqlite::Connection;

    fn fresh_state() -> AppState {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        AppState::for_test_with_conn(conn)
    }

    #[tokio::test]
    async fn complete_setup_persists_features() {
        let state = fresh_state();

        complete_setup(
            tauri::State::new(&state),
            CompleteSetupArgs {
                preset: "simple-retail".into(),
                features: vec![
                    "cash-payment".into(),
                    "barcode-scanning".into(),
                    "receipt-printing".into(),
                    "inventory-tracking".into(),
                    "categories-enabled".into(),
                    "tax-engine".into(),
                ],
            },
        )
        .await
        .unwrap();

        // Verify setup is marked complete.
        let status = get_setup_status(tauri::State::new(&state)).await.unwrap();
        assert!(status.completed);
        assert_eq!(status.preset, Some("simple-retail".into()));
    }

    #[tokio::test]
    async fn get_setup_status_defaults_to_not_completed() {
        let state = fresh_state();

        let status = get_setup_status(tauri::State::new(&state)).await.unwrap();
        assert!(!status.completed);
        assert_eq!(status.preset, None);
    }

    #[tokio::test]
    async fn complete_setup_skips_unknown_features() {
        let state = fresh_state();

        complete_setup(
            tauri::State::new(&state),
            CompleteSetupArgs {
                preset: "custom".into(),
                features: vec![
                    "cash-payment".into(),
                    "made-up-feature".into(), // unknown, should be skipped
                ],
            },
        )
        .await
        .unwrap();

        // Should still succeed.
        let status = get_setup_status(tauri::State::new(&state)).await.unwrap();
        assert!(status.completed);
    }
}
