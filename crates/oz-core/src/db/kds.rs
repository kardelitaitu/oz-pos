//! KDS (Kitchen Display System) CRUD — order ticket lifecycle.

use rusqlite::params;

use crate::error::CoreError;
use crate::{
    CreateKdsLineItemInput, CreateKdsOrderInput, KdsLineItem, KdsModifier, KdsOrder, KdsStatus,
};

use super::Store;

impl Store<'_> {
    fn row_to_kds_order(row: &rusqlite::Row) -> rusqlite::Result<KdsOrder> {
        Ok(KdsOrder {
            id: row.get("id")?,
            sale_id: row.get("sale_id")?,
            store_id: row.get("store_id")?,
            target_instance_id: row.get("target_instance_id")?,
            status: row.get("status")?,
            items_summary: row.get("items_summary")?,
            item_count: row.get("item_count")?,
            display_number: row.get("display_number")?,
            received_at: row.get("received_at")?,
            started_at: row.get("started_at")?,
            ready_at: row.get("ready_at")?,
            served_at: row.get("served_at")?,
            prep_time_seconds: row.get("prep_time_seconds")?,
            kitchen_zone: row.get("kitchen_zone")?,
            notes: row.get("notes")?,
            table_number: row.get("table_number")?,
            priority: row.get::<_, i64>("priority")? != 0,
        })
    }

    /// Create a KDS order from input, auto-incrementing the display number per day.
    pub fn create_kds_order(&self, input: CreateKdsOrderInput) -> Result<KdsOrder, CoreError> {
        self.create_kds_order_with_target(input, None)
    }

    /// Create a KDS order and persist the topology-selected target instance.
    pub fn create_kds_order_routed(
        &self,
        input: CreateKdsOrderInput,
        target_instance_id: Option<&str>,
    ) -> Result<KdsOrder, CoreError> {
        let targets = target_instance_id
            .map(str::to_owned)
            .into_iter()
            .collect::<Vec<_>>();
        self.create_kds_order_fanout(input, &targets)
    }

    /// Create one KDS order and attach zero or more delivery targets.
    ///
    /// The order remains unique per sale/zone; fan-out is represented by the
    /// normalized `kds_order_targets` table rather than duplicate orders.
    pub fn create_kds_order_fanout(
        &self,
        input: CreateKdsOrderInput,
        target_instance_ids: &[String],
    ) -> Result<KdsOrder, CoreError> {
        let primary_target = target_instance_ids.first().map(String::as_str);
        let order = self.create_kds_order_with_target(input, primary_target)?;
        for target_instance_id in target_instance_ids {
            self.conn.execute(
                "INSERT OR IGNORE INTO kds_order_targets (kds_order_id, target_instance_id) VALUES (?1, ?2)",
                params![order.id, target_instance_id],
            )?;
        }
        Ok(order)
    }

    fn create_kds_order_with_target(
        &self,
        input: CreateKdsOrderInput,
        target_instance_id: Option<&str>,
    ) -> Result<KdsOrder, CoreError> {
        if input.sale_id.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "sale_id",
                message: "sale_id must not be empty".into(),
            });
        }
        if input.items_summary.trim().is_empty() {
            return Err(CoreError::Validation {
                field: "items_summary",
                message: "items_summary must not be empty".into(),
            });
        }
        if input.item_count <= 0 {
            return Err(CoreError::Validation {
                field: "item_count",
                message: "item_count must be positive".into(),
            });
        }

        let id = uuid::Uuid::now_v7().to_string();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let tx = self.conn.unchecked_transaction()?;

        // Upsert the daily counter.
        tx.execute(
            "INSERT INTO kds_daily_counters (date, counter) VALUES (?1, 1)
             ON CONFLICT(date) DO UPDATE SET counter = counter + 1",
            params![today],
        )?;

        // Read back the counter.
        let display_number: i64 = tx.query_row(
            "SELECT counter FROM kds_daily_counters WHERE date = ?1",
            params![today],
            |row| row.get(0),
        )?;

        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        tx.execute(
            "INSERT INTO kds_orders (id, sale_id, store_id, target_instance_id, status, items_summary, item_count,
                                     display_number, received_at, kitchen_zone, notes, table_number, priority)
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                id,
                input.sale_id,
                input.store_id,
                target_instance_id,
                input.items_summary,
                input.item_count,
                display_number,
                now,
                input.kitchen_zone,
                input.notes,
                input.table_number,
                input.priority,
            ],
        )?;

        tx.commit()?;

        self.get_kds_order(&id)?.ok_or_else(|| {
            CoreError::Internal("KDS order was inserted but could not be read back".into())
        })
    }

    /// List KDS orders, optionally filtered by status. Ordered by received_at DESC.
    pub fn list_kds_orders(&self, status_filter: Option<&str>) -> Result<Vec<KdsOrder>, CoreError> {
        let mut sql = String::from(
            "SELECT id, sale_id, store_id, target_instance_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes, table_number, priority
             FROM kds_orders",
        );
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = if let Some(s) = status_filter {
            sql.push_str(" WHERE status = ?1");
            vec![Box::new(s.to_owned())]
        } else {
            vec![]
        };
        sql.push_str(" ORDER BY received_at DESC");

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_kds_order)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// List orders visible to one KDS workspace instance.
    ///
    /// Legacy orders without a target remain visible to every instance.
    pub fn list_kds_orders_for_instance(
        &self,
        status_filter: Option<&str>,
        instance_id: &str,
    ) -> Result<Vec<KdsOrder>, CoreError> {
        let orders = self.list_kds_orders(status_filter)?;
        self.filter_orders_for_instance(orders, instance_id)
    }

    fn filter_orders_for_instance(
        &self,
        orders: Vec<KdsOrder>,
        instance_id: &str,
    ) -> Result<Vec<KdsOrder>, CoreError> {
        orders
            .into_iter()
            .map(|order| {
                Ok(self
                    .order_visible_to_instance(&order, instance_id)?
                    .then_some(order))
            })
            .filter_map(|result| result.transpose())
            .collect()
    }

    /// Return an order only when it is visible to the requested KDS instance.
    ///
    /// A targeted order is hidden from every other instance, including direct
    /// lookups; legacy untargeted orders remain visible for compatibility.
    pub fn get_kds_order_for_instance(
        &self,
        id: &str,
        instance_id: &str,
    ) -> Result<Option<KdsOrder>, CoreError> {
        let Some(order) = self.get_kds_order(id)? else {
            return Ok(None);
        };
        Ok(self
            .order_visible_to_instance(&order, instance_id)?
            .then_some(order))
    }

    /// Require that a KDS order belongs to the current instance.
    ///
    /// Inaccessible orders deliberately return `NotFound` rather than an
    /// authorization detail so a direct IPC caller cannot probe another
    /// display's ticket IDs.
    pub fn ensure_kds_order_visible_to_instance(
        &self,
        id: &str,
        instance_id: &str,
    ) -> Result<(), CoreError> {
        if self.get_kds_order_for_instance(id, instance_id)?.is_some() {
            Ok(())
        } else {
            Err(CoreError::NotFound {
                entity: "kds_order",
                id: id.to_owned(),
            })
        }
    }

    fn order_visible_to_instance(
        &self,
        order: &KdsOrder,
        instance_id: &str,
    ) -> Result<bool, CoreError> {
        let target_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM kds_order_targets WHERE kds_order_id = ?1",
            params![order.id],
            |row| row.get(0),
        )?;
        if target_count > 0 {
            let matching_target: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM kds_order_targets WHERE kds_order_id = ?1 AND target_instance_id = ?2",
                params![order.id, instance_id],
                |row| row.get(0),
            )?;
            return Ok(matching_target > 0);
        }
        Ok(order
            .target_instance_id
            .as_deref()
            .is_none_or(|target| target == instance_id))
    }

    /// Get a single KDS order by its id.
    pub fn get_kds_order(&self, id: &str) -> Result<Option<KdsOrder>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sale_id, store_id, target_instance_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes, table_number, priority
             FROM kds_orders WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], Self::row_to_kds_order);
        match result {
            Ok(order) => Ok(Some(order)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a KDS order by the originating sale id.
    pub fn get_kds_order_by_sale(&self, sale_id: &str) -> Result<Option<KdsOrder>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sale_id, store_id, target_instance_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes, table_number, priority
             FROM kds_orders WHERE sale_id = ?1",
        )?;
        let result = stmt.query_row(params![sale_id], Self::row_to_kds_order);
        match result {
            Ok(order) => Ok(Some(order)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update the items (summary + count) on an existing KDS order.
    ///
    /// Used when FOH adds items to an order mid-preparation, or when
    /// kitchen staff correct the items shown on a ticket.
    ///
    /// When `input.line_items` is `Some`, the existing kds_line_items
    /// for this order are deleted and replaced with the new ones, and
    /// the summary/count are re-derived from the structured data.
    /// Update an order only when it belongs to the current KDS instance.
    pub fn update_kds_order_items_for_instance(
        &self,
        input: crate::UpdateKdsOrderItemsInput,
        instance_id: &str,
    ) -> Result<KdsOrder, CoreError> {
        self.ensure_kds_order_visible_to_instance(&input.id, instance_id)?;
        self.update_kds_order_items(input)
    }

    /// Update an order status only when it belongs to the current KDS instance.
    pub fn update_kds_status_for_instance(
        &self,
        id: &str,
        new_status: &str,
        instance_id: &str,
    ) -> Result<KdsOrder, CoreError> {
        self.ensure_kds_order_visible_to_instance(id, instance_id)?;
        self.update_kds_status(id, new_status)
    }

    /// Update an order's summary and optionally replace its structured line items.
    pub fn update_kds_order_items(
        &self,
        input: crate::UpdateKdsOrderItemsInput,
    ) -> Result<KdsOrder, CoreError> {
        // ── Resolve final summary/count ────────────────────────────
        let (final_summary, final_count) = if let Some(ref line_items) = input.line_items {
            if line_items.is_empty() {
                return Err(CoreError::Validation {
                    field: "line_items",
                    message: "line_items must not be empty when provided".into(),
                });
            }
            Store::derive_kds_summary(line_items)
        } else {
            if input.items_summary.trim().is_empty() {
                return Err(CoreError::Validation {
                    field: "items_summary",
                    message: "items_summary must not be empty".into(),
                });
            }
            if input.item_count <= 0 {
                return Err(CoreError::Validation {
                    field: "item_count",
                    message: "item_count must be positive".into(),
                });
            }
            (input.items_summary.clone(), input.item_count)
        };

        let tx = self.conn.unchecked_transaction()?;

        // ── Replace line items when provided ───────────────────────
        if let Some(ref line_items) = input.line_items {
            tx.execute(
                "DELETE FROM kds_line_items WHERE kds_order_id = ?1",
                rusqlite::params![input.id],
            )?;
            self.create_kds_line_items_in_tx(&tx, &input.id, line_items)?;
        }

        tx.execute(
            "UPDATE kds_orders SET items_summary = ?1, item_count = ?2 WHERE id = ?3",
            rusqlite::params![final_summary, final_count, input.id],
        )?;

        tx.commit()?;

        self.get_kds_order(&input.id)?
            .ok_or_else(|| CoreError::NotFound {
                entity: "kds_order",
                id: input.id,
            })
    }

    /// Update the status of a KDS order. Automatically sets the corresponding
    /// timestamp field (started_at, ready_at, served_at) based on the new status.
    pub fn update_kds_status(&self, id: &str, new_status: &str) -> Result<KdsOrder, CoreError> {
        let valid = KdsStatus::from_str(new_status).is_some();
        if !valid {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("invalid KDS status: {new_status}"),
            });
        }

        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        let timestamp_col = match new_status {
            "preparing" => "started_at",
            "ready" => "ready_at",
            "served" => "served_at",
            _ => "",
        };

        if timestamp_col.is_empty() {
            self.conn.execute(
                "UPDATE kds_orders SET status = ?1 WHERE id = ?2",
                params![new_status, id],
            )?;
        } else {
            let sql =
                format!("UPDATE kds_orders SET status = ?1, {timestamp_col} = ?2 WHERE id = ?3");
            self.conn.execute(&sql, params![new_status, now, id])?;
        }

        self.get_kds_order(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "kds_order",
            id: id.to_owned(),
        })
    }

    /// Get the kitchen queue: orders with status 'pending', 'preparing', or 'ready',
    /// ordered by status priority then received_at ASC (oldest first).
    ///
    /// When `zone_filter` is `Some(zone)`, only orders with that `kitchen_zone`
    /// value are returned. Pass `Some("")` to match orders with no zone assigned.
    /// When `None`, all orders are returned (no zone filtering).
    pub fn get_kds_queue(&self, zone_filter: Option<&str>) -> Result<Vec<KdsOrder>, CoreError> {
        let mut sql = String::from(
            "SELECT id, sale_id, store_id, target_instance_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes, table_number, priority
             FROM kds_orders
             WHERE status IN ('pending', 'preparing', 'ready')",
        );

        let params: Vec<Box<dyn rusqlite::types::ToSql>> = if let Some(zone) = zone_filter {
            if zone.is_empty() {
                sql.push_str(" AND (kitchen_zone IS NULL OR kitchen_zone = '')");
                vec![]
            } else {
                sql.push_str(" AND kitchen_zone = ?1");
                vec![Box::new(zone.to_owned())]
            }
        } else {
            vec![]
        };

        sql.push_str(
            " ORDER BY
                CASE status
                    WHEN 'pending' THEN 1
                    WHEN 'preparing' THEN 2
                    WHEN 'ready' THEN 3
                END,
                received_at ASC",
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_kds_order)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get the active queue visible to one KDS workspace instance.
    pub fn get_kds_queue_for_instance(
        &self,
        zone_filter: Option<&str>,
        instance_id: &str,
    ) -> Result<Vec<KdsOrder>, CoreError> {
        let orders = self.get_kds_queue(zone_filter)?;
        self.filter_orders_for_instance(orders, instance_id)
    }

    /// Complete a sale to KDS orders: creates one KDS ticket per kitchen zone
    /// from a completed sale for items whose product type is `restaurant` or `both`.
    ///
    /// When `store_id` is provided (from the caller's `SessionContext`), the
    /// resulting KDS orders are tagged with that store for defense-in-depth
    /// filtering on KDS tablets (ADR #8).
    ///
    /// Returns an empty `Vec` when the sale has no restaurant-eligible items.
    pub fn complete_sale_to_kds(
        &self,
        sale_id: &str,
        store_id: Option<&str>,
    ) -> Result<Vec<KdsOrder>, CoreError> {
        self.complete_sale_to_kds_routed(sale_id, store_id, None)
    }

    /// Complete a sale to KDS orders routed to one topology-selected instance.
    pub fn complete_sale_to_kds_routed(
        &self,
        sale_id: &str,
        store_id: Option<&str>,
        target_instance_id: Option<&str>,
    ) -> Result<Vec<KdsOrder>, CoreError> {
        let targets = target_instance_id
            .map(str::to_owned)
            .into_iter()
            .collect::<Vec<_>>();
        self.complete_sale_to_kds_fanout(sale_id, store_id, &targets)
    }

    /// Complete a sale and deliver each zone ticket to every target instance.
    ///
    /// One `kds_orders` row is created per sale/zone. The normalized target
    /// table carries fan-out delivery without violating `sale_id` uniqueness.
    pub fn complete_sale_to_kds_fanout(
        &self,
        sale_id: &str,
        store_id: Option<&str>,
        target_instance_ids: &[String],
    ) -> Result<Vec<KdsOrder>, CoreError> {
        let sale = self.get_sale(sale_id)?.ok_or_else(|| CoreError::NotFound {
            entity: "sale",
            id: sale_id.to_owned(),
        })?;

        // Keep only lines whose product is restaurant or both.
        let kds_lines: Vec<_> = sale
            .lines
            .iter()
            .filter(|l| {
                self.product_type_by_sku(&l.sku)
                    .ok()
                    .flatten()
                    .is_some_and(|pt| pt == "restaurant" || pt == "both")
            })
            .collect();

        if kds_lines.is_empty() {
            return Ok(vec![]);
        }

        // Group eligible lines by kitchen zone.
        let mut by_zone: std::collections::BTreeMap<Option<String>, Vec<&crate::SaleLine>> =
            std::collections::BTreeMap::new();
        for line in &kds_lines {
            let zone = self
                .product_kitchen_zone_by_sku(&line.sku)
                .ok()
                .flatten()
                .filter(|z| !z.is_empty());
            by_zone.entry(zone).or_default().push(line);
        }

        // Look up the table name assigned to this sale (TODO 1b).
        let table_number: Option<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT name FROM tables WHERE active_sale_id = ?1")?;
            match stmt.query_row(params![sale_id], |row| row.get::<_, String>(0)) {
                Ok(name) => Some(name),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(e) => return Err(e.into()),
            }
        };

        let mut orders = Vec::with_capacity(by_zone.len());
        for (zone, lines) in by_zone {
            // Build structured line items with course + modifier data (TODO 2a).
            let structured_items: Vec<CreateKdsLineItemInput> = lines
                .iter()
                .map(|l| {
                    let display_name = self
                        .product_name_by_sku(&l.sku)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| l.sku.clone());

                    // Parse modifiers_json from the sale line.
                    let modifiers: Vec<KdsModifier> = l
                        .modifiers_json
                        .as_deref()
                        .filter(|j| !j.is_empty())
                        .and_then(|j| serde_json::from_str(j).ok())
                        .unwrap_or_default();

                    CreateKdsLineItemInput {
                        sku: l.sku.clone(),
                        display_name,
                        qty: l.qty,
                        course: l.course.clone(),
                        modifiers,
                    }
                })
                .collect();

            let (items_summary, item_count) = Store::derive_kds_summary(&structured_items);

            let order = self.create_kds_order_fanout(
                CreateKdsOrderInput {
                    sale_id: sale_id.to_owned(),
                    store_id: store_id.map(|s| s.to_owned()),
                    items_summary,
                    item_count,
                    kitchen_zone: zone,
                    notes: String::new(),
                    table_number: table_number.clone(),
                    priority: false,
                },
                target_instance_ids,
            )?;

            // Create the structured line items in the new kds_line_items table.
            self.create_kds_line_items(&order.id, &structured_items)?;

            orders.push(order);
        }

        Ok(orders)
    }

    fn product_type_by_sku(&self, sku: &str) -> Result<Option<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT product_type FROM products WHERE sku = ?1")?;
        let result = stmt.query_row(params![sku], |row| row.get::<_, String>(0));
        match result {
            Ok(pt) => Ok(Some(pt)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn product_name_by_sku(&self, sku: &str) -> Result<Option<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM products WHERE sku = ?1")?;
        let result = stmt.query_row(params![sku], |row| row.get::<_, String>(0));
        match result {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn product_kitchen_zone_by_sku(&self, sku: &str) -> Result<Option<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT kitchen_zone FROM products WHERE sku = ?1")?;
        let result = stmt.query_row(params![sku], |row| row.get::<_, Option<String>>(0));
        match result {
            Ok(zone) => Ok(zone),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ── KDS line items (TODO 2a) ────────────────────────────────────

    fn row_to_kds_line_item(row: &rusqlite::Row) -> rusqlite::Result<KdsLineItem> {
        let modifiers_json: Option<String> = row.get("modifiers_json")?;
        let modifiers: Vec<KdsModifier> = match modifiers_json {
            Some(json) if !json.is_empty() => serde_json::from_str(&json).unwrap_or_default(),
            _ => vec![],
        };
        Ok(KdsLineItem {
            id: row.get("id")?,
            kds_order_id: row.get("kds_order_id")?,
            sku: row.get("sku")?,
            display_name: row.get("display_name")?,
            qty: row.get("qty")?,
            course: row.get("course")?,
            modifiers,
            line_position: row.get("line_position")?,
            item_status: row.get("item_status")?,
            started_at: row.get("started_at")?,
            ready_at: row.get("ready_at")?,
            served_at: row.get("served_at")?,
            created_at: row.get("created_at")?,
        })
    }

    /// Create KDS line items for an order.
    pub fn create_kds_line_items(
        &self,
        order_id: &str,
        items: &[CreateKdsLineItemInput],
    ) -> Result<Vec<KdsLineItem>, CoreError> {
        if items.is_empty() {
            return Ok(vec![]);
        }
        let tx = self.conn.unchecked_transaction()?;
        let result = self.create_kds_line_items_in_tx(&tx, order_id, items)?;
        tx.commit()?;
        Ok(result)
    }

    /// Internal: Insert line items inside an existing transaction.
    ///
    /// Used by both `create_kds_line_items` (for initial creation) and
    /// `update_kds_order_items` (for replacement after deletion).
    fn create_kds_line_items_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        order_id: &str,
        items: &[CreateKdsLineItemInput],
    ) -> Result<Vec<KdsLineItem>, CoreError> {
        if items.is_empty() {
            return Ok(vec![]);
        }
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let mut ids = Vec::with_capacity(items.len());

        for (i, item) in items.iter().enumerate() {
            let id = uuid::Uuid::now_v7().to_string();
            let modifiers_json = if item.modifiers.is_empty() {
                None
            } else {
                Some(
                    serde_json::to_string(&item.modifiers).map_err(|e| CoreError::Validation {
                        field: "modifiers",
                        message: format!("serializing modifiers: {e}"),
                    })?,
                )
            };
            tx.execute(
                "INSERT INTO kds_line_items
                    (id, kds_order_id, sku, display_name, qty, course, modifiers_json,
                     line_position, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    order_id,
                    item.sku,
                    item.display_name,
                    item.qty,
                    item.course,
                    modifiers_json,
                    i as i64,
                    now,
                ],
            )?;
            ids.push(id);
        }

        // Read back the inserted items.
        let placeholders: Vec<String> = (0..ids.len()).map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id, kds_order_id, sku, display_name, qty, course, modifiers_json,
                    line_position, item_status, started_at, ready_at, served_at, created_at
             FROM kds_line_items WHERE id IN ({})
             ORDER BY line_position",
            placeholders.join(",")
        );
        let mut stmt = tx.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> = ids
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(params_refs.as_slice(), Self::row_to_kds_line_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Get line items only when the parent order belongs to the current instance.
    pub fn get_kds_order_lines_for_instance(
        &self,
        order_id: &str,
        instance_id: &str,
    ) -> Result<Vec<KdsLineItem>, CoreError> {
        self.ensure_kds_order_visible_to_instance(order_id, instance_id)?;
        self.get_kds_order_lines(order_id)
    }

    /// Get all line items for a KDS order, ordered by course then position.
    pub fn get_kds_order_lines(&self, order_id: &str) -> Result<Vec<KdsLineItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kds_order_id, sku, display_name, qty, course, modifiers_json,
                    line_position, item_status, started_at, ready_at, served_at, created_at
             FROM kds_line_items
             WHERE kds_order_id = ?1
             ORDER BY
                 CASE course
                     WHEN 'appetizer' THEN 0
                     WHEN 'main' THEN 1
                     WHEN 'side' THEN 2
                     WHEN 'dessert' THEN 3
                     WHEN 'beverage' THEN 4
                     ELSE 99
                 END,
                 line_position",
        )?;
        let rows = stmt.query_map(params![order_id], Self::row_to_kds_line_item)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Update the status of a single KDS line item. Automatically sets
    /// the corresponding timestamp (started_at, ready_at, served_at)
    /// based on the new status.
    /// Update a line item only when its parent order belongs to the current
    /// KDS instance.
    pub fn update_kds_line_item_status_for_instance(
        &self,
        item_id: &str,
        new_status: &str,
        instance_id: &str,
    ) -> Result<KdsLineItem, CoreError> {
        let order_id: String = self
            .conn
            .query_row(
                "SELECT kds_order_id FROM kds_line_items WHERE id = ?1",
                params![item_id],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound {
                    entity: "kds_line_item",
                    id: item_id.to_owned(),
                },
                other => CoreError::Db(other),
            })?;
        self.ensure_kds_order_visible_to_instance(&order_id, instance_id)?;
        self.update_kds_line_item_status(item_id, new_status)
    }

    /// Update a line item's status and its corresponding workflow timestamp.
    pub fn update_kds_line_item_status(
        &self,
        item_id: &str,
        new_status: &str,
    ) -> Result<KdsLineItem, CoreError> {
        if KdsStatus::from_str(new_status).is_none() {
            return Err(CoreError::Validation {
                field: "item_status",
                message: format!("invalid KDS line item status: {new_status}"),
            });
        }

        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        let timestamp_col = match new_status {
            "preparing" => "started_at",
            "ready" => "ready_at",
            "served" => "served_at",
            _ => "",
        };

        let rows = if timestamp_col.is_empty() {
            self.conn.execute(
                "UPDATE kds_line_items SET item_status = ?1 WHERE id = ?2",
                params![new_status, item_id],
            )?
        } else {
            let sql = format!(
                "UPDATE kds_line_items SET item_status = ?1, {timestamp_col} = ?2 WHERE id = ?3"
            );
            self.conn.execute(&sql, params![new_status, now, item_id])?
        };

        if rows == 0 {
            return Err(CoreError::NotFound {
                entity: "kds_line_item",
                id: item_id.to_owned(),
            });
        }

        let mut stmt = self.conn.prepare(
            "SELECT id, kds_order_id, sku, display_name, qty, course, modifiers_json,
                    line_position, item_status, started_at, ready_at, served_at, created_at
             FROM kds_line_items WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![item_id], Self::row_to_kds_line_item);
        match result {
            Ok(item) => Ok(item),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(CoreError::NotFound {
                entity: "kds_line_item",
                id: item_id.to_owned(),
            }),
            Err(e) => Err(e.into()),
        }
    }

    /// Derive the flat items_summary and item_count from structured line items.
    pub fn derive_kds_summary(items: &[CreateKdsLineItemInput]) -> (String, i64) {
        let summary = items
            .iter()
            .map(|i| {
                if i.qty > 1 {
                    format!("{} x{}", i.display_name, i.qty)
                } else {
                    i.display_name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let count: i64 = items.iter().map(|i| i.qty).sum();
        (summary, count)
    }
}

#[cfg(test)]
#[path = "kds_tests.rs"]
mod tests;
