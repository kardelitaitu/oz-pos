-- 079_inventory_location_id.sql
-- ADR #18 §2a: link inventory rows to inventory_locations via a `location_id` FK.
--
-- Implementation note: this migration uses ADD-COLUMN form (not table rebuild)
-- for two reasons:
--
--   1. Migration 069 already added a `warehouse_id` column. The rebuild form
--      would either drop or rename it, breaking the existing tests
--      `migration_069_adds_scoping_columns` and the per-warehouse
--      `idx_inventory_warehouse_product` index introduced by ADR #4 Phase 2.
--
--   2. The ADR's §2a composite-primary-key rebuild
--      `(PRIMARY KEY (product_id, location_id))` is a Phase 1 task — it
--      requires Rust-side aggregation refactor of `list_products` /
--      `list_warehouse_products` queries (per ADR §2a note on duplicate
--      rows). That refactor is deferred to a follow-up Phase 1 migration
--      that bundles with §6 sale-deduction flow changes.
--
-- Until the composite-PK migration lands, multiple inventory rows may exist
-- per `(product_id, location_id)` pair if more than one location_id value is
-- recorded for the same product. In practice this is benign — the
-- canonical-UUID default ensures all pre-existing and follow-on writes
-- converge on the same `location_id`. The composite rebuild in Phase 1 is
-- a correctness tightener, not a runtime-blocker.
--
-- ON DELETE RESTRICT enforces ADR §2 / §5 invariant: a location with stock
-- cannot be hard-deleted. The seed locations from migration 078 are
-- `is_active = 1` and MUST stay that way per §5 location-deletion-policy.

ALTER TABLE inventory ADD COLUMN location_id TEXT
    NOT NULL DEFAULT '01926b3a-0000-7000-8000-000000000001'
    REFERENCES inventory_locations(id) ON DELETE RESTRICT;

-- Per-location composite index. ADR §13 finding 35 — kept NON-OVERLAPPING
-- with migration 069's `idx_inventory_warehouse_product (warehouse_id, product_id)`.
-- Migration 069's index serves the warehouse-keyed audit path (ADR #4 Phase 2);
-- this index serves the location-keyed query path (ADR-18 §9 examples).
-- Cross-direction queries ("product X at location Y") are rare and can be
-- served by either single-column index.
CREATE INDEX IF NOT EXISTS idx_inventory_location_product
    ON inventory(location_id, product_id);
