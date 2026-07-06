//! Active cart persistence — load, save, delete, and list active carts.
//!
//! Carts are serialised as JSON blobs in the `active_carts` table so they
//! survive application restarts.  This is the same strategy used for
//! [`held_carts`](super::sales).

use foundation::Cart;
use foundation::CartId;
use uuid::Uuid;

use crate::error::CoreError;

use super::Store;

impl Store<'_> {
    /// Persist (insert or update) an active cart in SQLite.
    ///
    /// The cart is serialised to JSON via `serde_json`.  If a cart with
    /// the same id already exists it is replaced.
    pub fn save_active_cart(&self, cart: &Cart) -> Result<(), CoreError> {
        let id = cart.id().to_string();
        let cart_data = serde_json::to_string(cart)
            .map_err(|e| CoreError::Internal(format!("serialising cart {id}: {e}")))?;
        self.conn.execute(
            "INSERT INTO active_carts (id, cart_data, updated_at)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(id) DO UPDATE SET
                cart_data = excluded.cart_data,
                updated_at = excluded.updated_at",
            rusqlite::params![id, cart_data],
        )?;
        Ok(())
    }

    /// Load an active cart by id.  Returns `None` when the id does not
    /// exist in the database.
    pub fn load_active_cart(&self, id: &CartId) -> Result<Option<Cart>, CoreError> {
        let id_str = id.to_string();
        let result: Result<String, rusqlite::Error> = self.conn.query_row(
            "SELECT cart_data FROM active_carts WHERE id = ?1",
            rusqlite::params![id_str],
            |row| row.get(0),
        );
        match result {
            Ok(json) => {
                let cart: Cart = serde_json::from_str(&json).map_err(|e| {
                    CoreError::Internal(format!("deserialising cart {id_str}: {e}"))
                })?;
                Ok(Some(cart))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete an active cart by id.  Succeeds even if the id does not
    /// exist (no-op delete).
    pub fn delete_active_cart(&self, id: &CartId) -> Result<(), CoreError> {
        self.conn.execute(
            "DELETE FROM active_carts WHERE id = ?1",
            rusqlite::params![id.to_string()],
        )?;
        Ok(())
    }

    /// Return all active cart ids, most-recently-updated first.
    pub fn list_active_carts(&self) -> Result<Vec<CartId>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM active_carts ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            let id_str: String = row?;
            let uuid = Uuid::parse_str(&id_str).map_err(|e| {
                CoreError::Internal(format!("invalid cart id in DB: {id_str}: {e}"))
            })?;
            ids.push(CartId(uuid));
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use foundation::{Cart, CartLine, Currency, Money, Sku};
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

    fn usd() -> Currency {
        "USD".parse().unwrap()
    }

    fn price(minor: i64) -> Money {
        Money {
            minor_units: minor,
            currency: usd(),
        }
    }

    fn sample_cart() -> Cart {
        let mut cart = Cart::new(usd());
        cart.add_line(CartLine::new(Sku::new("COFFEE"), 2, price(350)))
            .unwrap();
        cart.add_line(CartLine::new(Sku::new("BAGEL"), 1, price(450)))
            .unwrap();
        cart
    }

    #[test]
    fn save_and_load_roundtrip() {
        let conn = fresh();
        let s = store(&conn);
        let cart = sample_cart();
        let id = cart.id();

        s.save_active_cart(&cart).unwrap();
        let loaded = s.load_active_cart(&id).unwrap().unwrap();
        assert_eq!(loaded.id(), id);
        assert_eq!(loaded.line_count(), 2);
        assert_eq!(loaded.total().unwrap().minor_units, 1150);
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let conn = fresh();
        let s = store(&conn);
        let id = CartId::new();
        let result = s.load_active_cart(&id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_removes_cart() {
        let conn = fresh();
        let s = store(&conn);
        let cart = sample_cart();
        let id = cart.id();

        s.save_active_cart(&cart).unwrap();
        s.delete_active_cart(&id).unwrap();
        assert!(s.load_active_cart(&id).unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_is_noop() {
        let conn = fresh();
        let s = store(&conn);
        let id = CartId::new();
        // Should not error.
        s.delete_active_cart(&id).unwrap();
    }

    #[test]
    fn update_overwrites_existing() {
        let conn = fresh();
        let s = store(&conn);
        let mut cart = sample_cart();
        let id = cart.id();

        s.save_active_cart(&cart).unwrap();

        // Modify and save again.
        cart.add_line(CartLine::new(Sku::new("TEA"), 3, price(200)))
            .unwrap();
        s.save_active_cart(&cart).unwrap();

        let loaded = s.load_active_cart(&id).unwrap().unwrap();
        assert_eq!(loaded.line_count(), 3);
        assert_eq!(loaded.total().unwrap().minor_units, 1750);
    }

    #[test]
    fn list_active_carts_returns_all() {
        let conn = fresh();
        let s = store(&conn);

        let cart_a = Cart::new(usd());
        let id_a = cart_a.id();
        s.save_active_cart(&cart_a).unwrap();

        let cart_b = Cart::new(usd());
        let id_b = cart_b.id();
        s.save_active_cart(&cart_b).unwrap();

        let ids = s.list_active_carts().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id_a));
        assert!(ids.contains(&id_b));
    }

    #[test]
    fn list_active_carts_empty() {
        let conn = fresh();
        let s = store(&conn);
        let ids = s.list_active_carts().unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn list_active_carts_ordered_by_update() {
        let conn = fresh();
        let s = store(&conn);

        let cart_a = Cart::new(usd());
        let id_a = cart_a.id();
        s.save_active_cart(&cart_a).unwrap();

        // Small delay so timestamps differ.
        std::thread::sleep(std::time::Duration::from_millis(10));

        let cart_b = Cart::new(usd());
        let id_b = cart_b.id();
        s.save_active_cart(&cart_b).unwrap();

        let ids = s.list_active_carts().unwrap();
        // id_b should be first (most recently updated).
        assert_eq!(ids[0], id_b);
        assert_eq!(ids[1], id_a);
    }
}
