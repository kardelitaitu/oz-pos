-- 034_active_carts.sql
-- Persist active (in-progress) carts to SQLite so they survive a
-- process restart. Carts are stored as serialised JSON in the
-- `cart_data` column, mirroring the same approach used for held carts.
--
-- On startup the app can load any active carts and present them as
-- "pending" orders that the cashier can resume.

CREATE TABLE IF NOT EXISTS active_carts (
    id              TEXT PRIMARY KEY,
    currency        TEXT NOT NULL DEFAULT 'USD',
    cart_data       TEXT NOT NULL,
    line_count      INTEGER NOT NULL DEFAULT 0,
    total_minor     INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Index for listing active carts by last-updated (most recent first).
CREATE INDEX IF NOT EXISTS idx_active_carts_updated_at ON active_carts(updated_at DESC);
