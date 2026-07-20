-- 099_inventory_transactions_created_at_index.sql
-- Adds a non-unique index on inventory_transactions.created_at for faster
-- audit-log queries. The TransitAuditScreen and inventory transaction history
-- both order by created_at DESC; without this index, every query scans the
-- full table.
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_created
ON inventory_transactions(created_at);
