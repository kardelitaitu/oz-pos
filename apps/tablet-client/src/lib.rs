//! OZ-POS tablet shell (Tauri v2 mobile).
//!
//! Registers the same business modules as the desktop client but
//! with a mobile-optimised Tauri configuration (no window, touch
//! gestures, mobile plugins).
//!
//! The heavy lifting (DB, commands, event handlers) is delegated to
//! the shared crates (`oz-core`, `platform-kernel`, `modules-*`).
//! This file wires them into a Tauri v2 mobile app.

pub mod commands;
pub mod error;
pub mod state;

use tauri::Manager;
use oz_core::db::Store;
use oz_core::sync_client::SyncConfig;
use crate::error::AppError;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    oz_logging::init();

    let result: Result<(), AppError> = tauri::Builder::default()
        .setup(|app| {
            let state = AppState::new(app.handle())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            // ── Module system lifecycle (shared startup) ──────────────
            platform_startup::init_module_system(&state.kernel, &state.db_path)?;

            // ── Background sync daemon ────────────────────────────────
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        let db = state.db.lock().await;
                        let store = Store::new(&db);
                        match SyncConfig::from_settings(&store) {
                            Ok(Some(config)) => {
                                if let Err(e) = oz_core::sync_client::sync_pending(&store, &config) {
                                    tracing::error!(error = %e, "sync cycle failed");
                                }
                            }
                            Ok(None) => {}
                            Err(e) => tracing::error!(error = %e, "failed to load sync config"),
                        }
                    }
                }
            });

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::audit::list_audit_log,
            commands::auth::staff_login,
            commands::customers::list_customers,
            commands::customers::get_customer,
            commands::customers::create_customer,
            commands::customers::update_customer,
            commands::customers::delete_customer,
            commands::staff::list_staff,
            commands::staff::list_staff,
            commands::staff::list_roles,
            commands::staff::create_staff,
            commands::staff::update_staff,
            commands::categories::list_categories,
            commands::categories::create_category,
            commands::categories::delete_category,
            commands::currencies::currency_info,
            commands::currencies::list_currencies,
            commands::currencies::get_default_currency,
            commands::currencies::set_default_currency,
            commands::exchange_rates::list_exchange_rates,
            commands::exchange_rates::create_exchange_rate,
            commands::exchange_rates::delete_exchange_rate,
            commands::features::list_all_features,
            commands::features::set_feature,
            commands::health::ping,
            commands::health::version,
            commands::pos::start_sale,
            commands::pos::add_line,
            commands::pos::complete_sale,
            commands::pos::set_cart_discount,
            commands::pos::hold_cart,
            commands::pos::list_held_carts,
            commands::pos::get_held_cart,
            commands::pos::delete_held_cart,
            commands::history::list_sales,
            commands::history::get_sale,
            commands::history::export_daily_summary,
            commands::history::export_sales_by_hour,
            commands::history::export_eod_report,
            commands::void::void_sale,
            commands::hardware::open_cash_drawer,
            commands::hardware::print_receipt,
            commands::hardware::print_sales_receipt,
            commands::hardware::list_scanners,
            commands::hardware::start_scanner,
            commands::hardware::stop_scanner,
            commands::settings::get_receipt_settings,
            commands::settings::set_receipt_settings,
            commands::settings::get_store_settings,
            commands::settings::set_store_settings,
            commands::setup::get_enabled_features,
            commands::setup::complete_setup,
            commands::products::list_products,
            commands::products::create_product,
            commands::products::update_product,
            commands::products::delete_product,
            commands::products::lookup_by_barcode,
            commands::products::adjust_stock,
            commands::product_variants::list_product_variants,
            commands::product_variants::get_product_variant,
            commands::product_variants::create_product_variant,
            commands::product_variants::update_product_variant,
            commands::product_variants::delete_product_variant,
            commands::setup::get_setup_status,
            commands::tax::list_tax_rates,
            commands::tax::create_tax_rate,
            commands::tax::update_tax_rate,
            commands::tax::delete_tax_rate,
            commands::tax::list_category_tax_rates,
            commands::tax::set_category_tax_rates,
            commands::terminals::list_terminals,
            commands::terminals::get_terminal,
            commands::terminals::register_terminal,
            commands::terminals::update_terminal,
            commands::terminals::ping_terminal,
            commands::terminals::delete_terminal,
            commands::offline::enqueue_offline,
            commands::offline::list_pending_offline,
            commands::offline::list_all_offline,
            commands::offline::pending_offline_count,
            commands::offline::retry_offline_sync,
            commands::offline::delete_offline_item,
            commands::sync::get_sync_settings,
            commands::sync::update_sync_settings,
            commands::sync::trigger_sync,
            commands::sync::pending_sync_count,
            commands::refunds::process_refund,
            commands::refunds::list_refunds,
        ])
        .run(tauri::generate_context!())
        .map_err(AppError::from);

    tracing::info!("tablet: shutting down");

    if let Err(e) = result {
        tracing::error!(error = %e, "OZ-POS tablet exited with error");
        std::process::exit(1);
    }
}
