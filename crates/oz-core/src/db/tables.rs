use rusqlite::params;

use crate::Table;
use crate::error::CoreError;

use super::Store;

/// Floor-plan geometry bounds (TBL-08): positions and sizes are persisted as
/// percentages of the floor plan, so every value must be finite, within
/// `0..=100`, and large enough to remain a usable interactive control.
const GEOMETRY_BOUNDS: (f64, f64) = (0.0, 100.0);
/// Minimum table width/height as a percentage so a persisted table can never
/// collapse into an unusably tiny (or zero-sized) control.
const GEOMETRY_MIN_SIZE: f64 = 2.0;

/// Validate floor-plan geometry for create/update (TBL-08).
///
/// Rejects non-finite (`NaN`/`inf`), negative, out-of-bounds, and zero-sized
/// values at the database boundary so invalid persisted input can never place
/// tables outside the floor plan or produce overlapping/tiny controls.
fn validate_table_geometry(table: &Table) -> Result<(), CoreError> {
    let fields = [
        ("pos_x", table.pos_x),
        ("pos_y", table.pos_y),
        ("width", table.width),
        ("height", table.height),
    ];
    for (field, value) in fields {
        if !value.is_finite() {
            return Err(CoreError::Validation {
                field,
                message: "must be a finite number".into(),
            });
        }
        if !(GEOMETRY_BOUNDS.0..=GEOMETRY_BOUNDS.1).contains(&value) {
            return Err(CoreError::Validation {
                field,
                message: "must be between 0 and 100".into(),
            });
        }
    }
    if table.width < GEOMETRY_MIN_SIZE || table.height < GEOMETRY_MIN_SIZE {
        return Err(CoreError::Validation {
            field: "width",
            message: format!("tables must be at least {GEOMETRY_MIN_SIZE}% wide and tall"),
        });
    }
    Ok(())
}

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
        if table.name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "table name must not be empty".into(),
            });
        }
        if table.capacity < 0 {
            return Err(CoreError::Validation {
                field: "capacity",
                message: "capacity must not be negative".into(),
            });
        }
        // TBL-08: reject unusable persisted geometry at the boundary.
        validate_table_geometry(table)?;
        let active_int: i64 = if table.active { 1 } else { 0 };
        let id = if table.id.is_empty() {
            uuid::Uuid::now_v7().to_string()
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
        if table.name.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "name",
                message: "table name must not be empty".into(),
            });
        }
        if table.capacity < 0 {
            return Err(CoreError::Validation {
                field: "capacity",
                message: "capacity must not be negative".into(),
            });
        }
        // TBL-08: reject unusable persisted geometry at the boundary.
        validate_table_geometry(table)?;
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
    ///
    /// TBL-04: rejects deletion while the table is occupied, reserved, or
    /// linked to an active sale — removing an operationally-referenced table
    /// would orphan the `active_sale_id` link that KDS table-number lookup
    /// relies on and hide a live floor-plan seat. Prefer deactivation
    /// (`active = false`) for lifecycle management; a free, unlinked table is
    /// still hard-deletable.
    pub fn delete_table(&self, id: &str) -> Result<(), CoreError> {
        let current = self.get_table(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "table",
            id: id.to_owned(),
        })?;
        if current.status == "occupied"
            || current.status == "reserved"
            || current.active_sale_id.is_some()
        {
            return Err(CoreError::Validation {
                field: "status",
                message: "cannot delete a table that is occupied, reserved, or linked to an active sale — deactivate it instead".into(),
            });
        }
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

    /// Update just the status field (available / occupied / reserved / cleaning).
    ///
    /// TBL-01 invariant: the `occupied` status is reserved for tables linked
    /// to an active sale. A caller may only reach `occupied` through
    /// [`Store::assign_table_order`] (which sets `status` and `active_sale_id`
    /// together); marking a table occupied without an active sale is rejected
    /// so the floor plan can never show unassigned occupancy and KDS
    /// table-number lookup stays consistent.
    pub fn update_table_status(&self, id: &str, status: &str) -> Result<Table, CoreError> {
        if crate::TableStatus::from_str(status).is_none() {
            return Err(CoreError::Validation {
                field: "status",
                message: "invalid table status".into(),
            });
        }
        if status == "occupied" {
            match self.get_table(id)? {
                Some(t) if t.active_sale_id.is_some() => {}
                Some(_) => {
                    return Err(CoreError::Validation {
                        field: "status",
                        message: "occupied requires an active sale — use assign_table_order".into(),
                    });
                }
                None => {
                    return Err(CoreError::NotFound {
                        entity: "table",
                        id: id.to_owned(),
                    });
                }
            }
        }
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
        migrations::fresh_db()
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
        // Plain status transitions (not `occupied`) work without a sale link.
        let t = s.update_table_status("t1", "cleaning").unwrap();
        assert_eq!(t.status, "cleaning");
        let t = s.update_table_status("t1", "available").unwrap();
        assert_eq!(t.status, "available");
    }

    // ── TBL-01: occupied requires an active sale ──────────────────

    #[test]
    fn occupy_without_sale_rejected() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        let err = s.update_table_status("t1", "occupied").unwrap_err();
        assert!(
            matches!(err, CoreError::Validation { field, message } if field == "status" && message.contains("active sale"))
        );
        // The status must be untouched after the rejection.
        let t = s.get_table("t1").unwrap().unwrap();
        assert_eq!(t.status, "available");
    }

    #[test]
    fn occupy_with_active_sale_allowed() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        let cart = crate::Cart::new("USD".parse().unwrap());
        let sale = crate::Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        let t = s.assign_table_order("t1", &sale.id).unwrap();
        assert_eq!(t.status, "occupied");
        // Re-asserting occupied on an already-occupied table stays valid.
        let again = s.update_table_status("t1", "occupied").unwrap();
        assert_eq!(again.status, "occupied");
        assert_eq!(again.active_sale_id, Some(sale.id.clone()));
    }

    #[test]
    fn occupy_missing_table_returns_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.update_table_status("nope", "occupied").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    // ── TBL-04: delete lifecycle protection ───────────────────────

    #[test]
    fn delete_occupied_table_rejected() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        let cart = crate::Cart::new("USD".parse().unwrap());
        let sale = crate::Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        s.assign_table_order("t1", &sale.id).unwrap();
        let err = s.delete_table("t1").unwrap_err();
        assert!(
            matches!(err, CoreError::Validation { field, message } if field == "status" && message.contains("deactivate"))
        );
        assert!(s.get_table("t1").unwrap().is_some());
    }

    #[test]
    fn delete_reserved_table_rejected() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        s.update_table_status("t1", "reserved").unwrap();
        let err = s.delete_table("t1").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn delete_table_with_active_sale_link_rejected() {
        let conn = fresh();
        let s = store(&conn);
        // Build the state the production way: assign a real sale, then reset
        // the status to `available` — `update_table_status` changes only the
        // status and leaves the sale link intact, yielding "not occupied or
        // reserved, but still linked to an active sale". Constructing this
        // directly with a fake sale id would trip the FK constraint at insert.
        s.create_table(&dummy_table("t1")).unwrap();
        let cart = crate::Cart::new("USD".parse().unwrap());
        let sale = crate::Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();
        s.assign_table_order("t1", &sale.id).unwrap();
        let linked = s.update_table_status("t1", "available").unwrap();
        assert_eq!(linked.status, "available");
        assert_eq!(linked.active_sale_id, Some(sale.id.clone()));
        let err = s.delete_table("t1").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn delete_free_table_still_allowed() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        s.update_table_status("t1", "cleaning").unwrap();
        s.delete_table("t1").unwrap();
        assert!(s.get_table("t1").unwrap().is_none());
    }

    // ── TBL-08: geometry validation ───────────────────────────────

    #[test]
    fn create_table_nan_geometry_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.pos_x = f64::NAN;
        let err = s.create_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "pos_x"));
    }

    #[test]
    fn create_table_negative_geometry_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.pos_y = -5.0;
        let err = s.create_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "pos_y"));
    }

    #[test]
    fn create_table_out_of_bounds_geometry_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.width = 120.0;
        let err = s.create_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "width"));
    }

    #[test]
    fn create_table_tiny_geometry_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.height = 1.0;
        let err = s.create_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "width"));
    }

    #[test]
    fn create_table_zero_geometry_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.width = 0.0;
        let err = s.create_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { .. }));
    }

    #[test]
    fn update_table_geometry_validated() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        s.create_table(&t).unwrap();
        t.width = 150.0;
        let err = s.update_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "width"));
        t.width = 10.0;
        t.height = 0.0;
        let err = s.update_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { .. }));
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

    // ── Additional edge-case tests ─────────────────────────────────

    #[test]
    fn update_table_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.update_table(&dummy_table("nonexistent")).unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "table"));
    }

    #[test]
    fn update_table_status_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.update_table_status("nope", "occupied").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn assign_table_order_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.assign_table_order("nope", "sale-1").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn release_table_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.release_table("nope").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { .. }));
    }

    #[test]
    fn create_table_with_empty_id_generates_uuid() {
        let conn = fresh();
        let s = store(&conn);
        let t = s.create_table(&dummy_table("")).unwrap();
        assert!(!t.id.is_empty());
        assert_ne!(t.id, "");
    }

    #[test]
    fn list_tables_ordered_by_sort_order_name() {
        let conn = fresh();
        let s = store(&conn);
        let mut a = dummy_table("a");
        a.name = "Z Table".into();
        a.sort_order = 2;
        let mut b = dummy_table("b");
        b.name = "A Table".into();
        b.sort_order = 1;
        let mut c = dummy_table("c");
        c.name = "B Table".into();
        c.sort_order = 1;
        s.create_table(&a).unwrap();
        s.create_table(&b).unwrap();
        s.create_table(&c).unwrap();

        let tables = s.list_tables(None).unwrap();
        assert_eq!(tables.len(), 3);
        // ORDER BY sort_order, name: b (1,A), c (1,B), a (2,Z)
        assert_eq!(tables[0].id, "b");
        assert_eq!(tables[1].id, "c");
        assert_eq!(tables[2].id, "a");
    }

    #[test]
    fn list_tables_excludes_inactive() {
        let conn = fresh();
        let s = store(&conn);
        let mut active = dummy_table("active");
        active.active = true;
        let mut inactive = dummy_table("inactive");
        inactive.active = false;
        s.create_table(&active).unwrap();
        s.create_table(&inactive).unwrap();

        let tables = s.list_tables(None).unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].id, "active");
    }

    #[test]
    fn list_sections_excludes_empty() {
        let conn = fresh();
        let s = store(&conn);
        let mut a = dummy_table("a");
        a.section = "Main".into();
        let mut b = dummy_table("b");
        b.section = "".into();
        s.create_table(&a).unwrap();
        s.create_table(&b).unwrap();

        let sections = s.list_sections().unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0], "Main");
    }

    // ── Validation tests ────────────────────────────────────────

    #[test]
    fn create_table_empty_name_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.name = "".into();
        let err = s.create_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn create_table_whitespace_name_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.name = "   ".into();
        let err = s.create_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn create_table_negative_capacity_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        t.capacity = -1;
        let err = s.create_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "capacity"));
    }

    #[test]
    fn update_table_empty_name_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        s.create_table(&t).unwrap();
        t.name = "".into();
        let err = s.update_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "name"));
    }

    #[test]
    fn update_table_negative_capacity_rejected() {
        let conn = fresh();
        let s = store(&conn);
        let mut t = dummy_table("t1");
        s.create_table(&t).unwrap();
        t.capacity = -5;
        let err = s.update_table(&t).unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "capacity"));
    }

    #[test]
    fn update_table_status_invalid_rejected() {
        let conn = fresh();
        let s = store(&conn);
        s.create_table(&dummy_table("t1")).unwrap();
        let err = s.update_table_status("t1", "invalid_status").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }
}
