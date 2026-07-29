-- 101_kds_table_number.sql — Add table_number column to kds_orders (TODO 1b)
--
-- The KDS ticket card previously read table_number via a type assertion
-- hack ((order as unknown as Record<string, unknown>)['table_number'])
-- because the column didn't exist. This migration adds it as a proper
-- nullable TEXT column so the UI can read order.table_number directly.
--
-- The column is populated at order-creation time from the tables table:
--   SELECT name FROM tables WHERE active_sale_id = ?1

ALTER TABLE kds_orders ADD COLUMN table_number TEXT;
