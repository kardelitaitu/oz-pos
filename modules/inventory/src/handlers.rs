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
/// For each line item in the completed sale, this handler first checks
/// if the product has a recipe (Bill of Materials) defined in the
/// `product_recipes` table. If it does, each ingredient's stock is
/// deducted by `qty × quantity_required` instead of deducting the
/// composite item itself. If no recipe exists, the handler falls back
/// to deducting the sold product's own stock directly.
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

    /// Deduct stock for a single line item, respecting BOM recipes.
    ///
    /// If the product has a recipe, deduct each ingredient by
    /// `qty_sold × quantity_required`. Otherwise, deduct the
    /// product itself.
    fn handle_line(&self, store: &Store<'_>, sku: &str, qty: i64) {
        // Look up the product ID by SKU.
        let product_id = match store.product_id_by_sku(sku) {
            Ok(Some(pid)) => pid,
            Ok(None) => {
                error!(sku, "inventory handler: product not found by SKU");
                return;
            }
            Err(e) => {
                error!(sku, error = %e, "inventory handler: failed to look up product");
                return;
            }
        };

        // Check if this product has a recipe (BOM ingredients).
        let ingredients = match store.get_recipe_ingredients(&product_id) {
            Ok(ings) => ings,
            Err(e) => {
                error!(
                    sku,
                    error = %e,
                    "inventory handler: failed to query recipe ingredients"
                );
                return;
            }
        };

        if ingredients.is_empty() {
            // Simple product — deduct directly.
            match store.adjust_stock(sku, -qty) {
                Ok(new_qty) => {
                    info!(
                        sku,
                        qty = -qty,
                        new_qty,
                        "inventory handler: stock decremented for simple product"
                    );
                }
                Err(e) => {
                    error!(
                        sku,
                        error = %e,
                        "inventory handler: failed to decrement stock"
                    );
                }
            }
        } else {
            // Composite product — deduct each ingredient by qty × quantity_required.
            for ingredient in &ingredients {
                let ingredient_sku = match store.product_sku_by_id(&ingredient.ingredient_product_id) {
                    Ok(Some(sku)) => sku,
                    Ok(None) => {
                        error!(
                            ingredient_id = %ingredient.ingredient_product_id,
                            "inventory handler: ingredient product not found by ID"
                        );
                        continue;
                    }
                    Err(e) => {
                        error!(
                            ingredient_id = %ingredient.ingredient_product_id,
                            error = %e,
                            "inventory handler: failed to look up ingredient SKU"
                        );
                        continue;
                    }
                };

                let deduct_qty = qty * ingredient.quantity_required;
                match store.adjust_stock(&ingredient_sku, -deduct_qty) {
                    Ok(new_qty) => {
                        info!(
                            sku = %ingredient_sku,
                            qty = -deduct_qty,
                            recipe_for = sku,
                            new_qty,
                            "inventory handler: BOM ingredient stock decremented"
                        );
                    }
                    Err(e) => {
                        error!(
                            sku = %ingredient_sku,
                            recipe_for = sku,
                            error = %e,
                            "inventory handler: failed to decrement BOM ingredient stock"
                        );
                    }
                }
            }
        }
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
            self.handle_line(&store, &line.sku, line.qty);
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

    // ── BOM / Recipe deduction tests ────────────────────────────

    fn seed_bom_products(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('burger', 'BURGER', 'Cheeseburger', 500, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('bun', 'BUN', 'Burger Bun', 100, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('patty', 'PATTY', 'Beef Patty', 200, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('cheese', 'CHEESE', 'Cheese Slice', 50, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO inventory (product_id, qty, updated_at) VALUES
                ('bun', 100, '2025-01-01T00:00:00.000Z'),
                ('patty', 50, '2025-01-01T00:00:00.000Z'),
                ('cheese', 200, '2025-01-01T00:00:00.000Z');
             INSERT INTO product_recipes (id, parent_product_id, ingredient_product_id, quantity_required, unit) VALUES
                ('r1', 'burger', 'bun', 1, 'pcs'),
                ('r2', 'burger', 'patty', 1, 'pcs'),
                ('r3', 'burger', 'cheese', 2, 'pcs');",
        )
        .unwrap();
    }

    #[test]
    fn handler_deducts_bom_ingredients_when_recipe_exists() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_bom_products(&conn);
        }

        let handler = InventoryStockHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-bom-1".into(),
            line_items: vec![oz_core::events::SaleCompletedLine {
                sku: "BURGER".into(),
                qty: 3,
                unit_price_minor: 500,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 1500,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);

        // 3 burgers should deduct: 3 buns, 3 patties, 6 cheese slices
        let bun_id = store.product_id_by_sku("BUN").unwrap().unwrap();
        let patty_id = store.product_id_by_sku("PATTY").unwrap().unwrap();
        let cheese_id = store.product_id_by_sku("CHEESE").unwrap().unwrap();

        assert_eq!(store.get_stock(&bun_id).unwrap(), 97);    // 100 - 3
        assert_eq!(store.get_stock(&patty_id).unwrap(), 47);  // 50 - 3
        assert_eq!(store.get_stock(&cheese_id).unwrap(), 194); // 200 - 6
    }

    #[test]
    fn handler_skips_bom_for_simple_products() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_bom_products(&conn);
        }

        let handler = InventoryStockHandler::new(db.clone());

        // Selling a simple product (BUN has no recipe) should deduct directly.
        let event = SaleCompleted {
            sale_id: "sale-simple-1".into(),
            line_items: vec![oz_core::events::SaleCompletedLine {
                sku: "BUN".into(),
                qty: 5,
                unit_price_minor: 100,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 500,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let bun_id = store.product_id_by_sku("BUN").unwrap().unwrap();
        assert_eq!(store.get_stock(&bun_id).unwrap(), 95); // 100 - 5
    }

    #[test]
    fn handler_mixed_bom_and_simple_products() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_bom_products(&conn);
        }

        let handler = InventoryStockHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-mixed-1".into(),
            line_items: vec![
                oz_core::events::SaleCompletedLine {
                    sku: "BURGER".into(),
                    qty: 2,
                    unit_price_minor: 500,
                    tax_minor: 0,
                    tax_rate_id: None,
                },
                oz_core::events::SaleCompletedLine {
                    sku: "BUN".into(),
                    qty: 10,
                    unit_price_minor: 100,
                    tax_minor: 0,
                    tax_rate_id: None,
                },
            ],
            total_minor: 2000,
            currency: "USD".into(),
            customer_id: None,
        };

        handler.handle(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let bun_id = store.product_id_by_sku("BUN").unwrap().unwrap();
        let patty_id = store.product_id_by_sku("PATTY").unwrap().unwrap();
        let cheese_id = store.product_id_by_sku("CHEESE").unwrap().unwrap();

        assert_eq!(store.get_stock(&bun_id).unwrap(),  88);  // 100 - 2 (BOM) - 10 (direct)
        assert_eq!(store.get_stock(&patty_id).unwrap(), 48);  // 50 - 2
        assert_eq!(store.get_stock(&cheese_id).unwrap(), 196); // 200 - 4
    }

    #[test]
    fn handler_deducts_bom_ingredients_via_event_bus() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_bom_products(&conn);
        }

        let bus = EventBus::new();
        let handler = InventoryStockHandler::new(db.clone());
        bus.subscribe("sale.completed", Box::new(handler));

        let event = SaleCompleted {
            sale_id: "sale-bus-1".into(),
            line_items: vec![oz_core::events::SaleCompletedLine {
                sku: "BURGER".into(),
                qty: 1,
                unit_price_minor: 500,
                tax_minor: 0,
                tax_rate_id: None,
            }],
            total_minor: 500,
            currency: "USD".into(),
            customer_id: None,
        };

        bus.publish(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let cheese_id = store.product_id_by_sku("CHEESE").unwrap().unwrap();
        assert_eq!(store.get_stock(&cheese_id).unwrap(), 198); // 200 - 2 (1 burger * 2 cheese)
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
