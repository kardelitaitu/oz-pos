-- 018_offline_queue.sql
-- Offline transaction queue for when the network is unavailable.
-- Sales are enqueued locally and synced when connectivity is restored.

CREATE TABLE IF NOT EXISTS offline_queue (
    id              TEXT PRIMARY KEY,
    action          TEXT NOT NULL,          -- e.g. "complete_sale", "void_sale"
    payload         TEXT NOT NULL,          -- JSON-serialized action data
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending | synced | failed
    retry_count     INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    synced_at       TEXT
);

CREATE INDEX IF NOT EXISTS idx_offline_queue_status ON offline_queue(status);
