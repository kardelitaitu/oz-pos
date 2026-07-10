//! Event handlers for the CRM module.
//!
//! These handlers respond to domain events published on the kernel
//! event bus. Each handler holds a reference to the shared database
//! connection so it can perform side effects atomically.

use std::sync::{Arc, Mutex};

use foundation::contracts::{EventHandler, ModuleResult};
use oz_core::db::Store;
use oz_core::events::SaleCompleted;
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

        let conn = self
            .db
            .lock()
            .map_err(|e| anyhow::anyhow!("crm handler: db lock failed: {e}"))?;
        let store = Store::new(&conn);

        // Fetch the current customer record.
        let mut customer = match store.get_customer(customer_id)? {
            Some(c) => c,
            None => {
                warn!(
                    customer_id = %customer_id,
                    sale_id = %event.sale_id,
                    "crm handler: customer not found"
                );
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

        // Persist the update. Use update_customer to save changes, but we need
        // to preserve the existing fields. update_customer only takes specific args,
        // so we do a direct SQL update for the spending/loyalty fields.
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute(
            "UPDATE customers SET total_spent_minor = ?1, loyalty_points = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![customer.total_spent_minor, customer.loyalty_points, now, customer_id],
        )?;

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
}
