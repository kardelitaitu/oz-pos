-- 076_tenant_id_reference.sql — tenant-scoped reference data.
--
-- Adds `tenant_id` to the three reference tables (products, tax_rates,
-- users) so the cloud server's /api/sync/snapshot endpoint can filter
-- by the JWT claims' tenant_id. Defaults to 'default' for backward
-- compatibility with single-tenant deployments.
--
-- TODO: When POST /api/v1/tax-rates and POST /api/v1/users endpoints
-- are added to oz-api, they must stamp `tenant_id` from JWT claims the
-- same way `create_product` does — otherwise new tax rates and users
-- will silently default to 'default' and leak across tenants.

ALTER TABLE products ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
ALTER TABLE tax_rates ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';
ALTER TABLE users ADD COLUMN tenant_id TEXT NOT NULL DEFAULT 'default';

CREATE INDEX IF NOT EXISTS idx_products_tenant ON products(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tax_rates_tenant ON tax_rates(tenant_id);
CREATE INDEX IF NOT EXISTS idx_users_tenant ON users(tenant_id);
