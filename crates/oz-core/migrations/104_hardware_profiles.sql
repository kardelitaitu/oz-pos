-- 104_hardware_profiles.sql
-- Hardware profiles — stores per-terminal hardware configuration in the DB
-- with schema versioning for forward-compatible evolution.
--
-- The existing terminal_profiles/*.json files remain as a fallback/cache
-- for backward compatibility, but the DB table is now the canonical store.
-- On read: DB first, JSON fallback, old SQLite settings as last resort.
-- On write: both DB and JSON are updated to keep them in sync.
--
-- schema_version starts at 1. Bump when the TerminalProfile Rust struct
-- gains or loses fields that would break deserialization of stored JSON.

CREATE TABLE IF NOT EXISTS hardware_profiles (
    terminal_id    TEXT PRIMARY KEY,
    profile_json   TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
