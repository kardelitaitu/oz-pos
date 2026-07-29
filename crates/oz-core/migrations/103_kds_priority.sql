-- Adds priority/rush flag to KDS orders for FOH to signal urgent tickets.
-- The column defaults to 0 (false) so existing rows are not urgent.
ALTER TABLE kds_orders ADD COLUMN priority INTEGER NOT NULL DEFAULT 0;
