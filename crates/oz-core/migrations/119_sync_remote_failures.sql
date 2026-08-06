-- 119_sync_remote_failures.sql
-- Durable retry and dead-letter state for malformed or permanently
-- incompatible remote items. The payload is retained so an operator can
-- inspect or manually requeue a quarantined item after remediation.

CREATE TABLE IF NOT EXISTS sync_remote_failures (
    item_id         TEXT PRIMARY KEY,
    action          TEXT NOT NULL,
    payload         TEXT NOT NULL,
    attempts        INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error      TEXT NOT NULL,
    first_failed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_failed_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    dead_lettered   INTEGER NOT NULL DEFAULT 0 CHECK (dead_lettered IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_sync_remote_failures_dead_lettered
    ON sync_remote_failures(dead_lettered, last_failed_at);
