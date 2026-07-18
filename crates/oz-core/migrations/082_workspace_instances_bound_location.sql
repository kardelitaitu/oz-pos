-- 082_workspace_instances_bound_location.sql
-- ADR #18 §5: each inventory workspace instance is bound to one location
-- as the default scope, with an optional location-picker dropdown in the
-- header to view/act on other locations.
--
-- Adds a NULLABLE `bound_location_id` FK column on `workspace_instances`.
-- SPECIFICALLY NULLABLE rather than NOT NULL because:
--
--   1. Legacy single-location deployments import as unbound admin consoles
--      (`bound_location_id IS NULL`) per ADR §5 fallback row.
--   2. Existing `workspace_instances` rows predate this migration; defaulting
--      them to the canonical default-location UUID would silently bind
--      every non-inventory workspace (e.g. retail-pos) to the default
--      inventory location, which is operationally wrong.
--   3. The NULL state is semantically meaningful — it represents the
--      "unbound admin console" cross-location aggregate view, not "unknown".
--
-- ON DELETE RESTRICT enforces ADR §5 / §2 invariant: a workspace cannot
-- outlive its bound location via hard-delete. The 'default' and 'transit'
-- locations (migration 078) are seeded is_active = 1 forever, so legacy
-- data is safe.
--
-- Per ADR §5 split-brain prevention: a workspace_instance MUST NOT have
-- both `bound_location_id` set AND rows in `workspace_inventory_locations`
-- (the §4 multi-binding table). The unified `get_workspace_locations()`
-- Rust resolver (Phase 1 follow-up) enforces this at the application
-- layer — SQLite CHECK constraints cannot reference other tables, so
-- database-level enforcement is not possible without triggers.

ALTER TABLE workspace_instances ADD COLUMN bound_location_id TEXT
    REFERENCES inventory_locations(id) ON DELETE RESTRICT;

-- Index for §5 default-scope queries ("which workspace is bound to
-- location X?"). Per-location audit dashboards and the workspace
-- resolver both filter by bound_location_id; an index keeps those
-- lookups out of the workspace_instances full-scan path.
-- Distinct leading column from any existing workspace_instances index
-- (migration 060 has idx_workspace_instances_type vs type_key).
CREATE INDEX IF NOT EXISTS idx_workspace_instances_bound_location
    ON workspace_instances(bound_location_id);
