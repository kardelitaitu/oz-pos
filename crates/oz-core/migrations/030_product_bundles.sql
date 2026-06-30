-- Product bundles: a bundle is a single SKU that contains multiple sub-items.

CREATE TABLE IF NOT EXISTS product_bundles (
    id          TEXT PRIMARY KEY,
    bundle_sku  TEXT NOT NULL UNIQUE REFERENCES products(sku),
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    -- The bundle's own price in minor units (overrides the sum of components if set)
    bundle_price_minor INTEGER,
    -- Currency (must match component currencies)
    currency    TEXT NOT NULL DEFAULT 'USD',
    active      INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS bundle_items (
    id          TEXT PRIMARY KEY,
    bundle_id   TEXT NOT NULL REFERENCES product_bundles(id),
    sku         TEXT NOT NULL REFERENCES products(sku),
    qty         INTEGER NOT NULL DEFAULT 1,
    -- Override the component's individual price (empty = use product's price)
    unit_price_minor INTEGER
);
