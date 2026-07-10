//! Supplier CRUD — list, get, create, update, delete.

use rusqlite::params;

use crate::Supplier;
use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// List all suppliers, ordered by name.
    pub fn list_suppliers(&self) -> Result<Vec<Supplier>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, code, name, contact_person, phone, email, address, tax_id,
                    payment_terms, notes, status, created_at, updated_at
             FROM suppliers ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Supplier {
                id: row.get("id")?,
                code: row.get("code")?,
                name: row.get("name")?,
                contact_person: row.get("contact_person")?,
                phone: row.get("phone")?,
                email: row.get("email")?,
                address: row.get("address")?,
                tax_id: row.get("tax_id")?,
                payment_terms: row.get("payment_terms")?,
                notes: row.get("notes")?,
                status: row.get("status")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single supplier by id.
    pub fn get_supplier(&self, id: &str) -> Result<Option<Supplier>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, code, name, contact_person, phone, email, address, tax_id,
                    payment_terms, notes, status, created_at, updated_at
             FROM suppliers WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(Supplier {
                id: row.get("id")?,
                code: row.get("code")?,
                name: row.get("name")?,
                contact_person: row.get("contact_person")?,
                phone: row.get("phone")?,
                email: row.get("email")?,
                address: row.get("address")?,
                tax_id: row.get("tax_id")?,
                payment_terms: row.get("payment_terms")?,
                notes: row.get("notes")?,
                status: row.get("status")?,
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

    /// Insert a new supplier.
    #[allow(clippy::too_many_arguments)]
    pub fn create_supplier(
        &self,
        code: &str,
        name: &str,
        contact_person: &str,
        phone: &str,
        email: &str,
        address: &str,
        tax_id: &str,
        payment_terms: &str,
        notes: &str,
    ) -> Result<Supplier, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "supplier name must not be empty".into(),
            });
        }
        if code.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "code",
                message: "supplier code must not be empty".into(),
            });
        }

        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.conn.execute(
            "INSERT INTO suppliers (id, code, name, contact_person, phone, email, address, tax_id,
                                    payment_terms, notes, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'active', ?11, ?12)",
            params![
                id,
                code.trim(),
                name.trim(),
                contact_person,
                phone,
                email,
                address,
                tax_id,
                payment_terms,
                notes,
                now,
                now
            ],
        )?;

        Ok(Supplier {
            id,
            code: code.trim().to_owned(),
            name: name.trim().to_owned(),
            contact_person: contact_person.to_owned(),
            phone: phone.to_owned(),
            email: email.to_owned(),
            address: address.to_owned(),
            tax_id: tax_id.to_owned(),
            payment_terms: payment_terms.to_owned(),
            notes: notes.to_owned(),
            status: "active".into(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Update an existing supplier.
    #[allow(clippy::too_many_arguments)]
    pub fn update_supplier(
        &self,
        id: &str,
        code: &str,
        name: &str,
        contact_person: &str,
        phone: &str,
        email: &str,
        address: &str,
        tax_id: &str,
        payment_terms: &str,
        notes: &str,
        status: &str,
    ) -> Result<Supplier, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "supplier name must not be empty".into(),
            });
        }
        if code.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "code",
                message: "supplier code must not be empty".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE suppliers SET code=?1, name=?2, contact_person=?3, phone=?4, email=?5,
                                  address=?6, tax_id=?7, payment_terms=?8, notes=?9,
                                  status=?10, updated_at=?11
             WHERE id=?12",
            params![
                code.trim(),
                name.trim(),
                contact_person,
                phone,
                email,
                address,
                tax_id,
                payment_terms,
                notes,
                status,
                now,
                id
            ],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "supplier",
                id: id.to_owned(),
            });
        }

        self.get_supplier(id)?.ok_or(CoreError::NotFound {
            entity: "supplier",
            id: id.to_owned(),
        })
    }

    /// Delete a supplier by id.
    pub fn delete_supplier(&self, id: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM suppliers WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "supplier",
                id: id.to_owned(),
            });
        }
        Ok(())
    }
}

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

    fn seed(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO suppliers (id, code, name, contact_person, phone, email, address, tax_id, payment_terms, notes, status, created_at, updated_at) VALUES
                ('sup-1', 'SUP001', 'Acme Corp',  'John Doe',  '+1-555-0101', 'john@acme.com',  '123 Main St', 'TAX-001', 'Net 30', 'Preferred', 'active', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('sup-2', 'SUP002', 'Beta Ltd',   'Jane Smith','+1-555-0102', 'jane@beta.com',  '456 Oak Ave', 'TAX-002', 'Net 15', '',         'active', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('sup-3', 'SUP003', 'Gamma Inc',  '',           '',            '',               '',           '',       '',       '',         'inactive', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();
    }

    #[test]
    fn list_empty() {
        let conn = fresh();
        let list = store(&conn).list_suppliers().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_all() {
        let conn = fresh();
        seed(&conn);
        let list = store(&conn).list_suppliers().unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn get_found() {
        let conn = fresh();
        seed(&conn);
        let s = store(&conn).get_supplier("sup-1").unwrap().unwrap();
        assert_eq!(s.name, "Acme Corp");
        assert_eq!(s.code, "SUP001");
    }

    #[test]
    fn get_not_found() {
        let conn = fresh();
        let s = store(&conn).get_supplier("nope").unwrap();
        assert!(s.is_none());
    }

    #[test]
    fn create_minimal() {
        let conn = fresh();
        let s = store(&conn)
            .create_supplier("SUP010", "New Co", "", "", "", "", "", "", "")
            .unwrap();
        assert_eq!(s.name, "New Co");
        assert_eq!(s.code, "SUP010");
    }

    #[test]
    fn create_empty_name() {
        let conn = fresh();
        let err = store(&conn)
            .create_supplier("SUP010", "", "", "", "", "", "", "", "")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field: "name", .. }));
    }

    #[test]
    fn update_basic() {
        let conn = fresh();
        seed(&conn);
        let updated = store(&conn)
            .update_supplier(
                "sup-1",
                "SUP001",
                "Acme Updated",
                "John",
                "",
                "",
                "",
                "",
                "",
                "",
                "active",
            )
            .unwrap();
        assert_eq!(updated.name, "Acme Updated");
    }

    #[test]
    fn update_not_found() {
        let conn = fresh();
        let err = store(&conn)
            .update_supplier("nope", "C", "X", "", "", "", "", "", "", "", "active")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn delete_removes() {
        let conn = fresh();
        seed(&conn);
        store(&conn).delete_supplier("sup-1").unwrap();
        assert!(store(&conn).get_supplier("sup-1").unwrap().is_none());
    }

    #[test]
    fn delete_not_found() {
        let conn = fresh();
        let err = store(&conn).delete_supplier("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }
}
