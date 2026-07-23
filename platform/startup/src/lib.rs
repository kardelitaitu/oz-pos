#![warn(missing_docs)]
// Allow `cfg(feature = "metrics")` from the transitive dependency on
// `oz-reporting` without requiring platform-startup to declare the feature.
#![allow(unexpected_cfgs)]

//! Shared application startup for OZ-POS desktop and tablet clients.
//!
//! Both `apps/desktop-client` and `apps/tablet-client` call this crate
//! to avoid duplicating module registration and event handler wiring.
//!
//! The background sync daemon remains in each client because it depends on
//! the client-specific `AppState` type.
//!
//! # Usage
//! ```no_run
//! # use platform_startup::init_module_system;
//! # use platform_kernel::Kernel;
//! # use tokio::sync::Mutex as AsyncMutex;
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let kernel = AsyncMutex::new(Kernel::new());
//! # let db_path = std::path::Path::new(":memory:");
//! // In your Tauri setup closure:
//! init_module_system(&kernel, db_path)?;
//! # Ok(())
//! # }
//! ```

pub mod console;
pub mod event_handlers;
pub mod metrics;
pub mod rate_sync;

use std::sync::{Arc, Mutex};

use oz_core::cache::Cache;
use platform_kernel::Kernel;
use rusqlite::Connection;
use tokio::sync::Mutex as AsyncMutex;
use tracing::info;

/// Open a WAL-mode SQLite connection for event handlers.
fn open_handler_connection(
    db_path: &std::path::Path,
) -> Result<Arc<Mutex<Connection>>, Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    Ok(Arc::new(Mutex::new(conn)))
}

/// Initialise the caching layer.
///
/// Attempts a Redis connection using `redis_url` and `ttl_seconds`.
/// Falls back to a no-op cache when Redis is unavailable or the
/// `cache-redis` feature is disabled.
pub fn init_cache(redis_url: &str, ttl_seconds: u64) -> Arc<dyn Cache> {
    oz_core::cache::create_cache(redis_url, ttl_seconds)
}

/// Register all business modules and wire event handlers on the kernel.
///
/// Called from each client's `setup` closure after `AppState` is created.
pub fn init_module_system(
    kernel: &AsyncMutex<Kernel>,
    db_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // ── Module system lifecycle ───────────────────────────────────────
    {
        let mut k = kernel.blocking_lock();
        k.register(Box::new(modules_inventory::InventoryModule::new()))?;
        k.register(Box::new(modules_crm::CrmModule::new()))?;
        k.register(Box::new(modules_tax::TaxModule::new()))?;
        k.register(Box::new(modules_settings::SettingsModule::new()))?;
        k.register(Box::new(modules_staff::StaffModule::new()))?;
        k.register(Box::new(modules_sales::SalesModule::new()))?;
        k.register(Box::new(modules_reporting::ReportingModule::new()))?;
        k.register(Box::new(modules_terminal::TerminalModule::new()))?;
        k.register(Box::new(modules_currency::CurrencyModule::new()))?;
        k.load_all()?;
        k.start_all()?;
        drop(k);

        // Open a second connection for event handlers (WAL allows concurrent readers).
        let handler_conn = open_handler_connection(db_path)?;

        // Wire event handlers on the bus.
        let k = kernel.blocking_lock();
        let bus = k.event_bus();

        bus.subscribe::<oz_core::events::SaleCompleted>(
            "sale.completed",
            Box::new(crate::event_handlers::SaleSyncEnqueuer::new(
                handler_conn.clone(),
            )),
        );
        bus.subscribe(
            "sale.completed",
            Box::new(modules_crm::handlers::CrmHistoryHandler::new(
                handler_conn.clone(),
            )),
        );
        bus.subscribe::<oz_core::events::SaleCompleted>(
            "sale.completed",
            Box::new(crate::event_handlers::AuditLogHandler::new(
                handler_conn.clone(),
            )),
        );
        bus.subscribe::<oz_core::events::ProductCreated>(
            "product.created",
            Box::new(crate::event_handlers::AuditLogHandler::new(
                handler_conn.clone(),
            )),
        );
        bus.subscribe::<oz_core::events::ProductCreated>(
            "product.created",
            Box::new(crate::event_handlers::InventorySyncEnqueuer::new(
                handler_conn.clone(),
            )),
        );
        bus.subscribe::<oz_core::events::StockAdjusted>(
            "stock.adjusted",
            Box::new(crate::event_handlers::AuditLogHandler::new(
                handler_conn.clone(),
            )),
        );
        bus.subscribe::<oz_core::events::StockAdjusted>(
            "stock.adjusted",
            Box::new(crate::event_handlers::InventorySyncEnqueuer::new(
                handler_conn.clone(),
            )),
        );
        bus.subscribe::<oz_core::events::SaleCompleted>(
            "sale.completed",
            Box::new(modules_reporting::handlers::SaleCompletedReporter::new(
                handler_conn.clone(),
            )),
        );
        bus.subscribe::<oz_core::events::SaleCompleted>(
            "sale.completed",
            Box::new(crate::event_handlers::LoyaltyEarnHandler::new(handler_conn)),
        );

        // ── ADR #22 Phase 0e: SettingsUpdated handler (non-blocking) ──
        bus.subscribe::<oz_core::events::SettingsUpdated>(
            "settings.updated",
            Box::new(crate::event_handlers::SettingsUpdatedHandler::new()),
        );

        // ── WhatsApp notification handlers (opt-in via feature flag + env vars) ─
        #[cfg(feature = "whatsapp-notifications")]
        {
            use oz_notification::NotificationClient;

            match oz_notification::whatsapp::WhatsAppClient::from_env() {
                Ok(whatsapp) => {
                    let client: std::sync::Arc<dyn NotificationClient> =
                        std::sync::Arc::new(whatsapp);

                    bus.subscribe::<oz_core::events::SaleCompleted>(
                        "sale.completed",
                        Box::new(oz_notification::handlers::OrderConfirmationHandler::new(
                            client.clone(),
                            std::env::var("WHATSAPP_STORE_PHONE").ok(),
                        )),
                    );
                    bus.subscribe::<oz_core::events::SaleCompleted>(
                        "sale.completed",
                        Box::new(oz_notification::handlers::PaymentReceiptHandler::new(
                            client.clone(),
                            std::env::var("WHATSAPP_RECEIPT_PHONE")
                                .unwrap_or_else(|_| "+6280000000000".into()),
                        )),
                    );
                    // Default threshold: alert when ≤ 5 items remaining.
                    let threshold: i64 = std::env::var("WHATSAPP_STOCK_ALERT_THRESHOLD")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(5);
                    let manager_phone = std::env::var("WHATSAPP_MANAGER_PHONE")
                        .unwrap_or_else(|_| "+6280000000000".into());
                    bus.subscribe::<oz_core::events::StockAdjusted>(
                        "stock.adjusted",
                        Box::new(oz_notification::handlers::StockLowAlertHandler::new(
                            client,
                            threshold,
                            manager_phone,
                        )),
                    );

                    tracing::info!(
                        "WhatsApp notification handlers wired (3 handlers on sale.completed + stock.adjusted)"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "WhatsApp notifications compiled in but env vars not set — handlers skipped"
                    );
                }
            }
        }
    }

    info!("module system initialised with event bus handlers");

    // Spawn the stale-pending-sale reaper as a background daemon.
    init_pending_sale_reaper(db_path);

    Ok(())
}

/// Spawn a background daemon with a watchdog that logs on panic or
/// unexpected exit.
///
/// Uses `tauri::async_runtime::spawn` (which is available during
/// synchronous Tauri `setup`, unlike bare `tokio::spawn`).  Panic
/// detection is done via a `oneshot` channel: if the daemon future
/// panics, the channel sender is dropped during unwind and the
/// watchdog sees a `RecvError`.
pub fn spawn_daemon(
    name: &'static str,
    fut: impl std::future::Future<Output = ()> + Send + 'static,
) {
    tauri::async_runtime::spawn(async move {
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Watchdog: fired when the daemon future resolves or panics.
        tauri::async_runtime::spawn(async move {
            match rx.await {
                Ok(()) => tracing::warn!("{name} exited unexpectedly"),
                Err(_) => tracing::error!("{name} panicked"),
            }
        });

        // Run the daemon.  If it panics, the `tx` drop during unwind
        // causes the watchdog to receive `Err(RecvError)`.
        fut.await;
        let _ = tx.send(());
    });
}

/// Spawn the ADR-20 stale-pending-sale reaper as a periodic background task.
///
/// Every 60 seconds, queries for pending sales whose `pending_expires_at`
/// has passed and auto-voids them, crediting stock back to original
/// deduction locations. Uses a separate WAL-mode connection so the
/// background task doesn't block or get blocked by the main connection.
///
/// If the database at `db_path` cannot be opened, the reaper logs an error
/// and exits — it does not crash the application.
pub fn init_pending_sale_reaper(db_path: &std::path::Path) {
    use oz_core::db::Store;
    use std::time::Duration;

    let path = db_path.to_owned();
    spawn_daemon("pending-sale-reaper", async move {
        // Create a dedicated connection for the reaper.
        let conn = match Connection::open(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(?e, "pending sale reaper: failed to open DB — skipping");
                return;
            }
        };
        conn.pragma_update(None, "foreign_keys", "ON").ok();
        conn.pragma_update(None, "journal_mode", "WAL").ok();

        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;

            let store = Store::new(&conn);
            match store.reap_stale_pending_sales() {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!("pending sale reaper: voided {count} stale sale(s)");
                    }
                }
                Err(e) => {
                    tracing::warn!("pending sale reaper: error: {e}");
                }
            }
        }
    });
}

/// Initialise and start the exchange rate auto-sync daemon.
///
/// Spawns a background task that periodically fetches exchange rates
/// from the public Frankfurter API and stores them in the database.
/// Returns the daemon handle so callers can inspect status or shut it
/// down.
pub async fn init_rate_sync(db: rate_sync::DbConnection) -> rate_sync::RateSyncDaemon {
    let daemon = rate_sync::RateSyncDaemon::new();
    daemon.start(db).await;
    daemon
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::Kernel;
    use rusqlite::Connection;

    /// Helper: create an in-memory SQLite database with migrations applied,
    /// and write it to a temp file so we can pass a path.
    fn create_temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut conn = Connection::open(&db_path).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        oz_core::migrations::run(&mut conn).unwrap();
        drop(conn);
        (dir, db_path)
    }

    #[test]
    fn init_module_system_registers_all_modules() {
        let kernel = AsyncMutex::new(Kernel::new());
        let (_dir, db_path) = create_temp_db();

        init_module_system(&kernel, &db_path).unwrap();

        let k = kernel.blocking_lock();
        // Verify modules are registered
        assert!(
            k.is_registered("inventory"),
            "inventory module should be registered"
        );
        assert!(k.is_registered("crm"), "crm module should be registered");
        assert!(k.is_registered("tax"), "tax module should be registered");
        assert!(
            k.is_registered("settings"),
            "settings module should be registered"
        );
        assert!(
            k.is_registered("staff"),
            "staff module should be registered"
        );
        assert!(
            k.is_registered("sales"),
            "sales module should be registered"
        );
        assert!(
            k.is_registered("reporting"),
            "reporting module should be registered"
        );
        assert!(
            k.is_registered("terminal"),
            "terminal module should be registered"
        );
        assert!(
            k.is_registered("currency"),
            "currency module should be registered"
        );
        assert_eq!(k.module_count(), 9);
    }

    #[test]
    fn init_module_system_loads_and_starts_modules() {
        let kernel = AsyncMutex::new(Kernel::new());
        let (_dir, db_path) = create_temp_db();

        init_module_system(&kernel, &db_path).unwrap();

        let k = kernel.blocking_lock();
        assert!(k.is_loaded(), "kernel should be loaded");
        assert!(k.is_started(), "kernel should be started");
    }

    #[test]
    fn init_module_system_wires_event_handlers() {
        let kernel = AsyncMutex::new(Kernel::new());
        let (_dir, db_path) = create_temp_db();

        init_module_system(&kernel, &db_path).unwrap();

        let k = kernel.blocking_lock();
        let bus = k.event_bus();
        // Verify event handlers are registered for key topics
        assert!(
            bus.has_handlers("sale.completed"),
            "sale.completed should have handlers"
        );
        assert!(
            bus.has_handlers("product.created"),
            "product.created should have handlers"
        );
        assert!(
            bus.has_handlers("stock.adjusted"),
            "stock.adjusted should have handlers"
        );
        // 5 handlers on sale.completed, 2 on product.created, 2 on stock.adjusted
        assert!(
            bus.handler_count() >= 5,
            "expected at least 5 handlers total"
        );
    }

    #[test]
    fn init_module_system_with_invalid_db_path_fails() {
        let kernel = AsyncMutex::new(Kernel::new());
        let bad_path = std::path::Path::new("/nonexistent/path/db.sqlite");

        let result = init_module_system(&kernel, bad_path);
        assert!(result.is_err(), "should fail with invalid path");
    }

    #[test]
    fn init_module_system_twice_registers_duplicate_modules() {
        let kernel = AsyncMutex::new(Kernel::new());
        let (_dir, db_path) = create_temp_db();

        init_module_system(&kernel, &db_path).unwrap();

        // Calling init again should fail because modules are already registered
        let result = init_module_system(&kernel, &db_path);
        assert!(
            result.is_err(),
            "second init should fail due to duplicate modules"
        );
    }

    #[test]
    fn event_bus_has_correct_handler_topics() {
        let kernel = AsyncMutex::new(Kernel::new());
        let (_dir, db_path) = create_temp_db();

        init_module_system(&kernel, &db_path).unwrap();

        let k = kernel.blocking_lock();
        let bus = k.event_bus();
        assert_eq!(bus.topic_count(), 4, "should have 4 event topics");
    }
}
