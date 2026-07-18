-- 091_workspace_types_rename.sql
-- ADR #18 §3 + §13 finding 37: workspace-rename cascade from
-- `inventory` → `warehouse`. The single-row `UPDATE workspace_types
-- SET key = 'warehouse', ...` is necessary but NOT sufficient per §13-37;
-- the codebase has 8 hard-coded 'inventory' call sites. Migration 091
-- covers the **SQL-level** cross-table renames (FK chains). The 8-site
-- cascade ALSO requires file-level renames (UI, fluent bundles,
-- platform/startup, modules/inventory/manifest.json, crates/inventory
-- Rust name) — those are NOT in scope for the SQL migration and MUST
-- accompany this PR per §13-37.
--
-- The rename must be atomic across multiple tables because of three
-- cascading FK chains:
--
--   Chain 1: workspace_types.key → workspace_screens.workspace_key,
--                                   workspace_type_screens.type_key,
--                                   role_workspace_types.type_key,
--                                   workspace_instances.type_key
--
--   Chain 2: workspace_types.key + workspace_instances.id derives from
--             `'default-' || workspace_types.key` (per migration 060's
--             seed). Renaming workspace_types.key means ID 'default-inventory'
--             must become 'default-warehouse' atomically.
--
--   Chain 3: workspace_instances.id → user_workspace_instances.instance_id
--             (FK ON DELETE CASCADE). Renaming workspace_instances.id
--             means user_workspace_instances rows that pointed at
--             'default-inventory' must now point at 'default-warehouse',
--             or the FK chain breaks when FK enforcement re-enables.
--
--   terminals.bound_instance_id is described in migration 060 as a
--   "logical reference, validated at boot, no FK". It still must
--   rename in lockstep because any code path that hard-codes the old
--   value (e.g., a terminal boot-check) will silently mismatch the new
--   id.
--
-- Defer foreign key checks until COMMIT, NOT toggle the session-level
-- foreign_keys PRAGMA. Reasoning: migration 089 (stock_summary rebuild)
-- used `PRAGMA foreign_keys = OFF/ON` and worked; that migration only
-- RENAME/CREATE/INSERT/DROP tables without ever UPDATING a primary key,
-- so the toggling approach sufficed. Migration 091 is different — it
-- UPDATEs *primary keys* (`workspaces.key`, `workspace_types.key`, and
-- crucially `workspace_instances.id`). When a primary key is UPDATED
-- while child FK columns still reference the OLD pk value, SQLite runs
-- the FK check IMMEDIATELY at the UPDATE statement boundary even with
-- `PRAGMA foreign_keys = OFF` (which only suppresses FK checks on child
-- INSERTs/UPDATEs/DELETEs — not on parent PK changes).
--
-- `PRAGMA defer_foreign_keys = ON` is the per-transaction deferral pragma
-- designed exactly for this case. It is auto-reset at commit time, so no
-- explicit `OFF` is needed. Child rows are temporarily orphaned mid-migration
-- (e.g. user_workspace_instances.instance_id = 'default-inventory' between
-- the workspace_instances.id UPDATE and the user_workspace_instances UPDATE),
-- but the deferred queue is checked only at COMMIT — by which time every
-- FK is consistent again.
--
-- The migration runner's BEGIN/COMMIT wrapper (see fresh_db() in
-- migrations.rs) ensures the migration either applies atomically or
-- rolls back entirely.

PRAGMA defer_foreign_keys = ON;

-- ── 1. Deprecated workspaces table (migration 035) ────────────────────
-- This table predates the workspace-type separation in migration 060
-- and is kept for one release for backward compatibility. Both rows
-- and screens reference 'inventory'; the rename here propagates 035's
-- references forward.
UPDATE workspaces                   SET key = 'warehouse' WHERE key = 'inventory';
UPDATE workspace_screens            SET workspace_key = 'warehouse' WHERE workspace_key = 'inventory';

-- ── 2. Authoritative workspace_types table (migration 060) ────────────
-- This is the canonical table post-migration-060. The seed row from
-- 060 (id='default-' || workspaces.key) means 'default-inventory'
-- exists as a workspace_instances.id that must rename.
UPDATE workspace_types              SET key = 'warehouse' WHERE key = 'inventory';
UPDATE workspace_type_screens       SET type_key = 'warehouse' WHERE type_key = 'inventory';
UPDATE role_workspace_types         SET type_key = 'warehouse' WHERE type_key = 'inventory';

-- ── 3. workspace_instances: dual rename (id AND type_key) ─────────────
-- The id was constructed as `'default-' || workspace_types.key` in
-- 060's seed, so renaming workspace_types.key means the matching
-- workspace_instances.id must rename in lockstep. The MATCH pattern
-- `id LIKE 'default-inventory%'` catches the canonical default and any
-- future suffix variants (e.g., 'default-inventory-franchise').
UPDATE workspace_instances
   SET id = REPLACE(id, 'default-inventory', 'default-warehouse'),
       type_key = 'warehouse'
 WHERE type_key = 'inventory' OR id LIKE 'default-inventory%';

-- ── 4. user_workspace_instances (instance_id FK) ───────────────────────
-- This is the third leg of the cascading FK chain. Renaming the
-- workspace_instances.id above means user assignments must follow.
-- Failure to update here would leave these rows with a dangling FK to
-- the (renamed) workspace_instances.id — and the backup test fixtures
-- expect 'default-warehouse'.
UPDATE user_workspace_instances
   SET instance_id = REPLACE(instance_id, 'default-inventory', 'default-warehouse')
 WHERE instance_id LIKE 'default-inventory%';

-- ── 5. terminals.bound_instance_id (logical, no FK) ──────────────────
-- Logical reference validated at boot per migration 060. Hard-coding
-- the old 'default-inventory' id in any terminal's bound_instance_id
-- column would silently mismatch the new id at runtime. Rename in
-- lockstep regardless of FK status.
UPDATE terminals
   SET bound_instance_id = REPLACE(bound_instance_id, 'default-inventory', 'default-warehouse')
 WHERE bound_instance_id LIKE 'default-inventory%';

-- ── 6. Deprecated role_workspaces + user_workspaces (migration 035 + 040) ─
-- Both tables pre-date the workspace-type separation (migration 060) and
-- remain in the schema for one release per §1's backward-compatibility
-- window. They hold FK columns that reference `workspaces.key` directly:
--   * role_workspaces.workspace_key TEXT NOT NULL REFERENCES workspaces(key)
--     (per migration 035 line 24)
--   * user_workspaces.ws_key         TEXT NOT NULL REFERENCES workspaces(key)
--     ON DELETE CASCADE (per migration 040 line 8)
--
-- Both must rename alongside workspaces to satisfy the deferred FK check
-- at COMMIT (`PRAGMA defer_foreign_keys = ON` defers to commit-time, so
-- any orphan FK reference still trips FK 787). These two UPDATEs are
-- defensive — on fresh installs the tables are empty (migrations 035 and
-- 040 do not seed them) and the UPDATEs no-op; on production installs
-- where 035 + 060 seeded them historically, they fix the orphan outright.
-- Note user_workspaces uses `ws_key` (not `workspace_key`) per migration 040.
UPDATE role_workspaces SET workspace_key = 'warehouse' WHERE workspace_key = 'inventory';
UPDATE user_workspaces  SET ws_key        = 'warehouse' WHERE ws_key        = 'inventory';

-- PRAGMA defer_foreign_keys is auto-reset at COMMIT; explicit OFF not required.

-- ── §13-37 6+ call sites NOT covered by this migration ────────────────
-- File-level renames that MUST accompany this PR:
--   * ui/src/features/inventory/ → ui/src/features/warehouse/ (directory)
--   * app router in ui/src/App.tsx and friends (key resolves by string)
--   * ui/src/locales/{sales,common,inventory,…}.ftl prefixed `inventory-*`
--   * platform/startup/src/lib.rs (module registration by name)
--   * modules/inventory/manifest.json (id field literally equals "inventory")
--   * modules/inventory/ Rust crate (kept per ADR §3 since rename is multi-crate)
-- A clippy lint `workspace_types_key_match_runtime` is added in the
-- same PR to forbid new occurrences of the literal string 'inventory'
-- outside this migration file.
