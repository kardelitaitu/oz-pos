use rusqlite::params;

use crate::Table;
use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    fn row_to_table(row: &rusqlite::Row) -> rusqlite::Result<Table> {
        let active_int: i64 = row.get("active")?;
        Ok(Table {
            id: row.get("id")?,
            name: row.get("name")?,
            capacity: row.get("capacity")?,
            pos_x: row.get("pos_x")?,
            pos_y: row.get("pos_y")?,
            shape: row.get("shape")?,
            width: row.get("width")?,
            height: row.get("height")?,
            status: row.get("status")?,
            active_sale_id: row.get("active_sale_id")?,
            section: row.get("section")?,
            active: active_int != 0,
            sort_order: row.get("sort_order")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// List all active tables, optionally filtered by section.
    pub fn list_tables(&self, section: Option<&str>) -> Result<Vec<Table>, CoreError> {
        let mut stmt = match section {
            Some(_) => self.conn.prepare(
                "SELECT * FROM tables WHERE active = 1 AND section = ?1 ORDER BY sort_order, name",
            )?,
            None => self
                .conn
                .prepare("SELECT * FROM tables WHERE active = 1 ORDER BY sort_order, name")?,
        };
        let rows = if section.is_some() {
            stmt.query_map(params![section], Self::row_to_table)?
        } else {
            stmt.query_map([], Self::row_to_table)?
        };
        rows.map(|r| Ok(r?)).collect()
    }

    /// Look up a single table by id.
    pub fn get_table(&self, id: &str) -> Result<Option<Table>, CoreError> {
        let mut stmt = self.conn.prepare("SELECT * FROM tables WHERE id = ?1")?;
        let result = stmt.query_row(params![id], Self::row_to_table);
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert a new table; assigns a UUID if `table.id` is empty.
    pub fn create_table(&self, table: &Table) -> Result<Table, CoreError> {
        let active_int: i64 = if table.active { 1 } else { 0 };
        let id = if table.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            table.id.clone()
        };
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.conn.execute(
            "INSERT INTO tables (id, name, capacity, pos_x, pos_y, shape, width, height, status, active_sale_id, section, active, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                id, table.name, table.capacity,
                table.pos_x, table.pos_y, table.shape,
                table.width, table.height, table.status,
                table.active_sale_id, table.section, active_int,
                table.sort_order, now, now,
            ],
        )?;
        self.get_table(&id)?.ok_or_else(|| CoreError::NotFound {
            entity: "table",
            id: id.to_owned(),
        })
    }

    /// Update all fields of an existing table.
    pub fn update_table(&self, table: &Table) -> Result<Table, CoreError> {
        let active_int: i64 = if table.active { 1 } else { 0 };
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE tables SET name = ?1, capacity = ?2, pos_x = ?3, pos_y = ?4,
             shape = ?5, width = ?6, height = ?7, status = ?8,
             active_sale_id = ?9, section = ?10, active = ?11, sort_order = ?12,
             updated_at = ?13 WHERE id = ?14",
            params![
                table.name,
                table.capacity,
                table.pos_x,
                table.pos_y,
                table.shape,
                table.width,
                table.height,
                table.status,
                table.active_sale_id,
                table.section,
                active_int,
                table.sort_order,
                now,
                table.id,
            ],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "table",
                id: table.id.clone(),
            });
        }
        self.get_table(&table.id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "table",
                id: table.id.clone(),
            })
    }

    /// Hard-delete a table by id.
    pub fn delete_table(&self, id: &str) -> Result<(), CoreError> {
        let rows = self
            .conn
            .execute("DELETE FROM tables WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "table",
                id: id.to_owned(),
            });
        }
        Ok(())
    }

    /// Update just the status field (availabe / occupied / reserved / cleaning).
    pub fn update_table_status(&self, id: &str, status: &str) -> Result<Table, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE tables SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "table",
                id: id.to_owned(),
            });
        }
        self.get_table(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "table",
            id: id.to_owned(),
        })
    }

    /// Set table status to `occupied` and link it to an active sale.
    pub fn assign_table_order(&self, table_id: &str, sale_id: &str) -> Result<Table, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE tables SET status = 'occupied', active_sale_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![sale_id, now, table_id],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "table",
                id: table_id.to_owned(),
            });
        }
        self.get_table(table_id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "table",
                id: table_id.to_owned(),
            })
    }

    /// Release an occupied table: set status to cleaning, clear the sale link.
    pub fn release_table(&self, table_id: &str) -> Result<Table, CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE tables SET status = 'cleaning', active_sale_id = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, table_id],
        )?;
        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "table",
                id: table_id.to_owned(),
            });
        }
        self.get_table(table_id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "table",
                id: table_id.to_owned(),
            })
    }

    /// Return distinct non-empty section names from active tables.
    pub fn list_sections(&self) -> Result<Vec<String>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT section FROM tables WHERE active = 1 AND section != '' ORDER BY section",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|r| Ok(r?)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations::run(&mut conn).unwrap();
        conn
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn dummy_table(id: &str) -> Table {
        Table {
            id: id.into(),
            name: format!("Table {id}"),
            capacity: 4,
            pos_x: 10.0,
            pos_y: 20.0,
            shape: "circle".into(),
            width: 10.0,
            height: 10.0,
            status: "available".into(),
            active_sale_id: None,
            section: "Main".into(),
            active: true,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn create_and_get_table() {
        let conn = fresh();
        let s = store(&conn);
        let t = s.create_table(&dummy_table("t1")).unwrap();
        assert_eq!(t.name, "Table t1");
        assert_eq!(t.status, "available");
        let fetched = s.get_table("t1").unwrap().unwrap();
        assert_eq!(fetched.id, "t1");
    }

    #[test]
    fn list_tables_empty() {
        let conn = fresh();
        let s = store(&conn);
        assert!(s.list_tables(None).unwrap().is_empty());
    }

    #[test]
    fn list_tables_filters_section() {
        let conn = fresh();
        let s = store(&conn);
        let mut a = dummy_table("a");
        a.section = "Patio".into();
        let mut b = dummy_table("b");
        b.section = "Main".into();
        s.create_table(&a).unwrap();
        s.create_table(&b).unwrap();
        let patio = s.list_tables(Some("Patio")).unwrap();
        assert_eq!(patio.len(), 1);
        assert_eq!(patio[0].id, "a");
    }

    #[test]
    fn update_table_mutates() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.name = "Original".into();
        s.create_table(&t).unwrap();
        t.name = "Updated".into();
        let updated = s.update_table(&t).unwrap();
        assert_eq!(updated.name, "Updated");
    }

    #[test]
    fn delete_table_removes() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        s.delete_table("t1").unwrap();
        assert!(s.get_table("t1").unwrap().is_none());
    }

    #[test]
    fn delete_table_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.delete_table("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn update_table_status_works() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        let t = s.update_table_status("t1", "occupied").unwrap();
        assert_eq!(t.status, "occupied");
    }

    #[test]
    fn assign_and_release() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        // Create a sale to link.
        let cart = crate::Cart::new("USD".parse().unwrap());
        let sale = crate::Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        let t = s.assign_table_order("t1", &sale.id).unwrap();
        assert_eq!(t.status, "occupied");
        assert_eq!(t.active_sale_id, Some(sale.id.clone()));
        let released = s.release_table("t1").unwrap();
        assert_eq!(released.status, "cleaning");
        assert!(released.active_sale_id.is_none());
    }

    #[test]
    fn list_sections_returns_distinct() {
        let conn = fresh();
        let s = store(&conn);
        let mut a = dummy_table("a");
        a.section = "Patio".into();
        let mut b = dummy_table("b");
        b.section = "Patio".into();
        let mut c = dummy_table("c");
        c.section = "Bar".into();
        s.create_table(&a).unwrap();
        s.create_table(&b).unwrap();
        s.create_table(&c).unwrap();
        let sections = s.list_sections().unwrap();
        assert_eq!(sections.len(), 2);
        assert!(sections.contains(&"Bar".to_string()));
        assert!(sections.contains(&"Patio".to_string()));
    }

    #[test]
    fn get_table_not_found() {
        let conn = fresh();
        let s = store(&conn);
        assert!(s.get_table("nope").unwrap().is_none());
    }
}
