-- 064_kds_store_id.sql — Add store_id column to kds_orders (ADR #8)
--
-- Enables KDS tablets to filter orders by store_id for defense-in-depth
-- in multi-store deployments. The column is populated with the sale's
-- store context when the order is created.

ALTER TABLE kds_orders ADD COLUMN store_id TEXT;
