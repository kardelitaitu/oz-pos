-- 113_stock_count_actor_ids.sql — decouple stock-count actors from local auth FKs.
--
-- Session authentication is resolved from the global identity database, while
-- stock counts and adjustments live in per-store databases. Rebuild the count
-- tables so a valid global actor never requires a fake local users row.
-- Existing rows are copied before the old tables are removed.
--
-- FAIL-CLOSED DATA NOTE: the rebuilt tables add CHECK (qty >= 0) constraints
-- that the original schema (migration 046) lacked. If a legacy store database
-- contains a negative expected/counted/previous/adjusted quantity, this
-- migration aborts startup with a constraint error instead of silently
-- propagating inconsistent stock evidence. Correct such rows through an
-- audited stock adjustment before upgrading.

CREATE TABLE stock_counts_new (
    id           TEXT PRIMARY KEY,
    count_number TEXT NOT NULL UNIQUE,
    status       TEXT NOT NULL DEFAULT 'draft'
                 CHECK (status IN ('draft', 'in_progress', 'completed', 'cancelled')),
    count_type   TEXT NOT NULL DEFAULT 'full'
                 CHECK (count_type IN ('full', 'cyclic', 'spot')),
    notes        TEXT NOT NULL DEFAULT '',
    counted_by   TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    completed_at TEXT,
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    warehouse_id TEXT
);

INSERT INTO stock_counts_new (
    id, count_number, status, count_type, notes, counted_by,
    created_at, completed_at, updated_at, warehouse_id
)
SELECT
    id, count_number, status, count_type, notes, counted_by,
    created_at, completed_at, updated_at, warehouse_id
FROM stock_counts;

CREATE TABLE stock_count_lines_new (
    id           TEXT PRIMARY KEY,
    count_id     TEXT NOT NULL REFERENCES stock_counts_new(id) ON DELETE CASCADE,
    sku          TEXT NOT NULL,
    product_name TEXT NOT NULL DEFAULT '',
    expected_qty INTEGER NOT NULL DEFAULT 0 CHECK (expected_qty >= 0),
    counted_qty  INTEGER CHECK (counted_qty IS NULL OR counted_qty >= 0),
    difference   INTEGER NOT NULL DEFAULT 0,
    notes        TEXT NOT NULL DEFAULT ''
);

INSERT INTO stock_count_lines_new (
    id, count_id, sku, product_name, expected_qty, counted_qty, difference, notes
)
SELECT
    id, count_id, sku, product_name, expected_qty, counted_qty, difference, notes
FROM stock_count_lines;

CREATE TABLE stock_adjustments_new (
    id            TEXT PRIMARY KEY,
    count_id      TEXT REFERENCES stock_counts_new(id) ON DELETE SET NULL,
    sku           TEXT NOT NULL,
    product_name  TEXT NOT NULL DEFAULT '',
    previous_qty  INTEGER NOT NULL DEFAULT 0 CHECK (previous_qty >= 0),
    adjusted_qty  INTEGER NOT NULL DEFAULT 0 CHECK (adjusted_qty >= 0),
    reason        TEXT NOT NULL DEFAULT '',
    created_by    TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO stock_adjustments_new (
    id, count_id, sku, product_name, previous_qty, adjusted_qty,
    reason, created_by, created_at
)
SELECT
    id, count_id, sku, product_name, previous_qty, adjusted_qty,
    reason, created_by, created_at
FROM stock_adjustments;

DROP INDEX IF EXISTS idx_stock_count_lines_count_id;
DROP INDEX IF EXISTS idx_stock_adjustments_count_id;
DROP INDEX IF EXISTS idx_stock_counts_status;

DROP TABLE stock_count_lines;
DROP TABLE stock_adjustments;
DROP TABLE stock_counts;

ALTER TABLE stock_counts_new RENAME TO stock_counts;
ALTER TABLE stock_count_lines_new RENAME TO stock_count_lines;
ALTER TABLE stock_adjustments_new RENAME TO stock_adjustments;

CREATE INDEX idx_stock_count_lines_count_id ON stock_count_lines(count_id);
CREATE INDEX idx_stock_adjustments_count_id ON stock_adjustments(count_id);
CREATE INDEX idx_stock_counts_status ON stock_counts(status);
