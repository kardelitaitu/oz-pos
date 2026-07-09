-- Stock count / physical inventory tables for cycle counting and reconciliation.
-- Depends on: products table (002_products.sql), users table (021_shifts.sql).

CREATE TABLE IF NOT EXISTS stock_counts (
    id          TEXT PRIMARY KEY,
    count_number TEXT NOT NULL UNIQUE,
    status      TEXT NOT NULL DEFAULT 'draft'
                CHECK (status IN ('draft', 'in_progress', 'completed', 'cancelled')),
    count_type  TEXT NOT NULL DEFAULT 'full'
                CHECK (count_type IN ('full', 'cyclic', 'spot')),
    notes       TEXT NOT NULL DEFAULT '',
    counted_by  TEXT REFERENCES users(id),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    completed_at TEXT,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS stock_count_lines (
    id            TEXT PRIMARY KEY,
    count_id      TEXT NOT NULL REFERENCES stock_counts(id) ON DELETE CASCADE,
    sku           TEXT NOT NULL,
    product_name  TEXT NOT NULL DEFAULT '',
    expected_qty  INTEGER NOT NULL DEFAULT 0,
    counted_qty   INTEGER,
    difference    INTEGER NOT NULL DEFAULT 0,
    notes         TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS stock_adjustments (
    id            TEXT PRIMARY KEY,
    count_id      TEXT REFERENCES stock_counts(id) ON DELETE SET NULL,
    sku           TEXT NOT NULL,
    product_name  TEXT NOT NULL DEFAULT '',
    previous_qty  INTEGER NOT NULL DEFAULT 0,
    adjusted_qty  INTEGER NOT NULL DEFAULT 0,
    reason        TEXT NOT NULL DEFAULT '',
    created_by    TEXT REFERENCES users(id),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_stock_count_lines_count_id ON stock_count_lines(count_id);
CREATE INDEX IF NOT EXISTS idx_stock_adjustments_count_id ON stock_adjustments(count_id);
CREATE INDEX IF NOT EXISTS idx_stock_counts_status ON stock_counts(status);
