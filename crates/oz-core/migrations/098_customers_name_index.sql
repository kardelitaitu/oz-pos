-- 098_customers_name_index.sql
-- Adds a non-unique index on customers.name for faster name-based lookups.
-- The CustomerLookupScreen and searchCustomers Tauri command both filter by name;
-- without this index, every name search scans the full table.
CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(name);
