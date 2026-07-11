-- 012_product_taxes.sql — Junction table linking products to tax rates.
--
-- A product can have multiple tax rates (e.g. state + local) and a tax rate
-- can apply to many products. Deleting a product or tax rate cascades.

CREATE TABLE IF NOT EXISTS product_taxes (
    product_sku  TEXT NOT NULL REFERENCES products(sku) ON DELETE CASCADE,
    tax_rate_id  TEXT NOT NULL REFERENCES tax_rates(id) ON DELETE CASCADE,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (product_sku, tax_rate_id)
);
