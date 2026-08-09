-- 123_kds_target_instance.sql
--
-- Persist the topology-selected KDS workspace instance for each ticket.
-- NULL preserves legacy tickets created before runtime route compilation.

ALTER TABLE kds_orders ADD COLUMN target_instance_id TEXT;

CREATE INDEX IF NOT EXISTS idx_kds_orders_target_instance
    ON kds_orders(target_instance_id);
