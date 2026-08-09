-- 124_kds_order_targets.sql
--
-- A KDS order represents one sale/zone ticket. Delivery targets are
-- normalized separately so one sale can fan out to several KDS instances
-- without duplicating the uniquely-identified kds_orders row.

CREATE TABLE IF NOT EXISTS kds_order_targets (
    kds_order_id     TEXT NOT NULL REFERENCES kds_orders(id) ON DELETE CASCADE,
    target_instance_id TEXT NOT NULL,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (kds_order_id, target_instance_id)
);

CREATE INDEX IF NOT EXISTS idx_kds_order_targets_instance
    ON kds_order_targets(target_instance_id, kds_order_id);

-- Upgrade tickets created by Phase 7 before the normalized table existed.
INSERT OR IGNORE INTO kds_order_targets (kds_order_id, target_instance_id)
SELECT id, target_instance_id
FROM kds_orders
WHERE target_instance_id IS NOT NULL
  AND trim(target_instance_id) <> '';
