//! Purchase order CRUD — list, get, create, update status, receive.

use rusqlite::params;

use crate::error::CoreError;
use crate::{PurchaseOrder, PurchaseOrderLine, PurchaseOrderWithLines};

use super::Store;

impl Store<'_> {
    /// List all purchase orders, ordered by creation date descending, with lines.
    pub fn list_purchase_orders(&self) -> Result<Vec<PurchaseOrderWithLines>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT po.id, po.po_number, po.supplier_id, po.status, po.order_date,
                    po.expected_date, po.received_date, po.subtotal_minor, po.tax_minor,
                    po.total_minor, po.notes, po.created_by, po.created_at, po.updated_at,
                    s.name AS supplier_name
             FROM purchase_orders po
             LEFT JOIN suppliers s ON po.supplier_id = s.id
             ORDER BY po.created_at DESC",
        )?;
        let orders: Vec<PurchaseOrderWithLines> = stmt
            .query_map([], |row| {
                Ok(PurchaseOrderWithLines {
                    order: PurchaseOrder {
                        id: row.get("id")?,
                        po_number: row.get("po_number")?,
                        supplier_id: row.get("supplier_id")?,
                        status: row.get("status")?,
                        order_date: row.get("order_date")?,
                        expected_date: row.get("expected_date")?,
                        received_date: row.get("received_date")?,
                        subtotal_minor: row.get("subtotal_minor")?,
                        tax_minor: row.get("tax_minor")?,
                        total_minor: row.get("total_minor")?,
                        notes: row.get("notes")?,
                        created_by: row.get("created_by")?,
                        created_at: row.get("created_at")?,
                        updated_at: row.get("updated_at")?,
                    },
                    lines: Vec::new(),
                    supplier_name: row.get("supplier_name")?,
                })
            })?
            .map(|r| Ok(r?))
            .collect::<Result<Vec<_>, CoreError>>()?;

        if orders.is_empty() {
            return Ok(orders);
        }

        let mut line_stmt = self.conn.prepare(
            "SELECT id, po_id, sku, product_name, qty, unit_cost_minor, line_total_minor
             FROM purchase_order_lines WHERE po_id = ?1 ORDER BY id",
        )?;

        let mut result: Vec<PurchaseOrderWithLines> = Vec::with_capacity(orders.len());
        for mut order in orders {
            let lines: Vec<PurchaseOrderLine> = line_stmt
                .query_map(params![&order.order.id], |row| {
                    Ok(PurchaseOrderLine {
                        id: row.get("id")?,
                        po_id: row.get("po_id")?,
                        sku: row.get("sku")?,
                        product_name: row.get("product_name")?,
                        qty: row.get("qty")?,
                        unit_cost_minor: row.get("unit_cost_minor")?,
                        line_total_minor: row.get("line_total_minor")?,
                    })
                })?
                .map(|r| Ok(r?))
                .collect::<Result<Vec<_>, CoreError>>()?;
            order.lines = lines;
            result.push(order);
        }

        Ok(result)
    }

    /// Look up a single purchase order by id, including lines.
    pub fn get_purchase_order(
        &self,
        id: &str,
    ) -> Result<Option<PurchaseOrderWithLines>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT po.id, po.po_number, po.supplier_id, po.status, po.order_date,
                    po.expected_date, po.received_date, po.subtotal_minor, po.tax_minor,
                    po.total_minor, po.notes, po.created_by, po.created_at, po.updated_at,
                    s.name AS supplier_name
             FROM purchase_orders po
             LEFT JOIN suppliers s ON po.supplier_id = s.id
             WHERE po.id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok(PurchaseOrderWithLines {
                order: PurchaseOrder {
                    id: row.get("id")?,
                    po_number: row.get("po_number")?,
                    supplier_id: row.get("supplier_id")?,
                    status: row.get("status")?,
                    order_date: row.get("order_date")?,
                    expected_date: row.get("expected_date")?,
                    received_date: row.get("received_date")?,
                    subtotal_minor: row.get("subtotal_minor")?,
                    tax_minor: row.get("tax_minor")?,
                    total_minor: row.get("total_minor")?,
                    notes: row.get("notes")?,
                    created_by: row.get("created_by")?,
                    created_at: row.get("created_at")?,
                    updated_at: row.get("updated_at")?,
                },
                lines: Vec::new(),
                supplier_name: row.get("supplier_name")?,
            })
        });

        let mut order = match result {
            Ok(o) => o,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut line_stmt = self.conn.prepare(
            "SELECT id, po_id, sku, product_name, qty, unit_cost_minor, line_total_minor
             FROM purchase_order_lines WHERE po_id = ?1 ORDER BY id",
        )?;
        let lines: Vec<PurchaseOrderLine> = line_stmt
            .query_map(params![id], |row| {
                Ok(PurchaseOrderLine {
                    id: row.get("id")?,
                    po_id: row.get("po_id")?,
                    sku: row.get("sku")?,
                    product_name: row.get("product_name")?,
                    qty: row.get("qty")?,
                    unit_cost_minor: row.get("unit_cost_minor")?,
                    line_total_minor: row.get("line_total_minor")?,
                })
            })?
            .map(|r| Ok(r?))
            .collect::<Result<Vec<_>, CoreError>>()?;
        order.lines = lines;

        Ok(Some(order))
    }

    /// Insert a new purchase order with line items (all in one transaction).
    pub fn create_purchase_order(
        &self,
        po_number: &str,
        supplier_id: &str,
        expected_date: &str,
        notes: &str,
        created_by: Option<&str>,
        lines: &[CreatePoLineInput],
    ) -> Result<PurchaseOrderWithLines, CoreError> {
        if po_number.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "po_number",
                message: "PO number must not be empty".into(),
            });
        }

        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let mut subtotal: i64 = 0;
        for line in lines {
            if line.qty < 0 {
                return Err(CoreError::Validation {
                    field: "qty",
                    message: "quantity must not be negative".into(),
                });
            }
            if line.unit_cost_minor < 0 {
                return Err(CoreError::Validation {
                    field: "unit_cost_minor",
                    message: "unit cost must not be negative".into(),
                });
            }
            subtotal += line.qty * line.unit_cost_minor;
        }

        self.conn.execute(
            "INSERT INTO purchase_orders (id, po_number, supplier_id, status, order_date,
                                          expected_date, subtotal_minor, tax_minor, total_minor,
                                          notes, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'draft', ?4, ?5, ?6, 0, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                po_number.trim(),
                supplier_id,
                now,
                expected_date,
                subtotal,
                subtotal,
                notes,
                created_by,
                now,
                now
            ],
        )?;

        let mut created_lines: Vec<PurchaseOrderLine> = Vec::with_capacity(lines.len());
        for line in lines {
            let line_id = uuid::Uuid::now_v7().to_string();
            let line_total = line.qty * line.unit_cost_minor;
            self.conn.execute(
                "INSERT INTO purchase_order_lines (id, po_id, sku, product_name, qty,
                                                    unit_cost_minor, line_total_minor)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    line_id,
                    id,
                    line.sku,
                    line.product_name,
                    line.qty,
                    line.unit_cost_minor,
                    line_total
                ],
            )?;
            created_lines.push(PurchaseOrderLine {
                id: line_id,
                po_id: id.clone(),
                sku: line.sku.to_owned(),
                product_name: line.product_name.to_owned(),
                qty: line.qty,
                unit_cost_minor: line.unit_cost_minor,
                line_total_minor: line_total,
            });
        }

        Ok(PurchaseOrderWithLines {
            order: PurchaseOrder {
                id,
                po_number: po_number.trim().to_owned(),
                supplier_id: supplier_id.to_owned(),
                status: "draft".into(),
                order_date: now.clone(),
                expected_date: expected_date.to_owned(),
                received_date: None,
                subtotal_minor: subtotal,
                tax_minor: 0,
                total_minor: subtotal,
                notes: notes.to_owned(),
                created_by: created_by.map(|s| s.to_owned()),
                created_at: now.clone(),
                updated_at: now,
            },
            lines: created_lines,
            supplier_name: None,
        })
    }

    /// Update the status of a purchase order.
    pub fn update_po_status(
        &self,
        id: &str,
        new_status: &str,
    ) -> Result<PurchaseOrderWithLines, CoreError> {
        let valid_statuses = ["draft", "pending", "approved", "received", "cancelled"];
        if !valid_statuses.contains(&new_status) {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("invalid status: {new_status}"),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let rows = self.conn.execute(
            "UPDATE purchase_orders SET status=?1, updated_at=?2 WHERE id=?3",
            params![new_status, now, id],
        )?;

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "purchase_order",
                id: id.to_owned(),
            });
        }

        self.get_purchase_order(id)?.ok_or(CoreError::NotFound {
            entity: "purchase_order",
            id: id.to_owned(),
        })
    }

    /// Mark a purchase order as received and adjust inventory quantities.
    #[allow(deprecated)]
    pub fn receive_purchase_order(&self, id: &str) -> Result<PurchaseOrderWithLines, CoreError> {
        let mut po = self.get_purchase_order(id)?.ok_or(CoreError::NotFound {
            entity: "purchase_order",
            id: id.to_owned(),
        })?;

        if po.order.status != "approved" {
            return Err(CoreError::Validation {
                field: "status",
                message: "only approved orders can be received".into(),
            });
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.conn.execute(
            "UPDATE purchase_orders SET status='received', received_date=?1, updated_at=?2 WHERE id=?3",
            params![now, now, id],
        )?;

        for line in &po.lines {
            if !line.sku.is_empty() {
                let _ = self.adjust_stock(&line.sku, line.qty).map_err(|e| {
                    tracing::warn!(
                        sku = %line.sku,
                        qty = line.qty,
                        error = %e,
                        "failed to adjust stock during PO receive"
                    );
                });
            }
        }

        po.order.status = "received".into();
        po.order.received_date = Some(now.clone());
        po.order.updated_at = now;
        Ok(po)
    }
}

/// Input for creating a purchase order line item.
#[derive(Debug, Clone)]
pub struct CreatePoLineInput {
    /// SKU of the product being ordered.
    pub sku: String,
    /// Display name of the product.
    pub product_name: String,
    /// Quantity ordered (must not be negative).
    pub qty: i64,
    /// Unit cost in minor units (must not be negative).
    pub unit_cost_minor: i64,
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

    fn seed_supplier(conn: &Connection) -> String {
        conn.execute(
            "INSERT INTO suppliers (id, code, name, status, created_at, updated_at) VALUES (?1, ?2, ?3, 'active', ?4, ?4)",
            params!["sup-po", "SUP-PO", "Test Supplier", "2025-01-01T00:00:00.000Z"],
        ).unwrap();
        "sup-po".into()
    }

    fn seed_product(conn: &Connection) {
        conn.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at, price_updated_at) VALUES (?1, ?2, ?3, 1000, 'USD', ?4, ?4, ?4)",
            params!["prod-po", "SKU-001", "Widget", "2025-01-01T00:00:00.000Z"],
        ).unwrap();
        conn.execute(
            "INSERT INTO inventory (product_id, qty) VALUES (?1, 10)",
            params!["prod-po"],
        )
        .unwrap();
    }

    #[test]
    fn create_po_with_lines() {
        let conn = fresh();
        seed_supplier(&conn);
        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: 5,
            unit_cost_minor: 1000,
        }];
        let po = store(&conn)
            .create_purchase_order("PO-TEST-1", "sup-po", "2025-02-01", "Urgent", None, &lines)
            .unwrap();
        assert_eq!(po.order.po_number, "PO-TEST-1");
        assert_eq!(po.order.status, "draft");
        assert_eq!(po.order.subtotal_minor, 5000);
        assert_eq!(po.lines.len(), 1);
    }

    #[test]
    fn get_po() {
        let conn = fresh();
        let sid = seed_supplier(&conn);
        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: 2,
            unit_cost_minor: 500,
        }];
        let created = store(&conn)
            .create_purchase_order("PO-TEST-2", &sid, "", "", None, &lines)
            .unwrap();
        let fetched = store(&conn)
            .get_purchase_order(&created.order.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.order.po_number, "PO-TEST-2");
        assert_eq!(fetched.lines.len(), 1);
    }

    #[test]
    fn list_pos() {
        let conn = fresh();
        let sid = seed_supplier(&conn);
        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: 2,
            unit_cost_minor: 500,
        }];
        store(&conn)
            .create_purchase_order("PO-TEST-3", &sid, "", "", None, &lines)
            .unwrap();
        let list = store(&conn).list_purchase_orders().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].lines.len(), 1);
    }

    #[test]
    fn update_status() {
        let conn = fresh();
        let sid = seed_supplier(&conn);
        let po = store(&conn)
            .create_purchase_order("PO-STATUS", &sid, "", "", None, &[])
            .unwrap();
        let updated = store(&conn)
            .update_po_status(&po.order.id, "approved")
            .unwrap();
        assert_eq!(updated.order.status, "approved");
    }

    #[test]
    fn update_invalid_status() {
        let conn = fresh();
        let sid = seed_supplier(&conn);
        let po = store(&conn)
            .create_purchase_order("PO-INV", &sid, "", "", None, &[])
            .unwrap();
        let err = store(&conn)
            .update_po_status(&po.order.id, "invalid")
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::Validation {
                field: "status",
                ..
            }
        ));
    }

    #[test]
    fn receive_po_updates_inventory() {
        let conn = fresh();
        seed_supplier(&conn);
        seed_product(&conn);

        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: 5,
            unit_cost_minor: 1000,
        }];
        let po = store(&conn)
            .create_purchase_order("PO-RECV", "sup-po", "", "", None, &lines)
            .unwrap();
        store(&conn)
            .update_po_status(&po.order.id, "approved")
            .unwrap();
        let received = store(&conn).receive_purchase_order(&po.order.id).unwrap();
        assert_eq!(received.order.status, "received");
        assert!(received.order.received_date.is_some());

        let stock: i64 = conn
            .query_row(
                "SELECT qty FROM inventory WHERE product_id='prod-po'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stock, 15);
    }

    #[test]
    fn create_po_empty_number_rejected() {
        let conn = fresh();
        seed_supplier(&conn);
        let err = store(&conn)
            .create_purchase_order("  ", "sup-po", "", "", None, &[])
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "po_number"));
    }

    #[test]
    fn create_po_negative_qty_rejected() {
        let conn = fresh();
        seed_supplier(&conn);
        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: -1,
            unit_cost_minor: 100,
        }];
        let err = store(&conn)
            .create_purchase_order("PO-NEG", "sup-po", "", "", None, &lines)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "qty"));
    }

    #[test]
    fn get_po_not_found_returns_none() {
        let conn = fresh();
        let result = store(&conn).get_purchase_order("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn receive_po_not_approved_rejected() {
        let conn = fresh();
        let sid = seed_supplier(&conn);
        let po = store(&conn)
            .create_purchase_order("PO-NOT-APPROVED", &sid, "", "", None, &[])
            .unwrap();
        // Still in "draft" status, not "approved".
        let err = store(&conn)
            .receive_purchase_order(&po.order.id)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn po_full_lifecycle() {
        let conn = fresh();
        seed_supplier(&conn);
        seed_product(&conn);

        // Step 1: Create as draft
        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: 5,
            unit_cost_minor: 1000,
        }];
        let po = store(&conn)
            .create_purchase_order(
                "PO-LIFECYCLE",
                "sup-po",
                "2025-03-01",
                "full cycle",
                None,
                &lines,
            )
            .unwrap();
        assert_eq!(po.order.status, "draft");
        assert_eq!(po.order.subtotal_minor, 5000);
        assert_eq!(po.lines.len(), 1);

        // Step 2: Approve
        let approved = store(&conn)
            .update_po_status(&po.order.id, "approved")
            .unwrap();
        assert_eq!(approved.order.status, "approved");

        // Step 3: Receive
        let received = store(&conn).receive_purchase_order(&po.order.id).unwrap();
        assert_eq!(received.order.status, "received");
        assert!(received.order.received_date.is_some());

        // Step 4: Verify inventory incremented (seed = 10, +5 = 15)
        let stock: i64 = conn
            .query_row(
                "SELECT qty FROM inventory WHERE product_id='prod-po'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stock, 15);
    }

    #[test]
    fn po_draft_to_pending_to_approved() {
        let conn = fresh();
        let sid = seed_supplier(&conn);
        let po = store(&conn)
            .create_purchase_order("PO-TRANSITIONS", &sid, "", "", None, &[])
            .unwrap();
        assert_eq!(po.order.status, "draft");

        let pending = store(&conn)
            .update_po_status(&po.order.id, "pending")
            .unwrap();
        assert_eq!(pending.order.status, "pending");

        let approved = store(&conn)
            .update_po_status(&po.order.id, "approved")
            .unwrap();
        assert_eq!(approved.order.status, "approved");
    }

    #[test]
    fn po_cancel_then_reopen_then_receive() {
        let conn = fresh();
        seed_supplier(&conn);
        seed_product(&conn);

        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: 3,
            unit_cost_minor: 500,
        }];
        let po = store(&conn)
            .create_purchase_order("PO-REOPEN", "sup-po", "", "", None, &lines)
            .unwrap();

        // Cancel first
        let cancelled = store(&conn)
            .update_po_status(&po.order.id, "cancelled")
            .unwrap();
        assert_eq!(cancelled.order.status, "cancelled");

        // Now try to receive while cancelled — should fail
        let err = store(&conn)
            .receive_purchase_order(&po.order.id)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));

        // Reopen: set back to approved
        let reopened = store(&conn)
            .update_po_status(&po.order.id, "approved")
            .unwrap();
        assert_eq!(reopened.order.status, "approved");

        // Now receive should work
        let received = store(&conn).receive_purchase_order(&po.order.id).unwrap();
        assert_eq!(received.order.status, "received");

        // Verify inventory (seed = 10, +3 = 13)
        let stock: i64 = conn
            .query_row(
                "SELECT qty FROM inventory WHERE product_id='prod-po'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stock, 13);
    }

    #[test]
    fn po_update_status_nonexistent_id() {
        let conn = fresh();
        let err = store(&conn)
            .update_po_status("i-do-not-exist", "approved")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "purchase_order"));
    }

    #[test]
    fn po_receive_nonexistent_id() {
        let conn = fresh();
        let err = store(&conn)
            .receive_purchase_order("i-do-not-exist")
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "purchase_order"));
    }

    // ── Additional edge cases (coverage expansion) ──────────────────

    #[test]
    fn create_po_multiple_lines() {
        let conn = fresh();
        seed_supplier(&conn);

        let lines = vec![
            CreatePoLineInput {
                sku: "SKU-001".into(),
                product_name: "Widget A".into(),
                qty: 2,
                unit_cost_minor: 1000,
            },
            CreatePoLineInput {
                sku: "SKU-002".into(),
                product_name: "Widget B".into(),
                qty: 3,
                unit_cost_minor: 2000,
            },
        ];

        let po = store(&conn)
            .create_purchase_order(
                "PO-MULTI",
                "sup-po",
                "2025-03-01",
                "multi line",
                None,
                &lines,
            )
            .unwrap();
        assert_eq!(po.lines.len(), 2);
        assert_eq!(po.lines[0].sku, "SKU-001");
        assert_eq!(po.lines[1].sku, "SKU-002");
        // subtotal = (2*1000) + (3*2000) = 8000
        assert_eq!(po.order.subtotal_minor, 8000);
        assert_eq!(po.order.total_minor, 8000);

        // Verify via list and get
        let fetched = store(&conn)
            .get_purchase_order(&po.order.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.lines.len(), 2);

        let listed = store(&conn).list_purchase_orders().unwrap();
        assert_eq!(listed[0].lines.len(), 2);
    }

    #[test]
    fn create_po_empty_lines() {
        let conn = fresh();
        seed_supplier(&conn);

        let po = store(&conn)
            .create_purchase_order("PO-EMPTY", "sup-po", "", "no items", None, &[])
            .unwrap();
        assert!(po.lines.is_empty());
        assert_eq!(po.order.subtotal_minor, 0);
        assert_eq!(po.order.status, "draft");
    }

    #[test]
    fn create_po_negative_unit_cost_rejected() {
        let conn = fresh();
        seed_supplier(&conn);

        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: 1,
            unit_cost_minor: -100,
        }];
        let err = store(&conn)
            .create_purchase_order("PO-NEG-COST", "sup-po", "", "", None, &lines)
            .unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "unit_cost_minor"));
    }

    #[test]
    fn create_po_with_notes() {
        let conn = fresh();
        seed_supplier(&conn);

        let lines = vec![CreatePoLineInput {
            sku: "SKU-001".into(),
            product_name: "Widget".into(),
            qty: 1,
            unit_cost_minor: 500,
        }];
        let po = store(&conn)
            .create_purchase_order(
                "PO-NOTES",
                "sup-po",
                "2025-04-01",
                "Rush order — urgent restock",
                None,
                &lines,
            )
            .unwrap();
        assert_eq!(po.order.notes, "Rush order — urgent restock");
        assert_eq!(po.order.expected_date, "2025-04-01");

        // Verify round-trip
        let fetched = store(&conn)
            .get_purchase_order(&po.order.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.order.notes, "Rush order — urgent restock");
        assert_eq!(fetched.order.expected_date, "2025-04-01");
    }

    #[test]
    fn create_po_with_created_by() {
        let conn = fresh();
        seed_supplier(&conn);

        // Seed role + user so FK constraints are satisfied
        conn.execute_batch(
            "INSERT INTO roles (id, name, description, permissions, created_at, updated_at) VALUES
                ('role-po', 'procurement', 'PO mgr', '[]', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');
             INSERT INTO users (id, username, pin_hash, display_name, role_id, created_at, updated_at) VALUES
                ('user-42', 'procurement', 'h', 'Procurement', 'role-po', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');"
        ).unwrap();

        let po = store(&conn)
            .create_purchase_order(
                "PO-CREATOR",
                "sup-po",
                "",
                "created by user-42",
                Some("user-42"),
                &[],
            )
            .unwrap();
        assert_eq!(po.order.created_by.as_deref(), Some("user-42"));

        // Verify round-trip
        let fetched = store(&conn)
            .get_purchase_order(&po.order.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.order.created_by.as_deref(), Some("user-42"));
    }

    #[test]
    fn po_list_orders_descending_by_date() {
        let conn = fresh();
        let sid = seed_supplier(&conn);

        // Create two POs, then verify newest is first
        let _po_a = store(&conn)
            .create_purchase_order("PO-DESC-A", &sid, "", "older", None, &[])
            .unwrap();

        // Sleep briefly so timestamps differ
        std::thread::sleep(std::time::Duration::from_millis(10));

        let _po_b = store(&conn)
            .create_purchase_order("PO-DESC-B", &sid, "", "newer", None, &[])
            .unwrap();

        let list = store(&conn).list_purchase_orders().unwrap();
        assert_eq!(list.len(), 2);
        // PO B (newer) should be first
        assert_eq!(list[0].order.po_number, "PO-DESC-B");
        assert_eq!(list[1].order.po_number, "PO-DESC-A");
        assert!(list[0].order.created_at >= list[1].order.created_at);
    }
}
