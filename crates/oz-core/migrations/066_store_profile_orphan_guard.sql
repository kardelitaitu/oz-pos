-- 066_store_profile_orphan_guard.sql
-- ADR #6: Enforce ON DELETE RESTRICT on store_profiles foreign keys.
--
-- workspace_instances.store_id currently has NO foreign key constraint.
-- user_store_access.store_id currently has ON DELETE CASCADE (dangerous).
-- Both are fixed below so that deleting a store profile that still has
-- active workspace instances or user access grants is rejected by SQLite.
--
-- NOTE: PRAGMA foreign_keys cannot be changed inside a transaction
-- (it is silently ignored). This migration therefore uses dependency-
-- ordered DROP to avoid FK violations — leaf tables are dropped before
-- the tables they reference.

-- ══════════════════════════════════════════════════════════════════
-- 1. Create replacement tables with corrected FK definitions
-- ══════════════════════════════════════════════════════════════════

CREATE TABLE workspace_instances_new (
    id          TEXT PRIMARY KEY,
    type_key    TEXT NOT NULL REFERENCES workspace_types(key),
    store_id    TEXT NOT NULL REFERENCES store_profiles(id)
                              ON DELETE RESTRICT
                              ON UPDATE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    colour      TEXT,
    status      TEXT NOT NULL DEFAULT 'active',
    last_accessed_at TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE user_workspace_instances_new (
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    instance_id  TEXT NOT NULL REFERENCES workspace_instances_new(id) ON DELETE CASCADE,
    is_default   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, instance_id)
);

CREATE TABLE user_store_access_new (
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    store_id     TEXT NOT NULL REFERENCES store_profiles(id)
                              ON DELETE RESTRICT
                              ON UPDATE CASCADE,
    access_level TEXT NOT NULL DEFAULT 'operator',
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, store_id)
);

-- ══════════════════════════════════════════════════════════════════
-- 2. Copy data — only rows that satisfy the new FK constraints
-- ══════════════════════════════════════════════════════════════════

-- Only copy workspace instances whose store_id exists in store_profiles.
-- Migration 060 seeded instances with store_id='default' which may be
-- orphaned if store_profiles hasn't been seeded yet.
INSERT INTO workspace_instances_new
    SELECT id, type_key, store_id, name, description, colour, status,
           last_accessed_at, created_at, updated_at
    FROM workspace_instances
    WHERE store_id IN (SELECT id FROM store_profiles);

-- Only copy user-to-instance assignments for instances that still exist.
INSERT INTO user_workspace_instances_new
    SELECT user_id, instance_id, is_default, created_at
    FROM user_workspace_instances
    WHERE instance_id IN (SELECT id FROM workspace_instances_new);

-- Only copy store access grants for existing stores.
INSERT INTO user_store_access_new
    SELECT user_id, store_id, access_level, created_at
    FROM user_store_access
    WHERE store_id IN (SELECT id FROM store_profiles);

-- ══════════════════════════════════════════════════════════════════
-- 3. Drop old tables in dependency order (leaf tables first)
-- ══════════════════════════════════════════════════════════════════

-- user_workspace_instances is a leaf (nothing references it).
DROP TABLE user_workspace_instances;

-- workspace_instances was only referenced by user_workspace_instances
-- (now dropped), so this succeeds even with FK enforcement ON.
DROP TABLE workspace_instances;

-- user_store_access is a leaf (nothing references it).
DROP TABLE user_store_access;

-- ══════════════════════════════════════════════════════════════════
-- 4. Rename new tables to final names
-- ══════════════════════════════════════════════════════════════════

-- SQLite ALTER TABLE RENAME TO automatically updates FK references
-- in other tables. When workspace_instances_new → workspace_instances,
-- the FK in user_workspace_instances_new is updated to reference
-- workspace_instances(id).
ALTER TABLE workspace_instances_new RENAME TO workspace_instances;
ALTER TABLE user_workspace_instances_new RENAME TO user_workspace_instances;
ALTER TABLE user_store_access_new RENAME TO user_store_access;

-- ══════════════════════════════════════════════════════════════════
-- 5. Re-create indexes
-- ══════════════════════════════════════════════════════════════════

CREATE INDEX IF NOT EXISTS idx_workspace_instances_type
    ON workspace_instances(type_key);

CREATE INDEX IF NOT EXISTS idx_user_wsi_user_id
    ON user_workspace_instances(user_id);

CREATE INDEX IF NOT EXISTS idx_user_store_access_user_id
    ON user_store_access(user_id);
