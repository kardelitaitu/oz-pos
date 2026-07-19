//! KDS (Kitchen Display System) CRUD — order ticket lifecycle.

use rusqlite::params;

use crate::error::CoreError;
use crate::{CreateKdsOrderInput, KdsOrder, KdsStatus};

use super::Store;

impl Store<'_> {
    fn row_to_kds_order(row: &rusqlite::Row) -> rusqlite::Result<KdsOrder> {
        Ok(KdsOrder {
            id: row.get("id")?,
            sale_id: row.get("sale_id")?,
            store_id: row.get("store_id")?,
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
        })
    }

    /// Create a KDS order from input, auto-incrementing the display number per day.
    pub fn create_kds_order(&self, input: CreateKdsOrderInput) -> Result<KdsOrder, CoreError> {
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
            "INSERT INTO kds_orders (id, sale_id, store_id, status, items_summary, item_count,
                                     display_number, received_at, kitchen_zone, notes)
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                input.sale_id,
                input.store_id,
                input.items_summary,
                input.item_count,
                display_number,
                now,
                input.kitchen_zone,
                input.notes,
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
            "SELECT id, sale_id, store_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes
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

    /// Get a single KDS order by its id.
    pub fn get_kds_order(&self, id: &str) -> Result<Option<KdsOrder>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sale_id, store_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes
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
            "SELECT id, sale_id, store_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes
             FROM kds_orders WHERE sale_id = ?1",
        )?;
        let result = stmt.query_row(params![sale_id], Self::row_to_kds_order);
        match result {
            Ok(order) => Ok(Some(order)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
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
            "SELECT id, sale_id, store_id, status, items_summary, item_count, display_number,
                    received_at, started_at, ready_at, served_at,
                    prep_time_seconds, kitchen_zone, notes
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

        let mut orders = Vec::with_capacity(by_zone.len());
        for (zone, lines) in by_zone {
            let items_summary = lines
                .iter()
                .map(|l| {
                    let name = self
                        .product_name_by_sku(&l.sku)
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| l.sku.clone());
                    if l.qty > 1 {
                        format!("{name} x{}", l.qty)
                    } else {
                        name
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            let item_count: i64 = lines.iter().map(|l| l.qty).sum();

            orders.push(self.create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id.to_owned(),
                store_id: store_id.map(|s| s.to_owned()),
                items_summary,
                item_count,
                kitchen_zone: zone,
                notes: String::new(),
            })?);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use crate::{Cart, CartLine, Money, Sale, Sku};
    use rusqlite::Connection;

    fn fresh() -> Connection {
        migrations::fresh_db()
    }

    fn store(conn: &Connection) -> Store<'_> {
        Store::new(conn)
    }

    fn usd() -> crate::Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    fn seed_product(conn: &Connection, sku: &str, name: &str) {
        let s = store(conn);
        s.create_product(sku, name, price(500), None, None, 100, Some("restaurant"))
            .unwrap();
    }

    #[test]
    fn create_and_get_kds_order() {
        let conn = fresh();
        let s = store(&conn);
        seed_product(&conn, "COFFEE", "Coffee");

        // Create a minimal sale.
        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();

        let order = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id.clone(),
                store_id: None,
                items_summary: "Coffee x2, Bagel".into(),
                item_count: 3,
                kitchen_zone: None,
                notes: "No onions".into(),
            })
            .unwrap();

        assert_eq!(order.sale_id, sale_id);
        assert_eq!(order.items_summary, "Coffee x2, Bagel");
        assert_eq!(order.item_count, 3);
        assert_eq!(order.notes, "No onions");
        assert_eq!(order.status, "pending");
        assert!(order.display_number.is_some());
        assert!(order.display_number.unwrap() >= 1);

        let fetched = s.get_kds_order(&order.id).unwrap().unwrap();
        assert_eq!(fetched.id, order.id);
    }

    #[test]
    fn get_kds_order_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.get_kds_order("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_kds_order_by_sale() {
        let conn = fresh();
        let s = store(&conn);
        seed_product(&conn, "TEA", "Tea");

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();

        let order = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id.clone(),
                store_id: None,
                items_summary: "Tea".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        let by_sale = s.get_kds_order_by_sale(&sale_id).unwrap().unwrap();
        assert_eq!(by_sale.id, order.id);
    }

    #[test]
    fn update_kds_status_sets_timestamps() {
        let conn = fresh();
        let s = store(&conn);

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();

        let order = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id,
                store_id: None,
                items_summary: "Test".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        // Pending -> Preparing
        let updated = s.update_kds_status(&order.id, "preparing").unwrap();
        assert_eq!(updated.status, "preparing");
        assert!(updated.started_at.is_some());

        // Preparing -> Ready
        let updated = s.update_kds_status(&order.id, "ready").unwrap();
        assert_eq!(updated.status, "ready");
        assert!(updated.ready_at.is_some());

        // Ready -> Served
        let updated = s.update_kds_status(&order.id, "served").unwrap();
        assert_eq!(updated.status, "served");
        assert!(updated.served_at.is_some());
    }

    #[test]
    fn update_kds_status_invalid() {
        let conn = fresh();
        let s = store(&conn);

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();

        let order = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id,
                store_id: None,
                items_summary: "Test".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        let err = s.update_kds_status(&order.id, "bogus").unwrap_err();
        assert!(matches!(err, CoreError::Validation { field, .. } if field == "status"));
    }

    #[test]
    fn list_kds_orders_empty() {
        let conn = fresh();
        let s = store(&conn);
        let orders = s.list_kds_orders(None).unwrap();
        assert!(orders.is_empty());
    }

    #[test]
    fn list_kds_orders_with_status_filter() {
        let conn = fresh();
        let s = store(&conn);

        let sale_id1 = uuid::Uuid::now_v7().to_string();
        let sale_id2 = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        for sid in [&sale_id1, &sale_id2] {
            let test_sale = Sale {
                id: sid.to_string(),
                status: crate::SaleStatus::Completed,
                total: price(0),
                currency: usd(),
                line_count: 0,
                payment_method: None,
                tendered_minor: None,
                discount_percent: 0,
                discount_label: None,
                user_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                subtotal: price(0),
                tax_total: price(0),
                customer_id: None,
                lines: vec![],
                version: 1,
            };
            s.create_sale(&test_sale).unwrap();
        }

        s.create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id1,
            store_id: None,
            items_summary: "Order 1".into(),
            item_count: 1,
            kitchen_zone: None,
            notes: String::new(),
        })
        .unwrap();

        s.create_kds_order(CreateKdsOrderInput {
            sale_id: sale_id2,
            store_id: None,
            items_summary: "Order 2".into(),
            item_count: 2,
            kitchen_zone: None,
            notes: String::new(),
        })
        .unwrap();

        let all = s.list_kds_orders(None).unwrap();
        assert_eq!(all.len(), 2);

        let pending = s.list_kds_orders(Some("pending")).unwrap();
        assert_eq!(pending.len(), 2);

        let ready = s.list_kds_orders(Some("ready")).unwrap();
        assert_eq!(ready.len(), 0);
    }

    #[test]
    fn get_kds_queue_returns_pending_and_preparing() {
        let conn = fresh();
        let s = store(&conn);

        let sale_id1 = uuid::Uuid::now_v7().to_string();
        let sale_id2 = uuid::Uuid::now_v7().to_string();
        let sale_id3 = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        for sid in [&sale_id1, &sale_id2, &sale_id3] {
            let test_sale = Sale {
                id: sid.to_string(),
                status: crate::SaleStatus::Completed,
                total: price(0),
                currency: usd(),
                line_count: 0,
                payment_method: None,
                tendered_minor: None,
                discount_percent: 0,
                discount_label: None,
                user_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                subtotal: price(0),
                tax_total: price(0),
                customer_id: None,
                lines: vec![],
                version: 1,
            };
            s.create_sale(&test_sale).unwrap();
        }

        let _o1 = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id1,
                store_id: None,
                items_summary: "Pending".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        let o2 = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id2,
                store_id: None,
                items_summary: "Preparing".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        let o3 = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id3,
                store_id: None,
                items_summary: "Served".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        s.update_kds_status(&o2.id, "preparing").unwrap();
        s.update_kds_status(&o3.id, "served").unwrap();

        let queue = s.get_kds_queue(None).unwrap();
        // Queue should include pending + preparing + ready (but not served/cancelled).
        assert_eq!(queue.len(), 2);
        assert!(
            queue
                .iter()
                .all(|o| o.status == "pending" || o.status == "preparing" || o.status == "ready")
        );
    }

    #[test]
    fn complete_sale_to_kds_creates_order() {
        let conn = fresh();
        let s = store(&conn);

        seed_product(&conn, "COFFEE", "Fresh Coffee");
        seed_product(&conn, "BAGEL", "Everything Bagel");

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("BAGEL"), 1, price(450)))
            .unwrap();

        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        let orders = s.complete_sale_to_kds(&sale.id, None).unwrap();
        assert_eq!(orders.len(), 1);
        let order = &orders[0];
        assert_eq!(order.sale_id, sale.id);
        assert_eq!(order.status, "pending");
        assert!(order.items_summary.contains("Coffee"));
        assert!(order.items_summary.contains("Bagel"));
        assert_eq!(order.item_count, 3);
    }

    #[test]
    fn display_number_increments_per_day() {
        let conn = fresh();
        let s = store(&conn);

        let sale_id1 = uuid::Uuid::now_v7().to_string();
        let sale_id2 = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        for sid in [&sale_id1, &sale_id2] {
            let test_sale = Sale {
                id: sid.to_string(),
                status: crate::SaleStatus::Completed,
                total: price(0),
                currency: usd(),
                line_count: 0,
                payment_method: None,
                tendered_minor: None,
                discount_percent: 0,
                discount_label: None,
                user_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                subtotal: price(0),
                tax_total: price(0),
                customer_id: None,
                lines: vec![],
                version: 1,
            };
            s.create_sale(&test_sale).unwrap();
        }

        let o1 = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id1,
                store_id: None,
                items_summary: "First".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        let o2 = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id2,
                store_id: None,
                items_summary: "Second".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        assert_eq!(o1.display_number, Some(1));
        assert_eq!(o2.display_number, Some(2));
    }

    // ── CHECK constraint tests ──────────────────────────────────────

    #[test]
    fn kds_status_check_rejects_invalid_status_on_insert() {
        let conn = fresh();
        let s = store(&conn);

        let sale_id = uuid::Uuid::now_v7().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let test_sale = Sale {
            id: sale_id.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now,
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&test_sale).unwrap();

        // Attempt a raw INSERT with an invalid status — should fail the CHECK constraint.
        let id = uuid::Uuid::now_v7().to_string();
        let result = s.conn.execute(
            "INSERT INTO kds_orders (id, sale_id, status, items_summary, item_count, notes)
             VALUES (?1, ?2, 'bogus', 'Test', 1, '')",
            params![id, sale_id],
        );

        assert!(
            result.is_err(),
            "expected CHECK constraint error for invalid status"
        );
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("CHECK") || msg.contains("constraint") || msg.contains("abort"),
            "expected constraint violation message, got: {msg}"
        );
    }

    #[test]
    fn kds_status_check_accepts_valid_statuses() {
        let conn = fresh();
        let s = store(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Insert orders with each valid status. Each needs its own sale_id
        // because kds_orders.sale_id has a UNIQUE constraint.
        for status in &["pending", "preparing", "ready", "served", "cancelled"] {
            let sale_id = uuid::Uuid::now_v7().to_string();
            let test_sale = Sale {
                id: sale_id.clone(),
                status: crate::SaleStatus::Completed,
                total: price(0),
                currency: usd(),
                line_count: 0,
                payment_method: None,
                tendered_minor: None,
                discount_percent: 0,
                discount_label: None,
                user_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                subtotal: price(0),
                tax_total: price(0),
                customer_id: None,
                lines: vec![],
                version: 1,
            };
            s.create_sale(&test_sale).unwrap();

            let order_id = uuid::Uuid::now_v7().to_string();
            s.conn
                .execute(
                    "INSERT INTO kds_orders (id, sale_id, status, items_summary, item_count, notes)
                 VALUES (?1, ?2, ?3, 'Test', 1, '')",
                    params![order_id, sale_id, status],
                )
                .unwrap();
            let fetched = s.get_kds_order(&order_id).unwrap().unwrap();
            assert_eq!(fetched.status, *status);
        }
    }

    // ── Additional edge cases ─────────────────────────────────────

    #[test]
    fn update_kds_status_nonexistent_order_fails() {
        let conn = fresh();
        let s = store(&conn);
        let err = s.update_kds_status("no-such-order", "pending").unwrap_err();
        assert!(matches!(err, CoreError::NotFound { entity, .. } if entity == "kds_order"));
    }

    #[test]
    fn get_kds_queue_excludes_served_and_cancelled() {
        let conn = fresh();
        let s = store(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Create 4 orders with different statuses.
        let mut ids = Vec::new();
        for st in &["pending", "preparing", "served", "cancelled"] {
            let sale_id = uuid::Uuid::now_v7().to_string();
            let test_sale = Sale {
                id: sale_id.clone(),
                status: crate::SaleStatus::Completed,
                total: price(0),
                currency: usd(),
                line_count: 0,
                payment_method: None,
                tendered_minor: None,
                discount_percent: 0,
                discount_label: None,
                user_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                subtotal: price(0),
                tax_total: price(0),
                customer_id: None,
                lines: vec![],
                version: 1,
            };
            s.create_sale(&test_sale).unwrap();
            let order = s
                .create_kds_order(CreateKdsOrderInput {
                    sale_id,
                    store_id: None,
                    items_summary: st.to_string(),
                    item_count: 1,
                    kitchen_zone: None,
                    notes: String::new(),
                })
                .unwrap();
            if *st != "pending" {
                s.update_kds_status(&order.id, st).unwrap();
            }
            ids.push(order.id);
        }

        let queue = s.get_kds_queue(None).unwrap();
        assert_eq!(queue.len(), 2, "should only have pending + preparing");
        assert!(queue.iter().any(|o| o.status == "pending"));
        assert!(queue.iter().any(|o| o.status == "preparing"));
    }

    #[test]
    fn get_kds_queue_with_zone_filter() {
        let conn = fresh();
        let s = store(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        for zone in &["grill", "salad"] {
            let sale_id = uuid::Uuid::now_v7().to_string();
            let test_sale = Sale {
                id: sale_id.clone(),
                status: crate::SaleStatus::Completed,
                total: price(0),
                currency: usd(),
                line_count: 0,
                payment_method: None,
                tendered_minor: None,
                discount_percent: 0,
                discount_label: None,
                user_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                subtotal: price(0),
                tax_total: price(0),
                customer_id: None,
                lines: vec![],
                version: 1,
            };
            s.create_sale(&test_sale).unwrap();
            s.create_kds_order(CreateKdsOrderInput {
                sale_id,
                store_id: None,
                items_summary: format!("Order in {zone}"),
                item_count: 1,
                kitchen_zone: Some(zone.to_string()),
                notes: String::new(),
            })
            .unwrap();
        }

        let grill = s.get_kds_queue(Some("grill")).unwrap();
        assert_eq!(grill.len(), 1);
        assert!(grill[0].items_summary.contains("grill"));

        let salad = s.get_kds_queue(Some("salad")).unwrap();
        assert_eq!(salad.len(), 1);
        assert!(salad[0].items_summary.contains("salad"));
    }

    #[test]
    fn get_kds_queue_empty_zone_returns_unzoned_orders() {
        let conn = fresh();
        let s = store(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // One order with zone, one without.
        for (suffix, zone) in &[("zoned", Some("grill")), ("unzoned", None)] {
            let sale_id = uuid::Uuid::now_v7().to_string();
            let test_sale = Sale {
                id: sale_id.clone(),
                status: crate::SaleStatus::Completed,
                total: price(0),
                currency: usd(),
                line_count: 0,
                payment_method: None,
                tendered_minor: None,
                discount_percent: 0,
                discount_label: None,
                user_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                subtotal: price(0),
                tax_total: price(0),
                customer_id: None,
                lines: vec![],
                version: 1,
            };
            s.create_sale(&test_sale).unwrap();
            s.create_kds_order(CreateKdsOrderInput {
                sale_id,
                store_id: None,
                items_summary: format!("Order {suffix}"),
                item_count: 1,
                kitchen_zone: zone.map(|z| z.to_string()),
                notes: String::new(),
            })
            .unwrap();
        }

        let unzoned = s.get_kds_queue(Some("")).unwrap();
        assert_eq!(unzoned.len(), 1);
        assert!(unzoned[0].items_summary.contains("unzoned"));
    }

    #[test]
    fn complete_sale_to_kds_no_restaurant_lines_returns_empty() {
        let conn = fresh();
        let s = store(&conn);

        // Seed a retail-type product.
        s.create_product(
            "RETAIL-1",
            "Retail Item",
            price(500),
            None,
            None,
            100,
            Some("retail"),
        )
        .unwrap();

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("RETAIL-1"), 1, price(500)))
            .unwrap();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        let orders = s.complete_sale_to_kds(&sale.id, None).unwrap();
        assert!(orders.is_empty(), "no KDS orders for retail-only sale");
    }

    fn seed_product_with_zone(conn: &Connection, sku: &str, name: &str, zone: &str) {
        let s = store(conn);
        s.create_product(sku, name, price(500), None, None, 100, Some("restaurant"))
            .unwrap();
        // Set kitchen_zone directly via SQL (not exposed on create_product API).
        conn.execute(
            "UPDATE products SET kitchen_zone = ?1 WHERE sku = ?2",
            params![zone, sku],
        )
        .unwrap();
    }

    #[test]
    fn complete_sale_to_kds_groups_same_zone_items() {
        let conn = fresh();
        let s = store(&conn);

        // Seed products in the SAME zone (schema has UNIQUE constraint on sale_id).
        seed_product_with_zone(&conn, "STEAK", "Steak", "grill");
        seed_product_with_zone(&conn, "BURGER", "Burger", "grill");

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("STEAK"), 2, price(500)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("BURGER"), 3, price(300)))
            .unwrap();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        let orders = s.complete_sale_to_kds(&sale.id, None).unwrap();
        // One order because both products are in the same zone.
        assert_eq!(orders.len(), 1, "same zone items grouped into one order");
        let order = &orders[0];
        assert_eq!(order.kitchen_zone.as_deref(), Some("grill"));
        assert_eq!(order.item_count, 5);
        assert!(order.items_summary.contains("Steak"));
        assert!(order.items_summary.contains("Burger"));
    }

    #[test]
    fn complete_sale_to_kds_with_store_id() {
        let conn = fresh();
        let s = store(&conn);
        seed_product(&conn, "BURGER", "Burger");

        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("BURGER"), 1, price(500)))
            .unwrap();
        let sale = Sale::from_cart(&cart).unwrap();
        s.create_sale(&sale).unwrap();

        let orders = s.complete_sale_to_kds(&sale.id, Some("store-1")).unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].store_id, Some("store-1".to_string()));
    }

    #[test]
    fn get_kds_order_by_sale_not_found() {
        let conn = fresh();
        let s = store(&conn);
        let result = s.get_kds_order_by_sale("no-such-sale").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_kds_orders_ordered_by_received_at_desc() {
        let conn = fresh();
        let s = store(&conn);
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        // Create two orders with distinct timing (sleep to separate timestamps).
        let sale_id1 = uuid::Uuid::now_v7().to_string();
        let ts1 = Sale {
            id: sale_id1.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&ts1).unwrap();
        let o1 = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id1,
                store_id: None,
                items_summary: "First".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));

        let sale_id2 = uuid::Uuid::now_v7().to_string();
        let ts2 = Sale {
            id: sale_id2.clone(),
            status: crate::SaleStatus::Completed,
            total: price(0),
            currency: usd(),
            line_count: 0,
            payment_method: None,
            tendered_minor: None,
            discount_percent: 0,
            discount_label: None,
            user_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            subtotal: price(0),
            tax_total: price(0),
            customer_id: None,
            lines: vec![],
            version: 1,
        };
        s.create_sale(&ts2).unwrap();
        let o2 = s
            .create_kds_order(CreateKdsOrderInput {
                sale_id: sale_id2,
                store_id: None,
                items_summary: "Second".into(),
                item_count: 1,
                kitchen_zone: None,
                notes: String::new(),
            })
            .unwrap();

        let all = s.list_kds_orders(None).unwrap();
        assert_eq!(all.len(), 2);
        // Most recent first.
        assert_eq!(all[0].id, o2.id);
        assert_eq!(all[1].id, o1.id);
    }
}
