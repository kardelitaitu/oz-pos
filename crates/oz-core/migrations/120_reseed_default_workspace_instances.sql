-- 120_reseed_default_workspace_instances.sql
-- Self-healing repair for the migration 066 regression window
-- (commit b733de4f .. 0b416553).
--
-- Migration 066 (store_profile_orphan_guard) rebuilt workspace_instances
-- by copying only rows whose store_id existed in store_profiles. During the
-- broken window store_profiles was not seeded until AFTER migrations ran,
-- so on a fresh database every default workspace instance was dropped and
-- the new table ended up empty. Because migration 066 is recorded as
-- "applied" once, it never re-runs, leaving affected deployments with an
-- empty workspace_instances table — the owner logs in to an empty workspace
-- picker.
--
-- This migration Idempotently re-creates the SAME rows migration 060 would
-- have created, using the identical id/store_id convention:
--     id       = 'default-' || workspace_types.key
--     store_id = COALESCE(primary store, this store's own profile, 'default')
-- so it is safe to run on:
--   * broken databases (empty workspace_instances) -> re-populates,
--   * healthy databases (defaults already present) -> INSERT OR IGNORE no-ops,
-- The NOT EXISTS guard keys on the instance id (primary key) so re-running
-- 120 never collides with the rows 060 originally seeded.
--
-- The middle branch (any profile id != 'default' in THIS store DB) is the
-- multi-store repair case: each store DB is migrated independently and the
-- only profile it holds before the app seeds it is the legacy 'default' row
-- from 025 (is_primary = 0). A bare COALESCE(primary, 'default') therefore
-- re-seeds named stores under store_id = 'default', which the store-scoped
-- picker (wi.store_id = ?) never lists. Preferring the store's own profile
-- keeps the repair inside the store it is repairing; single-store DBs have
-- no non-'default' profile and fall through to 'default' unchanged.
--
-- FK safety:
--   type_key references workspace_types(key)        -> always present (060 seeds it)
--   store_id references store_profiles(id)           -> 'default' is seeded by 025
-- Both are guaranteed to exist, so FK enforcement stays ON throughout.

INSERT INTO workspace_instances (id, type_key, store_id, name, description, colour, status, last_accessed_at)
SELECT
    'default-' || wt.key,
    wt.key,
    COALESCE(
        (SELECT id FROM store_profiles WHERE is_primary = 1 LIMIT 1),
        (SELECT id FROM store_profiles WHERE id != 'default' ORDER BY created_at LIMIT 1),
        'default'
    ),
    wt.name,
    wt.description,
    NULL,
    'active',
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM workspace_types wt
WHERE NOT EXISTS (
    SELECT 1 FROM workspace_instances wi
    WHERE wi.id = 'default-' || wt.key
);
