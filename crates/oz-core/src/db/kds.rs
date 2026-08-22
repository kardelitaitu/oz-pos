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
        let tx = self.conn.unchecked_transaction()?;
        let order = self.create_kds_order_fanout_in_tx(&tx, input, target_instance_ids)?;
        tx.commit()?;
        Ok(order)
    }

    /// Same as [`Store::create_kds_order_fanout`] but inside a caller-owned
    /// transaction, so a multi-zone fanout commits atomically (a failure on
    /// one zone rolls back the whole set instead of leaving partial tickets).
    fn create_kds_order_fanout_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        input: CreateKdsOrderInput,
        target_instance_ids: &[String],
    ) -> Result<KdsOrder, CoreError> {
        let primary_target = target_instance_ids.first().map(String::as_str);
        let order = self.create_kds_order_with_target_in_tx(tx, input, primary_target)?;
        for target_instance_id in target_instance_ids {
            tx.execute(
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
        let tx = self.conn.unchecked_transaction()?;
        let order = self.create_kds_order_with_target_in_tx(&tx, input, target_instance_id)?;
        tx.commit()?;
        Ok(order)
    }

    fn create_kds_order_with_target_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
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
        let store_key = input.store_id.as_deref().unwrap_or("");

        // Upsert the daily counter, keyed by (date, store) so each store's
        // tickets start at #1 daily. '' = legacy single-store rows.
        tx.execute(
            "INSERT INTO kds_daily_counters (date, store_id, counter) VALUES (?1, ?2, 1)
             ON CONFLICT(date, store_id) DO UPDATE SET counter = counter + 1",
            params![today, store_key],
        )?;

        // Read back the counter.
        let display_number: i64 = tx.query_row(
            "SELECT counter FROM kds_daily_counters WHERE date = ?1 AND store_id = ?2",
            params![today, store_key],
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

        // Read back (within the caller's transaction so the whole fanout
        // commits atomically).
        let mut stmt = tx.prepare(
            "SELECT id, sale_id, store_id, target_instance_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes, table_number, priority
             FROM kds_orders WHERE id = ?1",
        )?;
        let order = stmt.query_row(params![id], Self::row_to_kds_order)?;
        Ok(order)
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

    /// Get the KDS orders originating from one sale (one per kitchen zone
    /// when the sale's items span multiple zones).
    pub fn get_kds_orders_by_sale(&self, sale_id: &str) -> Result<Vec<KdsOrder>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sale_id, store_id, target_instance_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes, table_number, priority
             FROM kds_orders WHERE sale_id = ?1",
        )?;
        let rows = stmt.query_map(params![sale_id], Self::row_to_kds_order)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
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
    ///
    /// Transitions are FORWARD-ONLY: `pending → preparing → ready → served`,
    /// plus `cancelled` from any non-terminal state. A regression (e.g. a
    /// stale offline replay moving a served order back to preparing) is
    /// rejected with a `Validation` error so ticket timestamps and the
    /// kitchen queue are never corrupted. `served` and `cancelled` are
    /// terminal. Reaching `served` computes `prep_time_seconds`
    /// (`served_at − started_at`).
    pub fn update_kds_status(&self, id: &str, new_status: &str) -> Result<KdsOrder, CoreError> {
        let valid = KdsStatus::from_str(new_status).is_some();
        if !valid {
            return Err(CoreError::Validation {
                field: "status",
                message: format!("invalid KDS status: {new_status}"),
            });
        }

        // ── State machine: reject regressions + transitions from terminal
        //    states before touching any row (no partial writes).
        let current = self.get_kds_order(id)?.ok_or_else(|| CoreError::NotFound {
            entity: "kds_order",
            id: id.to_owned(),
        })?;
        let allowed = |from: &str, to: &str| match (from, to) {
            // Forward progression.
            ("pending", "preparing") | ("preparing", "ready") | ("ready", "served") => true,
            // Cancellation from any active state.
            ("pending", "cancelled") | ("preparing", "cancelled") | ("ready", "cancelled") => true,
            // No-op (idempotent replay of the current state).
            (from, to) if from == to => true,
            // Everything else (regressions, terminal-state moves) is invalid.
            _ => false,
        };
        if !allowed(&current.status, new_status) {
            return Err(CoreError::Validation {
                field: "status",
                message: format!(
                    "invalid KDS status transition: {} -> {new_status}",
                    current.status
                ),
            });
        }

        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        // Compute prep time when the ticket is served: served_at - started_at.
        let prep_time = if new_status == "served" {
            match (current.started_at.as_deref(), Some(now.as_str())) {
                (Some(started), Some(served)) => {
                    let parse = |ts: &str| {
                        chrono::DateTime::parse_from_rfc3339(ts)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    };
                    match (parse(started), parse(served)) {
                        (Some(start), Some(served_dt)) => {
                            let secs = served_dt.signed_duration_since(start).num_seconds();
                            std::cmp::max(0, secs)
                        }
                        _ => current.prep_time_seconds,
                    }
                }
                _ => current.prep_time_seconds,
            }
        } else {
            current.prep_time_seconds
        };

        let timestamp_col = match new_status {
            "preparing" => "started_at",
            "ready" => "ready_at",
            "served" => "served_at",
            _ => "",
        };

        if timestamp_col.is_empty() {
            self.conn.execute(
                "UPDATE kds_orders SET status = ?1, prep_time_seconds = ?2 WHERE id = ?3",
                params![new_status, prep_time, id],
            )?;
        } else {
            let sql = format!(
                "UPDATE kds_orders SET status = ?1, {timestamp_col} = ?2, prep_time_seconds = ?3 WHERE id = ?4"
            );
            self.conn
                .execute(&sql, params![new_status, now, prep_time, id])?;
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
        // One transaction for the WHOLE fanout: a failure on any zone
        // (duplicate sale/zone, line-item error) rolls back every ticket
        // created so far, so the kitchen never sees a partial set.
        let tx = self.conn.unchecked_transaction()?;
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

            let order = self.create_kds_order_fanout_in_tx(
                &tx,
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
            self.create_kds_line_items_in_tx(&tx, &order.id, &structured_items)?;

            orders.push(order);
        }
        tx.commit()?;

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

    /// Look up the `kitchen_zone` for a product by SKU.
    ///
    /// Used by the multi-KDS routing engine to map line-item SKUs to
    /// kitchen zones, which in turn map to device `station_ids`.
    pub fn product_kitchen_zone_by_sku(&self, sku: &str) -> Result<Option<String>, CoreError> {
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
    ///
    /// Forward-only transitions (pending → preparing → ready → served, plus
    /// cancelled from any active state) — mirrors the order-level state
    /// machine so a stale offline replay cannot regress a line item.
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

        // Read the current status before mutating (no partial writes on
        // regression).
        let current_status: String = self
            .conn
            .query_row(
                "SELECT item_status FROM kds_line_items WHERE id = ?1",
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
        let allowed = |from: &str, to: &str| match (from, to) {
            ("pending", "preparing") | ("preparing", "ready") | ("ready", "served") => true,
            ("pending", "cancelled") | ("preparing", "cancelled") | ("ready", "cancelled") => true,
            (from, to) if from == to => true,
            _ => false,
        };
        if !allowed(&current_status, new_status) {
            return Err(CoreError::Validation {
                field: "item_status",
                message: format!(
                    "invalid KDS line item status transition: {current_status} -> {new_status}"
                ),
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

// ── KDS Device Management ───────────────────────────────────────

use crate::kds::{KdsConnectionStatus, KdsDevice, RegisterKdsDeviceInput};

impl Store<'_> {
    /// Register a new KDS device.
    ///
    /// Returns a `Validation` error if a device with the same name already
    /// exists under the same Restaurant POS.
    pub fn register_kds_device(
        &self,
        input: RegisterKdsDeviceInput,
    ) -> Result<KdsDevice, CoreError> {
        // Enforce unique name per restaurant POS.
        let existing: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM kds_devices WHERE name = ?1 AND restaurant_pos_id = ?2",
            params![input.name, input.restaurant_pos_id],
            |row| row.get(0),
        )?;
        if existing > 0 {
            return Err(CoreError::Validation {
                field: "name",
                message: format!(
                    "device name '{}' already exists for restaurant POS '{}'",
                    input.name, input.restaurant_pos_id
                ),
            });
        }

        let id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let station_ids_json = serde_json::to_string(&input.station_ids)
            .map_err(|e| CoreError::Internal(format!("serialize station_ids: {e}")))?;

        self.conn.execute(
            "INSERT INTO kds_devices (id, name, restaurant_pos_id, station_ids, pairing_token_hash, pairing_expires_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                id,
                input.name,
                input.restaurant_pos_id,
                station_ids_json,
                input.pairing_token_hash,
                input.pairing_expires_at,
                now,
            ],
        )?;

        Ok(KdsDevice {
            id,
            name: input.name,
            restaurant_pos_id: input.restaurant_pos_id,
            station_ids: input.station_ids,
            is_active: true,
            last_seen_at: None,
            connection_status: KdsConnectionStatus::Disconnected,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Validate a pairing token against a device's stored hash and expiry.
    ///
    /// Returns `Ok(true)` if the token hash matches AND the token has not
    /// expired. Returns `Ok(false)` if the device is not found.
    /// Returns `Err` for expired tokens or hash mismatches.
    pub fn validate_pairing_token(
        &self,
        token_hash: &str,
        device_id: &str,
    ) -> Result<bool, CoreError> {
        // Query the pairing fields directly (not exposed on domain struct).
        let result: Result<(String, String), _> = self.conn.query_row(
            "SELECT pairing_token_hash, pairing_expires_at FROM kds_devices WHERE id = ?1",
            params![device_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        let (stored_hash, expires_at) = match result {
            Ok(pair) => pair,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e.into()),
        };

        // Check hash match.
        if stored_hash != token_hash {
            return Err(CoreError::Validation {
                field: "token_hash",
                message: "pairing token hash mismatch".into(),
            });
        }

        // Check expiry.
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&expires_at)
            && chrono::Utc::now() > expires
        {
            return Err(CoreError::Validation {
                field: "pairing_expires_at",
                message: "pairing token has expired".into(),
            });
        }

        Ok(true)
    }

    /// Retrieve a KDS device by ID.
    pub fn get_kds_device(&self, id: &str) -> Result<Option<KdsDevice>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, restaurant_pos_id, station_ids, is_active, last_seen_at, connection_status, created_at, updated_at
             FROM kds_devices WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(self.row_to_kds_device(row)?)),
            None => Ok(None),
        }
    }

    /// List all KDS devices for a Restaurant POS.
    pub fn list_kds_devices_for_restaurant(
        &self,
        restaurant_pos_id: &str,
    ) -> Result<Vec<KdsDevice>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, restaurant_pos_id, station_ids, is_active, last_seen_at, connection_status, created_at, updated_at
             FROM kds_devices WHERE restaurant_pos_id = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map(params![restaurant_pos_id], |row| {
            Ok(KdsDeviceRow {
                id: row.get("id")?,
                name: row.get("name")?,
                restaurant_pos_id: row.get("restaurant_pos_id")?,
                station_ids: row.get("station_ids")?,
                is_active: row.get::<_, i64>("is_active")? != 0,
                last_seen_at: row.get("last_seen_at")?,
                connection_status: row.get("connection_status")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;
        rows.map(|r| {
            let row = r?;
            self.row_from_kds_device_row(row)
        })
        .collect()
    }

    /// Update a KDS device's connection status.
    pub fn update_kds_device_status(
        &self,
        id: &str,
        status: KdsConnectionStatus,
    ) -> Result<(), CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let last_seen = if status == KdsConnectionStatus::Connected {
            Some(now.clone())
        } else {
            None
        };
        let affected = self.conn.execute(
            "UPDATE kds_devices SET connection_status = ?1, last_seen_at = ?2, updated_at = ?3 WHERE id = ?4",
            params![status.as_str(), last_seen, now, id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "kds_device",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    /// Deactivate a KDS device.
    pub fn deactivate_kds_device(&self, id: &str) -> Result<(), CoreError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let affected = self.conn.execute(
            "UPDATE kds_devices SET is_active = 0, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound {
                entity: "kds_device",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    fn row_to_kds_device(&self, row: &rusqlite::Row) -> rusqlite::Result<KdsDevice> {
        let station_ids_str: String = row.get("station_ids")?;
        let station_ids: Vec<String> = serde_json::from_str(&station_ids_str).unwrap_or_default();
        let status_str: String = row.get("connection_status")?;
        Ok(KdsDevice {
            id: row.get("id")?,
            name: row.get("name")?,
            restaurant_pos_id: row.get("restaurant_pos_id")?,
            station_ids,
            is_active: row.get::<_, i64>("is_active")? != 0,
            last_seen_at: row.get("last_seen_at")?,
            connection_status: KdsConnectionStatus::parse_db(&status_str)
                .unwrap_or(KdsConnectionStatus::Disconnected),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn row_from_kds_device_row(&self, row: KdsDeviceRow) -> Result<KdsDevice, CoreError> {
        let station_ids: Vec<String> = serde_json::from_str(&row.station_ids).unwrap_or_default();
        Ok(KdsDevice {
            id: row.id,
            name: row.name,
            restaurant_pos_id: row.restaurant_pos_id,
            station_ids,
            is_active: row.is_active,
            last_seen_at: row.last_seen_at,
            connection_status: KdsConnectionStatus::parse_db(&row.connection_status)
                .unwrap_or(KdsConnectionStatus::Disconnected),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Intermediate row type for query_map closures.
struct KdsDeviceRow {
    id: String,
    name: String,
    restaurant_pos_id: String,
    station_ids: String,
    is_active: bool,
    last_seen_at: Option<String>,
    connection_status: String,
    created_at: String,
    updated_at: String,
}

// ── Order Acknowledgment ─────────────────────────────────────────

impl Store<'_> {
    /// Acknowledge a KDS order — the device accepted the ticket and started
    /// prep, so the order advances pending → preparing. Uses an
    /// `UPDATE WHERE status = 'pending'` pattern for optimistic locking:
    /// only one device can win the race. Returns `Ok(true)` on success,
    /// `Ok(false)` if another device already acknowledged it.
    pub fn ack_kds_order(&self, order_id: &str, device_id: &str) -> Result<bool, CoreError> {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let affected = self.conn.execute(
            "UPDATE kds_orders SET status = 'preparing', started_at = ?1,
             acked_by_device = ?2, acked_at = ?1
             WHERE id = ?3 AND status = 'pending'",
            params![now, device_id, order_id],
        )?;
        Ok(affected > 0)
    }
}

// ── KDS Event Replay & Cleanup ──────────────────────────────────

impl Store<'_> {
    /// Replay KDS orders created or updated since a given ISO-8601 timestamp.
    ///
    /// Used by KDS devices on reconnection to catch up with missed events.
    /// Returns orders whose `received_at` is strictly after `since`, ordered
    /// by `received_at ASC` (oldest first, so the device processes them
    /// in the correct sequence).
    pub fn replay_kds_orders_since(
        &self,
        since: &str,
        status_filter: Option<&str>,
    ) -> Result<Vec<KdsOrder>, CoreError> {
        let mut sql = String::from(
            "SELECT id, sale_id, store_id, target_instance_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes, table_number, priority
             FROM kds_orders WHERE received_at > ?1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(since.to_owned())];
        if let Some(s) = status_filter {
            sql.push_str(" AND status = ?2");
            params.push(Box::new(s.to_owned()));
        }
        sql.push_str(" ORDER BY received_at ASC");

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_kds_order)?;
        rows.map(|r| Ok(r?)).collect()
    }

    /// Prune KDS orders older than the given number of days.
    ///
    /// Returns the number of orders deleted. Used by the daily cleanup
    /// daemon to prevent unbounded event log growth (plan §4.0).
    /// Only prunes orders in terminal states (ready, served, cancelled).
    pub fn cleanup_old_kds_orders(&self, retention_days: i64) -> Result<usize, CoreError> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(retention_days))
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        // Delete line items first (FK constraint).
        let deleted_items = self.conn.execute(
            "DELETE FROM kds_line_items WHERE kds_order_id IN (
                SELECT id FROM kds_orders WHERE received_at < ?1
                AND status IN ('ready', 'served', 'cancelled')
            )",
            params![cutoff],
        )?;

        // Delete order targets.
        let deleted_targets = self.conn.execute(
            "DELETE FROM kds_order_targets WHERE kds_order_id IN (
                SELECT id FROM kds_orders WHERE received_at < ?1
                AND status IN ('ready', 'served', 'cancelled')
            )",
            params![cutoff],
        )?;

        // Delete orders.
        let deleted_orders = self.conn.execute(
            "DELETE FROM kds_orders WHERE received_at < ?1
             AND status IN ('ready', 'served', 'cancelled')",
            params![cutoff],
        )?;

        if deleted_orders > 0 {
            tracing::info!(
                orders = deleted_orders,
                line_items = deleted_items,
                targets = deleted_targets,
                retention_days,
                "KDS event log cleanup completed"
            );
        }

        Ok(deleted_orders)
    }
}

// ── KDS Device Health Monitoring ────────────────────────────────

impl Store<'_> {
    /// Mark connected devices as stale if they haven't communicated recently.
    ///
    /// A device is considered stale if `last_seen_at` is older than
    /// `stale_threshold_secs` seconds ago. Called periodically by the
    /// health monitoring daemon (plan §4.0).
    ///
    /// Returns the number of devices transitioned to stale.
    pub fn mark_stale_kds_devices(&self, stale_threshold_secs: i64) -> Result<usize, CoreError> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(stale_threshold_secs))
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        let affected = self.conn.execute(
            "UPDATE kds_devices
             SET connection_status = 'stale', updated_at = ?1
             WHERE connection_status = 'connected'
               AND last_seen_at IS NOT NULL
               AND last_seen_at < ?2",
            params![cutoff, cutoff],
        )?;

        if affected > 0 {
            tracing::info!(count = affected, "KDS devices marked stale");
        }

        Ok(affected)
    }

    /// Deactivate devices that have been stale for too long.
    ///
    /// A device is deactivated if `connection_status = 'stale'` and
    /// `updated_at` is older than `max_stale_duration_secs` seconds ago.
    /// This prevents permanently-offline devices from accumulating.
    ///
    /// Returns the number of devices deactivated.
    pub fn deactivate_stale_kds_devices(
        &self,
        max_stale_duration_secs: i64,
    ) -> Result<usize, CoreError> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(max_stale_duration_secs))
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        let affected = self.conn.execute(
            "UPDATE kds_devices
             SET is_active = 0, updated_at = ?1
             WHERE connection_status = 'stale'
               AND updated_at < ?2",
            params![cutoff, cutoff],
        )?;

        if affected > 0 {
            tracing::info!(
                count = affected,
                "KDS devices auto-deactivated after prolonged stale period"
            );
        }

        Ok(affected)
    }
}

#[cfg(test)]
#[path = "kds_tests.rs"]
mod tests;
