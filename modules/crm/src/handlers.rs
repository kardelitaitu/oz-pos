//! Event handlers for the CRM module.
//!
//! These handlers respond to domain events published on the kernel
//! event bus. Each handler holds a reference to the shared database
//! connection so it can perform side effects atomically.

use std::sync::{Arc, Mutex};

use crate::repository::CrmRepository;
use foundation::contracts::{EventHandler, ModuleResult};
use foundation::events::SaleCompleted;
use rusqlite::Connection;
use tracing::{info, warn};

/// Handler that updates customer purchase history when a sale completes.
///
/// If the sale is linked to a customer (`customer_id` is `Some`),
/// this handler increments `total_spent_minor` and `loyalty_points`
/// on the customer record.
///
/// Currently awards 1 loyalty point per 100 minor units spent.
#[derive(Debug)]
pub struct CrmHistoryHandler {
    db: Arc<Mutex<Connection>>,
}

impl CrmHistoryHandler {
    /// Create a new handler with a shared database connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

impl EventHandler<SaleCompleted> for CrmHistoryHandler {
    fn handle(&self, event: &SaleCompleted) -> ModuleResult {
        let Some(ref customer_id) = event.customer_id else {
            // No customer linked to this sale — nothing to update.
            info!(
                sale_id = %event.sale_id,
                "crm handler: sale has no linked customer, skipping"
            );
            return Ok(());
        };

        let mut conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("crm handler: db lock failed: {e}"))?;

        // Wrap the entire read-modify-write in a transaction so concurrent
        // SaleCompleted events for the same customer cannot race on the
        // read of total_spent_minor / loyalty_points (lost-update prevention).
        let tx = conn.transaction()?;
        let repo = CrmRepository::new(&tx);

        // Fetch the current customer record.
        let mut customer = match repo.get_customer(customer_id)? {
            Some(c) => c,
            None => {
                warn!(
                    customer_id = %customer_id,
                    sale_id = %event.sale_id,
                    "crm handler: customer not found"
                );
                // Transaction will auto-rollback on drop; explicit rollback
                // is not needed since no writes occurred yet.
                return Ok(());
            }
        };

        // Update spending and loyalty points.
        customer.total_spent_minor = customer
            .total_spent_minor
            .checked_add(event.total_minor)
            .ok_or_else(|| {
                anyhow::anyhow!("total_spent_minor overflow for customer {customer_id}")
            })?;

        // Award 1 loyalty point per full 100 minor units spent.
        let points_earned = event.total_minor / 100;
        customer.loyalty_points = customer
            .loyalty_points
            .checked_add(points_earned)
            .ok_or_else(|| anyhow::anyhow!("loyalty_points overflow for customer {customer_id}"))?;

        // Persist the update inside the transaction.
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        tx.execute(
            "UPDATE customers SET total_spent_minor = ?1, loyalty_points = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![customer.total_spent_minor, customer.loyalty_points, now, customer_id],
        )?;
        tx.commit()?;

        info!(
            customer_id = %customer_id,
            total_spent_minor = customer.total_spent_minor,
            loyalty_points = customer.loyalty_points,
            "crm handler: customer history updated"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oz_core::db::Store;
    use oz_core::migrations;
    use platform_kernel::EventBus;

    fn fresh_db() -> Arc<Mutex<Connection>> {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        Arc::new(Mutex::new(conn))
    }

    fn seed_customer(db: &Connection, id: &str, name: &str) {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        db.execute(
            "INSERT INTO customers (id, name, email, notes, total_spent_minor, loyalty_points, currency, created_at, updated_at)
             VALUES (?1, ?2, NULL, '', 0, 0, 'USD', ?3, ?3)",
            rusqlite::params![id, name, now],
        )
        .unwrap();
    }

    #[test]
    fn handler_updates_customer_spending() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_customer(&conn, "cust-1", "Alice");
        }

        let handler = CrmHistoryHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-1".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 1500,
            currency: "USD".into(),
            customer_id: Some("cust-1".into()),
        };

        handler.handle(&event).unwrap();

        // Verify customer was updated.
        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let customer = store.get_customer("cust-1").unwrap().unwrap();
        assert_eq!(customer.total_spent_minor, 1500);
        // 1500 / 100 = 15 loyalty points
        assert_eq!(customer.loyalty_points, 15);
    }

    #[test]
    fn handler_skips_when_no_customer() {
        let db = fresh_db();

        let handler = CrmHistoryHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-2".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 500,
            currency: "USD".into(),
            customer_id: None,
        };

        let result = handler.handle(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn handler_works_with_event_bus() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_customer(&conn, "cust-2", "Bob");
        }

        let bus = EventBus::new();
        let handler = CrmHistoryHandler::new(db.clone());
        bus.subscribe("sale.completed", Box::new(handler));

        let event = SaleCompleted {
            sale_id: "sale-3".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 2000,
            currency: "USD".into(),
            customer_id: Some("cust-2".into()),
        };

        bus.publish(&event).unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let customer = store.get_customer("cust-2").unwrap().unwrap();
        assert_eq!(customer.total_spent_minor, 2000);
        assert_eq!(customer.loyalty_points, 20);
    }

    #[test]
    fn handler_skips_when_customer_not_found() {
        let db = fresh_db();
        let handler = CrmHistoryHandler::new(db.clone());

        let event = SaleCompleted {
            sale_id: "sale-4".into(),
            store_id: None,
            line_items: vec![],
            total_minor: 500,
            currency: "USD".into(),
            customer_id: Some("nonexistent".into()),
        };

        let result = handler.handle(&event);
        assert!(result.is_ok(), "should not crash for nonexistent customer");
    }

    #[test]
    fn handler_accumulates_multiple_sales() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_customer(&conn, "cust-3", "Charlie");
        }

        let handler = CrmHistoryHandler::new(db.clone());

        handler
            .handle(&SaleCompleted {
                sale_id: "sale-a".into(),
                store_id: None,
                line_items: vec![],
                total_minor: 1000,
                currency: "USD".into(),
                customer_id: Some("cust-3".into()),
            })
            .unwrap();

        handler
            .handle(&SaleCompleted {
                sale_id: "sale-b".into(),
                store_id: None,
                line_items: vec![],
                total_minor: 700,
                currency: "USD".into(),
                customer_id: Some("cust-3".into()),
            })
            .unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let customer = store.get_customer("cust-3").unwrap().unwrap();
        assert_eq!(customer.total_spent_minor, 1700);
        assert_eq!(customer.loyalty_points, 17);
    }

    /// Thread-safety regression test: verifies the handler is safe to call
    /// from multiple threads sharing the same `Arc<Mutex<Connection>>`.
    /// The Mutex serializes access (preventing lost updates in practice),
    /// and the internal transaction provides defense-in-depth for the
    /// read-modify-write of customer spending/loyalty fields.
    #[test]
    fn handler_is_thread_safe_and_accumulates_correctly() {
        let db = fresh_db();
        {
            let conn = db.lock().unwrap();
            seed_customer(&conn, "cust-concurrent", "Dana");
        }

        let db_a = db.clone();
        let db_b = db.clone();

        // Simulate two concurrent SaleCompleted events on separate threads.
        let t1 = std::thread::spawn(move || {
            let handler = CrmHistoryHandler::new(db_a);
            handler
                .handle(&SaleCompleted {
                    sale_id: "sale-concurrent-a".into(),
                    store_id: None,
                    line_items: vec![],
                    total_minor: 1000,
                    currency: "USD".into(),
                    customer_id: Some("cust-concurrent".into()),
                })
                .unwrap();
        });

        let t2 = std::thread::spawn(move || {
            let handler = CrmHistoryHandler::new(db_b);
            handler
                .handle(&SaleCompleted {
                    sale_id: "sale-concurrent-b".into(),
                    store_id: None,
                    line_items: vec![],
                    total_minor: 2000,
                    currency: "USD".into(),
                    customer_id: Some("cust-concurrent".into()),
                })
                .unwrap();
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let conn = db.lock().unwrap();
        let store = Store::new(&conn);
        let customer = store.get_customer("cust-concurrent").unwrap().unwrap();
        // Both sales should be accumulated: 1000 + 2000 = 3000
        assert_eq!(customer.total_spent_minor, 3000);
        // Loyalty: 1000/100 + 2000/100 = 10 + 20 = 30
        assert_eq!(customer.loyalty_points, 30);
    }
}
