-- 135_sale_line_cost_snapshot.sql — freeze product cost (HPP) into sale_lines
-- (ADR #36 reporting follow-up).
--
-- Until now the reporting layer read `products.cost_minor` at query time, so
-- editing a product's HPP restated every historical sale's margin. This
-- migration adds `sale_lines.cost_minor` and backfills it with the product's
-- CURRENT cost so pre-existing rows are frozen at the value the reports
-- displayed before the upgrade. New sales snapshot the product's cost at
-- checkout (written by the application), and the reporting queries prefer
-- the per-line snapshot with a fallback to the current product cost (and 0).
--
-- Backfill semantics: `cost_minor` is NULL when a product is missing or has
-- no cost set; the COALESCE fallback in the reports keeps those rows at the
-- product's current cost / zero, matching prior behavior.
--
-- Plain ADD COLUMN is safe: nullable columns do not affect the
-- fresh-install-vs-upgrade fingerprint test, and the UPDATE is data-only.
-- Local-only like the rest of the HPP data — never synced.

ALTER TABLE sale_lines ADD COLUMN cost_minor INTEGER;

-- NULLIF: products.cost_minor defaults to 0 ("cost not set"); normalize it
-- to NULL so an unset cost never shadows a later-set product cost in the
-- reporting COALESCE fallback.
UPDATE sale_lines
SET cost_minor = (SELECT NULLIF(p.cost_minor, 0) FROM products p WHERE p.sku = sale_lines.sku);
