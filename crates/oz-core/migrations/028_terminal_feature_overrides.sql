-- 028_terminal_feature_overrides.sql
-- Per-terminal feature overrides table.
-- Allows overriding feature flags on a per-terminal basis.
CREATE TABLE IF NOT EXISTS terminal_feature_overrides (
    terminal_id TEXT NOT NULL REFERENCES terminals(id) ON DELETE CASCADE,
    feature     TEXT NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (terminal_id, feature)
);
