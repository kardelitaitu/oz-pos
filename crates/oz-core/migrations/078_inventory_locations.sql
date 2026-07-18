-- 078_inventory_locations.sql
-- ADR #18 §1: First-class inventory locations.
--
-- Locations are logical/physical places where stock lives. They are NOT
-- workspace instances — they are domain entities an inventory workspace
-- instance manages. A single inventory workspace can be bound to many
-- locations (per ADR §4 `workspace_inventory_locations`) or to a single
-- location via `workspace_instances.bound_location_id` (per ADR §5).
--
-- Two seed rows are required for legacy single-location deployments and the
-- transfer engine's `'transit'` pseudo-location. §13 finding 36 forbids
-- third-party-data-import collisions by adopting stable UUIDs:
--
--   01926b3a-0000-7000-8000-000000000001 — 'Default Inventory'
--   01926b3a-0000-7000-8000-000000000002 — 'In Transit'
--
-- The legacy free-text `'default'` and `'transit'` identifiers are RESERVED
-- as runtime name-lookup aliases; the canonical IDs are the UUIDs above.
-- The application layer resolves them by name at startup and caches the
-- UUID pair in memory for the lifetime of the process.
-- DO NOT substitute different UUIDs here — §13 finding 36 requires the same
-- value propagate through §2a, §2b, §2d, §5, and §13 acceptance criterion 36.

CREATE TABLE IF NOT EXISTS inventory_locations (
    id          TEXT PRIMARY KEY,                            -- UUID v7
    name        TEXT NOT NULL,                               -- 'Store Inventory', 'Warehouse A'
    type        TEXT NOT NULL DEFAULT 'store'
                CHECK (type IN ('store', 'warehouse', 'transit', 'damaged', 'virtual')),
    description TEXT NOT NULL DEFAULT '',
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_inventory_locations_type
    ON inventory_locations(type);
CREATE INDEX IF NOT EXISTS idx_inventory_locations_active
    ON inventory_locations(is_active);
CREATE UNIQUE INDEX IF NOT EXISTS idx_inventory_locations_name_unique
    ON inventory_locations(name) WHERE is_active = 1;

-- ── Seed canonical default location ─────────────────────────────────
-- Used by:
--   * Legacy `adjust_stock` (products.rs) backward-compatible routing
--   * Default value for migrations 079 and 080 `location_id` ADD COLUMN
--   * Stock-transfers inventory backfill (per ADR §2d note)
INSERT OR IGNORE INTO inventory_locations (id, name, type, description)
VALUES ('01926b3a-0000-7000-8000-000000000001',
        'Default Inventory',
        'store',
        'Canonical default location for legacy single-location deployments and migration backfills. ADR-18 §13 finding 36.');

-- ── Seed canonical transit location ─────────────────────────────────
-- Used by ADR §7 cross-location transfer flow:
--   1. Deduct qty from source location → adjust_stock_at_location(reason='transfer-out')
--   2. Credit qty to transit → adjust_stock_at_location(reason='transfer-in-transit')
--   3. On receive: deduct qty from transit → credit qty to destination
--   4. On cancel: reverse the deduction
-- This pseudo-location is `is_active = 1` but hidden from front-end
-- location pickers — only the transfer engine resolves its UUID literal.
INSERT OR IGNORE INTO inventory_locations (id, name, type, description)
VALUES ('01926b3a-0000-7000-8000-000000000002',
        'In Transit',
        'transit',
        'System-managed pseudo-location for in-flight stock between source and destination during a transfer. ADR-18 §7.');
