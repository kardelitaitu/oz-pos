-- 095_held_carts_deduction_location.sql
-- ADR-19 §5.3 / §6.3: persist the deduction location on held carts so
-- that restoring a held cart also restores its deduction location lock.
--
-- Before this migration, a held cart saved via `hold_cart` only stored
-- the cart_data JSON blob, losing the `deduction_location_id` that was
-- locked at cart-start time (migration 094). When the cashier restored
-- the held cart and tried to add new lines, `add_line_scoped` would
-- reject with "has no deduction location lock" because the restored
-- active cart had no location lock set.
--
-- The column is NULLABLE because:
--   1. Pre-095 held carts have no location data (backward compat).
--   2. Legacy/unbound workspaces may hold carts without a location lock.
--   3. The Rust layer enforces non-NULL at runtime for new carts in
--      scoped (multi-location) workspaces.
--
-- ON DELETE RESTRICT: a location with held carts cannot be hard-deleted.

ALTER TABLE held_carts ADD COLUMN deduction_location_id TEXT
    REFERENCES inventory_locations(id) ON DELETE RESTRICT;
