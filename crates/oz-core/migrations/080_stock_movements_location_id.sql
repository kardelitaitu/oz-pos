-- 080_stock_movements_location_id.sql
-- ADR #18 §2b: link stock_movement deltas (and the archive mirror) to
-- inventory_locations via a `location_id` FK.
--
-- Pattern matches migration 067 (which added `store_id TEXT NOT NULL DEFAULT ''`):
-- NOT NULL + canonical-UUID default. Existing rows backfill to the default
-- location so the legacy SUM(delta) aggregate is preserved per-product.
--
-- ON DELETE RESTRICT enforces ADR §2 invariant: a location with stock
-- movements cannot be hard-deleted. The audit-archive gets the same FK so
-- historical ledger entries retain full location provenance even after the
-- live ledger is pruned.

ALTER TABLE stock_movements ADD COLUMN location_id TEXT
    NOT NULL DEFAULT '01926b3a-0000-7000-8000-000000000001'
    REFERENCES inventory_locations(id) ON DELETE RESTRICT;

ALTER TABLE stock_movements_archive ADD COLUMN location_id TEXT
    NOT NULL DEFAULT '01926b3a-0000-7000-8000-000000000001'
    REFERENCES inventory_locations(id) ON DELETE RESTRICT;

-- ── Per-location indexes (ADR-18 §13 finding 35) ────────────────────────
-- DELIBERATELY non-overlapping with migration 063's
--   idx_stock_movements_item ON stock_movements(item_id, created_at)
-- and with the archive equivalent.
--
--   * Migration 063 serves PER-PRODUCT queries (the crdt delta ledger's
--     primary access pattern is "all deltas of product X").
--   * This migration serves PER-LOCATION queries ("everything Budi did at
--     Warehouse A today", per ADR §9 example queries).
-- Cross-direction queries are rare in audit dashboards so no composite
-- `(item_id, location_id, created_at)` is added — either single-column
-- index can serve them with a covering-engine plan.
CREATE INDEX IF NOT EXISTS idx_stock_movements_location_created
    ON stock_movements(location_id, created_at);

CREATE INDEX IF NOT EXISTS idx_stock_movements_archive_location_created
    ON stock_movements_archive(location_id, created_at);
