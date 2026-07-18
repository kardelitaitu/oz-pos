-- 083_workspace_inventory_locations.sql
-- ADR #18 §4: Multi-binding framework — a single POS workspace instance
-- can bind to MULTIPLE inventory locations. This is the multi-binding
-- companion to migration 082's single-binding `bound_location_id` FK.
--
-- Per the ADR's schema (verbatim except for comments):
--
--   CREATE TABLE workspace_inventory_locations (
--       id                  TEXT PRIMARY KEY,
--       instance_id         TEXT NOT NULL REFERENCES workspace_instances(id) ON DELETE CASCADE,
--       location_id         TEXT NOT NULL REFERENCES inventory_locations(id) ON DELETE RESTRICT,
--       is_primary          INTEGER NOT NULL DEFAULT 0,  -- exactly one primary per instance
--       allow_negative_stock INTEGER NOT NULL DEFAULT 0, -- per-location override
--       sort_order          INTEGER NOT NULL DEFAULT 0,
--       UNIQUE(instance_id, location_id)
--   );
--
-- Semantics:
--
--   * `is_primary = 1` designates the location from which the linked
--     workspace DEDUCTS stock on sale. Exactly one row per instance MAY
--     have `is_primary = 1`; the ADR enforces this at the application
--     layer (SQLite CHECK constraints cannot reference other rows, so no
--     database-level enforcement is possible without triggers).
--
--   * `allow_negative_stock = 1` lets this specific bound location go
--     below zero when stock is insufficient. Per ADR §4 alert-requirement,
--     the backend MUST emit a `stock.negative` warning event whenever a
--     deduction under this flag drives stock below zero, and the inventory
--     dashboard MUST show a "stock below zero" badge with affected SKUs
--     and locations. This prevents the flag from silently turning
--     inventory into a suggestion system.
--
--   * `UNIQUE(instance_id, location_id)` is the natural primary composite
--     access key — a workspace cannot be bound twice to the same location.
--     The implicit unique-index on this pair also serves the §4 access
--     pattern "all locations bound to instance X" without needing a
--     separate index; the explicit idx_ws_inv_locations_instance below
--     makes that intent searchable in `sqlite_master` for ops/troubleshooting.
--
--   * `sort_order` lets the front-end picker order bound locations
--     predictably when is_primary = 1 row is followed by is_primary = 0
--     lookups sorted by sort_order, name.
--
-- §5 split-brain prevention: a workspace_instance MUST NOT have BOTH
-- `bound_location_id` set AND rows in `workspace_inventory_locations`.
-- The unified `get_workspace_locations()` Rust resolver (Phase 1
-- follow-up) enforces this XOR at the application layer; SQLite has no
-- cross-table CHECK construct without triggers.

CREATE TABLE IF NOT EXISTS workspace_inventory_locations (
    id                   TEXT PRIMARY KEY,
    instance_id          TEXT NOT NULL REFERENCES workspace_instances(id) ON DELETE CASCADE,
    location_id          TEXT NOT NULL REFERENCES inventory_locations(id) ON DELETE RESTRICT,
    is_primary           INTEGER NOT NULL DEFAULT 0
                         CHECK (is_primary IN (0, 1)),
    allow_negative_stock INTEGER NOT NULL DEFAULT 0
                         CHECK (allow_negative_stock IN (0, 1)),
    sort_order           INTEGER NOT NULL DEFAULT 0,
    UNIQUE(instance_id, location_id)
);

-- The UNIQUE(instance_id, location_id) constraint above already creates
-- an implicit index serving the primary access pattern ("all locations
-- bound to instance X"). The explicit index below is redundant but
-- auditable — makes the intent searchable in sqlite_master rather than
-- implicit-on-the-unique-constraint. Per ADR §4's documented schema.
CREATE INDEX IF NOT EXISTS idx_ws_inv_locations_instance
    ON workspace_inventory_locations(instance_id);

-- Reverse-direction lookup: "which workspaces are bound to location Y?"
-- Supports §5 resolver queries and per-location audit dashboards filtering
-- "show me all workspaces that can see Warehouse A stock".
-- Non-overlapping with the (instance_id)-leading index above.
CREATE INDEX IF NOT EXISTS idx_ws_inv_locations_location
    ON workspace_inventory_locations(location_id);

-- Single-primary enforcement is enforced at the application layer per ADR §4,
-- but we DO bind a partial-unique index on (instance_id) WHERE is_primary = 1
-- so SQLite blocks accidental duplicate primaries at write time. This is
-- the part of §4 the ADR keeps at the database layer.
CREATE UNIQUE INDEX IF NOT EXISTS idx_ws_inv_locations_one_primary_per_instance
    ON workspace_inventory_locations(instance_id) WHERE is_primary = 1;
