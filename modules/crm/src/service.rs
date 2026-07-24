//! CRM Service — customer workflows and relationship management.

use crate::models::Customer;
use crate::repository::CrmRepository;
use rusqlite::Connection;

/// Service encapsulating customer business logic.
pub struct CrmService;

impl CrmService {
    /// Retrieve customer by ID.
    pub fn get_customer(conn: &Connection, id: &str) -> Result<Option<Customer>, anyhow::Error> {
        let repo = CrmRepository::new(conn);
        repo.get_customer(id)
    }

    /// Create and persist a new customer.
    pub fn create_customer(
        conn: &mut Connection,
        customer: &Customer,
    ) -> Result<(), anyhow::Error> {
        let tx = conn.transaction()?;
        {
            let repo = CrmRepository::new(&tx);
            repo.create_customer_tx(&tx, customer)?;
        }
        tx.commit()?;
        Ok(())
    }
}
