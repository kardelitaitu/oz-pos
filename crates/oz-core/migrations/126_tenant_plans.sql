-- ADR sync-plan-gating (E1): per-tenant cloud sync plans.
--
-- Sync is a paid feature: a tenant on the `free` plan may run the POS
-- locally but cannot push/pull to the cloud server. The plan is keyed by
-- tenant_id (the same value carried in JWT claims), so every terminal of a
-- store inherits the store's plan. Enforcement is server-side; this table
-- is the source of truth. When no row exists the server treats the tenant
-- as `free` once plan enforcement is enabled (`OZ_ENFORCE_PLANS`).
CREATE TABLE IF NOT EXISTS tenant_plans (
    tenant_id   TEXT PRIMARY KEY,
    plan        TEXT NOT NULL DEFAULT 'free'
                CHECK (plan IN ('free', 'pro')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
