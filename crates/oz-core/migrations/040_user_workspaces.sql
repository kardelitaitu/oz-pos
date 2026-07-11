-- Per-user workspace access override.
-- When a user has rows here, these replace (not supplement) the
-- workspaces inherited from their role. The owner role always sees
-- all workspaces regardless.
CREATE TABLE IF NOT EXISTS user_workspaces (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ws_key     TEXT NOT NULL REFERENCES workspaces(key) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, ws_key)
);
