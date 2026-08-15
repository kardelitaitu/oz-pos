-- ── RLS deployment cutover: restricted app role + FORCE ROW LEVEL SECURITY ──
--
-- Turns the schema-side tenant isolation (already shipped: `ENABLE ROW LEVEL
-- SECURITY` + a `tenant_isolation` policy on every tenant-scoped table, keyed
-- on the `oz.tenant_id` session GUC) into ACTIVE enforcement.
--
-- Today the app connects as the table owner, which bypasses RLS entirely, so
-- the policies ship inert. This script is the cutover: it creates a
-- restricted non-owner role (`oz_app`), grants it DML on the tenant tables,
-- and sets FORCE ROW LEVEL SECURITY so the owner bypass no longer applies.
-- From then on, every query that touches a tenant table must run with
-- `oz.tenant_id` set or it returns ZERO rows (and writes are rejected).
--
-- Run as a superuser / the database owner against the cloud database:
--
--     psql "$DATABASE_URL" -f scripts/rls-cutover.sql
--
-- The app connection must then use the `oz_app` role. To use a different
-- role name, sed-replace `oz_app` below (the test in
-- `apps/cloud-server/src/db.rs` executes this file verbatim, so keep the
-- body pure SQL — no psql-only directives like \set).
--
-- The tenant-scoped table list MUST stay in sync with `RLS_SQL` in
-- `scripts/generate-pg-migration.py` (that array is the source of truth —
-- it is applied to the generated PG schema at deploy time).
--
-- Behaviour after cutover:
--   * Every request transaction must set `SET LOCAL oz.tenant_id = '<tenant>'
--     as its first statement (the cloud server's sync data layer already does
--     this in `apps/cloud-server/src/sync_store.rs`); the local setting
--     auto-resets when the transaction ends, so a pooled connection can never
--     leak one tenant's GUC to the next borrower.
--   * Server-side, signature-authenticated paths that are NOT tenant-
--     authenticated (Stripe/Square webhooks) do not set the GUC; their
--     `stripe_customers` mapping lookup is the tenant *resolution* step, so
--     it is inherently pre-tenant. Webhook plan updates write through the
--     SQLite store (dual-write), which is not RLS-protected.
--   * `SELECT COUNT(DISTINCT tenant_id) FROM offline_queue` (the /status
--     global counter) intentionally runs without the GUC and reads 0 under
--     enforcement — it is an operator-facing aggregate, not tenant data.
--
-- Every statement is idempotent and safe to re-run, so the script can be
-- executed as-is (psql autocommit). To make it reversible, wrap it in a
-- transaction and roll back — the whole body is transactional DDL (CREATE
-- ROLE, GRANT, ALTER TABLE ... FORCE all roll back atomically):
--
--     BEGIN; \i scripts/rls-cutover.sql; ROLLBACK;
--
-- Or reverse a committed cutover with:
--
--     ALTER TABLE ... NO FORCE ROW LEVEL SECURITY  (all 15 tables)
--     DROP ROLE oz_app;

-- 1. Restricted app role (idempotent; NOLOGIN by default — the operator
--    enables login and sets a strong password afterwards):
--        ALTER ROLE oz_app LOGIN PASSWORD '<strong>';
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'oz_app') THEN
        CREATE ROLE oz_app NOLOGIN;
    END IF;
END $$;

-- 2. DML on every tenant-scoped table (same list as RLS_SQL in
--    scripts/generate-pg-migration.py — keep them in sync).
DO $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['bundle_items','offline_queue','product_activity',
                            'product_bundles','product_taxes','product_variants',
                            'products','sales','sent_reports','stripe_customers',
                            'sync_terminals','tax_rates','tenant_plans',
                            'tenant_subscription','users']
    LOOP
        EXECUTE format(
            'GRANT SELECT, INSERT, UPDATE, DELETE ON %I TO oz_app', t
        );
    END LOOP;
    EXECUTE 'GRANT USAGE ON SCHEMA public TO oz_app';
END $$;

-- 3. FORCE ROW LEVEL SECURITY: the owner bypass no longer applies, so a
--    missed `WHERE tenant_id = ?` fails closed instead of leaking.
DO $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['bundle_items','offline_queue','product_activity',
                            'product_bundles','product_taxes','product_variants',
                            'products','sales','sent_reports','stripe_customers',
                            'sync_terminals','tax_rates','tenant_plans',
                            'tenant_subscription','users']
    LOOP
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
    END LOOP;
END $$;

-- 4. Verification (informational — expect 15 rows, all `t`/`t`):
--    SELECT tablename, rowsecurity, forcerowsecurity
--      FROM pg_tables
--     WHERE schemaname = 'public'
--       AND tablename IN ('bundle_items','offline_queue','product_activity',
--                         'product_bundles','product_taxes','product_variants',
--                         'products','sales','sent_reports','stripe_customers',
--                         'sync_terminals','tax_rates','tenant_plans',
--                         'tenant_subscription','users')
--     ORDER BY tablename;
