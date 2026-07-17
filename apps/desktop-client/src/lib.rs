#![warn(missing_docs)]

//! Tauri v2 application entry point.
//!
//! Wires the [`AppState`] (DB connection, driver registry, config) into the
//! Tauri builder, registers all `#[tauri::command]` handlers, and starts the
//! runtime. Mobile builds use the same code via `#[cfg_attr(mobile,
//! tauri::mobile_entry_point)]`.
//!
//! Adding a new command:
//! 1. Define `pub async fn` with `#[tauri::command]` in `commands/<feature>.rs`.
//! 2. Add it to the `invoke_handler!` macro below in the same order as the
//!    `commands` module re-exports.
//! 3. Document the command in the `tauri-ipc` skill.

/// All `#[tauri::command]` handlers, organised by domain.
pub mod commands;
/// Single error type for every Tauri command.
pub mod error;
/// LAN event forwarding for multi-terminal setups.
pub mod lan_server;
/// Global application state (DB, kernel, sync daemon, registry).
pub mod state;

/// Embed `Microsoft.Windows.Common-Controls` v6 dependency into the
/// test binary's manifest via an MSVC `.drectve` linker directive
/// section.  Required by `WebView2Loader.dll` at startup, which the
/// test binary otherwise lacks (it bypasses `tauri-bundler`).
///
/// `/MANIFESTINPUT` causes `CVT1100: duplicate resource` on `[[bin]]`
/// test targets; `/MANIFESTDEPENDENCY` in `build.rs` fails with
/// `LNK1181` because Cargo splits the argument on spaces.  The
/// `.drectve` section injects the directives directly into the object
/// file, bypassing Cargo's argument parsing entirely.
///
/// See: https://github.com/orgs/tauri-apps/discussions/11179
///
/// **NOTE:** If you modify the byte string below, update the array size
/// (currently 184).  The compiler error message will report the exact
/// expected size if there's a mismatch.
#[cfg(all(test, windows, target_env = "msvc"))]
#[used]
#[unsafe(link_section = ".drectve")]
#[rustfmt::skip]
static TEST_MANIFEST_DIRECTIVES: [u8; 184] = *b" /MANIFEST:EMBED /MANIFESTDEPENDENCY:\"type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'\"\x00";

use crate::error::AppError;
use crate::state::AppState;
use tauri::Manager;

/// Application entry point, called by `main.rs`.
///
/// Initialises logging, loads the database, starts the sync daemon,
/// registers all Tauri commands, and starts the event loop. Mobile
/// builds use the same code via `#[cfg_attr(mobile, tauri::mobile_entry_point)]`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise tokio-console before any other tracing setup.
    platform_startup::console::init_console_subscriber();

    // Initialise structured logging early so the very first line of Tauri
    // output is captured.
    oz_logging::init();

    let result: Result<(), AppError> = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let state = AppState::new(app.handle())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            // ── Module system lifecycle (shared startup) ──────────────
            platform_startup::init_module_system(&state.kernel, &state.db_path)?;

            app.manage(state);

            // ── Background sync daemon ────────────────────────────────
            let db = app.state::<AppState>().db.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                state.sync_daemon.start(db).await;
            });

            // ── Background prune daemon (ADR #6 Q4 / P-1 Ledger Retention) ─
            let prune_db = app.state::<AppState>().db.clone();
            tauri::async_runtime::spawn(async move {
                platform_sync::daemon::SyncDaemon::start_prune_task(prune_db);
            });

            // ── LAN event forwarder ────────────────────────────────────
            let forwarder = crate::lan_server::LanEventForwarder::new();
            let handle = forwarder.handle();
            tauri::async_runtime::spawn(forwarder.run());

            // Subscribe event bus handlers for LAN forwarding.
            // Use try_lock() because .setup() is synchronous.
            {
                let state = app.state::<AppState>();
                if let Ok(kernel) = state.kernel.try_lock() {
                    let bus = kernel.event_bus();
                    bus.subscribe(
                        "sale.completed",
                        Box::new(handle.sale_completed_handler()),
                    );
                    bus.subscribe(
                        "order.course_fired",
                        Box::new(handle.course_fired_handler()),
                    );
                    tracing::info!(
                        "LAN event forwarder handlers registered for sale.completed and order.course_fired"
                    );
                } else {
                    tracing::warn!("kernel lock contended, LAN handlers not registered");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::audit::list_audit_log,
            commands::auth::staff_login,
            commands::auth::staff_check_username,
            commands::auth::create_session,
            commands::auth::destroy_session,
            commands::branding::get_brand_settings,
            commands::branding::set_brand_primary_colour,
            commands::branding::set_brand_logo_path,
            commands::branding::set_brand_store_name,
            commands::branding::pick_logo_file,
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
            commands::data::get_backup_status,
            commands::data::create_backup,
            commands::data::export_data,
            commands::data::import_preview,
            commands::data::import_data,
            commands::staff::list_staff,
            commands::staff::list_roles,
            commands::staff::create_staff,
            commands::staff::update_staff,
            commands::staff::bootstrap_owner,
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
            commands::features::set_features_bulk,
            commands::features::set_feature,
            commands::gift_cards::issue_gift_card,
            commands::gift_cards::get_gift_card,
            commands::gift_cards::list_gift_cards,
            commands::gift_cards::get_gift_card_balance,
            commands::gift_cards::redeem_gift_card,
            commands::gift_cards::top_up_gift_card,
            commands::gift_cards::freeze_gift_card,
            commands::gift_cards::unfreeze_gift_card,
            commands::inventory_counts::create_stock_count,
            commands::inventory_counts::get_stock_count,
            commands::inventory_counts::list_stock_counts,
            commands::inventory_counts::get_count_lines,
            commands::inventory_counts::add_count_line,
            commands::inventory_counts::update_count_line,
            commands::inventory_counts::remove_count_line,
            commands::inventory_counts::complete_stock_count,
            commands::inventory_counts::update_stock_count_status,
            commands::inventory_counts::list_stock_adjustments,
            commands::purchasing::list_suppliers,
            commands::purchasing::get_supplier,
            commands::purchasing::create_supplier,
            commands::purchasing::update_supplier,
            commands::purchasing::list_purchase_orders,
            commands::purchasing::get_purchase_order,
            commands::purchasing::create_purchase_order,
            commands::purchasing::update_po_status,
            commands::purchasing::receive_purchase_order,
            commands::stock_transfers::create_stock_transfer,
            commands::stock_transfers::get_stock_transfer,
            commands::stock_transfers::list_stock_transfers,
            commands::stock_transfers::get_stock_transfer_lines,
            commands::stock_transfers::add_stock_transfer_line,
            commands::stock_transfers::remove_stock_transfer_line,
            commands::stock_transfers::send_stock_transfer,
            commands::stock_transfers::receive_stock_transfer,
            commands::stock_transfers::cancel_stock_transfer,
            commands::health::ping,
            commands::health::version,
            commands::health::get_local_ip,
            commands::pos::start_sale,
            commands::pos::start_sale_scoped,
            commands::pos::add_line,
            commands::pos::add_line_scoped,
            commands::pos::complete_sale,
            commands::pos::complete_sale_scoped,
            commands::pos::set_cart_discount,
            commands::pos::set_cart_discount_scoped,
            commands::pos::override_line_price,
            commands::pos::override_line_price_scoped,
            commands::pos::list_active_carts,
            commands::pos::list_active_carts_scoped,
            commands::pos::get_active_cart,
            commands::pos::get_active_cart_scoped,
            commands::pos::hold_cart,
            commands::pos::hold_cart_scoped,
            commands::pos::list_held_carts,
            commands::pos::list_held_carts_scoped,
            commands::pos::list_open_bills,
            commands::pos::list_open_bills_scoped,
            commands::pos::get_held_cart,
            commands::pos::get_held_cart_scoped,
            commands::pos::compute_cart_tax,
            commands::pos::compute_cart_tax_scoped,
            commands::pos::delete_held_cart,
            commands::pos::delete_held_cart_scoped,
            commands::plugins::reload_plugins,
            commands::kds::list_kds_orders,
            commands::kds::list_kds_orders_scoped,
            commands::kds::get_kds_queue,
            commands::kds::get_kds_queue_scoped,
            commands::kds::update_kds_status,
            commands::kds::update_kds_status_scoped,
            commands::kds::create_kds_order_from_sale,
            commands::kds::create_kds_order_from_sale_scoped,
            commands::kds::get_kds_order,
            commands::kds::get_kds_order_scoped,
            commands::history::list_sales,
            commands::history::list_sales_scoped,
            commands::history::get_sale,
            commands::history::get_sale_scoped,
            commands::history::export_daily_summary,
            commands::history::export_daily_summary_scoped,
            commands::history::export_sales_by_hour,
            commands::history::export_sales_by_hour_scoped,
            commands::history::export_eod_report,
            commands::history::export_eod_report_scoped,
            commands::void::void_sale,
            commands::void::void_sale_scoped,
            commands::hardware::open_cash_drawer,
            commands::hardware::print_receipt,
            commands::hardware::print_sales_receipt,
            commands::hardware::list_scanners,
            commands::hardware::start_scanner,
            commands::hardware::stop_scanner,
            commands::settings::get_receipt_settings,
            commands::settings::set_receipt_settings,
            commands::settings::set_receipt_settings_scoped,
            commands::settings::get_store_settings,
            commands::settings::set_store_settings,
            commands::settings::set_store_settings_scoped,
            commands::settings::get_credit_settings,
            commands::settings::set_credit_settings,
            commands::settings::set_credit_settings_scoped,
            commands::settings::list_credit_sales,
            commands::settings::settle_credit,
            commands::settings::settle_credit_scoped,
            commands::settings::get_hardware_settings,
            commands::settings::set_hardware_settings,
            commands::settings::set_hardware_settings_scoped,
            commands::settings::get_user_preferences,
            commands::settings::set_user_preferences,
            commands::settings::get_user_preferences_scoped,
            commands::settings::set_user_preferences_scoped,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::set_setting_scoped,
            commands::setup::get_enabled_features,
            commands::setup::complete_setup,
            commands::setup::dismiss_setup_wizard,
            commands::products::list_products,
            commands::products::list_products_scoped,
            commands::products::create_product,
            commands::products::create_product_scoped,
            commands::products::update_product,
            commands::products::update_product_scoped,
            commands::products::delete_product,
            commands::products::delete_product_scoped,
            commands::products::lookup_by_barcode,
            commands::products::lookup_by_barcode_scoped,
            commands::products::lookup_product_by_sku,
            commands::products::lookup_product_by_sku_scoped,
            commands::products::adjust_stock,
            commands::products::adjust_stock_scoped,
            commands::products::get_product_track_serial,
            commands::promotions::list_promotions,
            commands::promotions::list_promotions_scoped,
            commands::promotions::get_promotion,
            commands::promotions::get_promotion_scoped,
            commands::promotions::create_promotion,
            commands::promotions::create_promotion_scoped,
            commands::promotions::update_promotion,
            commands::promotions::update_promotion_scoped,
            commands::promotions::delete_promotion,
            commands::promotions::delete_promotion_scoped,
            commands::promotions::apply_promotion,
            commands::promotions::apply_promotion_scoped,
            commands::promotions::get_sale_promotions,
            commands::promotions::get_sale_promotions_scoped,
            commands::product_variants::list_product_variants,
            commands::product_variants::get_product_variant,
            commands::product_variants::create_product_variant,
            commands::product_variants::update_product_variant,
            commands::product_variants::delete_product_variant,
            commands::setup::seed_default_roles,
            commands::setup::seed_default_roles_scoped,
            commands::setup::get_setup_status,
            commands::tax::list_tax_rates,
            commands::tax::create_tax_rate,
            commands::tax::update_tax_rate,
            commands::tax::delete_tax_rate,
            commands::tax::list_category_tax_rates,
            commands::tax::set_category_tax_rates,
            commands::terminals::list_terminals,
            commands::terminals::list_terminals_scoped,
            commands::terminals::get_terminal,
            commands::terminals::get_terminal_scoped,
            commands::terminals::register_terminal,
            commands::terminals::register_terminal_scoped,
            commands::terminals::update_terminal,
            commands::terminals::update_terminal_scoped,
            commands::terminals::ping_terminal,
            commands::terminals::ping_terminal_scoped,
            commands::terminals::delete_terminal,
            commands::terminals::delete_terminal_scoped,
            commands::terminals::list_terminal_overrides,
            commands::terminals::list_terminal_overrides_scoped,
            commands::terminals::set_terminal_override,
            commands::terminals::set_terminal_override_scoped,
            commands::terminals::delete_terminal_override,
            commands::terminals::delete_terminal_override_scoped,
            commands::terminals::set_device_binding,
            commands::terminals::set_device_binding_scoped,
            commands::terminals::get_device_binding,
            commands::terminals::get_device_binding_scoped,
            commands::terminals::clear_device_binding,
            commands::terminals::clear_device_binding_scoped,
            commands::terminals::get_terminal_profile,
            commands::terminals::get_terminal_profile_scoped,
            commands::terminals::set_terminal_profile,
            commands::terminals::set_terminal_profile_scoped,
            commands::terminals::list_terminal_profiles,
            commands::terminals::list_terminal_profiles_scoped,
            commands::terminals::delete_terminal_profile,
            commands::terminals::delete_terminal_profile_scoped,
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
            commands::refunds::process_refund_scoped,
            commands::refunds::list_refunds,
            commands::refunds::list_refunds_scoped,
            commands::refunds::lookup_sale_by_receipt_barcode,
            commands::refunds::lookup_sale_by_receipt_barcode_scoped,
            commands::reports::get_menu_engineering,
            commands::reports::get_daily_revenue,
            commands::reports::get_weekly_revenue,
            commands::reports::get_monthly_revenue,
            commands::reports::get_top_products,
            commands::reports::get_hourly_heatmap,
            commands::reports::get_low_stock_alerts,
            commands::reports::get_category_breakdown,
            commands::shifts::open_shift,
            commands::shifts::open_shift_scoped,
            commands::shifts::close_shift,
            commands::shifts::close_shift_scoped,
            commands::shifts::get_active_shift,
            commands::shifts::get_active_shift_scoped,
            commands::shifts::list_shifts,
            commands::shifts::get_shift,
            commands::shifts::get_shift_report,
            commands::shifts::create_cash_payout,
            commands::hardware::list_displays,
            commands::hardware::display_show,
            commands::hardware::display_clear,
            commands::scale::read_scale_weight,
            commands::store_profiles::list_store_profiles,
            commands::store_profiles::get_store_profile,
            commands::store_profiles::get_primary_store,
            commands::store_profiles::create_store_profile,
            commands::store_profiles::update_store_profile,
            commands::store_profiles::set_primary_store,
            commands::store_profiles::delete_store_profile,
            commands::tables::list_tables,
            commands::tables::list_tables_scoped,
            commands::tables::get_table,
            commands::tables::get_table_scoped,
            commands::tables::create_table,
            commands::tables::create_table_scoped,
            commands::tables::update_table,
            commands::tables::update_table_scoped,
            commands::tables::delete_table,
            commands::tables::delete_table_scoped,
            commands::tables::update_table_status,
            commands::tables::update_table_status_scoped,
            commands::tables::assign_table_order,
            commands::tables::assign_table_order_scoped,
            commands::tables::release_table,
            commands::tables::release_table_scoped,
            commands::tables::list_sections,
            commands::tables::list_sections_scoped,
            commands::workspaces::list_workspaces_scoped,
            commands::workspaces::list_workspaces,
            commands::workspaces::get_workspace_instance_scoped,
            commands::workspaces::get_workspace_instance,
            commands::workspaces::create_workspace_instance_scoped,
            commands::workspaces::recover_workspace_instances_scoped,
            commands::workspaces::suspend_surplus_workspace_instances_scoped,
            commands::workspaces::create_workspace_instance,
            commands::workspaces::list_workspace_types,
            commands::workspaces::list_all_workspaces,
            commands::workspaces::list_all_workspaces_scoped,
            commands::workspaces::set_user_workspaces,
            commands::workspaces::set_user_workspaces_scoped,
            commands::workspaces::get_user_workspaces,
            commands::workspaces::get_user_workspaces_scoped,
            commands::workspaces::set_user_workspace_instances_scoped,
            commands::workspaces::set_user_workspace_instances,
            commands::workspaces::get_user_workspace_instances_scoped,
            commands::workspaces::get_user_workspace_instances,
            commands::workspaces::resolve_boot_store,
            commands::workspaces::list_workspace_screens_scoped,
            commands::workspaces::list_workspace_screens,
            commands::license::activate_license,
            commands::license::get_machine_id,
            commands::license::renew_license,
            commands::license::get_license_status,
            commands::license::check_license_status,
        ])
        .run(tauri::generate_context!())
        .map_err(AppError::from);

    tracing::info!("application shutting down");
    // Kernel shutdown happens in AppState::drop() — see state.rs.

    if let Err(e) = result {
        tracing::error!(error = %e, "OZ-POS exited with error");
        std::process::exit(1);
    }
}
