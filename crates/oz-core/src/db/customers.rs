//! Customer CRUD — list, get, create, update, delete.

use rusqlite::params;

use foundation::{Email, Phone};

use crate::Customer;
use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// List all customers, ordered by name.
    pub fn list_customers(&self) -> Result<Vec<Customer>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, email, phone, loyalty_points, total_spent_minor, currency,
                    notes, created_at, updated_at
             FROM customers ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            let email_raw: Option<String> = row.get("email")?;
            let phone_raw: Option<String> = row.get("phone")?;
            Ok(Customer {
                id: row.get("id")?,
                name: row.get("name")?,
                email: email_raw.and_then(|s| Email::new(&s).ok()),
                phone: phone_raw.and_then(|s| Phone::new(&s).ok()),
                loyalty_points: row.get("loyalty_points")?,
                total_spent_minor: row.get("total_spent_minor")?,
                currency: row.get("currency")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single customer by id.
    pub fn get_customer(&self, id: &str) -> Result<Option<Customer>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, email, phone, loyalty_points, total_spent_minor, currency,
                    notes, created_at, updated_at
             FROM customers WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            let email_raw: Option<String> = row.get("email")?;
            let phone_raw: Option<String> = row.get("phone")?;
            Ok(Customer {
                id: row.get("id")?,
                name: row.get("name")?,
                email: email_raw.and_then(|s| Email::new(&s).ok()),
                phone: phone_raw.and_then(|s| Phone::new(&s).ok()),
                loyalty_points: row.get("loyalty_points")?,
                total_spent_minor: row.get("total_spent_minor")?,
                currency: row.get("currency")?,
                notes: row.get("notes")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new customer.
    pub fn create_customer(
        &self,
        name: &str,
        email: Option<&str>,
        phone: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Customer, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "customer name must not be empty".into(),
            });
        }

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.conn.execute(
            "INSERT INTO customers (id, name, email, phone, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                name.trim(),
                email,
                phone,
                notes.unwrap_or_default(),
                now,
                now
            ],
        )?;

        Ok(Customer {
            id,
            name: name.trim().to_owned(),
            email: email.and_then(|s| Email::new(s).ok()),
            phone: phone.and_then(|s| Phone::new(s).ok()),
            loyalty_points: 0,
            total_spent_minor: 0,
            currency: "USD".into(),
            notes: notes.unwrap_or_default().to_owned(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing customer.
    pub fn update_customer(
        &self,
        id: &str,
        name: &str,
        email: Option<&str>,
        phone: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Customer, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "customer name must not be empty".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE customers SET name = ?1, email = ?2, phone = ?3, notes = ?4, updated_at = ?5 WHERE id = ?6",
            params![name.trim(), email, phone, notes.unwrap_or_default(), now, id],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "customer",
                id: id.to_owned(),
            });
        }

        self.get_customer(id)?.ok_or(CoreError::NotFound {
            entity: "customer",
            id: id.to_owned(),
        })
    }

    /// Delete a customer by id.
    pub fn delete_customer(&self, id: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM customers WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "customer",
                id: id.to_owned(),
            });
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn seed_customers(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO customers (id, name, email, phone, notes, created_at, updated_at) VALUES
                ('cust-1', 'Alice',  'alice@example.com',  '+1-555-0101', 'Regular',   '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('cust-2', 'Bob',    NULL,                 '+1-555-0102', '',          '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('cust-3', 'Carol',  'carol@example.com',  NULL,          'VIP',       '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    // ── List ────────────────────────────────────────────────────────

    #[test]
    fn list_customers_empty_db() {
        let conn = fresh();
        let customers = store(&conn).list_customers().unwrap();
        assert!(customers.is_empty());
    }

    #[test]
    fn list_customers_returns_all() {
        let conn = fresh();
        seed_customers(&conn);
        let customers = store(&conn).list_customers().unwrap();
        assert_eq!(customers.len(), 3);
        assert_eq!(customers[0].name, "Alice");
        assert_eq!(customers[1].name, "Bob");
        assert_eq!(customers[2].name, "Carol");
    }

    // ── Get ─────────────────────────────────────────────────────────

    #[test]
    fn get_customer_found() {
        let conn = fresh();
        seed_customers(&conn);
        let c = store(&conn).get_customer("cust-1").unwrap().unwrap();
        assert_eq!(c.name, "Alice");
        assert_eq!(
            c.email.as_ref().map(|e| e.as_str()),
            Some("alice@example.com")
        );
        assert_eq!(c.phone.as_ref().map(|p| p.as_str()), Some("+1-555-0101"));
        assert_eq!(c.notes, "Regular");
    }

    #[test]
    fn get_customer_not_found() {
        let conn = fresh();
        let c = store(&conn).get_customer("nope").unwrap();
        assert!(c.is_none());
    }

    #[test]
    fn get_customer_nullable_fields() {
        let conn = fresh();
        seed_customers(&conn);
        let c = store(&conn).get_customer("cust-2").unwrap().unwrap();
        assert_eq!(c.name, "Bob");
        assert!(c.email.is_none());
        assert_eq!(c.phone.as_ref().map(|p| p.as_str()), Some("+1-555-0102"));
    }

    // ── Create ──────────────────────────────────────────────────────

    #[test]
    fn create_customer_minimal() {
        let conn = fresh();
        let c = store(&conn)
            .create_customer("Diana", None, None, None)
            .unwrap();
        assert_eq!(c.name, "Diana");
        assert!(c.email.is_none());
        assert!(c.phone.is_none());
        assert_eq!(c.notes, "");
        assert!(!c.id.is_empty());
    }

    #[test]
    fn create_customer_with_all_fields() {
        let conn = fresh();
        let c = store(&conn)
            .create_customer(
                "Diana",
                Some("diana@test.com"),
                Some("555-0100"), // Phone needs digits; dashes alone won't parse
                Some("Preferred"),
            )
            .unwrap();
        assert_eq!(c.name, "Diana");
        assert_eq!(c.email.as_ref().map(|e| e.as_str()), Some("diana@test.com"));
        assert_eq!(c.phone.as_ref().map(|p| p.as_str()), Some("555-0100"));
        assert_eq!(c.notes, "Preferred");
        assert_eq!(c.loyalty_points, 0);
        assert_eq!(c.total_spent_minor, 0);
    }

    #[test]
    fn create_customer_empty_name() {
        let conn = fresh();
        let err = store(&conn)
            .create_customer("   ", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    // ── Update ──────────────────────────────────────────────────────

    #[test]
    fn update_customer_basic() {
        let conn = fresh();
        seed_customers(&conn);
        let updated = store(&conn)
            .update_customer(
                "cust-1",
                "Alice Updated",
                Some("alice@new.com"),
                None,
                Some("Changed"),
            )
            .unwrap();
        assert_eq!(updated.name, "Alice Updated");
        assert_eq!(
            updated.email.as_ref().map(|e| e.as_str()),
            Some("alice@new.com")
        );
        assert_eq!(updated.notes, "Changed");
        assert!(updated.updated_at.as_str() > "2025-01-01");
    }

    #[test]
    fn update_customer_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_customer("nope", "X", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_customer_empty_name() {
        let conn = fresh();
        seed_customers(&conn);
        let err = store(&conn)
            .update_customer("cust-1", "", None, None, None)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    // ── Delete ──────────────────────────────────────────────────────

    #[test]
    fn delete_customer_removes_row() {
        let conn = fresh();
        seed_customers(&conn);
        store(&conn).delete_customer("cust-1").unwrap();
        let c = store(&conn).get_customer("cust-1").unwrap();
        assert!(c.is_none());
    }

    #[test]
    fn delete_customer_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_customer("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }
}
