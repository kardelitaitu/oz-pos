//! Shared application-level event handlers.
//!
//! These handlers are cross-cutting concerns that don't belong to a
//! single business module. They are registered on the kernel's event
//! bus by [`crate::init_module_system`].

use std::sync::{Arc, Mutex};

use foundation::contracts::{EventHandler, ModuleResult};
use oz_core::audit::AuditEntry;
use oz_core::db::Store;
use oz_core::events::{ProductCreated, SaleCompleted, StockAdjusted};
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

        store
            .enqueue_offline("complete_sale", &payload)
            .map_err(|e| {
                error!(
                    sale_id = %event.sale_id,
                    error = %e,
                    "sync enqueuer: failed to enqueue completed sale"
                );
                anyhow::anyhow!("sync enqueuer: enqueue_offline failed: {e}")
            })?;

        info!(
            sale_id = %event.sale_id,
            "sync enqueuer: sale queued for sync"
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
            line_items: vec![],
            total_minor: 100,
            currency: "USD".into(),
            customer_id: None,
        };
        let event2 = SaleCompleted {
            sale_id: "sale-b".into(),
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
            line_items: vec![],
            total_minor: 100,
            currency: "USD".into(),
            customer_id: None,
        };
        let event2 = SaleCompleted {
            sale_id: "sale-queue-2".into(),
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
}
