-- 013_held_carts.sql
-- Persist held (parked) orders for later resumption.
--
-- The cart state is stored as JSON in the `cart_data` column so we
-- don't need to replicate the full cart schema. This keeps the
-- hold/resume flow simple: serialize on hold, deserialize on resume.

CREATE TABLE IF NOT EXISTS held_carts (
    id              TEXT PRIMARY KEY,
    label           TEXT NOT NULL,
    cart_data       TEXT NOT NULL,
    item_count      INTEGER NOT NULL DEFAULT 0,
    total_minor     INTEGER NOT NULL DEFAULT 0,
    currency        TEXT NOT NULL DEFAULT 'USD',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Index for listing held carts in chronological order (most recent first).
CREATE INDEX IF NOT EXISTS idx_held_carts_created_at ON held_carts(created_at DESC);
