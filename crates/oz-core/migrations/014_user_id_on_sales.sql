-- 014_user_id_on_sales.sql
-- Add user_id column to the sales table for tracking which cashier
-- processed each sale.
--
-- The column is nullable because existing sales won't have a user_id.
-- New sales will populate this field via the complete_sale IPC command.

ALTER TABLE sales ADD COLUMN user_id TEXT;
