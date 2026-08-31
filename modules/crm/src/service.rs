/*
last audited 25-07-26 by RSA-Agent (modules-crm slice A: service verified)
crate: modules-crm | status: SAFE | lint: CLEAN
findings: clean thin service facade
next: none | perf: N/A
*/
//! CRM Service — customer workflows and relationship management.

use crate::error::CrmError;
use crate::models::Customer;
use crate::repository::CrmRepository;
use rusqlite::Connection;

/// Service encapsulating customer business logic.
pub struct CrmService;

impl CrmService {
    /// Retrieve customer by ID.
    pub fn get_customer(conn: &Connection, id: &str) -> Result<Option<Customer>, CrmError> {
        let repo = CrmRepository::new(conn);
        repo.get_customer(id)
    }

    /// Create and persist a new customer.
    pub fn create_customer(conn: &mut Connection, customer: &Customer) -> Result<(), CrmError> {
        let tx = conn.transaction()?;
        {
            let repo = CrmRepository::new(&tx);
            repo.create_customer_tx(&tx, customer)?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
