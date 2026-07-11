-- ADR #5: Subscription Tier & Entitlement Architecture
--
-- The tenant_subscription table lives in the GLOBAL database (alongside
-- store_profiles, terminals, users, and roles). It governs how many stores,
-- registers per store, and workspace types a tenant is allowed.
--
-- The signature column prevents local tampering -- it is verified against
-- oz-pos-updater.key.pub on startup and before quota checks.

CREATE TABLE IF NOT EXISTS tenant_subscription (
    tenant_id          TEXT PRIMARY KEY,
    tier_key           TEXT NOT NULL,        -- 'free', 'pro', 'premium', 'enterprise'
    status             TEXT NOT NULL,        -- 'active', 'past_due', 'canceled'
    expires_at         TEXT NULL,            -- ISO timestamp (NULL = lifetime/free)
    max_stores         INTEGER NOT NULL,
    max_pos_instances  INTEGER NOT NULL,     -- Per-store register limit
    allowed_types_json TEXT NOT NULL,        -- '["restaurant-pos", "store-pos", "admin"]'
    signature          TEXT NOT NULL,        -- RSA/HMAC signature from apps/cloud-server
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Seed a default free-tier subscription for the default tenant.
-- This ensures single-store deployments work out of the box without
-- needing a cloud-server signature. The signature 'BOOTSTRAP_FREE'
-- is a sentinel value that passes local verification (no cloud key needed)
-- but will be rejected by cloud-server on first sync -- prompting the user
-- to activate a real subscription online.
INSERT OR IGNORE INTO tenant_subscription (
    tenant_id, tier_key, status, expires_at,
    max_stores, max_pos_instances, allowed_types_json, signature
) VALUES (
    'default',
    'free',
    'active',
    NULL,
    1,
    1,
    '["store-pos", "restaurant-pos", "admin"]',
    'BOOTSTRAP_FREE'
);

-- Add last_accessed_at to workspace_instances so that when a tier upgrade
-- triggers automatic recovery of QuotaSuspended instances, they are restored
-- in most-recently-used order.
ALTER TABLE workspace_instances ADD COLUMN last_accessed_at TEXT;
