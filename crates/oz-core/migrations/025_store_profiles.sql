-- 025_store_profiles.sql
-- Multi-store support: each location has its own identity, settings,
-- and feature flags. The primary store is created at first startup.
-- Adding a new store inserts a row here; feature flags and settings
-- gain a `store_id` column in a follow-up migration.

CREATE TABLE IF NOT EXISTS store_profiles (
    id          TEXT PRIMARY KEY,                                   -- "default" or UUID for additional stores
    name        TEXT NOT NULL,
    address     TEXT DEFAULT '',
    tax_id      TEXT DEFAULT '',
    currency    TEXT NOT NULL DEFAULT 'USD',
    timezone    TEXT NOT NULL DEFAULT 'UTC',
    is_primary  INTEGER NOT NULL DEFAULT 0,                        -- exactly one store is the primary
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_store_profiles_primary
    ON store_profiles(is_primary) WHERE is_primary = 1;

-- Seed the default store so foreign key constraints in
-- later migrations (066) have a valid reference. Single-store
-- deployments use this row; multi-store deployments add more.
-- is_primary is 0 here — the store_profiles table has a unique
-- partial index on is_primary=1, and making this the primary
-- would conflict with tests that create their own primary stores.
INSERT OR IGNORE INTO store_profiles (id, name, is_primary)
VALUES ('default', 'Default Store', 0);
