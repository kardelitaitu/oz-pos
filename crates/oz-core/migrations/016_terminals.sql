-- 016_terminals.sql
-- Terminal registration table for multi-terminal support.
-- Each POS device registers itself as a terminal with a unique ID,
-- name, and device identifier.

CREATE TABLE IF NOT EXISTS terminals (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    device_id       TEXT NOT NULL UNIQUE,
    terminal_secret TEXT,                   -- optional shared secret for sync auth
    is_active       INTEGER NOT NULL DEFAULT 1,
    last_seen_at    TEXT,
    metadata        TEXT,                   -- JSON blob for extra info
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_terminals_device_id ON terminals(device_id);
