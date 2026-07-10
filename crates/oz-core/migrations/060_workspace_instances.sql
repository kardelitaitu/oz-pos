-- 060_workspace_instances.sql
-- Workspace Type/Instance Separation (ADR #4 Phase 1)
--
-- Separates workspace TYPE (UI template) from workspace INSTANCE
-- (deployment within a store). Old tables (workspaces, workspace_screens,
-- role_workspaces, user_workspaces) are kept for one release.

-- ── Workspace Types (global UI templates) ─────────────────────────────

CREATE TABLE IF NOT EXISTS workspace_types (
    key            TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    layout_mode    TEXT NOT NULL DEFAULT 'fullscreen',  -- 'fullscreen' | 'sidebar'
    icon           TEXT NOT NULL DEFAULT '',
    sort_order     INTEGER NOT NULL DEFAULT 0,
    accent_colour  TEXT NOT NULL DEFAULT ''
);

-- Nav items within a workspace type
CREATE TABLE IF NOT EXISTS workspace_type_screens (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    type_key    TEXT NOT NULL REFERENCES workspace_types(key),
    screen_key  TEXT NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    UNIQUE(type_key, screen_key)
);

-- ── Workspace Instances (per-store deployments) ───────────────────────

CREATE TABLE IF NOT EXISTS workspace_instances (
    id          TEXT PRIMARY KEY,
    type_key    TEXT NOT NULL REFERENCES workspace_types(key),
    store_id    TEXT NOT NULL,                             -- boot-time validation field
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    colour      TEXT,                                      -- optional accent colour override
    status      TEXT NOT NULL DEFAULT 'active',             -- 'active', 'quota_suspended', 'archived'
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_workspace_instances_type
    ON workspace_instances(type_key);

-- ── User-to-Instance Assignments ─────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_workspace_instances (
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    instance_id  TEXT NOT NULL REFERENCES workspace_instances(id) ON DELETE CASCADE,
    is_default   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, instance_id)
);

CREATE INDEX IF NOT EXISTS idx_user_wsi_user_id
    ON user_workspace_instances(user_id);

-- ── Role-to-Type Access ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS role_workspace_types (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    role_id   TEXT NOT NULL REFERENCES roles(id),
    type_key  TEXT NOT NULL REFERENCES workspace_types(key),
    UNIQUE(role_id, type_key)
);

-- ── Store-Level User Access (multi-store chain access control) ───────

CREATE TABLE IF NOT EXISTS user_store_access (
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    store_id     TEXT NOT NULL REFERENCES store_profiles(id) ON DELETE CASCADE,
    access_level TEXT NOT NULL DEFAULT 'operator',  -- 'operator' | 'manager' | 'viewer'
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, store_id)
);

-- ── Device Binding Columns on Terminals ──────────────────────────────
--
-- These ALTER TABLE statements are safe because the migration runner
-- (platform_core::database::run) only applies each migration once via the
-- schema_migrations table. If this migration ID is already applied, the
-- runner skips it entirely.
--
-- bound_store_id FK references store_profiles (global DB)
-- bound_instance_id is a logical reference (validated at boot, no FK)

ALTER TABLE terminals ADD COLUMN bound_store_id TEXT REFERENCES store_profiles(id);
ALTER TABLE terminals ADD COLUMN bound_instance_id TEXT;
ALTER TABLE terminals ADD COLUMN binding_signature TEXT;

-- ── Seed: Copy Workspaces → Workspace Types ──────────────────────────

INSERT OR IGNORE INTO workspace_types (key, name, description, layout_mode, icon, sort_order, accent_colour)
SELECT
    key,
    name,
    description,
    CASE key
        WHEN 'kds' THEN 'fullscreen'
        WHEN 'restaurant-pos' THEN 'fullscreen'
        WHEN 'store-pos' THEN 'fullscreen'
        ELSE 'sidebar'
    END,
    icon,
    CASE key
        WHEN 'restaurant-pos' THEN 1
        WHEN 'store-pos' THEN 2
        WHEN 'kds' THEN 3
        WHEN 'inventory' THEN 4
        WHEN 'admin' THEN 5
        ELSE 99
    END,
    ''
FROM workspaces;

-- ── Seed: Copy Workspace Screens → Workspace Type Screens ────────────

INSERT OR IGNORE INTO workspace_type_screens (type_key, screen_key, sort_order)
SELECT
    workspace_key AS type_key,
    screen_key,
    sort_order
FROM workspace_screens;

-- ── Seed: Create Default Instances (one per type, primary store) ─────

INSERT OR IGNORE INTO workspace_instances (id, type_key, store_id, name, description)
SELECT
    'default-' || wt.key,
    wt.key,
    COALESCE(
        (SELECT id FROM store_profiles WHERE is_primary = 1 LIMIT 1),
        'default'
    ),
    wt.name,
    wt.description
FROM workspace_types wt;

-- ── Migrate: user_workspaces → point at default instances ────────────

INSERT OR IGNORE INTO user_workspace_instances (user_id, instance_id, is_default, created_at)
SELECT
    uw.user_id,
    'default-' || uw.ws_key,
    0,
    uw.created_at
FROM user_workspaces uw
WHERE EXISTS (
    SELECT 1 FROM workspace_instances wi
    WHERE wi.id = 'default-' || uw.ws_key
);

-- ── Migrate: role_workspaces → role_workspace_types ──────────────────

INSERT OR IGNORE INTO role_workspace_types (role_id, type_key)
SELECT
    role_id,
    workspace_key AS type_key
FROM role_workspaces;

-- ── Phase 2 Prep: user_store_access for owner ────────────────────────

-- Assign the owner role to the primary store by default for backward compat.
-- In single-store mode this is transparent — owner always sees the sole store.
-- Falls back to the first store if no primary store is set (edge case).
INSERT OR IGNORE INTO user_store_access (user_id, store_id, access_level)
SELECT
    u.id AS user_id,
    sp.id AS store_id,
    'manager'
FROM users u
CROSS JOIN (
    SELECT id FROM store_profiles WHERE is_primary = 1
    UNION ALL
    SELECT id FROM store_profiles WHERE is_primary != 1 OR is_primary IS NULL
    LIMIT 1
) sp
WHERE u.role_id = 'role-owner';

-- ── Note ─────────────────────────────────────────────────────────────
-- Old tables (workspaces, workspace_screens, role_workspaces, user_workspaces)
-- are NOT dropped. They remain for one release to support backward compatibility
-- and will be removed in a future migration once all consumers have migrated.
