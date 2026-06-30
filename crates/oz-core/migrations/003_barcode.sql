-- 003_barcode.sql — add barcode column to the products table.
--
-- Barcodes are optional (nullable) and must be unique when present.
-- Two products can both have a NULL barcode (SQLite treats NULLs as
-- distinct for UNIQUE constraints).
--
-- SQLite's ALTER TABLE ADD COLUMN does not support the UNIQUE constraint,
-- so we add the column first then enforce uniqueness via a unique index.

ALTER TABLE products ADD COLUMN barcode TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS uq_products_barcode ON products(barcode);
