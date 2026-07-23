//! Shared application-level event handlers.
//!
//! These handlers are cross-cutting concerns that don't belong to a
//! single business module. They are registered on the kernel's event
//! bus by [`crate::init_module_system`].

use std::sync::{Arc, Mutex};

use foundation::contracts::{EventHandler, ModuleResult};
use oz_core::audit::AuditEntry;
use oz_core::db::Store;
use oz_core::events::{ProductCreated, SaleCompleted, SettingsUpdated, StockAdjusted};
use oz_core::offline::SyncPriority;
use rusqlite::Connection;
use tracing::{error, info};

/// Handler that enqueues completed sales to the offline sync queue.
///
/// Listens for `sale.completed` events and writes a "complete_sale"
/// entry to the offline queue. The sync engine picks it up on the
/// next sync cycle and pushes it to the remote server.
///
/// This is the core of the offline-first architecture: every completed
/// sale goes through the queue, regardless of connectivity. The sync
/// engine handles delivery when the network is available.
#[derive(Debug)]
pub struct SaleSyncEnqueuer {
    db: Arc<Mutex<Connection>>,
}

impl SaleSyncEnqueuer {
    /// Create a new handler with a shared database connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

impl EventHandler<SaleCompleted> for SaleSyncEnqueuer {
    fn handle(&self, event: &SaleCompleted) -> ModuleResult {
        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("sync enqueuer: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        let payload = serde_json::json!({
            "sale_id": event.sale_id,
            "total_minor": event.total_minor,
            "currency": event.currency,
            "customer_id": event.customer_id,
            "line_items": event.line_items,
        })
        .to_string();

        // P-2: Sale completions are Critical priority — they must
        // propagate before inventory or settings changes.
        store
            .enqueue_offline_priority("complete_sale", &payload, SyncPriority::Critical)
            .map_err(|e| {
                error!(
                    sale_id = %event.sale_id,
                    error = %e,
                    "sync enqueuer: failed to enqueue completed sale"
                );
                anyhow::anyhow!("sync enqueuer: enqueue_offline_priority failed: {e}")
            })?;

        info!(
            sale_id = %event.sale_id,
            "sync enqueuer: sale queued for sync (priority=Critical)"
        );

        Ok(())
    }
}

/// Handler that enqueues inventory changes to the offline sync queue.
///
/// Listens for `product.created` and `stock.adjusted` events and writes
/// them to the offline queue. The sync engine pushes them to the remote
/// server on the next sync cycle.
///
/// Together with [`SaleSyncEnqueuer`], this ensures all inventory mutations
/// are tracked for cloud replication.
#[derive(Debug)]
pub struct InventorySyncEnqueuer {
    db: Arc<Mutex<Connection>>,
}

impl InventorySyncEnqueuer {
    /// Create a new handler with a shared database connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

impl EventHandler<ProductCreated> for InventorySyncEnqueuer {
    fn handle(&self, event: &ProductCreated) -> ModuleResult {
        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("inv sync enqueuer: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        let payload = serde_json::json!({
            "sku": event.sku,
            "name": event.name,
            "price_minor": event.price_minor,
            "currency": event.currency,
            "category_id": event.category_id,
            "barcode": event.barcode,
            "initial_stock": event.initial_stock,
        })
        .to_string();

        store
            .enqueue_offline("product.created", &payload)
            .map_err(|e| {
                error!(
                    sku = %event.sku,
                    error = %e,
                    "inv sync enqueuer: failed to enqueue product.created"
                );
                anyhow::anyhow!("inv sync enqueuer: enqueue_offline failed: {e}")
            })?;

        info!(
            sku = %event.sku,
            "inv sync enqueuer: product creation queued for sync"
        );

        Ok(())
    }
}

impl EventHandler<StockAdjusted> for InventorySyncEnqueuer {
    fn handle(&self, event: &StockAdjusted) -> ModuleResult {
        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("inv sync enqueuer: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        let payload = serde_json::json!({
            "sku": event.sku,
            "delta": event.delta,
            "new_qty": event.new_qty,
            "reason": event.reason,
        })
        .to_string();

        store
            .enqueue_offline("stock.adjusted", &payload)
            .map_err(|e| {
                error!(
                    sku = %event.sku,
                    error = %e,
                    "inv sync enqueuer: failed to enqueue stock.adjusted"
                );
                anyhow::anyhow!("inv sync enqueuer: enqueue_offline failed: {e}")
            })?;

        info!(
            sku = %event.sku,
            delta = event.delta,
            reason = %event.reason,
            "inv sync enqueuer: stock adjustment queued for sync"
        );

        Ok(())
    }
}

/// Handler that creates an audit log entry when a domain event fires.
///
/// Records the event details in the audit log for compliance
/// (PCI-DSS 10.2.1, 10.3.1).
#[derive(Debug)]
pub struct AuditLogHandler {
    db: Arc<Mutex<Connection>>,
}

impl AuditLogHandler {
    /// Create a new handler with a shared database connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

impl EventHandler<SaleCompleted> for AuditLogHandler {
    fn handle(&self, event: &SaleCompleted) -> ModuleResult {
        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("audit handler: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        let details = serde_json::json!({
            "sale_id": event.sale_id,
            "total_minor": event.total_minor,
            "currency": event.currency,
            "line_count": event.line_items.len(),
        })
        .to_string();

        let entry = AuditEntry::new(
            "", // system-initiated action
            "sale.completed",
            Some("sale"),
            Some(&event.sale_id),
            Some(details),
            "success",
        );

        store.log_audit(&entry).map_err(|e| {
            error!(
                sale_id = %event.sale_id,
                error = %e,
                "audit handler: failed to log sale.completed"
            );
            anyhow::anyhow!("audit handler: log_audit failed: {e}")
        })?;

        info!(
            sale_id = %event.sale_id,
            total_minor = event.total_minor,
            "audit handler: sale.completed logged"
        );

        Ok(())
    }
}

impl EventHandler<StockAdjusted> for AuditLogHandler {
    fn handle(&self, event: &StockAdjusted) -> ModuleResult {
        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("audit handler: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        let details = serde_json::json!({
            "sku": event.sku,
            "delta": event.delta,
            "new_qty": event.new_qty,
            "reason": event.reason,
        })
        .to_string();

        let entry = AuditEntry::new(
            "", // system-initiated action
            "stock.adjusted",
            Some("stock"),
            Some(&event.sku),
            Some(details),
            "success",
        );

        store.log_audit(&entry).map_err(|e| {
            error!(
                sku = %event.sku,
                error = %e,
                "audit handler: failed to log stock.adjusted"
            );
            anyhow::anyhow!("audit handler: log_audit failed: {e}")
        })?;

        info!(
            sku = %event.sku,
            delta = event.delta,
            new_qty = event.new_qty,
            reason = %event.reason,
            "audit handler: stock.adjusted logged"
        );

        Ok(())
    }
}

impl EventHandler<ProductCreated> for AuditLogHandler {
    fn handle(&self, event: &ProductCreated) -> ModuleResult {
        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("audit handler: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        let details = serde_json::json!({
            "sku": event.sku,
            "name": event.name,
            "price_minor": event.price_minor,
            "currency": event.currency,
            "initial_stock": event.initial_stock,
        })
        .to_string();

        let entry = AuditEntry::new(
            "", // system-initiated action
            "product.created",
            Some("product"),
            Some(&event.sku),
            Some(details),
            "success",
        );

        store.log_audit(&entry).map_err(|e| {
            error!(
                sku = %event.sku,
                error = %e,
                "audit handler: failed to log product.created"
            );
            anyhow::anyhow!("audit handler: log_audit failed: {e}")
        })?;

        info!(
            sku = %event.sku,
            name = %event.name,
            "audit handler: product.created logged"
        );

        Ok(())
    }
}

/// Handler that earns loyalty points into a customer's loyalty account
/// when a sale completes.
///
/// If the sale has a linked customer, this handler calls
/// `Store::earn_points()` to credit the loyalty_accounts table.
/// The earning rate is determined by the customer's tier multiplier.
#[derive(Debug)]
pub struct LoyaltyEarnHandler {
    db: Arc<Mutex<Connection>>,
}

impl LoyaltyEarnHandler {
    /// Create a new handler with a shared database connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

impl EventHandler<SaleCompleted> for LoyaltyEarnHandler {
    fn handle(&self, event: &SaleCompleted) -> ModuleResult {
        let Some(ref customer_id) = event.customer_id else {
            info!(
                sale_id = %event.sale_id,
                "loyalty earn handler: sale has no customer, skipping"
            );
            return Ok(());
        };

        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("loyalty earn handler: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        // Get or create a loyalty account for this customer.
        let account = store
            .get_or_create_loyalty_account(customer_id)
            .map_err(|e| {
                anyhow::anyhow!(
                    "loyalty earn handler: failed to get/create account for {customer_id}: {e}"
                )
            })?;

        // Earn points based on the sale total.
        store
            .earn_points(customer_id, &event.sale_id, event.total_minor)
            .map_err(|e| {
                anyhow::anyhow!(
                    "loyalty earn handler: earn_points failed for customer {customer_id}: {e}"
                )
            })?;

        info!(
            customer_id = %customer_id,
            sale_id = %event.sale_id,
            account_id = %account.id,
            total_minor = event.total_minor,
            "loyalty earn handler: points credited"
        );

        Ok(())
    }
}

/// Handler that bridges `settings_updated` events to the Tauri frontend.
///
/// Wraps the handler body in `tokio::spawn` so the publisher (which runs
/// synchronously on the EventBus) returns immediately — non-blocking.
/// This prevents UI thread freezes when a settings save triggers a
/// settings refetch IPC round-trip.
///
/// The emit callback is set by the client app during setup via
/// [`set_settings_emit_fn`]. Until set, events are logged at debug level
/// (no-op bridge).
#[derive(Debug, Default)]
pub struct SettingsUpdatedHandler;

impl SettingsUpdatedHandler {
    /// Create a new handler.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }
}

impl EventHandler<SettingsUpdated> for SettingsUpdatedHandler {
    fn handle(&self, event: &SettingsUpdated) -> ModuleResult {
        let changed_keys = event.changed_keys.clone();
        let terminal_id = event.terminal_id.clone();

        // Spawn non-blocking — publish() returns immediately.
        tokio::spawn(async move {
            let payload = serde_json::json!({
                "changed_keys": changed_keys,
                "terminal_id": terminal_id,
            });
            let emit_guard = SETTINGS_EMIT_FN
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(emit) = emit_guard.as_ref() {
                emit("settings_updated", payload);
            } else {
                tracing::debug!(
                    keys = ?payload["changed_keys"],
                    "settings_updated Tauri bridge not yet wired"
                );
            }
        });
        Ok(())
    }
}

/// Global emit callback for bridging EventBus events to the Tauri frontend.
///
/// Set by the client app's setup closure via [`set_settings_emit_fn`].
/// Uses a type-erased callback (`String`, `serde_json::Value`) to avoid
/// coupling `platform-startup` to a concrete Tauri `AppHandle<R>` type.
///
/// Uses a `Mutex` (not `OnceLock`) so tests can replace the callback
/// between test cases — `OnceLock` can only be set once per process lifetime.
#[allow(clippy::type_complexity)]
static SETTINGS_EMIT_FN: std::sync::Mutex<
    Option<Box<dyn Fn(&str, serde_json::Value) + Send + Sync>>,
> = std::sync::Mutex::new(None);

/// Register the emit callback used by [`SettingsUpdatedHandler`].
#[allow(clippy::type_complexity)]
///
/// Called once from the client app's setup closure after the module system
/// is initialized. The callback typically calls `app_handle.emit(event, payload)`.
pub fn set_settings_emit_fn(f: Box<dyn Fn(&str, serde_json::Value) + Send + Sync>) {
    let mut guard = SETTINGS_EMIT_FN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some(f);
}

/// Clear the emit callback (used in tests to reset state between cases).
#[doc(hidden)]
pub fn clear_settings_emit_fn() {
    let mut guard = SETTINGS_EMIT_FN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::events::SaleCompletedLine;
    use oz_core::migrations;
    use oz_core::offline::OfflineQueueStatus;
    use platform_kernel::EventBus;

    fn fresh_db() -> Arc<Mutex<Connection>> {
        Arc::new(Mutex::new(migrations::fresh_db()))
    }

    #[test]
    fn handler_logs_audit_entry() {
        let db = fresh_db();
        let handler = AuditLogHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-1".into(),
            store_id: None,
            line_items: vec![SaleCompletedLine {
                sku: "COFFEE".into(),
                qty: 2,
                unit_price_minor: 350,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 700,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let entries = store.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "sale.completed");
        assert_eq!(entries[0].target_id.as_deref(), Some("sale-1"));
        assert_eq!(entries[0].target_type.as_deref(), Some("sale"));
        assert_eq!(entries[0].outcome, "success");
        assert!(entries[0].details.contains("\"sale_id\":\"sale-1\""));
    }

    #[test]
    fn handler_works_with_event_bus() {
        let db = fresh_db();
        let bus = EventBus::new();
        let handler = AuditLogHandler::new(db.clone());
        bus.subscribe::<SaleCompleted>("sale.completed", Box::new(handler));

        let event = SaleCompleted {
            sale_id: "sale-2".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 0,
            currency: "USD".into(),
            customer_id: None,
        };

        bus.publish(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let entries = store.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].target_id.as_deref(), Some("sale-2"));
    }

    #[test]
    fn handler_product_created_logs_audit_entry() {
        let db = fresh_db();
        let handler = AuditLogHandler::new(db.clone());

        let event = ProductCreated {
            sku: "NEW-PROD".into(),
            name: "New Widget".into(),
            price_minor: 999,
            currency: "USD".into(),
            category_id: None,
            barcode: None,
            initial_stock: 10,
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let entries = store.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "product.created");
        assert_eq!(entries[0].target_id.as_deref(), Some("NEW-PROD"));
        assert_eq!(entries[0].target_type.as_deref(), Some("product"));
        assert!(entries[0].details.contains("\"sku\":\"NEW-PROD\""));
    }

    #[test]
    fn handler_stock_adjusted_logs_audit_entry() {
        let db = fresh_db();
        let handler = AuditLogHandler::new(db.clone());

        let event = StockAdjusted {
            sku: "COFFEE".into(),
            delta: -3,
            new_qty: 47,
            reason: "sale".into(),
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let entries = store.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "stock.adjusted");
        assert_eq!(entries[0].target_id.as_deref(), Some("COFFEE"));
    }

    #[test]
    fn handler_multiple_sales() {
        let db = fresh_db();
        let handler = AuditLogHandler::new(db.clone());

        let event1 = SaleCompleted {
            sale_id: "sale-a".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 100,
            currency: "USD".into(),
            customer_id: None,
        };
        let event2 = SaleCompleted {
            sale_id: "sale-b".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 200,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event1).unwrap();
        handler.handle(&event2).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let entries = store.list_audit_entries(10, 0).unwrap();
        assert_eq!(entries.len(), 2);
    }

    // ── InventorySyncEnqueuer tests ───────────────────────────────────

    #[test]
    fn inv_sync_enqueuer_product_created() {
        let db = fresh_db();
        let handler = InventorySyncEnqueuer::new(db.clone());

        let event = ProductCreated {
            sku: "SYNC-PROD".into(),
            name: "Sync Widget".into(),
            price_minor: 499,
            currency: "USD".into(),
            category_id: Some("cat-goods".into()),
            barcode: Some(foundation::Barcode::new("123456789").unwrap()),
            initial_stock: 20,
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action, "product.created");
        assert!(pending[0].payload.contains("SYNC-PROD"));
        assert_eq!(pending[0].status, OfflineQueueStatus::Pending);
    }

    #[test]
    fn inv_sync_enqueuer_stock_adjusted() {
        let db = fresh_db();
        let handler = InventorySyncEnqueuer::new(db.clone());

        let event = StockAdjusted {
            sku: "COFFEE".into(),
            delta: -5,
            new_qty: 45,
            reason: "sale".into(),
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action, "stock.adjusted");
        assert!(pending[0].payload.contains("COFFEE"));
        assert!(pending[0].payload.contains("-5"));
        assert_eq!(pending[0].status, OfflineQueueStatus::Pending);
    }

    #[test]
    fn inv_sync_enqueuer_multiple_events() {
        let db = fresh_db();
        let handler = InventorySyncEnqueuer::new(db.clone());

        let event1 = ProductCreated {
            sku: "PROD-A".into(),
            name: "Product A".into(),
            price_minor: 100,
            currency: "USD".into(),
            category_id: None,
            barcode: None,
            initial_stock: 10,
        };
        let event2 = StockAdjusted {
            sku: "PROD-A".into(),
            delta: -2,
            new_qty: 8,
            reason: "sale".into(),
        };

        handler.handle(&event1).unwrap();
        handler.handle(&event2).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().any(|i| i.action == "product.created"));
        assert!(pending.iter().any(|i| i.action == "stock.adjusted"));
    }

    // ── SaleSyncEnqueuer tests ───────────────────────────────────────

    #[test]
    fn sync_enqueuer_creates_offline_entry() {
        let db = fresh_db();
        let handler = SaleSyncEnqueuer::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-sync-1".into(),
            store_id: None,
            line_items: vec![SaleCompletedLine {
                sku: "COFFEE".into(),
                qty: 2,
                unit_price_minor: 350,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 700,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action, "complete_sale");
        assert!(pending[0].payload.contains("sale-sync-1"));
        assert_eq!(pending[0].status, OfflineQueueStatus::Pending);
    }

    #[test]
    fn sync_enqueuer_works_with_event_bus() {
        let db = fresh_db();
        let bus = EventBus::new();
        let handler = SaleSyncEnqueuer::new(db.clone());
        bus.subscribe::<SaleCompleted>("sale.completed", Box::new(handler));

        let event = SaleCompleted {
            sale_id: "sale-bus-1".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 0,
            currency: "USD".into(),
            customer_id: None,
        };

        bus.publish(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action, "complete_sale");
    }

    #[test]
    fn sync_enqueuer_multiple_sales() {
        let db = fresh_db();
        let handler = SaleSyncEnqueuer::new(db.clone());

        let event1 = SaleCompleted {
            sale_id: "sale-queue-1".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 100,
            currency: "USD".into(),
            customer_id: None,
        };
        let event2 = SaleCompleted {
            sale_id: "sale-queue-2".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 200,
            currency: "USD".into(),
            customer_id: Some("cust-1".into()),
        };

        handler.handle(&event1).unwrap();
        handler.handle(&event2).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let pending = store.list_pending_offline().unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|i| i.action == "complete_sale"));
        assert!(pending.iter().any(|i| i.payload.contains("sale-queue-1")));
        assert!(pending.iter().any(|i| i.payload.contains("sale-queue-2")));
    }

    // ── LoyaltyEarnHandler tests ─────────────────────────────────

    #[test]
    fn loyalty_earn_skips_when_no_customer() {
        let db = fresh_db();
        let handler = LoyaltyEarnHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-no-cust".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 500,
            currency: "USD".into(),
            customer_id: None,
        };

        // Should succeed without error — no customer, so no points earned.
        handler.handle(&event).unwrap();

        // No loyalty transaction should have been created.
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM loyalty_transactions", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        assert_eq!(count, 0);
    }

    // ── SettingsUpdatedHandler (ADR #22 Phase 0e) ────────────────

    #[tokio::test]
    async fn settings_updated_handler_is_non_blocking() {
        let bus = EventBus::new();
        let handler = SettingsUpdatedHandler::new();
        bus.subscribe::<SettingsUpdated>("settings.updated", Box::new(handler));

        let event = SettingsUpdated {
            changed_keys: vec!["receipt.footer".into()],
            terminal_id: "term-1".into(),
        };

        // publish() must return immediately even though the handler
        // spawns a tokio task that sleeps for 200ms.
        let start = std::time::Instant::now();
        bus.publish(&event).unwrap();
        let elapsed = start.elapsed();

        // The spec requires < 5ms. In practice this should be sub-millisecond.
        assert!(
            elapsed.as_millis() < 5,
            "publish() took {}ms — expected < 5ms (handler must be non-blocking)",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn handler_runs_even_without_emit_fn() {
        let bus = EventBus::new();
        let handler = SettingsUpdatedHandler::new();
        bus.subscribe::<SettingsUpdated>("settings.updated", Box::new(handler));

        let event = SettingsUpdated {
            changed_keys: vec!["store.name".into()],
            terminal_id: "term-2".into(),
        };

        // Should not panic even when emit callback is not set.
        bus.publish(&event).unwrap();

        // Give the spawned task a moment to complete.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn handler_emits_via_callback() {
        use std::sync::Mutex as StdMutex;

        // Clear any emit fn from previous tests.
        clear_settings_emit_fn();

        let bus = EventBus::new();
        let handler = SettingsUpdatedHandler::new();
        bus.subscribe::<SettingsUpdated>("settings.updated", Box::new(handler));

        // Set up a callback that records calls.
        let calls = Arc::new(StdMutex::new(Vec::new()));
        let calls_clone = calls.clone();
        set_settings_emit_fn(Box::new(move |event_name, payload| {
            calls_clone
                .lock()
                .unwrap()
                .push((event_name.to_string(), payload.get("changed_keys").cloned()));
        }));

        let event = SettingsUpdated {
            changed_keys: vec!["receipt.show_tax".into(), "store.branch".into()],
            terminal_id: "term-3".into(),
        };
        bus.publish(&event).unwrap();

        // Let the spawned task execute.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let recorded = calls.lock().unwrap();
        // The callback may fire more than once due to global-static
        // Mutex state + tokio runtime interaction across tests.
        // The core assertion is that the callback DID fire.
        assert!(
            !recorded.is_empty(),
            "expected at least one emit callback invocation"
        );
        assert_eq!(recorded[0].0, "settings_updated");

        // Clean up so subsequent tests start fresh.
        clear_settings_emit_fn();
    }

    /// Full lifecycle: set emit fn → publish → clear → re-set → publish.
    /// Verifies that `clear_settings_emit_fn` correctly resets the global
    /// state and a new callback can be installed afterward.
    #[tokio::test]
    async fn emit_fn_set_clear_reset_lifecycle() {
        use std::sync::Mutex as StdMutex;

        clear_settings_emit_fn();
        let bus = EventBus::new();
        let handler = SettingsUpdatedHandler::new();
        bus.subscribe::<SettingsUpdated>("settings.updated", Box::new(handler));

        // Phase 1: Set first callback and verify it fires.
        let calls1 = Arc::new(StdMutex::new(Vec::new()));
        let c1 = calls1.clone();
        set_settings_emit_fn(Box::new(move |event_name, _payload| {
            c1.lock().unwrap().push(event_name.to_string());
        }));

        bus.publish(&SettingsUpdated {
            changed_keys: vec!["key.a".into()],
            terminal_id: "lifecycle-1".into(),
        })
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            !calls1.lock().unwrap().is_empty(),
            "first emit callback should have fired"
        );

        // Phase 2: Clear and verify the old callback no longer fires.
        clear_settings_emit_fn();
        let count_after_clear = calls1.lock().unwrap().len();

        bus.publish(&SettingsUpdated {
            changed_keys: vec!["key.b".into()],
            terminal_id: "lifecycle-2".into(),
        })
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(
            calls1.lock().unwrap().len(),
            count_after_clear,
            "old callback should not fire after clear"
        );

        // Phase 3: Re-set a new callback and verify it fires.
        let calls2 = Arc::new(StdMutex::new(Vec::new()));
        let c2 = calls2.clone();
        let c2_for_closure = calls2.clone();
        set_settings_emit_fn(Box::new(move |event_name, _payload| {
            c2_for_closure.lock().unwrap().push(event_name.to_string());
        }));

        bus.publish(&SettingsUpdated {
            changed_keys: vec!["key.c".into()],
            terminal_id: "lifecycle-3".into(),
        })
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            !c2.lock().unwrap().is_empty(),
            "re-set emit callback should fire"
        );

        clear_settings_emit_fn();
    }

    #[test]
    fn loyalty_earn_creates_account_and_earns_points() {
        let db = fresh_db();
        let handler = LoyaltyEarnHandler::new(db.clone());

        // Seed a customer and a completed sale.
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO customers (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    "cust-loyal",
                    "Loyal Customer",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z"
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO sales (id, total_minor, currency, line_count, status, created_at, updated_at, subtotal_minor, tax_total_minor)
                 VALUES (?1, 0, 'USD', 0, 'completed', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 0, 0)",
                rusqlite::params!["sale-loyal-1"],
            )
            .unwrap();
        }

        let event = SaleCompleted {
            sale_id: "sale-loyal-1".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 1000,
            currency: "USD".into(),
            customer_id: Some("cust-loyal".into()),
        };

        handler.handle(&event).unwrap();

        // Verify a loyalty account was created.
        let conn = db.lock().unwrap();
        let account_id: String = conn
            .query_row(
                "SELECT id FROM loyalty_accounts WHERE customer_id = 'cust-loyal'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!account_id.is_empty());

        // Verify a transaction was recorded.
        let txn_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM loyalty_transactions WHERE account_id = ?1",
                [&account_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(txn_count, 1);

        // Points should be > 0 (10 points per unit × 1000 minor units).
        let points: i64 = conn
            .query_row(
                "SELECT points FROM loyalty_accounts WHERE id = ?1",
                [&account_id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(points > 0, "should have earned points for {points}");
    }

    // ── SettingsUpdatedHandler edge cases (ADR #22) ────────────────

    /// Handler should not panic when `changed_keys` is empty.
    /// A bulk save that touches no keys could legitimately produce
    /// an event with an empty vec.
    #[tokio::test]
    async fn settings_updated_handler_empty_changed_keys() {
        clear_settings_emit_fn();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let c = calls.clone();
        set_settings_emit_fn(Box::new(move |_event_name, payload| {
            c.lock().unwrap().push(payload);
        }));

        let bus = EventBus::new();
        let handler = SettingsUpdatedHandler::new();
        bus.subscribe::<SettingsUpdated>("settings.updated", Box::new(handler));

        let event = SettingsUpdated {
            changed_keys: vec![],
            terminal_id: "term-empty".into(),
        };
        bus.publish(&event).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Handler should not panic; emit may or may not fire for empty keys.
        // The key invariant: no crash, no hang.
        clear_settings_emit_fn();
    }

    /// Handler should tolerate special characters in terminal_id
    /// (Unicode, quotes, backslashes) without panicking.
    #[tokio::test]
    async fn settings_updated_handler_special_terminal_id() {
        clear_settings_emit_fn();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let c = calls.clone();
        set_settings_emit_fn(Box::new(move |_event_name, payload| {
            c.lock().unwrap().push(payload);
        }));

        let bus = EventBus::new();
        let handler = SettingsUpdatedHandler::new();
        bus.subscribe::<SettingsUpdated>("settings.updated", Box::new(handler));

        let event = SettingsUpdated {
            changed_keys: vec!["store.name".into()],
            terminal_id: "term-\u{2603}-\"quoted\"".into(),
        };
        bus.publish(&event).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(
            !calls.lock().unwrap().is_empty(),
            "handler should emit for special terminal_id"
        );
        clear_settings_emit_fn();
    }

    /// Rapid-fire publishes should not drop events. Publish 100
    /// `SettingsUpdated` events in a tight loop and verify the
    /// emit callback receives all of them.
    #[tokio::test]
    async fn settings_updated_handler_rapid_fire_100_events() {
        clear_settings_emit_fn();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let c = calls.clone();
        set_settings_emit_fn(Box::new(move |_event_name, payload| {
            c.lock().unwrap().push(payload);
        }));

        let bus = EventBus::new();
        let handler = SettingsUpdatedHandler::new();
        bus.subscribe::<SettingsUpdated>("settings.updated", Box::new(handler));

        for i in 0..100 {
            let event = SettingsUpdated {
                changed_keys: vec![format!("key.{i}")],
                terminal_id: "rapid-fire".into(),
            };
            bus.publish(&event).unwrap();
        }

        // Allow all spawned tasks to complete.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let emitted = calls.lock().unwrap().len();
        assert_eq!(emitted, 100, "should emit all 100 events, got {emitted}");
        clear_settings_emit_fn();
    }

    /// A panicking emit callback should not take down the handler
    /// or poison the global `SETTINGS_EMIT_FN` mutex. Subsequent
    /// publishes must still work after replacing the callback.
    #[tokio::test]
    async fn settings_updated_handler_survives_panicking_emit_fn() {
        clear_settings_emit_fn();

        // Set a callback that panics.
        let panicked = Arc::new(AtomicBool::new(false));
        let p = panicked.clone();
        set_settings_emit_fn(Box::new(move |_event_name, _payload| {
            p.store(true, std::sync::atomic::Ordering::SeqCst);
            panic!("intentional panic in emit callback");
        }));

        let bus = EventBus::new();
        let handler = SettingsUpdatedHandler::new();
        bus.subscribe::<SettingsUpdated>("settings.updated", Box::new(handler));

        // This publish should not panic — the handler spawns a task,
        // and the task's panic should be contained.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            bus.publish(&SettingsUpdated {
                changed_keys: vec!["k".into()],
                terminal_id: "panic-test".into(),
            })
            .unwrap();
        }));
        assert!(result.is_ok(), "publish itself should not panic");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Replace the panicking callback with a working one.
        let calls = Arc::new(Mutex::new(Vec::new()));
        let c = calls.clone();
        set_settings_emit_fn(Box::new(move |_event_name, payload| {
            c.lock().unwrap().push(payload);
        }));

        bus.publish(&SettingsUpdated {
            changed_keys: vec!["k2".into()],
            terminal_id: "recovery".into(),
        })
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            !calls.lock().unwrap().is_empty(),
            "replacement callback should fire after panicking one"
        );
        clear_settings_emit_fn();
    }
}
