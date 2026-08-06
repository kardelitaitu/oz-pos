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

    /// List customers visible to one store (soft-scoping layer, migration
    /// 069/117), ordered by name.
    ///
    /// A store sees the shared global customer base (`store_id IS NULL`)
    /// plus its own tagged rows — never another store's rows. In the
    /// per-store database model every row is NULL, so this degenerates to
    /// the global list; it is the enforcement surface for shared/cloud
    /// databases where `store_id` is the soft-scoping column.
    pub fn list_customers_for_store(&self, store_id: &str) -> Result<Vec<Customer>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, email, phone, loyalty_points, total_spent_minor, currency,
                    notes, created_at, updated_at
             FROM customers
             WHERE store_id IS NULL OR store_id = ?1
             ORDER BY name",
        )?;
        let rows = stmt.query_map(params![store_id], |row| {
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

    /// Search customers by name, email, or phone with a bounded page.
    ///
    /// CUST-06: keeps the PII surface delivered to the renderer bounded —
    /// the query runs server-side with an explicit sort order and a caller-
    /// supplied page size (clamped to `[1, 100]`). Returns the matching
    /// rows plus the total match count for pagination.
    pub fn search_customers(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(Vec<Customer>, u64), CoreError> {
        let trimmed = query.trim();
        let bounded = limit.clamp(1, 100);
        // Escape the LIKE wildcards so user input with literal % or _ does
        // not broaden the match beyond intent (e.g. searching "50%" must not
        // match every row).
        let escaped = trimmed
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        // ESCAPE '\' so user input with literal % or _ does not broaden the
        // match beyond intent.
        let filter = "(name LIKE ?1 ESCAPE '\\' OR COALESCE(email, '') LIKE ?1 ESCAPE '\\' OR COALESCE(phone, '') LIKE ?1 ESCAPE '\\')";

        let total: u64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM customers WHERE {filter}"),
            params![pattern],
            |row| row.get(0),
        )?;

        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, name, email, phone, loyalty_points, total_spent_minor, currency,
                    notes, created_at, updated_at
             FROM customers WHERE {filter} ORDER BY name LIMIT ?2 OFFSET ?3"
        ))?;
        let rows = stmt.query_map(params![pattern, bounded, offset], |row| {
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
        let items = rows
            .map(|r| Ok(r?))
            .collect::<Result<Vec<_>, CoreError>>()?;
        Ok((items, total))
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

        let id = uuid::Uuid::now_v7().to_string();
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

    // ── Additional edge cases ─────────────────────────────────────

    #[test]
    fn list_customers_ordered_by_name() {
        let conn = fresh();
        // Seed out of alphabetical order.
        conn.execute_batch(
            "INSERT INTO customers (id, name, created_at, updated_at) VALUES
                ('c-z', 'Zara',  '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('c-a', 'Alpha', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('c-m', 'Mike',  '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');",
        )
        .unwrap();
        let customers = store(&conn).list_customers().unwrap();
        assert_eq!(customers.len(), 3);
        assert_eq!(customers[0].name, "Alpha");
        assert_eq!(customers[1].name, "Mike");
        assert_eq!(customers[2].name, "Zara");
    }

    #[test]
    fn update_customer_clear_email_and_phone() {
        let conn = fresh();
        seed_customers(&conn);
        // cust-1 had email and phone; update to clear them.
        let updated = store(&conn)
            .update_customer("cust-1", "Alice", None, None, Some("Cleared fields"))
            .unwrap();
        assert_eq!(updated.name, "Alice");
        assert!(updated.email.is_none(), "email should be cleared");
        assert!(updated.phone.is_none(), "phone should be cleared");
        assert_eq!(updated.notes, "Cleared fields");
    }

    #[test]
    fn create_customer_invalid_email_saved_as_none() {
        let conn = fresh();
        let c = store(&conn)
            .create_customer("Test", Some("not-an-email"), None, None)
            .unwrap();
        // Email::new("not-an-email") returns Err, so and_then returns None.
        assert!(c.email.is_none());
        assert_eq!(c.name, "Test");
    }

    // ── Search (CUST-06) ───────────────────────────────────────────

    #[test]
    fn search_customers_matches_name_email_and_phone() {
        let conn = fresh();
        seed_customers(&conn);

        let (by_name, total) = store(&conn).search_customers("Alice", 100, 0).unwrap();
        assert_eq!(total, 1);
        assert_eq!(by_name[0].id, "cust-1");

        let (by_email, _) = store(&conn)
            .search_customers("carol@example.com", 100, 0)
            .unwrap();
        assert_eq!(by_email.len(), 1);
        assert_eq!(by_email[0].id, "cust-3");

        let (by_phone, _) = store(&conn).search_customers("555-0102", 100, 0).unwrap();
        assert_eq!(by_phone.len(), 1);
        assert_eq!(by_phone[0].id, "cust-2");
    }

    #[test]
    fn search_customers_is_bounded_and_paginated() {
        let conn = fresh();
        for i in 0..5 {
            conn.execute(
                "INSERT INTO customers (id, name, created_at, updated_at)
                 VALUES (?1, ?2, '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
                params![format!("c-{i}"), format!("Person {i}")],
            )
            .unwrap();
        }

        let (page1, total) = store(&conn).search_customers("Person", 2, 0).unwrap();
        assert_eq!(total, 5);
        assert_eq!(page1.len(), 2);

        let (page3, _) = store(&conn).search_customers("Person", 2, 4).unwrap();
        assert_eq!(page3.len(), 1);

        let (oversized, _) = store(&conn).search_customers("Person", 10_000, 0).unwrap();
        assert!(oversized.len() <= 100, "limit must be clamped to 100");
    }

    #[test]
    fn search_customers_literal_wildcards_are_escaped() {
        let conn = fresh();
        seed_customers(&conn);
        conn.execute(
            "INSERT INTO customers (id, name, created_at, updated_at)
             VALUES ('c-pct', '100%', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z')",
            [],
        )
        .unwrap();

        // Escaped: a bare % matches only rows with a literal %, never all.
        let (items, total) = store(&conn).search_customers("%", 100, 0).unwrap();
        assert_eq!(total, 1, "a bare % must not broaden to every row");
        assert_eq!(items[0].id, "c-pct");

        // Same for the single-char wildcard _: no customer name contains a
        // literal underscore, so an escaped _ matches nothing (it must not
        // broaden to match every row).
        let (items, total) = store(&conn).search_customers("_", 100, 0).unwrap();
        assert_eq!(total, 0, "a bare _ must not broaden to every row");
        assert!(items.is_empty());

        let (items, _) = store(&conn).search_customers("100%", 100, 0).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn search_customers_empty_query_returns_all_bounded() {
        let conn = fresh();
        seed_customers(&conn);
        let (items, total) = store(&conn).search_customers("", 100, 0).unwrap();
        assert_eq!(total, 3);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn search_customers_no_match_returns_empty() {
        let conn = fresh();
        seed_customers(&conn);
        let (items, total) = store(&conn)
            .search_customers("zzz-no-such", 100, 0)
            .unwrap();
        assert!(items.is_empty());
        assert_eq!(total, 0);
    }
}
