-- 001_sales.sql — initial sales schema.
--
-- All monetary values are stored as INTEGER minor units (e.g., cents for
-- USD). The application layer (Cart, Money) is responsible for currency
-- matching and overflow checks; SQLite stores the number verbatim.

CREATE TABLE IF NOT EXISTS products (
    id          TEXT PRIMARY KEY,
    sku         TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    price_minor INTEGER NOT NULL CHECK (price_minor >= 0),
    currency    TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_products_sku ON products(sku);

CREATE TABLE IF NOT EXISTS sales (
    id          TEXT PRIMARY KEY,
    total_minor INTEGER NOT NULL,
    currency    TEXT NOT NULL,
    line_count  INTEGER NOT NULL CHECK (line_count > 0),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_sales_created_at ON sales(created_at);

CREATE TABLE IF NOT EXISTS sale_lines (
    id            TEXT PRIMARY KEY,
    sale_id       TEXT NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    sku           TEXT NOT NULL,
    qty           INTEGER NOT NULL CHECK (qty > 0),
    unit_minor    INTEGER NOT NULL,
    line_minor    INTEGER NOT NULL,
    currency      TEXT NOT NULL,
    line_position INTEGER NOT NULL,
    UNIQUE (sale_id, line_position)
);

CREATE INDEX IF NOT EXISTS idx_sale_lines_sale_id ON sale_lines(sale_id);
CREATE INDEX IF NOT EXISTS idx_sale_lines_sku ON sale_lines(sku);
