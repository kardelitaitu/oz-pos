-- Add payment tracking fields to the sales table.

ALTER TABLE sales ADD COLUMN payment_method TEXT;
ALTER TABLE sales ADD COLUMN tendered_minor INTEGER;
