-- 069_data_scoping_columns.sql
-- ADR #4 Phase 2: Soft Data Scoping
--
-- Adds `store_id` to core domain tables so queries can be scoped
-- to a single store in multi-store deployments. Existing rows get NULL,
-- meaning "unscoped / legacy / global shared" — visible to all stores
-- during the transition period.
--
-- `warehouse_id` is added to inventory and stock_counts for
-- multi-warehouse routing (future enhancement).
--
-- B-Tree indexes are prefixed with the scoping column so SQLite can
-- jump directly to the target store's index slice on scoped queries.

-- ── store_id on domain tables ─────────────────────────────────────────

ALTER TABLE products ADD COLUMN store_id TEXT;
ALTER TABLE sales ADD COLUMN store_id TEXT;
-- Denormalized: sale_lines.store_id mirrors sales.store_id so scoped
-- line-item queries (common in reports) can filter by store without
-- joining through the sales table.
ALTER TABLE sale_lines ADD COLUMN store_id TEXT;
ALTER TABLE customers ADD COLUMN store_id TEXT;

-- ── warehouse_id on stock-related tables ──────────────────────────────

ALTER TABLE inventory ADD COLUMN warehouse_id TEXT;
ALTER TABLE stock_counts ADD COLUMN warehouse_id TEXT;

-- ── Prefix compound B-Tree indexes ───────────────────────────────────
--
-- SQLite uses these to isolate multi-million row tables by store
-- in < 1 ms: the query planner seeks directly to the target store_id
-- prefix slice and ignores the rest of the index.

CREATE INDEX IF NOT EXISTS idx_sales_store_status
    ON sales(store_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_sale_lines_store_sale
    ON sale_lines(store_id, sale_id);

CREATE INDEX IF NOT EXISTS idx_products_store_category
    ON products(store_id, category_id);

CREATE INDEX IF NOT EXISTS idx_inventory_warehouse_product
    ON inventory(warehouse_id, product_id);

CREATE INDEX IF NOT EXISTS idx_customers_store
    ON customers(store_id);
