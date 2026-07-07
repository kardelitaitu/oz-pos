//! Event handlers for the Inventory module.
//!
//! These handlers respond to domain events published on the kernel
//! event bus. Each handler holds a reference to the shared database
//! connection so it can perform side effects atomically.

use std::sync::{Arc, Mutex};

use foundation::contracts::{EventHandler, ModuleResult};
use oz_core::db::Store;
use oz_core::events::SaleCompleted;
use rusqlite::Connection;
use tracing::{error, info};

/// Handler that decrements stock when a sale is completed.
///
/// For each line item in the completed sale, this handler calls
/// `Store::adjust_stock(sku, -qty)` to reduce the inventory level.
///
/// If a product is not found by SKU, the handler logs a warning
/// and continues (the product may be a non-inventory item).
#[derive(Debug)]
pub struct InventoryStockHandler {
    db: Arc<Mutex<Connection>>,
}

impl InventoryStockHandler {
    /// Create a new handler with a shared database connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

impl EventHandler<SaleCompleted> for InventoryStockHandler {
    fn handle(&self, event: &SaleCompleted) -> ModuleResult {
        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("inventory handler: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        for line in &event.line_items {
            match store.adjust_stock(&line.sku, -line.qty) {
                Ok(new_qty) => {
                    info!(
                        sku = %line.sku,
                        qty = %(-line.qty),
                        new_qty,
                        "inventory handler: stock decremented for sale"
                    );
                }
                Err(e) => {
                    // If the product doesn't exist, it may be a non-inventory item.
                    // Log and continue rather than failing the entire event.
                    error!(
                        sku = %line.sku,
                        error = %e,
                        "inventory handler: failed to decrement stock"
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::migrations;
    use platform_kernel::EventBus;

    fn fresh_db() -> Arc<Mutex<Connection>> {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        Arc::new(Mutex::new(conn))
    }

    fn seed_product(db: &Connection) {
        db.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
             VALUES ('p1', 'COFFEE', 'Coffee', 350, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty, updated_at) VALUES ('p1', 10, '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();
    }

    #[test]
    fn handler_decrements_stock() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_product(&conn);
        }

        let handler = InventoryStockHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-1".into(),
            line_items: vec![oz_core::events::SaleCompletedLine {
                sku: "COFFEE".into(),
                qty: 3,
                unit_price_minor: 350,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 1050,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        // Verify stock was decremented.
        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let product_id = store.product_id_by_sku("COFFEE").unwrap().unwrap();
        let qty = store.get_stock(&product_id).unwrap();
        assert_eq!(qty, 7);
    }

    #[test]
    fn handler_works_with_event_bus() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_product(&conn);
        }

        let bus = EventBus::new();
        let handler = InventoryStockHandler::new(db.clone());
        bus.subscribe("sale.completed", Box::new(handler));

        let event = SaleCompleted {
            sale_id: "sale-2".into(),
            line_items: vec![oz_core::events::SaleCompletedLine {
                sku: "COFFEE".into(),
                qty: 1,
                unit_price_minor: 350,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 350,
            currency: "USD".into(),
            customer_id: None,
        };

        bus.publish(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let product_id = store.product_id_by_sku("COFFEE").unwrap().unwrap();
        let qty = store.get_stock(&product_id).unwrap();
        assert_eq!(qty, 9);
    }

    #[test]
    fn handler_unknown_sku_does_not_crash() {
        let db = fresh_db();
        let handler = InventoryStockHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-3".into(),
            line_items: vec![oz_core::events::SaleCompletedLine {
                sku: "UNKNOWN".into(),
                qty: 1,
                unit_price_minor: 100,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 100,
            currency: "USD".into(),
            customer_id: None,
        };

        // Should not panic or error.
        let result = handler.handle(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn handler_multiple_line_items() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            // Add a second product.
            conn.execute_batch(
                "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at)
                 VALUES ('p2', 'TEA', 'Tea', 250, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
                 INSERT INTO inventory (product_id, qty, updated_at) VALUES ('p2', 15, '2025-01-01T00:00:00.000Z');",
            )
            .unwrap();
            seed_product(&conn);
        }

        let handler = InventoryStockHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-4".into(),
            line_items: vec![
                oz_core::events::SaleCompletedLine {
                    sku: "COFFEE".into(),
                    qty: 2,
                    unit_price_minor: 350,
                    tax_minor: 0,
                    tax_rate_id: None,
                },
                oz_core::events::SaleCompletedLine {
                    sku: "TEA".into(),
                    qty: 5,
                    unit_price_minor: 250,
                    tax_minor: 0,
                    tax_rate_id: None,
                },
            ],
            total_minor: 1950,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let coffee_id = store.product_id_by_sku("COFFEE").unwrap().unwrap();
        let tea_id = store.product_id_by_sku("TEA").unwrap().unwrap();
        assert_eq!(store.get_stock(&coffee_id).unwrap(), 8); // 10 - 2
        assert_eq!(store.get_stock(&tea_id).unwrap(), 10); // 15 - 5
    }
}
