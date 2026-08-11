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
/// Background email report scheduler.
pub mod email_scheduler;
/// Single error type for every Tauri command.
pub mod error;
/// LAN event forwarding for multi-terminal setups.
pub mod lan_server;
/// Global application state (DB, kernel, sync daemon, registry).
pub mod state;

/// Debug-only bootstrap that connects the desktop client to the local
/// dev sync server (`scripts/start-local-sync.bat` → `:3099`) without
/// manual Settings configuration. Excluded from release builds.
#[cfg(debug_assertions)]
mod sync_bootstrap;

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
use tauri::{Emitter, Manager};

/// Application entry point, called by `main.rs`.
///
/// Initialises logging, loads the database, starts the sync daemon,
/// registers all Tauri commands, and starts the event loop. Mobile
/// builds use the same code via `#[cfg_attr(mobile, tauri::mobile_entry_point)]`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(deprecated)]
pub fn run() {
    // Initialise tokio-console before any other tracing setup.
    platform_startup::console::init_console_subscriber();

    // Initialise structured logging early so the very first line of Tauri
    // output is captured. Uses try_init so a second invocation (e.g.
    // by a plugin or test harness) does not panic.
    let _ = oz_logging::try_init();

    let result: Result<(), AppError> = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = AppState::new(app.handle())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            // ── Module system lifecycle (shared startup) ──────────────
            platform_startup::init_module_system(&state.kernel, &state.db_path)?;

            // ── settings_updated → Tauri event bridge (ADR #22 Phase 0e) ─
            // The frontend SettingsContext subscribes to `settings_updated`
            // and triggers a debounced scoped refetch. Without this the
            // SettingsUpdated handler only logs "settings_updated Tauri
            // bridge not yet wired" and settings changes never reach the
            // UI via the event bus (the save-handler markSettingsUpdated
            // path covers same-terminal saves; this closes the loop for
            // the EventBus-published events).
            let app_handle = app.handle().clone();
            platform_startup::event_handlers::set_settings_emit_fn(Box::new(
                move |event_name, payload| {
                    let _ = app_handle.emit(event_name, payload);
                },
            ));

            app.manage(state);

            // Recover a journaled cross-database topology Apply at startup,
            // before the user can issue another mutation. The Apply mutex
            // also serializes this recovery with any early UI request.
            let recovery_app_handle = app.handle().clone();
            platform_startup::spawn_daemon("topology recovery", async move {
                let state = recovery_app_handle.state::<AppState>();
                if let Err(error) = commands::topology::recover_pending_topology_apply_at_startup(&state).await {
                    tracing::error!(error = %error, "topology recovery failed; Apply remains blocked until recovery succeeds");
                }
            });

            // ── Show the main window after state restore ────────────
            // The window starts with visible:false to prevent initial
            // position flash while window-state restores its position/size.
            // After setup completes we explicitly show it. If the window
            // is not found (e.g. headless/CI), this is a no-op.
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.show();
            }

            // ── Auto-provision local sync (debug builds only) ────────
            // A fresh dev DB ships with an empty `sync_server_url` and sync
            // disabled, so the background daemon silently no-ops until the
            // user manually configures Settings → Sync. If the local dev
            // server (`start-local-sync.bat` → :3099) is up, request a JWT
            // and persist the connection. Spawned BEFORE the sync daemon
            // so the daemon's first tick (60–120s out) sees the fresh
            // config. Never runs in release builds — an existing
            // configuration is never touched.
            #[cfg(debug_assertions)]
            {
                let bootstrap_db = app.state::<AppState>().db.clone();
                platform_startup::spawn_daemon("sync auto-provision", async move {
                    crate::sync_bootstrap::auto_provision_local_sync(bootstrap_db).await;
                });
            }

            // ── Background sync daemon ────────────────────────────────
            let db = app.state::<AppState>().db.clone();
            let app_handle = app.handle().clone();
            // SYNC-10: a settings change made on ANOTHER terminal and pulled
            // by either daemon is re-emitted as the `settings_updated` Tauri
            // event — the same wire shape the frontend SettingsContext
            // listens for — so the UI refetches the changed scope. Local
            // saves already publish the domain event; this closes the loop
            // for the sync-applied path. The sink is shared by the SQLite
            // and PostgreSQL daemons.
            let settings_sink = commands::sync::settings_changed_sink(&app_handle);
            let sqlite_sink = settings_sink.clone();
            let pg_sink = settings_sink.clone();
            platform_startup::spawn_daemon("sync daemon", async move {
                let state = app_handle.state::<AppState>();
                state
                    .sync_daemon
                    .start_with_sink(db, sqlite_sink)
                    .await;
            });

            // ── Background PostgreSQL sync daemon ─────────────────────
            // The optional PG transport. The daemon no-ops on every tick
            // while `pg_sync.enabled` is off and re-reads the connection
            // settings each cycle, so this unconditional spawn mirrors the
            // SQLite daemon's — the pg_sync_start / pg_sync_stop commands
            // control the same instance.
            let pg_db = app.state::<AppState>().db.clone();
            let pg_app_handle = app.handle().clone();
            platform_startup::spawn_daemon("pg sync daemon", async move {
                let state = pg_app_handle.state::<AppState>();
                state
                    .pg_sync_daemon
                    .start_with_sink(pg_db, pg_sink)
                    .await;
            });

            // ── Background prune daemon (ADR #6 Q4 / P-1 Ledger Retention) ─
            let prune_db = app.state::<AppState>().db.clone();
            platform_startup::spawn_daemon("prune daemon", async move {
                platform_sync::daemon::SyncDaemon::start_prune_task(prune_db);
            });

            // ── Background email report scheduler ──────────────────
            let email_db = app.state::<AppState>().db.clone();
            platform_startup::spawn_daemon("email report scheduler", async move {
                crate::email_scheduler::run_scheduler_loop(email_db).await;
            });

            // ── Background session cleanup daemon (TTL expiry) ──────
            // Runs every 5 minutes to sweep expired sessions from the
            // in-memory store. Expired sessions are also caught during
            // resolve_session, so this is a safety net + memory reclaimer.
            {
                let session_store = app.state::<AppState>().session_store.clone();
                platform_startup::spawn_daemon("session cleanup", async move {
                    let mut interval = tokio::time::interval(
                        std::time::Duration::from_secs(300),
                    );
                    // Skip the first tick so startup isn't delayed.
                    interval.tick().await;
                    loop {
                        interval.tick().await;
                        let Ok(mut store) = session_store.write() else {
                            tracing::warn!(
                                "session store lock poisoned — skipping cleanup cycle"
                            );
                            continue;
                        };
                        let before = store.len();
                        store.retain(|token, ctx| {
                            if ctx.is_expired() {
                                tracing::trace!(
                                    token = %token,
                                    "session cleanup: removing expired session"
                                );
                                false
                            } else {
                                true
                            }
                        });
                        let pruned = before - store.len();
                        if pruned > 0 {
                            tracing::info!(
                                "session cleanup: pruned {pruned} expired session(s), \
                                 {remaining} remain",
                                remaining = store.len()
                            );
                        }
                    }
                });
            }

            // ── LAN event forwarder ────────────────────────────────────
            // Read LAN server config from the settings table (C-4).
            // Default: loopback-only, no PSK. External bind requires
            // both lan_server.bind="0.0.0.0" AND a non-empty lan_server.psk.
            let (lan_bind_addr, lan_psk) = {
                let state = app.state::<AppState>();
                let db = state.db.blocking_lock();
                let bind = oz_core::Settings::get(&db, "lan_server.bind")
                    .unwrap_or(None)
                    .unwrap_or_else(|| "127.0.0.1".to_string());
                let psk = oz_core::Settings::get(&db, "lan_server.psk")
                    .unwrap_or(None)
                    .filter(|s| !s.is_empty());
                // Reject external bind without a PSK.
                let bind = if bind == "0.0.0.0" && psk.is_none() {
                    tracing::warn!(
                        "lan_server.bind is 0.0.0.0 but lan_server.psk is empty — falling back to 127.0.0.1"
                    );
                    "127.0.0.1".to_string()
                } else {
                    bind
                };
                (format!("{bind}:9180"), psk)
            };
            let forwarder = crate::lan_server::LanEventForwarder::new(lan_bind_addr, lan_psk);
            let handle = forwarder.handle();
            platform_startup::spawn_daemon("LAN event forwarder", forwarder.run());

            // Subscribe event bus handlers for LAN forwarding.
            // .setup() is synchronous, so we can't use .await.
            // A single try_lock() could silently skip LAN handler
            // registration if the kernel lock is momentarily held
            // during startup. Use a bounded retry loop to give the
            // lock holder time to finish without risking a deadlock.
            const LAN_LOCK_RETRIES: usize = 10;
            {
                let state = app.state::<AppState>();
                let mut registered = false;
                for _ in 0..LAN_LOCK_RETRIES {
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
                        registered = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                if !registered {
                    tracing::warn!(
                        "kernel lock contended after 100ms, LAN handlers not registered — \
                         LAN event forwarding disabled for this session"
                    );
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::audit::list_audit_log,
            commands::audit::list_audit_log_scoped,
            commands::audit::get_audit_review_status_scoped,
            commands::audit::mark_audit_reviewed_scoped,
            commands::audit::export_audit_log_scoped,
            commands::auth::staff_login,
            commands::auth::staff_check_username,
            commands::auth::create_session,
            commands::auth::destroy_session,
            commands::branding::get_brand_settings,
            commands::branding::get_brand_settings_scoped,
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
            commands::customers::list_customers_scoped,
            commands::customers::search_customers_scoped,
            commands::customers::get_customer_history_scoped,
            commands::customers::get_customer,
            commands::customers::create_customer_scoped,
            commands::customers::update_customer_scoped,
            commands::customers::delete_customer_scoped,
            commands::loyalty::get_loyalty_account_scoped,
            commands::loyalty::list_loyalty_accounts_scoped,
            commands::loyalty::earn_loyalty_points_scoped,
            commands::loyalty::redeem_loyalty_points_scoped,
            commands::loyalty::list_loyalty_tiers_scoped,
            commands::loyalty::update_loyalty_tier_scoped,
            commands::loyalty::get_points_value_scoped,
            commands::loyalty::get_or_create_loyalty_account_scoped,
            commands::data::get_backup_status,
            commands::data::create_backup,
            commands::data::export_data,
            commands::data::import_preview,
            commands::email::send_test_report,
            commands::email::get_report_schedule,
            commands::email::save_report_schedule,
            commands::data::import_data,
            commands::staff::list_staff_scoped,
            commands::staff::list_roles_scoped,
            commands::staff::create_staff_scoped,
            commands::staff::update_staff_scoped,
            commands::staff::get_staff_profile_scoped,
            commands::staff::bootstrap_owner,
            commands::categories::list_categories,
            commands::categories::list_categories_scoped,
            commands::categories::create_category,
            commands::categories::create_category_scoped,
            commands::categories::update_category,
            commands::categories::update_category_scoped,
            commands::categories::delete_category,
            commands::categories::delete_category_scoped,
            commands::currencies::currency_info,
            commands::currencies::list_currencies,
            commands::currencies::list_currencies_scoped,
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
            commands::inventory_counts::create_stock_count_scoped,
            commands::inventory_counts::get_stock_count_scoped,
            commands::inventory_counts::list_stock_counts_scoped,
            commands::inventory_counts::get_count_lines_scoped,
            commands::inventory_counts::add_count_line_scoped,
            commands::inventory_counts::update_count_line_scoped,
            commands::inventory_counts::remove_count_line_scoped,
            commands::inventory_counts::complete_stock_count_scoped,
            commands::inventory_counts::update_stock_count_status_scoped,
            commands::inventory_counts::list_stock_adjustments_scoped,
            commands::purchasing::list_suppliers,
            commands::purchasing::get_supplier,
            commands::purchasing::create_supplier,
            commands::purchasing::update_supplier,
            commands::purchasing::list_purchase_orders,
            commands::purchasing::get_purchase_order,
            commands::purchasing::create_purchase_order,
            commands::purchasing::update_po_status,
            commands::purchasing::receive_purchase_order,
            commands::stock_transfers::create_stock_transfer_scoped,
            commands::stock_transfers::get_stock_transfer_scoped,
            commands::stock_transfers::list_stock_transfers_scoped,
            commands::stock_transfers::list_in_transit_transfers_scoped,
            commands::stock_transfers::get_stock_transfer_lines_scoped,
            commands::stock_transfers::add_stock_transfer_line_scoped,
            commands::stock_transfers::remove_stock_transfer_line_scoped,
            commands::stock_transfers::send_stock_transfer_scoped,
            commands::stock_transfers::receive_stock_transfer_scoped,
            commands::stock_transfers::cancel_stock_transfer_scoped,
            commands::health::ping,
            commands::health::version,
            commands::health::version_scoped,
            commands::health::get_device_id,
            commands::health::get_local_ip,
            commands::pos::start_sale,
            commands::pos::start_sale_scoped,
            commands::pos::add_line,
            commands::pos::add_line_scoped,
            commands::pos::complete_sale,
            commands::pos::complete_sale_scoped,
            commands::pos::complete_sale_with_resolved_shortfalls_scoped,
            commands::pos::set_cart_discount,
            commands::pos::set_cart_discount_scoped,
            commands::pos::override_line_price,
            commands::pos::override_line_price_scoped,
            commands::pos::override_cart_deduction_location_scoped,
            commands::pos::get_cart_deduction_location,
            commands::pos::hold_cart,
            commands::pos::hold_cart_scoped,
            commands::pos::list_held_carts,
            commands::pos::list_held_carts_scoped,
            commands::pos::list_open_bills,
            commands::pos::list_open_bills_scoped,
            commands::pos::get_held_cart,
            commands::pos::get_held_cart_scoped,
            commands::pos::compute_cart_tax_scoped,
            commands::pos::delete_held_cart,
            commands::pos::delete_held_cart_scoped,
            commands::inventory::create_inventory_location,
            commands::inventory::list_inventory_locations,
            commands::inventory::update_inventory_location,
            commands::inventory::deactivate_inventory_location,
            commands::inventory::set_workspace_inventory_locations,
            commands::inventory::get_workspace_inventory_locations,
            commands::inventory::start_inventory_shift,
            commands::inventory::end_inventory_shift,
            commands::inventory::get_active_inventory_shift,
            commands::inventory::list_inventory_shifts,
            commands::inventory::create_inventory_transaction,
            commands::inventory::list_inventory_transactions,
            commands::inventory::list_inventory_transactions_for_shift,
            commands::inventory::get_inventory_transaction,
            commands::inventory::set_stock_threshold,
            commands::inventory::get_stock_thresholds,
            commands::inventory::delete_stock_threshold,
            commands::inventory::active_stock_alerts_scoped,
            commands::inventory::acknowledge_stock_alert_scoped,
            commands::inventory::finalize_sale,
            commands::inventory::void_pending_sale,
            commands::inventory::get_low_stock_alerts_at_location_scoped,
            commands::inventory::get_workspace_locations_scoped,
            commands::inventory::invalidate_location_cache_scoped,
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
            commands::kds::get_kds_order_lines_scoped,
            commands::kds::update_kds_line_item_status_scoped,
            commands::kds::update_kds_order_items,
            commands::kds::update_kds_order_items_scoped,
            commands::kds::print_kds_chit_scoped,
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
            commands::hardware::print_sales_receipt_scoped,
            commands::hardware::list_scanners,
            commands::hardware::start_scanner,
            commands::hardware::stop_scanner,
            commands::settings::get_receipt_settings,
            commands::settings::get_receipt_settings_scoped,
            commands::settings::set_receipt_settings,
            commands::settings::set_receipt_settings_scoped,
            commands::settings::get_store_settings,
            commands::settings::get_store_settings_scoped,
            commands::settings::set_store_settings,
            commands::settings::set_store_settings_scoped,
            commands::settings::get_credit_settings,
            commands::settings::set_credit_settings,
            commands::settings::set_credit_settings_scoped,
            commands::settings::list_credit_sales,
            commands::settings::list_credit_sales_scoped,
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
            commands::settings::set_settings,
            commands::settings::set_settings_scoped,
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
            commands::products::get_product_track_serial_scoped,
            commands::products::get_product_track_serial_batch,
            commands::products::get_product_track_serial_batch_scoped,
            commands::products::record_product_search_scoped,
            commands::browser::open_product_images_scoped,
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
            commands::setup::seed_default_roles_scoped,
            commands::setup::get_setup_status,
            commands::tax::list_tax_rates_scoped,
            commands::tax::create_tax_rate_scoped,
            commands::tax::update_tax_rate_scoped,
            commands::tax::delete_tax_rate_scoped,
            commands::tax::get_tax_rate_dependency_counts_scoped,
            commands::tax::list_category_tax_rates_scoped,
            commands::tax::set_category_tax_rates_scoped,
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
            commands::offline::offline_queue_status_summary,
            commands::offline::list_all_offline,
            commands::offline::pending_offline_count,
            commands::offline::retry_offline_sync,
            commands::offline::delete_offline_item,
            commands::offline::requeue_remote_failure,
            commands::offline::list_remote_failures,
            commands::sync::get_sync_settings,
            commands::sync::get_sync_settings_scoped,
            commands::sync::update_sync_settings,
            commands::sync::sync_run,
            commands::sync::sync_pull,
            commands::sync::pending_sync_count,
            commands::sync::test_sync_connection,
            commands::sync::request_sync_token,
            commands::sync::get_sync_plan,
            commands::sync::get_pg_sync_settings,
            commands::sync::update_pg_sync_settings,
            commands::sync::pg_sync_status,
            commands::sync::pg_sync_start,
            commands::sync::pg_sync_stop,
            commands::refunds::process_refund,
            commands::refunds::process_refund_scoped,
            commands::refunds::list_refunds,
            commands::refunds::list_refunds_scoped,
            commands::refunds::lookup_sale_by_receipt_barcode,
            commands::refunds::lookup_sale_by_receipt_barcode_scoped,
            commands::reports::get_menu_engineering_scoped,
            commands::reports::get_sale_line_margins_scoped,
            commands::reports::get_daily_revenue_scoped,
            commands::reports::get_weekly_revenue_scoped,
            commands::reports::get_monthly_revenue_scoped,
            commands::reports::get_top_products_scoped,
            commands::reports::get_category_popularity_scoped,
            commands::reports::get_category_popularity_trend_scoped,
            commands::reports::get_category_forecast_scoped,
            commands::reports::get_hourly_heatmap_scoped,
            commands::reports::get_low_stock_alerts_scoped,
            commands::reports::get_category_breakdown_scoped,
            commands::reports::build_custom_report_scoped,
            commands::analytics::get_staff_analytics_scoped,
            commands::analytics::get_staff_analytics_daily_scoped,
            commands::security::get_key_rotation_info,
            commands::security::rotate_encryption_key,
            commands::shifts::open_shift,
            commands::shifts::open_shift_scoped,
            commands::shifts::close_shift,
            commands::shifts::close_shift_scoped,
            commands::shifts::get_active_shift,
            commands::shifts::get_active_shift_scoped,
            commands::shifts::list_shifts,
            commands::shifts::list_shifts_scoped,
            commands::shifts::get_shift,
            commands::shifts::get_shift_report,
            commands::shifts::create_cash_payout,
            commands::hardware::list_displays,
            commands::hardware::display_show,
            commands::hardware::display_clear,
            commands::hardware::discover_hardware,
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
            commands::workspaces::list_workspaces_for_store_scoped,
            commands::workspaces::get_workspace_instance_scoped,
            commands::workspaces::create_workspace_instance_scoped,
            commands::workspaces::update_workspace_instance_scoped,
            commands::workspaces::archive_workspace_instance_scoped,
            commands::workspaces::recover_workspace_instances_scoped,
            commands::workspaces::suspend_surplus_workspace_instances_scoped,
            commands::workspaces::list_all_workspaces_scoped,
            commands::workspaces::set_user_workspace_instances_scoped,
            commands::workspaces::get_user_workspace_instances_scoped,
            commands::workspaces::resolve_boot_store,
            commands::workspaces::list_workspace_screens_scoped,
            commands::workspaces::list_workspace_screens,
            commands::license::activate_license,
            commands::license::get_machine_id,
            commands::license::renew_license,
            commands::license::get_license_status,
            commands::license::check_license_status,
            // The legacy unscoped save_topology command is intentionally not
            // registered. All production writes use the authenticated,
            // revision-aware apply_topology_diff command.
            commands::topology::load_topology,
            commands::topology::can_save_topology,
            commands::topology::apply_topology_diff,
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
