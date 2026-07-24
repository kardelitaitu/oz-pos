//! CRM Repository — database persistence for customer profiles.

use crate::models::Customer;
use foundation::{Email, Phone};
use rusqlite::{Connection, Transaction, params};

/// Database repository for customer records.
pub struct CrmRepository<'a> {
    conn: &'a Connection,
}

impl<'a> CrmRepository<'a> {
    /// Create a new `CrmRepository`.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Retrieve a customer by ID.
    pub fn get_customer(&self, id: &str) -> Result<Option<Customer>, anyhow::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, email, phone, loyalty_points, total_spent_minor, currency, notes, created_at, updated_at
             FROM customers WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        let row = match rows.next()? {
            Some(r) => r,
            None => return Ok(None),
        };

        let email_str: Option<String> = row.get(2)?;
        let email = email_str.and_then(|e| Email::new(e).ok());

        let phone_str: Option<String> = row.get(3)?;
        let phone = phone_str.and_then(|p| Phone::new(p).ok());

        Ok(Some(Customer {
            id: row.get(0)?,
            name: row.get(1)?,
            email,
            phone,
            loyalty_points: row.get(4)?,
            total_spent_minor: row.get(5)?,
            currency: row.get(6)?,
            notes: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        }))
    }

    /// Insert a customer inside a transaction.
    pub fn create_customer_tx(
        &self,
        tx: &Transaction,
        customer: &Customer,
    ) -> Result<(), anyhow::Error> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        tx.execute(
            "INSERT INTO customers (id, name, email, phone, loyalty_points, total_spent_minor, currency, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                customer.id,
                customer.name,
                customer.email.as_ref().map(|e| e.as_str()),
                customer.phone.as_ref().map(|p| p.as_str()),
                customer.loyalty_points,
                customer.total_spent_minor,
                customer.currency,
                customer.notes,
                if customer.created_at.is_empty() { &now } else { &customer.created_at },
                if customer.updated_at.is_empty() { &now } else { &customer.updated_at },
            ],
        )?;
        Ok(())
    }
}
