-- User per-display preferences (card size, font size, etc).
-- Scoped per staff member so each user's layout choices persist
-- across sessions and terminals.

CREATE TABLE IF NOT EXISTS user_preferences (
    user_id    TEXT NOT NULL,
    pref_key   TEXT NOT NULL,
    pref_value TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (user_id, pref_key)
);
