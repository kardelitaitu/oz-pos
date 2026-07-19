//! Inventory management DB methods — locations CRUD, shifts, transaction logs, thresholds.

use crate::error::CoreError;
use crate::{
    InventoryLocation, InventoryShift, InventoryTransaction, InventoryTransactionLine,
    StockThreshold, Store, WorkspaceInventoryLocation,
};
use rusqlite::params;

/// Input structure for adding lines to a transaction.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InventoryTransactionLineInput {
    /// Product SKU.
    pub sku: String,
    /// Product Name.
    pub product_name: String,
    /// Positive magnitude of change.
    pub qty: i64,
    /// Signed delta adjustment.
    pub delta: i64,
    /// Optional barcode value scanned.
    pub barcode_scanned: Option<String>,
}

impl Store<'_> {
    // ── Locations CRUD ──────────────────────────────────────────────────

    /// Create a new inventory location.
    pub fn create_inventory_location(
        &self,
        name: &str,
        location_type: &str,
        description: &str,
    ) -> Result<String, CoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Validate location type against allowed values
        match location_type {
            "store" | "warehouse" | "transit" | "damaged" | "virtual" => {}
            other => {
                return Err(CoreError::Validation {
                    field: "type",
                    message: format!("invalid location type: {}", other),
                });
            }
        }

        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO inventory_locations (id, name, type, description, is_active, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
            params![id, name, location_type, description, now],
        )?;
        tx.commit()?;

        Ok(id)
    }

    /// List all inventory locations (including inactive ones).
    pub fn list_inventory_locations(&self) -> Result<Vec<InventoryLocation>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, description, is_active, created_at, updated_at \
             FROM inventory_locations ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let active_int: i64 = row.get(4)?;
            Ok(InventoryLocation {
                id: row.get(0)?,
                name: row.get(1)?,
                location_type: row.get(2)?,
                description: row.get(3)?,
                is_active: active_int == 1,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut locs = Vec::new();
        for r in rows {
            locs.push(r?);
        }
        Ok(locs)
    }

    /// Update an existing inventory location's details.
    pub fn update_inventory_location(
        &self,
        id: &str,
        name: &str,
        location_type: &str,
        description: &str,
    ) -> Result<(), CoreError> {
        // Validate location type against allowed values
        match location_type {
            "store" | "warehouse" | "transit" | "damaged" | "virtual" => {}
            other => {
                return Err(CoreError::Validation {
                    field: "type",
                    message: format!("invalid location type: {}", other),
                });
            }
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let tx = self.conn.unchecked_transaction()?;
        let updated = tx.execute(
            "UPDATE inventory_locations SET name = ?1, type = ?2, description = ?3, updated_at = ?4 \
             WHERE id = ?5",
            params![name, location_type, description, now, id],
        )?;
        if updated == 0 {
            return Err(CoreError::NotFound {
                entity: "inventory_location",
                id: id.to_owned(),
            });
        }
        tx.commit()?;
        Ok(())
    }

    /// Deactivate an inventory location. Enforces constraints that the location
    /// must have zero stock and no pending in-flight transfers.
    pub fn deactivate_inventory_location(&self, id: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        // Constraint 1: Check that there is no positive stock in stock_summary for this location
        let stock_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM stock_summary WHERE location_id = ?1 AND qty > 0",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if stock_count > 0 {
            return Err(CoreError::Validation {
                field: "location",
                message: "cannot deactivate location with active stock".into(),
            });
        }

        // Constraint 2: Check that there are no in-flight (draft / pending / in_transit / received_partial) transfers involving this location
        let transfer_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM stock_transfers \
             WHERE (source_location_id = ?1 OR destination_location_id = ?1) \
             AND status IN ('draft', 'pending', 'in_transit', 'received_partial')",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if transfer_count > 0 {
            return Err(CoreError::Validation {
                field: "location",
                message: "cannot deactivate location with pending stock transfers".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        tx.execute(
            "UPDATE inventory_locations SET is_active = 0, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        tx.commit()?;
        Ok(())
    }

    // ── Workspace Locations ─────────────────────────────────────────────

    /// Set locations associated with a workspace instance, with priority and allowance settings.
    pub fn set_workspace_inventory_locations(
        &self,
        instance_id: &str,
        locations: &[WorkspaceInventoryLocation],
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        // Delete existing bindings
        tx.execute(
            "DELETE FROM workspace_inventory_locations WHERE instance_id = ?1",
            params![instance_id],
        )?;

        // Insert new bindings
        for loc in locations {
            let id = if loc.id.is_empty() {
                uuid::Uuid::now_v7().to_string()
            } else {
                loc.id.clone()
            };
            tx.execute(
                "INSERT INTO workspace_inventory_locations (id, instance_id, location_id, is_primary, allow_negative_stock, sort_order) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    id,
                    instance_id,
                    loc.location_id,
                    if loc.is_primary { 1 } else { 0 },
                    if loc.allow_negative_stock { 1 } else { 0 },
                    loc.sort_order
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Retrieve the locations associated with a workspace instance.
    pub fn get_workspace_inventory_locations(
        &self,
        instance_id: &str,
    ) -> Result<Vec<WorkspaceInventoryLocation>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, instance_id, location_id, is_primary, allow_negative_stock, sort_order \
             FROM workspace_inventory_locations WHERE instance_id = ?1 ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map(params![instance_id], |row| {
            let prim_int: i64 = row.get(3)?;
            let neg_int: i64 = row.get(4)?;
            Ok(WorkspaceInventoryLocation {
                id: row.get(0)?,
                instance_id: row.get(1)?,
                location_id: row.get(2)?,
                is_primary: prim_int == 1,
                allow_negative_stock: neg_int == 1,
                sort_order: row.get(5)?,
            })
        })?;

        let mut locs = Vec::new();
        for r in rows {
            locs.push(r?);
        }
        Ok(locs)
    }

    // ── Inventory Shifts ────────────────────────────────────────────────

    /// Start a new inventory shift for a user at a location.
    /// Checks that the user does not already have an open shift.
    pub fn start_inventory_shift(
        &self,
        user_id: &str,
        location_id: &str,
        terminal_id: Option<&str>,
        notes: &str,
    ) -> Result<InventoryShift, CoreError> {
        let tx = self.conn.unchecked_transaction()?;

        // Enforce that only one shift is open at a time for this user.
        let active_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM inventory_shifts WHERE user_id = ?1 AND status = 'active'",
                params![user_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if active_count > 0 {
            return Err(CoreError::Validation {
                field: "shift",
                message: "user already has an active inventory shift open".into(),
            });
        }

        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        tx.execute(
            "INSERT INTO inventory_shifts (id, user_id, location_id, terminal_id, started_at, ended_at, status, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'active', ?6)",
            params![id, user_id, location_id, terminal_id, now, notes],
        )?;

        tx.commit()?;

        Ok(InventoryShift {
            id,
            user_id: user_id.to_owned(),
            location_id: location_id.to_owned(),
            terminal_id: terminal_id.map(String::from),
            started_at: now,
            ended_at: None,
            status: "active".into(),
            notes: notes.to_owned(),
        })
    }

    /// Close an active inventory shift.
    pub fn end_inventory_shift(&self, shift_id: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let updated = tx.execute(
            "UPDATE inventory_shifts SET ended_at = ?1, status = 'ended', updated_at = ?2 \
             WHERE id = ?3 AND status = 'active'",
            params![now, now, shift_id],
        )?;

        if updated == 0 {
            return Err(CoreError::NotFound {
                entity: "active_inventory_shift",
                id: shift_id.to_owned(),
            });
        }

        tx.commit()?;
        Ok(())
    }

    /// Retrieve the currently active shift for a user, if any.
    pub fn get_active_inventory_shift(
        &self,
        user_id: &str,
    ) -> Result<Option<InventoryShift>, CoreError> {
        let res = self.conn.query_row(
            "SELECT id, user_id, location_id, terminal_id, started_at, ended_at, status, notes \
             FROM inventory_shifts WHERE user_id = ?1 AND status = 'active'",
            params![user_id],
            |row| {
                Ok(InventoryShift {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    location_id: row.get(2)?,
                    terminal_id: row.get(3)?,
                    started_at: row.get(4)?,
                    ended_at: row.get(5)?,
                    status: row.get(6)?,
                    notes: row.get(7)?,
                })
            },
        );

        match res {
            Ok(shift) => Ok(Some(shift)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Db(e)),
        }
    }

    /// List all inventory shifts, ordered by started_at descending.
    pub fn list_inventory_shifts(&self) -> Result<Vec<InventoryShift>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, location_id, terminal_id, started_at, ended_at, status, notes \
             FROM inventory_shifts ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(InventoryShift {
                id: row.get(0)?,
                user_id: row.get(1)?,
                location_id: row.get(2)?,
                terminal_id: row.get(3)?,
                started_at: row.get(4)?,
                ended_at: row.get(5)?,
                status: row.get(6)?,
                notes: row.get(7)?,
            })
        })?;

        let mut shifts = Vec::new();
        for r in rows {
            shifts.push(r?);
        }
        Ok(shifts)
    }

    // ── Inventory Transactions ──────────────────────────────────────────

    /// Create a new inventory transaction audit log session and execute adjustments.
    pub fn create_inventory_transaction(
        &self,
        transaction_type: crate::inventory_transaction::InventoryTransactionType,
        location_id: &str,
        staff_id: &str,
        notes: &str,
        lines: &[InventoryTransactionLineInput],
    ) -> Result<String, CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Insert transaction header
        tx.execute(
            "INSERT INTO inventory_transactions (id, type, location_id, staff_id, notes, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, transaction_type.as_stored_str(), location_id, staff_id, notes, now],
        )?;

        // Insert lines and adjust stock
        for (i, line) in lines.iter().enumerate() {
            let line_id = uuid::Uuid::now_v7().to_string();
            let sort_order = (i + 1) as i64;

            tx.execute(
                "INSERT INTO inventory_transaction_lines (id, transaction_id, sku, product_name, qty, barcode_scanned, sort_order) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![line_id, id, line.sku, line.product_name, line.qty, line.barcode_scanned, sort_order],
            )?;

            // Adjust stock
            let tx_id = crate::inventory_transaction::InventoryTransactionId::from(id.clone());
            let stf_id = crate::user::UserId::from(staff_id.to_owned());
            self.adjust_stock_at_location_with_reason(
                &tx,
                &line.sku,
                line.delta,
                &crate::inventory::LocationId::from(location_id),
                Some(transaction_type.as_stored_str()),
                Some(&tx_id),
                None, // terminal_id
                Some(&stf_id),
            )?;
        }

        tx.commit()?;
        Ok(id)
    }

    /// List all inventory transaction headers, sorted by created_at descending.
    pub fn list_inventory_transactions(&self) -> Result<Vec<InventoryTransaction>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type, location_id, staff_id, transfer_id, purchase_order_id, notes, created_at \
             FROM inventory_transactions ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let type_str: String = row.get(1)?;
            let ttype =
                crate::inventory_transaction::InventoryTransactionType::from_stored_str(&type_str)
                    .unwrap_or(
                        crate::inventory_transaction::InventoryTransactionType::ManualAdjustment,
                    );
            Ok(InventoryTransaction {
                id: crate::inventory_transaction::InventoryTransactionId::from(
                    row.get::<_, String>(0)?,
                ),
                transaction_type: ttype,
                location_id: row.get(2)?,
                staff_id: row.get(3)?,
                transfer_id: row.get(4)?,
                purchase_order_id: row.get(5)?,
                notes: row.get(6).unwrap_or_default(),
                created_at: row.get(7)?,
            })
        })?;

        let mut txs = Vec::new();
        for r in rows {
            txs.push(r?);
        }
        Ok(txs)
    }

    /// Retrieve a single transaction by ID and all of its details lines.
    pub fn get_inventory_transaction(
        &self,
        id: &str,
    ) -> Result<Option<(InventoryTransaction, Vec<InventoryTransactionLine>)>, CoreError> {
        let header_res = self.conn.query_row(
            "SELECT id, type, location_id, staff_id, transfer_id, purchase_order_id, notes, created_at \
             FROM inventory_transactions WHERE id = ?1",
            params![id],
            |row| {
                let type_str: String = row.get(1)?;
                let ttype = crate::inventory_transaction::InventoryTransactionType::from_stored_str(&type_str)
                    .unwrap_or(crate::inventory_transaction::InventoryTransactionType::ManualAdjustment);
                Ok(InventoryTransaction {
                    id: crate::inventory_transaction::InventoryTransactionId::from(row.get::<_, String>(0)?),
                    transaction_type: ttype,
                    location_id: row.get(2)?,
                    staff_id: row.get(3)?,
                    transfer_id: row.get(4)?,
                    purchase_order_id: row.get(5)?,
                    notes: row.get(6).unwrap_or_default(),
                    created_at: row.get(7)?,
                })
            },
        );

        let header = match header_res {
            Ok(h) => h,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(CoreError::Db(e)),
        };

        // Query lines
        let mut stmt = self.conn.prepare(
            "SELECT id, transaction_id, sku, product_name, qty, barcode_scanned, sort_order \
             FROM inventory_transaction_lines WHERE transaction_id = ?1 ORDER BY sort_order ASC",
        )?;
        let lines_map = stmt.query_map(params![id], |row| {
            Ok(InventoryTransactionLine {
                id: row.get(0)?,
                transaction_id: crate::inventory_transaction::InventoryTransactionId::from(
                    row.get::<_, String>(1)?,
                ),
                sku: row.get(2)?,
                product_name: row.get(3)?,
                qty: row.get(4)?,
                barcode_scanned: row.get(5)?,
                sort_order: row.get(6)?,
            })
        })?;

        let mut lines = Vec::new();
        for r in lines_map {
            lines.push(r?);
        }

        Ok(Some((header, lines)))
    }

    /// Configure a stock threshold alert boundary for a product at a location.
    pub fn set_stock_threshold(
        &self,
        product_id: &str,
        location_id: Option<&str>,
        threshold: i64,
        enabled: bool,
    ) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Check if a row already exists for this unique combination
        let existing_id: Option<String> = match location_id {
            Some(loc) => tx
                .query_row(
                    "SELECT id FROM stock_thresholds WHERE product_id = ?1 AND location_id = ?2",
                    params![product_id, loc],
                    |row| row.get(0),
                )
                .ok(),
            None => tx
                .query_row(
                    "SELECT id FROM stock_thresholds WHERE product_id = ?1 AND location_id IS NULL",
                    params![product_id],
                    |row| row.get(0),
                )
                .ok(),
        };

        if let Some(id) = existing_id {
            tx.execute(
                "UPDATE stock_thresholds SET threshold = ?1, enabled = ?2, updated_at = ?3 WHERE id = ?4",
                params![threshold, if enabled { 1 } else { 0 }, now, id],
            )?;
        } else {
            let new_id = uuid::Uuid::now_v7().to_string();
            tx.execute(
                "INSERT INTO stock_thresholds (id, product_id, location_id, threshold, enabled, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![new_id, product_id, location_id, threshold, if enabled { 1 } else { 0 }, now],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// List all stock thresholds configured for a location (or global if location_id is None).
    pub fn get_stock_thresholds(
        &self,
        location_id: Option<&str>,
    ) -> Result<Vec<StockThreshold>, CoreError> {
        let mut stmt = if location_id.is_some() {
            self.conn.prepare(
                "SELECT id, product_id, location_id, threshold, enabled, created_at, updated_at \
                 FROM stock_thresholds WHERE location_id = ?1",
            )?
        } else {
            self.conn.prepare(
                "SELECT id, product_id, location_id, threshold, enabled, created_at, updated_at \
                 FROM stock_thresholds WHERE location_id IS NULL",
            )?
        };

        let parse_row = |row: &rusqlite::Row<'_>| {
            let en_int: i64 = row.get(4)?;
            Ok(StockThreshold {
                id: row.get(0)?,
                product_id: row.get(1)?,
                location_id: row.get(2)?,
                threshold: row.get(3)?,
                enabled: en_int == 1,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        };

        let rows = if let Some(loc) = location_id {
            stmt.query_map(params![loc], parse_row)?
        } else {
            stmt.query_map([], parse_row)?
        };

        let mut thresholds = Vec::new();
        for r in rows {
            thresholds.push(r?);
        }
        Ok(thresholds)
    }

    /// Delete a stock threshold configuration by ID.
    pub fn delete_stock_threshold(&self, id: &str) -> Result<(), CoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM stock_thresholds WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(())
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

    #[test]
    fn test_locations_crud() {
        let conn = fresh();
        let s = store(&conn);

        let id1 = s
            .create_inventory_location("Warehouse A", "warehouse", "Primary warehouse")
            .unwrap();
        let _id2 = s
            .create_inventory_location("Store Front", "store", "POS register floor")
            .unwrap();

        let locs = s.list_inventory_locations().unwrap();
        assert_eq!(locs.len(), 4); // 2 seeded default/transit + 2 new
        assert_eq!(locs[2].name, "Store Front");
        assert_eq!(locs[3].name, "Warehouse A");

        s.update_inventory_location(&id1, "Warehouse A Updated", "warehouse", "Updated desc")
            .unwrap();
        let locs = s.list_inventory_locations().unwrap();
        let updated = locs.iter().find(|l| l.id == id1).unwrap();
        assert_eq!(updated.name, "Warehouse A Updated");
        assert_eq!(updated.description, "Updated desc");

        s.deactivate_inventory_location(&id1).unwrap();
        let locs = s.list_inventory_locations().unwrap();
        let deactivated = locs.iter().find(|l| l.id == id1).unwrap();
        assert!(!deactivated.is_active);
    }

    #[test]
    fn test_workspace_locations() {
        let conn = fresh();
        // Seed workspace type and instance
        conn.execute(
            "INSERT OR IGNORE INTO workspace_types (key, name) VALUES ('retail', 'Retail POS')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspace_instances (id, type_key, store_id, name) VALUES ('ws-1', 'retail', 'default', 'Main POS')",
            []
        ).unwrap();
        let s = store(&conn);

        let loc_id = s
            .create_inventory_location("Warehouse A", "warehouse", "")
            .unwrap();
        let bindings = vec![WorkspaceInventoryLocation {
            id: "".to_owned(),
            instance_id: "ws-1".to_owned(),
            location_id: loc_id.clone(),
            is_primary: true,
            allow_negative_stock: true,
            sort_order: 1,
        }];

        s.set_workspace_inventory_locations("ws-1", &bindings)
            .unwrap();
        let retrieved = s.get_workspace_inventory_locations("ws-1").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].location_id, loc_id);
        assert!(retrieved[0].allow_negative_stock);
    }

    #[test]
    fn test_shifts() {
        let conn = fresh();
        // Seed a role and user
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-1', 'Role', 'Desc', '[]')",
            []
        ).unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) VALUES ('u-1', 'user', 'hash', 'User', 'r-1')",
            []
        ).unwrap();
        let s = store(&conn);

        let loc_id = s
            .create_inventory_location("Warehouse A", "warehouse", "")
            .unwrap();

        // Start shift
        let shift = s
            .start_inventory_shift("u-1", &loc_id, None, "shift notes")
            .unwrap();
        assert_eq!(shift.status, "active");
        assert!(shift.ended_at.is_none());

        // Attempting to open another active shift should error
        let err = s
            .start_inventory_shift("u-1", &loc_id, None, "")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { .. }));

        let active = s.get_active_inventory_shift("u-1").unwrap();
        assert_eq!(active.unwrap().id, shift.id);

        s.end_inventory_shift(&shift.id).unwrap();
        let active = s.get_active_inventory_shift("u-1").unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn test_thresholds() {
        let conn = fresh();
        // Seed a product
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency) VALUES ('p-1', 'SKU-1', 'Prod 1', 100, 'USD')",
            []
        ).unwrap();
        let s = store(&conn);

        let loc_id = s
            .create_inventory_location("Warehouse A", "warehouse", "")
            .unwrap();

        s.set_stock_threshold("p-1", Some(&loc_id), 10, true)
            .unwrap();
        let list = s.get_stock_thresholds(Some(&loc_id)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].threshold, 10);

        s.delete_stock_threshold(&list[0].id).unwrap();
        let list = s.get_stock_thresholds(Some(&loc_id)).unwrap();
        assert_eq!(list.len(), 0);
    }

    // ── Validation & Error Paths ──────────────────────────────────────

    #[test]
    fn create_inventory_location_invalid_type_errors() {
        let conn = fresh();
        let s = store(&conn);
        let err = s
            .create_inventory_location("Bad", "invalid_type", "")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field: "type", .. }));
    }

    #[test]
    fn update_inventory_location_nonexistent_errors() {
        let conn = fresh();
        let s = store(&conn);
        let err = s
            .update_inventory_location("nonexistent-id", "New", "store", "")
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::NotFound {
                entity: "inventory_location",
                ..
            }
        ));
    }

    #[test]
    fn deactivate_inventory_location_with_stock_errors() {
        let conn = fresh();
        let s = store(&conn);

        let loc_id = s
            .create_inventory_location("Test Loc", "store", "")
            .unwrap();
        // Seed a product with stock at this location
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES ('prod-1', 'SKU-1', 'Prod', 100, 'USD', 'retail')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty) VALUES ('prod-1', ?1, 5)",
            params![loc_id],
        )
        .unwrap();

        let err = s.deactivate_inventory_location(&loc_id).unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "location",
                ..
            }
        ));
        assert!(
            err.to_string().contains("active stock"),
            "expected active stock message, got: {}",
            err
        );
    }

    #[test]
    fn deactivate_inventory_location_nonexistent_succeeds() {
        let conn = fresh();
        let s = store(&conn);
        // deactivate on a non-existent location currently succeeds (no row matched)
        assert!(s.deactivate_inventory_location("nonexistent").is_ok());
    }

    #[test]
    fn get_workspace_locations_empty_for_unbound_workspace() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO workspace_types (key, name) VALUES ('retail', 'Retail POS')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspace_instances (id, type_key, store_id, name) \
             VALUES ('ws-empty', 'retail', 'default', 'Empty')",
            [],
        )
        .unwrap();

        let locs = s.get_workspace_inventory_locations("ws-empty").unwrap();
        assert!(locs.is_empty());
    }

    #[test]
    fn end_inventory_shift_nonexistent_errors() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.end_inventory_shift("nonexistent-shift").unwrap_err();
        assert!(matches!(
            err,
            CoreError::NotFound {
                entity: "active_inventory_shift",
                ..
            }
        ));
    }

    #[test]
    fn list_inventory_shifts_empty_returns_empty() {
        let conn = fresh();
        let s = store(&conn);
        let shifts = s.list_inventory_shifts().unwrap();
        assert!(shifts.is_empty());
    }
    #[test]
    fn test_inventory_transaction_lifecycle() {
        let conn = fresh();
        let s = store(&conn);

        // Seed FK rows: role + user for staff_id constraint
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-inv', 'InvRole', '', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) VALUES ('staff-1', 'staff1', 'hash', 'Staff 1', 'r-inv')",
            [],
        )
        .unwrap();

        // Seed a location and product with stock
        let loc_id = s.create_inventory_location("Store", "store", "").unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES ('prod-stock', 'STOCK-SKU', 'Stocked', 1000, 'USD', 'retail')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty) VALUES ('prod-stock', ?1, 100)",
            params![loc_id],
        )
        .unwrap();

        // Create a stock-count transaction with one line (no delta change, just audit)
        let lines = vec![InventoryTransactionLineInput {
            sku: "STOCK-SKU".into(),
            product_name: "Stocked Product".into(),
            qty: 50,
            delta: 0,
            barcode_scanned: None,
        }];
        let tx_id = s
            .create_inventory_transaction(
                crate::inventory_transaction::InventoryTransactionType::StockCount,
                &loc_id,
                "staff-1",
                "audit notes",
                &lines,
            )
            .unwrap();
        assert!(!tx_id.is_empty());

        // Verify it appears in list
        let txns = s.list_inventory_transactions().unwrap();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].id.as_str(), tx_id);
        assert_eq!(txns[0].notes, "audit notes");

        // Verify we can get the full transaction with lines
        let (header, detail_lines) = s.get_inventory_transaction(&tx_id).unwrap().unwrap();
        assert_eq!(header.id.as_str(), tx_id);
        assert_eq!(detail_lines.len(), 1);
        assert_eq!(detail_lines[0].sku, "STOCK-SKU");
        assert_eq!(detail_lines[0].qty, 50);
    }

    #[test]
    fn get_inventory_transaction_not_found_returns_none() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.get_inventory_transaction("nonexistent-tx").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_inventory_transactions_empty() {
        let conn = fresh();
        let s = store(&conn);
        let txns = s.list_inventory_transactions().unwrap();
        assert!(txns.is_empty());
    }

    #[test]
    fn test_stock_threshold_upsert_updates_existing() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency) VALUES ('p-upsert', 'SKU-U', 'Upsert', 100, 'USD')",
            [],
        )
        .unwrap();

        // Create initial threshold
        s.set_stock_threshold("p-upsert", None, 10, true).unwrap();
        let list = s.get_stock_thresholds(None).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].threshold, 10);
        assert!(list[0].enabled);

        // Upsert: update threshold value and disable
        s.set_stock_threshold("p-upsert", None, 25, false).unwrap();
        let list = s.get_stock_thresholds(None).unwrap();
        assert_eq!(list.len(), 1, "upsert should not create duplicate");
        assert_eq!(list[0].threshold, 25);
        assert!(!list[0].enabled);
    }

    #[test]
    fn test_stock_threshold_global_vs_per_location() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency) VALUES ('p-glob', 'SKU-G', 'Global', 100, 'USD')",
            [],
        )
        .unwrap();
        let loc_id = s
            .create_inventory_location("Test Loc", "store", "")
            .unwrap();

        // Set a global threshold (null location_id)
        s.set_stock_threshold("p-glob", None, 5, true).unwrap();
        // Set a per-location threshold
        s.set_stock_threshold("p-glob", Some(&loc_id), 15, true)
            .unwrap();

        let global_list = s.get_stock_thresholds(None).unwrap();
        assert_eq!(global_list.len(), 1);
        assert_eq!(global_list[0].threshold, 5);

        let loc_list = s.get_stock_thresholds(Some(&loc_id)).unwrap();
        assert_eq!(loc_list.len(), 1);
        assert_eq!(loc_list[0].threshold, 15);
    }

    #[test]
    fn set_workspace_locations_replaces_existing_bindings() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT OR IGNORE INTO workspace_types (key, name) VALUES ('retail', 'Retail POS')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspace_instances (id, type_key, store_id, name) \
             VALUES ('ws-replace', 'retail', 'default', 'Replace')",
            [],
        )
        .unwrap();

        let loc_a = s.create_inventory_location("Loc A", "store", "").unwrap();
        let loc_b = s
            .create_inventory_location("Loc B", "warehouse", "")
            .unwrap();

        // Set initial binding
        let initial = vec![WorkspaceInventoryLocation {
            id: String::new(),
            instance_id: "ws-replace".into(),
            location_id: loc_a.clone(),
            is_primary: true,
            allow_negative_stock: false,
            sort_order: 0,
        }];
        s.set_workspace_inventory_locations("ws-replace", &initial)
            .unwrap();

        // Replace with two bindings (different locations, different settings)
        let replacement = vec![
            WorkspaceInventoryLocation {
                id: String::new(),
                instance_id: "ws-replace".into(),
                location_id: loc_b.clone(),
                is_primary: true,
                allow_negative_stock: true,
                sort_order: 0,
            },
            WorkspaceInventoryLocation {
                id: String::new(),
                instance_id: "ws-replace".into(),
                location_id: loc_a.clone(),
                is_primary: false,
                allow_negative_stock: false,
                sort_order: 1,
            },
        ];
        s.set_workspace_inventory_locations("ws-replace", &replacement)
            .unwrap();

        let retrieved = s.get_workspace_inventory_locations("ws-replace").unwrap();
        assert_eq!(retrieved.len(), 2);
        // First should be primary (loc_b)
        assert_eq!(retrieved[0].location_id, loc_b);
        assert!(retrieved[0].is_primary);
        assert!(retrieved[0].allow_negative_stock);
        // Second should be secondary (loc_a)
        assert_eq!(retrieved[1].location_id, loc_a);
        assert!(!retrieved[1].is_primary);
        assert!(!retrieved[1].allow_negative_stock);
    }

    #[test]
    fn update_inventory_location_invalid_type_errors() {
        let conn = fresh();
        let s = store(&conn);
        let loc_id = s.create_inventory_location("Valid", "store", "").unwrap();
        let err = s
            .update_inventory_location(&loc_id, "Bad", "invalid_type", "")
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field: "type", .. }));
    }

    #[test]
    fn create_inventory_transaction_adjusts_stock() {
        let conn = fresh();
        let s = store(&conn);

        // Seed FK rows: role + user for staff_id constraint
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-inv2', 'InvRole2', '', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) VALUES ('staff-2', 'staff2', 'hash', 'Staff 2', 'r-inv2')",
            [],
        )
        .unwrap();

        let loc_id = s
            .create_inventory_location("Warehouse", "warehouse", "")
            .unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES ('prod-delta', 'DELTA', 'Delta Item', 500, 'USD', 'retail')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty) VALUES ('prod-delta', ?1, 50)",
            params![loc_id],
        )
        .unwrap();

        // Create a manual adjustment: add 10 units
        let lines = vec![InventoryTransactionLineInput {
            sku: "DELTA".into(),
            product_name: "Delta Item".into(),
            qty: 10,
            delta: 10, // positive = credit
            barcode_scanned: None,
        }];
        s.create_inventory_transaction(
            crate::inventory_transaction::InventoryTransactionType::ManualAdjustment,
            &loc_id,
            "staff-2",
            "added 10 units",
            &lines,
        )
        .unwrap();

        // Verify stock increased
        let stock: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary \
                 WHERE item_id = 'prod-delta' AND location_id = ?1",
                params![loc_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock, 60, "stock should have increased by 10");
    }

    // ── Extended edge cases (coverage 19→30) ──────────────────────────

    #[test]
    fn start_shift_nonexistent_user_fk_errors() {
        let conn = fresh();
        let s = store(&conn);
        let loc_id = s
            .create_inventory_location("Test Loc", "store", "")
            .unwrap();
        let err = s
            .start_inventory_shift("nonexistent-user", &loc_id, None, "")
            .unwrap_err();
        // FK violation on users(id) returns a rusqlite error wrapped in CoreError::Db
        assert!(matches!(err, CoreError::Db(_)));
    }

    #[test]
    fn end_already_ended_shift_errors() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-eae', 'Role', '', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) \
             VALUES ('u-eae', 'user', 'hash', 'User', 'r-eae')",
            [],
        )
        .unwrap();
        let loc_id = s.create_inventory_location("Loc", "store", "").unwrap();

        let shift = s.start_inventory_shift("u-eae", &loc_id, None, "").unwrap();
        s.end_inventory_shift(&shift.id).unwrap();

        // Ending again should error
        let err = s.end_inventory_shift(&shift.id).unwrap_err();
        assert!(
            matches!(err, CoreError::NotFound { entity, .. } if entity == "active_inventory_shift")
        );
    }

    #[test]
    fn list_shifts_orders_by_started_at_desc() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-ord', 'Role', '', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) \
             VALUES ('u-ord', 'user', 'hash', 'User', 'r-ord')",
            [],
        )
        .unwrap();
        let loc_id = s.create_inventory_location("Loc", "store", "").unwrap();

        let shift1 = s
            .start_inventory_shift("u-ord", &loc_id, None, "first")
            .unwrap();
        // End shift1 first so we can start shift2
        s.end_inventory_shift(&shift1.id).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        let shift2 = s
            .start_inventory_shift("u-ord", &loc_id, None, "second")
            .unwrap();

        let all = s.list_inventory_shifts().unwrap();
        assert_eq!(all.len(), 2);
        // Most recent first
        assert_eq!(all[0].notes, "second");
        assert_eq!(all[1].notes, "first");
    }

    #[test]
    fn deactivate_location_with_pending_transfers_errors() {
        let conn = fresh();
        let s = store(&conn);
        let loc_id = s.create_inventory_location("Loc", "store", "").unwrap();

        // Seed user + transfer referencing this location
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) \
             VALUES ('r-deact', 'Role', '', '[]', 'now', 'now')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) \
             VALUES ('u-deact', 'user', 'hash', 'User', 'r-deact', 'now', 'now')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_transfers (id, transfer_number, status, source_location_id, destination_location_id, \
             created_by, created_at, updated_at) \
             VALUES ('tr-pend', 'TRF-1', 'in_transit', ?1, ?1, 'u-deact', 'now', 'now')",
            params![loc_id],
        )
        .unwrap();

        let err = s.deactivate_inventory_location(&loc_id).unwrap_err();
        assert!(
            err.to_string().contains("pending stock transfers"),
            "expected pending transfer message, got: {}",
            err
        );
    }

    #[test]
    fn create_transaction_with_multiple_lines_and_barcode() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-ml', 'Role', '', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) VALUES ('staff-ml', 's', 'hash', 'S', 'r-ml')",
            [],
        )
        .unwrap();
        let loc_id = s.create_inventory_location("Loc", "store", "").unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES ('p-a', 'SKU-A', 'A', 100, 'USD', 'retail')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES ('p-b', 'SKU-B', 'B', 200, 'USD', 'retail')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty) VALUES ('p-a', ?1, 20)",
            params![loc_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty) VALUES ('p-b', ?1, 30)",
            params![loc_id],
        )
        .unwrap();

        let lines = vec![
            InventoryTransactionLineInput {
                sku: "SKU-A".into(),
                product_name: "Product A".into(),
                qty: 10,
                delta: -5,
                barcode_scanned: Some("BARCODE-A".into()),
            },
            InventoryTransactionLineInput {
                sku: "SKU-B".into(),
                product_name: "Product B".into(),
                qty: 5,
                delta: 3,
                barcode_scanned: None,
            },
        ];
        let tx_id = s
            .create_inventory_transaction(
                crate::inventory_transaction::InventoryTransactionType::ManualAdjustment,
                &loc_id,
                "staff-ml",
                "multi-line + barcode",
                &lines,
            )
            .unwrap();

        let (_, detail_lines) = s.get_inventory_transaction(&tx_id).unwrap().unwrap();
        assert_eq!(detail_lines.len(), 2);
        // Lines ordered by sort_order
        assert_eq!(detail_lines[0].sku, "SKU-A");
        assert_eq!(detail_lines[0].qty, 10);
        assert_eq!(
            detail_lines[0].barcode_scanned.as_deref(),
            Some("BARCODE-A")
        );
        assert_eq!(detail_lines[1].sku, "SKU-B");
        assert_eq!(detail_lines[1].qty, 5);
        assert!(detail_lines[1].barcode_scanned.is_none());
    }

    #[test]
    fn list_transactions_orders_by_created_at_desc() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-tord', 'Role', '', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) \
             VALUES ('staff-tord', 's', 'hash', 'S', 'r-tord')",
            [],
        )
        .unwrap();
        let loc_id = s.create_inventory_location("Loc", "store", "").unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES ('p-tord', 'SKU-T', 'T', 100, 'USD', 'retail')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty) VALUES ('p-tord', ?1, 100)",
            params![loc_id],
        )
        .unwrap();

        let line = vec![InventoryTransactionLineInput {
            sku: "SKU-T".into(),
            product_name: "T".into(),
            qty: 1,
            delta: 0,
            barcode_scanned: None,
        }];
        let tx1 = s
            .create_inventory_transaction(
                crate::inventory_transaction::InventoryTransactionType::StockCount,
                &loc_id,
                "staff-tord",
                "first",
                &line,
            )
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let tx2 = s
            .create_inventory_transaction(
                crate::inventory_transaction::InventoryTransactionType::StockCount,
                &loc_id,
                "staff-tord",
                "second",
                &line,
            )
            .unwrap();

        let all = s.list_inventory_transactions().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id.as_str(), tx2, "most recent first");
        assert_eq!(all[1].id.as_str(), tx1);
    }

    #[test]
    fn delete_nonexistent_threshold_succeeds() {
        let conn = fresh();
        let s = store(&conn);
        // Deleting a non-existent threshold should not error (DELETE with no match is a no-op)
        s.delete_stock_threshold("nonexistent-id").unwrap();
    }

    #[test]
    fn get_thresholds_for_location_with_none_returns_empty() {
        let conn = fresh();
        let s = store(&conn);
        let loc_id = s
            .create_inventory_location("Empty Loc", "store", "")
            .unwrap();
        let list = s.get_stock_thresholds(Some(&loc_id)).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn create_transaction_without_stock_change_preserves_qty() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-d0', 'Role', '', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) VALUES ('staff-d0', 's', 'hash', 'S', 'r-d0')",
            [],
        )
        .unwrap();
        let loc_id = s.create_inventory_location("Loc", "store", "").unwrap();
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, product_type) \
             VALUES ('p-d0', 'SKU-D0', 'D0', 100, 'USD', 'retail')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stock_summary (item_id, location_id, qty) VALUES ('p-d0', ?1, 40)",
            params![loc_id],
        )
        .unwrap();

        let lines = vec![InventoryTransactionLineInput {
            sku: "SKU-D0".into(),
            product_name: "D0".into(),
            qty: 10,
            delta: 0, // zero delta — no stock change
            barcode_scanned: None,
        }];
        s.create_inventory_transaction(
            crate::inventory_transaction::InventoryTransactionType::StockCount,
            &loc_id,
            "staff-d0",
            "zero delta audit",
            &lines,
        )
        .unwrap();

        // Stock should be unchanged
        let stock: i64 = conn
            .query_row(
                "SELECT COALESCE(qty, 0) FROM stock_summary \
                 WHERE item_id = 'p-d0' AND location_id = ?1",
                params![loc_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stock, 40, "zero-delta transaction should not change stock");
    }

    #[test]
    fn start_shift_with_terminal_id_stores_terminal() {
        let conn = fresh();
        let s = store(&conn);
        conn.execute(
            "INSERT INTO roles (id, name, description, permissions) VALUES ('r-term', 'Role', '', '[]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, pin_hash, display_name, role_id) \
             VALUES ('u-term', 'user', 'hash', 'User', 'r-term')",
            [],
        )
        .unwrap();
        let loc_id = s.create_inventory_location("Loc", "store", "").unwrap();

        // Seed a terminal for the FK reference
        conn.execute(
            "INSERT INTO terminals (id, name, device_id, is_active, created_at, updated_at) \
             VALUES ('term-1', 'Terminal 1', 'dev-term', 1, 'now', 'now')",
            [],
        )
        .unwrap();

        let shift = s
            .start_inventory_shift("u-term", &loc_id, Some("term-1"), "with terminal")
            .unwrap();
        assert_eq!(shift.terminal_id.as_deref(), Some("term-1"));
        assert_eq!(shift.notes, "with terminal");
    }

    #[test]
    fn list_locations_returns_in_order_by_name() {
        let conn = fresh();
        let s = store(&conn);
        let _c = s.create_inventory_location("Zebra", "store", "").unwrap();
        let _a = s
            .create_inventory_location("Alpha", "warehouse", "")
            .unwrap();
        let _m = s.create_inventory_location("Mike", "store", "").unwrap();

        let locs = s.list_inventory_locations().unwrap();
        // 2 seeded (canonical default + transit) + 3 new = 5
        assert_eq!(locs.len(), 5);
        // Our custom ones should be ordered: Alpha, Mike, Zebra (among the seeded ones)
        let names: Vec<&str> = locs.iter().map(|l| l.name.as_str()).collect();
        let alpha_pos = names.iter().position(|n| *n == "Alpha").unwrap();
        let mike_pos = names.iter().position(|n| *n == "Mike").unwrap();
        let zebra_pos = names.iter().position(|n| *n == "Zebra").unwrap();
        assert!(alpha_pos < mike_pos, "Alpha should come before Mike");
        assert!(mike_pos < zebra_pos, "Mike should come before Zebra");
    }
}
