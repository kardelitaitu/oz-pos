-- 002_products.sql — categories, inventory, and settings tables.
--
-- 001_sales.sql already creates the `products` table. This migration adds
-- the tables that round out the product domain: categories for grouping,
-- inventory for stock tracking, and a key-value `settings` table for
-- feature flags and store configuration.
--
-- All monetary values are INTEGER minor units. See 001_sales.sql for
-- the rationale.

-- Extend products with a category reference (001 created the table).
ALTER TABLE products ADD COLUMN category_id TEXT REFERENCES categories(id);
CREATE INDEX IF NOT EXISTS idx_products_category_id ON products(category_id);

CREATE TABLE IF NOT EXISTS categories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    colour     TEXT NOT NULL DEFAULT '#6366f1',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_categories_name ON categories(name);

CREATE TABLE IF NOT EXISTS inventory (
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    qty        INTEGER NOT NULL DEFAULT 0 CHECK (qty >= 0),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (product_id)
);

CREATE TABLE IF NOT EXISTS settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
