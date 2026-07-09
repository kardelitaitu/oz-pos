-- 051_product_recipes.sql — Recipe / Bill of Materials (BOM) for composite products.
--
-- When a composite menu item (e.g. "Cheeseburger") is sold, the system
-- deducts individual raw ingredients (e.g. "Burger Bun", "Beef Patty",
-- "Cheese Slice") from inventory using this BOM mapping.
--
-- Each row maps a parent product to one of its ingredient products and
-- the required quantity of that ingredient. Multiple rows share the same
-- parent_product_id to define the full recipe.

CREATE TABLE IF NOT EXISTS product_recipes (
    id                    TEXT PRIMARY KEY,
    parent_product_id     TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    ingredient_product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    quantity_required     INTEGER NOT NULL CHECK (quantity_required > 0),
    unit                  TEXT NOT NULL DEFAULT 'pcs',
    created_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (parent_product_id, ingredient_product_id)
);

CREATE INDEX IF NOT EXISTS idx_product_recipes_parent ON product_recipes(parent_product_id);
CREATE INDEX IF NOT EXISTS idx_product_recipes_ingredient ON product_recipes(ingredient_product_id);
