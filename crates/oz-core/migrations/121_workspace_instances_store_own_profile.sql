-- 121_workspace_instances_store_own_profile.sql
-- Follow-up to 120 (120_reseed_default_workspace_instances.sql). The
-- released 120 re-seeds default workspace instances under
--     store_id = COALESCE(primary store, 'default')
-- On a multi-store database with no primary profile at migration time,
-- that lands every canonical instance under store_id = 'default' — which
-- the store-scoped picker (wi.store_id = ?) never lists, so a named store
-- logs in to an empty workspace picker. Because 120 was already applied on
-- upgraded databases, its definition must never be edited in place
-- (audit/29 DB-02); this migration performs the same repair in two
-- idempotent statements:
--   1. INSERT (with the improved COALESCE) for databases that have not run
--      120 yet, so fresh installs seed correctly from the start;
--   2. UPDATE the rows 120 seeded under 'default' to the preferred profile
--      for databases that ran the original 120.
-- The middle branch (any profile id != 'default' in THIS store DB) covers
-- the multi-store repair: each store DB is migrated independently and holds
-- only the legacy 'default' row from 025 (is_primary = 0) before the app
-- seeds it, so preferring the store's own profile keeps the repair inside
-- the store it is repairing. Single-store DBs have no non-'default'
-- profile and fall through to 'default' unchanged. Both statements are
-- safe to run repeatedly (NOT EXISTS guard / COALESCE fallback keeps the
-- current value when no better profile exists).

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

UPDATE workspace_instances
SET store_id = COALESCE(
    (SELECT id FROM store_profiles WHERE is_primary = 1 LIMIT 1),
    (SELECT id FROM store_profiles WHERE id != 'default' ORDER BY created_at LIMIT 1),
    store_id
)
WHERE id LIKE 'default-%'
  AND store_id = 'default';
