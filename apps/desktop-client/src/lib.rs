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

pub mod commands;
pub mod error;
pub mod state;

use crate::error::AppError;
use crate::state::AppState;
use tauri::Manager;

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::audit::list_audit_log,
            commands::auth::staff_login,
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
            commands::plugins::reload_plugins,
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
            commands::settings::get_setting,
            commands::settings::set_setting,
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
            commands::setup::seed_default_roles,
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
            commands::terminals::get_terminal_profile,
            commands::terminals::set_terminal_profile,
            commands::terminals::list_terminal_profiles,
            commands::terminals::delete_terminal_profile,
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
            commands::shifts::open_shift,
            commands::shifts::close_shift,
            commands::shifts::get_active_shift,
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
            commands::tables::get_table,
            commands::tables::create_table,
            commands::tables::update_table,
            commands::tables::delete_table,
            commands::tables::update_table_status,
            commands::tables::assign_table_order,
            commands::tables::release_table,
            commands::tables::list_sections,
            commands::workspaces::list_workspaces,
            commands::workspaces::list_all_workspaces,
            commands::workspaces::set_user_workspaces,
            commands::workspaces::get_user_workspaces,
            commands::workspaces::list_workspace_screens,
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
