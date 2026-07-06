ALTER TABLE products ADD COLUMN price_updated_at TEXT DEFAULT '';
UPDATE products SET price_updated_at = updated_at WHERE price_updated_at = '';
