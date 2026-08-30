//! KDS order lifecycle - creation, queries, and status/item updates.
//!
//! Key functions: create_kds_order (plus routed/fanout variants),
//! list_kds_orders(+_for_instance), instance-visibility filtering,
//! get_kds_order(+_by_sale), update_kds_order_items, update_kds_status,
//! and the queue getters.
//!
//! Invariants: creation cores shared with the fanout path live in the
//! parent module (private visibility reaches all child parts); fanout
//! tickets are normalized via kds_order_targets so each zone gets
//! exactly one ticket; instance scoping is defense-in-depth.

use crate::db::Store;
use crate::error::CoreError;
use crate::{CreateKdsOrderInput, KdsOrder, KdsStatus};
use rusqlite::params;

impl Store<'_> {
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
}
