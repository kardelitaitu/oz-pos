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
                    field: "type".into(),
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
                    field: "type".into(),
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
                field: "location".into(),
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
                field: "location".into(),
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
                field: "shift".into(),
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
            "INSERT INTO inventory_transactions (id, row_type, location_id, staff_id, notes, created_at) \
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
            "SELECT id, row_type, location_id, staff_id, transfer_id, purchase_order_id, notes, created_at \
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
            "SELECT id, row_type, location_id, staff_id, transfer_id, purchase_order_id, notes, created_at \
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
}
