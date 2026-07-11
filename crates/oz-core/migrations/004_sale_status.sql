-- 004_sale_status.sql — add status and updated_at columns to the sales table.
--
-- 001_sales.sql created the sales table with id, total_minor, currency,
-- line_count, and created_at. This migration adds the columns needed by
-- the Sale domain type's state machine.

ALTER TABLE sales ADD COLUMN status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE sales ADD COLUMN updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
