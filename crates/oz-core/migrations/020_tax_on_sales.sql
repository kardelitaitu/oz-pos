-- 020_tax_on_sales.sql — Tax breakdown on sale lines and order records.
--
-- This migration adds per-line tax tracking to sale_lines and aggregate
-- tax totals to the sales table, enabling tax breakdown display on
-- receipts and in order history.

ALTER TABLE sale_lines ADD COLUMN tax_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sale_lines ADD COLUMN tax_rate_id TEXT REFERENCES tax_rates(id);

ALTER TABLE sales ADD COLUMN subtotal_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sales ADD COLUMN tax_total_minor INTEGER NOT NULL DEFAULT 0;
