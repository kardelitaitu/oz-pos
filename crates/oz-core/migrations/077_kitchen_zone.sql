-- 077_kitchen_zone.sql — Kitchen zone routing for products and KDS orders.
--
-- Enables multi-zone kitchen deployments where products are assigned to a
-- zone (e.g. "front" or "back" kitchen) and KDS orders are automatically
-- split per zone at sale completion. Each KDS device can then filter its
-- queue to show only orders for its assigned zone.

ALTER TABLE products ADD COLUMN kitchen_zone TEXT;
ALTER TABLE kds_orders ADD COLUMN kitchen_zone TEXT;
