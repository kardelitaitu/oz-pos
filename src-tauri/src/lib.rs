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

use tauri::Manager;
use crate::error::AppError;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise structured logging early so the very first line of Tauri
    // output is captured.
    oz_logging::init();

    let result: Result<(), AppError> = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = AppState::new(app.handle())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
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
            commands::sales::start_sale,
            commands::sales::add_line,
            commands::sales::complete_sale,
            commands::sales::list_sales,
            commands::sales::get_sale,
            commands::sales::export_daily_summary,
            commands::sales::export_sales_by_hour,
            commands::sales::set_cart_discount,
            commands::sales::void_sale,
            commands::sales::export_eod_report,
            commands::sales::hold_cart,
            commands::sales::list_held_carts,
            commands::sales::get_held_cart,
            commands::sales::delete_held_cart,
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
            commands::setup::get_setup_status,
            commands::tax::list_tax_rates,
            commands::tax::create_tax_rate,
            commands::tax::update_tax_rate,
            commands::tax::delete_tax_rate,
        ])
        .run(tauri::generate_context!())
        .map_err(AppError::from);

    if let Err(e) = result {
        tracing::error!(error = %e, "OZ-POS exited with error");
        std::process::exit(1);
    }
}
