#![warn(missing_docs)]

//! OZ-POS tablet shell (Tauri v2 mobile).
//!
//! Registers the same business modules as the desktop client but
//! with a mobile-optimised Tauri configuration (no window, touch
//! gestures, mobile plugins).
//!
//! The heavy lifting (DB, commands, event handlers) is delegated to
//! the shared crates (`oz-core`, `platform-kernel`, `modules-*`).
//! This file wires them into a Tauri v2 mobile app.

/// All `#[tauri::command]` handlers.
pub mod commands;
/// Single error type for every Tauri command.
pub mod error;
/// Global application state (DB, kernel, sync daemon).
pub mod state;

#[cfg(not(test))]
use crate::error::AppError;
#[cfg(not(test))]
use crate::state::AppState;
#[cfg(not(test))]
use oz_core::db::Store;
#[cfg(not(test))]
use oz_core::sync_client::SyncConfig;
#[cfg(not(test))]
use tauri::Manager;

/// Application entry point, called by `main.rs`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise tokio-console before any other tracing setup.
    platform_startup::console::init_console_subscriber();

    oz_logging::init(); // Gated out of test builds to keep the test binary free of WebView2Loader.dll
    // linkage (see commit 562f1f0 for full diagnosis).
    #[cfg(not(test))]
    {
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
                                    if let Err(e) =
                                        oz_core::sync_client::sync_pending(&store, &config)
                                    {
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
                commands::auth::create_session,
                commands::auth::destroy_session,
                commands::branding::get_brand_settings,
                commands::branding::set_brand_primary_colour,
                commands::branding::set_brand_logo_path,
                commands::branding::set_brand_store_name,
                commands::bundles::list_bundles,
                commands::bundles::get_bundle,
                commands::bundles::create_bundle,
                commands::bundles::update_bundle,
                commands::bundles::delete_bundle,
                commands::bundles::lookup_bundle_by_sku,
                commands::customers::list_customers,
                commands::customers::get_customer,
                commands::customers::create_customer,
                commands::customers::update_customer,
                commands::customers::delete_customer,
                commands::loyalty::get_loyalty_account,
                commands::loyalty::list_loyalty_accounts,
                commands::loyalty::earn_loyalty_points,
                commands::loyalty::redeem_loyalty_points,
                commands::loyalty::list_loyalty_tiers,
                commands::loyalty::update_loyalty_tier,
                commands::loyalty::get_points_value,
                commands::loyalty::get_or_create_loyalty_account,
                commands::staff::list_staff,
                commands::staff::list_staff,
                commands::staff::list_roles,
                commands::staff::create_staff,
                commands::staff::update_staff,
                commands::categories::list_categories,
                commands::categories::create_category,
                commands::categories::update_category,
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
                commands::gift_cards::issue_gift_card,
                commands::gift_cards::get_gift_card,
                commands::gift_cards::list_gift_cards,
                commands::gift_cards::get_gift_card_balance,
                commands::gift_cards::redeem_gift_card,
                commands::gift_cards::top_up_gift_card,
                commands::gift_cards::freeze_gift_card,
                commands::gift_cards::unfreeze_gift_card,
                commands::health::ping,
                commands::health::version,
                commands::health::get_local_ip,
                commands::pos::start_sale,
                commands::pos::add_line,
                commands::pos::complete_sale,
                commands::pos::set_cart_discount,
                commands::pos::override_line_price,
                commands::pos::list_active_carts,
                commands::pos::get_active_cart,
                commands::pos::hold_cart,
                commands::pos::list_held_carts,
                commands::pos::list_open_bills,
                commands::pos::get_held_cart,
                commands::pos::compute_cart_tax,
                commands::pos::delete_held_cart,
                commands::kds::list_kds_orders,
                commands::kds::get_kds_queue,
                commands::kds::update_kds_status,
                commands::kds::create_kds_order_from_sale,
                commands::kds::get_kds_order,
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
                commands::settings::get_user_preferences,
                commands::settings::set_user_preferences,
                commands::setup::get_enabled_features,
                commands::setup::complete_setup,
                commands::setup::dismiss_setup_wizard,
                commands::products::list_products,
                commands::products::create_product,
                commands::products::update_product,
                commands::products::delete_product,
                commands::products::lookup_by_barcode,
                commands::products::lookup_product_by_sku,
                commands::products::adjust_stock,
                commands::products::get_product_track_serial,
                commands::promotions::list_promotions,
                commands::promotions::get_promotion,
                commands::promotions::create_promotion,
                commands::promotions::update_promotion,
                commands::promotions::delete_promotion,
                commands::promotions::apply_promotion,
                commands::promotions::get_sale_promotions,
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
                commands::terminals::list_terminal_overrides,
                commands::terminals::set_terminal_override,
                commands::terminals::delete_terminal_override,
                commands::offline::enqueue_offline,
                commands::offline::list_pending_offline,
                commands::offline::list_all_offline,
                commands::offline::pending_offline_count,
                commands::offline::retry_offline_sync,
                commands::offline::delete_offline_item,
                commands::sync::get_sync_settings,
                commands::sync::update_sync_settings,
                commands::sync::sync_run,
                commands::sync::sync_pull,
                commands::sync::pending_sync_count,
                commands::refunds::process_refund,
                commands::refunds::list_refunds,
                commands::refunds::lookup_sale_by_receipt_barcode,
                commands::reports::get_daily_revenue,
                commands::reports::get_weekly_revenue,
                commands::reports::get_monthly_revenue,
                commands::reports::get_top_products,
                commands::reports::get_hourly_heatmap,
                commands::reports::get_low_stock_alerts,
                commands::reports::get_category_breakdown,
                commands::scale::read_scale_weight,
                commands::tables::list_tables,
                commands::tables::get_table,
                commands::tables::create_table,
                commands::tables::update_table,
                commands::tables::delete_table,
                commands::tables::update_table_status,
                commands::tables::assign_table_order,
                commands::tables::release_table,
                commands::tables::list_sections,
            ])
            .run(tauri::generate_context!())
            .map_err(AppError::from);

        tracing::info!("tablet: shutting down");
        // Kernel shutdown happens in AppState::drop() — see state.rs.

        if let Err(e) = result {
            tracing::error!(error = %e, "OZ-POS tablet exited with error");
            std::process::exit(1);
        }
    }
}
