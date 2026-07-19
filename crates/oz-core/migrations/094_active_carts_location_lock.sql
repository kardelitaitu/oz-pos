-- 094_active_carts_location_lock.sql
-- ADR-19 §5.1: lock the deduction location at cart-start time so the
-- payment gateway capture always has a known stock source BEFORE funds
-- are captured (§13-31 pre-capture ordering).
--
-- `deduction_location_id` — FK to `inventory_locations.id`. Set ONCE when
--   the cart panel mounts (via `resolve_primary_location`). NOT NULL for
--   new carts; nullable in the schema because pre-094 rows have no location
--   and the Rust layer enforces non-NULL at runtime for new carts.
--   ON DELETE RESTRICT: a location with active carts cannot be hard-deleted.
--
-- `location_override_at` — ISO-8601 timestamp of the last cashier override
--   (via FastPINOverlay per ADR-6 pattern). NULL when no override is active.
--   The Rust layer reads this to surface the override badge in CartPanel.

ALTER TABLE active_carts ADD COLUMN deduction_location_id TEXT
    REFERENCES inventory_locations(id) ON DELETE RESTRICT;

ALTER TABLE active_carts ADD COLUMN location_override_at TEXT;
