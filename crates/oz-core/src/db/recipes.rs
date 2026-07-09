//! Recipe / BOM queries — composite product ingredient lookups.
//!
//! When a composite menu item is sold, the system needs to know which
//! raw ingredients to deduct from inventory rather than (or in addition
//! to) deducting the composite item's own stock level.

use rusqlite::params;

use crate::error::CoreError;
use crate::recipe::RecipeItem;

use super::Store;

impl Store<'_> {
    /// Look up all ingredient rows for a composite product.
    ///
    /// Returns an empty vec if the product has no recipe (i.e. it is a
    /// simple product that should be deducted directly).
    pub fn get_recipe_ingredients(
        &self,
        parent_product_id: &str,
    ) -> Result<Vec<RecipeItem>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, parent_product_id, ingredient_product_id, quantity_required, unit
             FROM product_recipes
             WHERE parent_product_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![parent_product_id], |row| {
            Ok(RecipeItem {
                id: row.get("id")?,
                parent_product_id: row.get("parent_product_id")?,
                ingredient_product_id: row.get("ingredient_product_id")?,
                quantity_required: row.get("quantity_required")?,
                unit: row.get("unit")?,
            })
        })?;
        let results: Result<Vec<_>, _> = rows.collect();
        Ok(results?)
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

    fn seed_products(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO products (id, sku, name, price_minor, currency, created_at, updated_at) VALUES
                ('burger', 'BURGER', 'Cheeseburger', 500, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('bun', 'BUN', 'Burger Bun', 100, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('patty', 'PATTY', 'Beef Patty', 200, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('cheese', 'CHEESE', 'Cheese Slice', 50, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z'),
                ('salad', 'SALAD', 'Side Salad', 150, 'USD', '2025-01-01T00:00:00.000Z', '2025-01-01T00:00:00.000Z');",
        ).unwrap();
    }

    fn seed_recipe(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO product_recipes (id, parent_product_id, ingredient_product_id, quantity_required, unit) VALUES
                ('r1', 'burger', 'bun', 1, 'pcs'),
                ('r2', 'burger', 'patty', 1, 'pcs'),
                ('r3', 'burger', 'cheese', 2, 'pcs');",
        ).unwrap();
    }

    // ── Get recipe ingredients ──────────────────────────────────────

    #[test]
    fn get_recipe_ingredients_for_composite_product() {
        let conn = fresh();
        seed_products(&conn);
        seed_recipe(&conn);

        let store = Store::new(&conn);
        let ingredients = store.get_recipe_ingredients("burger").unwrap();

        assert_eq!(ingredients.len(), 3);

        // Order should be by id ASC: r1, r2, r3
        assert_eq!(ingredients[0].ingredient_product_id, "bun");
        assert_eq!(ingredients[0].quantity_required, 1);
        assert_eq!(ingredients[1].ingredient_product_id, "patty");
        assert_eq!(ingredients[1].quantity_required, 1);
        assert_eq!(ingredients[2].ingredient_product_id, "cheese");
        assert_eq!(ingredients[2].quantity_required, 2);
    }

    #[test]
    fn get_recipe_ingredients_returns_empty_for_simple_product() {
        let conn = fresh();
        seed_products(&conn);

        let store = Store::new(&conn);
        let ingredients = store.get_recipe_ingredients("salad").unwrap();

        assert!(ingredients.is_empty());
    }

    #[test]
    fn get_recipe_ingredients_returns_empty_for_unknown_product() {
        let conn = fresh();
        let store = Store::new(&conn);
        let ingredients = store.get_recipe_ingredients("nonexistent").unwrap();

        assert!(ingredients.is_empty());
    }

    #[test]
    fn get_recipe_ingredients_multiple_products() {
        let conn = fresh();
        seed_products(&conn);
        seed_recipe(&conn);

        // Add a recipe for salad
        conn.execute_batch(
            "INSERT INTO product_recipes (id, parent_product_id, ingredient_product_id, quantity_required, unit) VALUES
                ('r4', 'salad', 'bun', 1, 'pcs');",
        ).unwrap();

        let store = Store::new(&conn);
        let burger_ings = store.get_recipe_ingredients("burger").unwrap();
        let salad_ings = store.get_recipe_ingredients("salad").unwrap();

        assert_eq!(burger_ings.len(), 3);
        assert_eq!(salad_ings.len(), 1);
        assert_eq!(salad_ings[0].ingredient_product_id, "bun");
    }
}
