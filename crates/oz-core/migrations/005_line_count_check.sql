-- 005_line_count_check.sql — relax line_count CHECK to allow empty sales.
--
-- The original CHECK (line_count > 0) prevents persisting an empty cart
-- as a sale. Sale::from_cart can produce a sale with 0 lines (empty
-- cart), and the code should handle this gracefully at the database
-- layer too.

-- SQLite doesn't support ALTER TABLE to modify a CHECK constraint.
-- We must recreate the table. Since the sales table may have data in
-- production, we copy existing rows.

CREATE TABLE sales_new (
    id          TEXT PRIMARY KEY,
    total_minor INTEGER NOT NULL,
    currency    TEXT NOT NULL,
    line_count  INTEGER NOT NULL CHECK (line_count >= 0),
    status      TEXT NOT NULL DEFAULT 'pending',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO sales_new (id, total_minor, currency, line_count, status, created_at, updated_at)
    SELECT id, total_minor, currency, line_count, status, created_at, updated_at FROM sales;

DROP TABLE sales;

ALTER TABLE sales_new RENAME TO sales;

CREATE INDEX IF NOT EXISTS idx_sales_created_at ON sales(created_at);
