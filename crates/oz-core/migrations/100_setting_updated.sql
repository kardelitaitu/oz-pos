-- 100_setting_updated.sql
-- Delta ledger for settings changes — one row per (key, terminal_id) change.
-- Enables per-key LWW version tracking and the settings_updated event used by
-- the SettingsContext provider (Phase 0b) for cross-terminal conflict detection.
--
-- Version strategy: per (key, terminal_id) counter. Each write increments
-- MAX(version) + 1 for the pair, giving every terminal an independent
-- monotonic version for each key it writes.
--
-- Indexes:
--   idx_setting_updated_key_version — ordered scan for latest version per key
--   idx_setting_updated_terminal    — per-terminal audit trail
CREATE TABLE IF NOT EXISTS setting_updated (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    key         TEXT    NOT NULL,
    value       TEXT    NOT NULL,
    terminal_id TEXT    NOT NULL DEFAULT 'unknown',
    version     INTEGER NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_setting_updated_key_version
    ON setting_updated(key, version DESC);

CREATE INDEX IF NOT EXISTS idx_setting_updated_terminal
    ON setting_updated(terminal_id, created_at DESC);
