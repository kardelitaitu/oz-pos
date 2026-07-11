-- 015_product_variants.sql
-- Product variants (size, colour, flavour) for a parent product.
-- Each variant has its own SKU, price override (optional), and barcode.

CREATE TABLE IF NOT EXISTS product_variants (
    id              TEXT PRIMARY KEY,
    parent_sku      TEXT NOT NULL REFERENCES products(sku) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    sku             TEXT NOT NULL UNIQUE,
    price_minor     INTEGER,           -- NULL means use parent price
    currency        TEXT,
    barcode         TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_product_variants_parent ON product_variants(parent_sku);
CREATE UNIQUE INDEX IF NOT EXISTS uq_product_variants_barcode ON product_variants(barcode);
