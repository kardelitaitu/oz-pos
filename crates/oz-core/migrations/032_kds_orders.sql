-- Kitchen Display System: tracks order status in the kitchen.

-- Extends the sales system with kitchen-specific status and timestamps.
CREATE TABLE IF NOT EXISTS kds_orders (
    id          TEXT PRIMARY KEY,
    sale_id     TEXT NOT NULL UNIQUE REFERENCES sales(id),
    -- 'pending', 'preparing', 'ready', 'served', 'cancelled'
    status      TEXT NOT NULL DEFAULT 'pending',
    -- Comma-separated list of item display names for the ticket
    items_summary TEXT NOT NULL DEFAULT '',
    -- Total items count
    item_count  INTEGER NOT NULL DEFAULT 0,
    -- Order display number (human-readable, auto-increment per day)
    display_number INTEGER,
    -- Timestamps for each stage
    received_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    started_at  TEXT,
    ready_at    TEXT,
    served_at   TEXT,
    -- Estimated preparation time in seconds
    prep_time_seconds INTEGER DEFAULT 0,
    -- Notes from the POS (e.g., "no onions")
    notes       TEXT NOT NULL DEFAULT ''
);

-- Auto-increment counter per day for display numbers.
CREATE TABLE IF NOT EXISTS kds_daily_counters (
    date        TEXT PRIMARY KEY,
    counter     INTEGER NOT NULL DEFAULT 0
);
