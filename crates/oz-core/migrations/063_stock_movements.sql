-- 063_stock_movements.sql — CRDT Delta Ledger for Inventory (ADR #6)
--
-- Adds the `stock_movements` table, an append-only delta ledger that
-- replaces absolute quantity overwrites with immutable delta rows.
-- Current stock quantity is computed as SUM(delta) for a given product.
--
-- The existing `inventory` table is retained as a materialized cache,
-- updated on every delta write for backward compatibility and query
-- performance. It is rebuilt from the ledger during migration.

CREATE TABLE IF NOT EXISTS stock_movements (
    id                  TEXT PRIMARY KEY,        -- UUID v4
    item_id             TEXT NOT NULL,           -- product ID (FK to products.id)
    delta               INTEGER NOT NULL,        -- +N or -N
    reason              TEXT,                    -- 'sale', 'restock', 'correction', 'stock-take', etc.
    source_terminal_id  TEXT,                    -- terminal that performed the operation
    source_user_id      TEXT,                    -- user who performed the operation
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_stock_movements_item
    ON stock_movements(item_id, created_at);

-- Materialised stock summary — rebuilt from the delta ledger.
-- One row per product, updated on every delta write.
-- This avoids aggregating the full history on every inventory lookup.
CREATE TABLE IF NOT EXISTS stock_summary (
    item_id     TEXT PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    qty         INTEGER NOT NULL DEFAULT 0,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Backfill stock_summary from the existing inventory table so
-- existing stock levels are preserved.
INSERT OR IGNORE INTO stock_summary (item_id, qty, updated_at)
    SELECT product_id, qty, updated_at FROM inventory
    WHERE qty > 0;

-- Seed an initial stock_movement row for each existing inventory entry
-- so the ledger is consistent with the materialised summary.
INSERT OR IGNORE INTO stock_movements (id, item_id, delta, reason, created_at)
    SELECT
        lower(hex(randomblob(16))),  -- UUID v4 compatible ID
        product_id,
        qty,
        'migration-seed',
        updated_at
    FROM inventory
    WHERE qty > 0;
