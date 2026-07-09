-- 055_offline_queue_tenant.sql — add tenant_id column for multi-store cloud sync.
--
-- Defaults to 'default' so existing single-store deployments continue to work
-- without any configuration change. Multi-store cloud deployments will set
-- this to a unique tenant/store identifier.

ALTER TABLE offline_queue ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';

CREATE INDEX IF NOT EXISTS idx_offline_queue_tenant_status ON offline_queue(tenant_id, status);
