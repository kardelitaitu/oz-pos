-- 054_product_cost.sql — add cost price column to products table.
--
-- The cost_minor field stores the product's cost in minor units (e.g. cents
-- for USD). It defaults to 0 and is used by menu engineering analytics to
-- calculate contribution margin: (unit_price - unit_cost) × quantity.

ALTER TABLE products ADD COLUMN cost_minor INTEGER NOT NULL DEFAULT 0;
