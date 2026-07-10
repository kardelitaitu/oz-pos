# ADR #9: License Server Architecture

**Status:** Proposed
**Date:** 2026-07-10
**Author:** Architecture Team & OZ-POS Contributors
**Tags:** licensing, activation, subscription-signing, admin-dashboard

---

## Context

OZ-POS is sold as licensed software. A customer buys a license (e.g., "Pro tier, 2 stores") and receives a license key. The POS software needs to validate this key, activate the license, and receive a cryptographically signed `tenant_subscription` record that governs feature access, store quotas, and instance limits (per ADR #5).

The license server is the **signing authority** — it holds the private key and issues signed subscription payloads. Once activated, the POS operates offline using the locally-stored signed subscription (ADR #5's 14-day offline grace). The license server is only contacted during activation, renewal, and tier changes.

---

## Decision

### 1. Separate Docker Service

The license server is a **separate Docker container** from the cloud sync server (`apps/cloud-server/`). This is the right separation because:

| Concern | License Server | Cloud Sync Server |
|---|---|---|
| **Purpose** | Validate keys, sign subscriptions | Sync POS data between stores |
| **Holds** | RSA private key | No secrets beyond JWT signing key |
| **Contacted** | Activation, renewal, tier change | Continuous sync push/pull |
| **Critical-path?** | No — POS operates offline after activation | No — POS operates offline, syncs when connected |
| **Admin access** | Developer dashboard (manage keys, tenants) | None (headless) |
| **Database** | PostgreSQL (SaaS, multi-tenant) | SQLite or PostgreSQL |
| **Attack surface** | Web UI for admin + public activation API | Public sync API only |

### 2. License Key Flow

```
CUSTOMER                            LICENSE SERVER                    POS (LOCAL)
─────────                            ──────────────                    ───────────
1. Buys license
   ← Receives key:
     OZ-PRO-ABC1-DEF2-GHI3

2. Installs OZ-POS
   → Enters key in setup wizard ──────────────────────────────────→  stores key

3.                                    ← POST /api/v1/license/activate ←
                                          { key, tenant_id, device_fingerprint }
                                     →
                                     Validates key:
                                     - Key exists and unused ✓
                                     - Tier: "pro"
                                     - max_stores: 2
                                     - max_pos_instances: 3

                                     Signs tenant_subscription
                                     with RSA private key:
                                     { tenant_id, tier_key, max_stores,
                                       max_pos_instances, allowed_types,
                                       expires_at, signature }

                                     Returns signed subscription ──→  stores in
                                                                      tenant_subscription
                                                                      table (global DB)

4.                                                                    Verifies signature
                                                                      against embedded
                                                                      public key ✓

                                                                      POS operates
                                                                      with Pro tier
```

### 3. API Endpoints

#### Public (POS Client — no auth, rate-limited)

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/v1/license/activate` | Activate a license key. Body: `{ key, tenant_id, machine_id }`. Returns signed `tenant_subscription`. |
| `POST` | `/api/v1/license/renew` | Renew an existing subscription. Body: `{ tenant_id, api_key }`. Returns fresh signed subscription. |
| `GET` | `/api/v1/license/status/:tenant_id` | Check current subscription status (public info only: tier, active, expires_at). |

#### Admin (Developer Dashboard — authenticated)

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/v1/admin/login` | Admin login. Body: `{ email, password }`. Returns session token. |
| `GET` | `/api/v1/admin/dashboard` | Dashboard overview: active tenants, key usage, revenue metrics. |
| `GET` | `/api/v1/admin/tenants` | List all tenants with filters (tier, status, search). |
| `GET` | `/api/v1/admin/tenants/:id` | Tenant detail: subscription, activation date, machines. |
| `POST` | `/api/v1/admin/keys/generate` | Generate new license keys. Body: `{ tier, count, max_stores }`. |
| `GET` | `/api/v1/admin/keys` | List all keys with status (unused, activated, revoked). |
| `POST` | `/api/v1/admin/tenants/:id/tier` | Change a tenant's tier (re-signs subscription). |
| `POST` | `/api/v1/admin/tenants/:id/revoke` | Revoke a license. |

### 4. Database Schema (PostgreSQL)

```sql
-- License keys generated by the developer
CREATE TABLE license_keys (
    key             TEXT PRIMARY KEY,           -- 'OZ-PRO-ABC1-DEF2-GHI3'
    tier_key        TEXT NOT NULL,              -- 'free' | 'pro' | 'premium' | 'enterprise'
    max_stores      INTEGER NOT NULL,
    max_pos_instances INTEGER NOT NULL,
    allowed_types   JSONB NOT NULL,             -- ["restaurant-pos","store-pos","inventory","admin"]
    status          TEXT NOT NULL DEFAULT 'unused',  -- 'unused' | 'activated' | 'expired' | 'revoked'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,       -- Key becomes invalid if unused past this date
    activated_at    TIMESTAMPTZ,
    activated_by    TEXT,                       -- tenant_id that used this key
    revoked_at      TIMESTAMPTZ,
    notes           TEXT                        -- admin notes
);

-- Tenants (one per customer installation)
CREATE TABLE tenants (
    id              TEXT PRIMARY KEY,           -- Client-generated UUID (V4 or ULID)
    -- Contact information
    business_name   TEXT NOT NULL,              -- "Downtown Café"
    contact_name    TEXT NOT NULL,              -- "John Doe"
    email           TEXT NOT NULL,              -- For license keys, renewal reminders, support
    phone           TEXT,                       -- Optional: "+1-555-0123"
    company         TEXT,                       -- Optional: legal company name if different
    address_line1   TEXT,
    address_line2   TEXT,
    city            TEXT,
    state           TEXT,
    postal_code     TEXT,
    country         TEXT NOT NULL DEFAULT 'US',
    -- Notes
    notes           TEXT,                       -- Admin notes (e.g., "Upgraded from Free, June 2026")
    -- License metadata
    api_key         TEXT NOT NULL UNIQUE,       -- Generated on activation; used for renew/status
    status          TEXT NOT NULL DEFAULT 'active',  -- 'active' | 'suspended' | 'revoked'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Subscription history — one row per activation/renewal/change.
-- The CURRENT subscription is the row with the latest created_at.
-- This gives a full audit trail of tier changes and renewals.
CREATE TABLE subscriptions (
    id              SERIAL PRIMARY KEY,
    tenant_id       TEXT NOT NULL REFERENCES tenants(id),
    tier_key        TEXT NOT NULL,              -- 'free' | 'pro' | 'premium' | 'enterprise'
    max_stores      INTEGER NOT NULL,
    max_pos_instances INTEGER NOT NULL,
    allowed_types   JSONB NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active',  -- 'active' | 'expired' | 'grace_period' | 'revoked'
    starts_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,       -- When the subscription period ends
    grace_until     TIMESTAMPTZ,                -- NULL or expires_at + 14 days offline grace
    -- The POS stores this payload + signature locally.
    -- On expiry, the POS enters grace period (14 days offline).
    -- After grace_until, the POS reverts to Free tier quotas.
    signed_payload  TEXT NOT NULL,              -- JSON payload (the subscription data)
    signature       TEXT NOT NULL,              -- RSA signature over signed_payload
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscriptions_tenant ON subscriptions(tenant_id, created_at DESC);

-- Machine registrations (track which devices activated)
CREATE TABLE tenant_machines (
    id              TEXT PRIMARY KEY,           -- machine_id from POS
    tenant_id       TEXT NOT NULL REFERENCES tenants(id),
    first_seen_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, id)
);

-- Admin users (developer team)
CREATE TABLE admin_users (
    id              SERIAL PRIMARY KEY,
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    name            TEXT NOT NULL,
    role            TEXT NOT NULL DEFAULT 'admin',  -- 'admin' | 'superadmin'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### 4a. Expiration & Grace Period Dates

Every subscription has three critical date boundaries:

```
                    starts_at           expires_at         grace_until
                       │                    │                   │
  ─────────────────────┼────────────────────┼───────────────────┼─────────→
                       │◄── active period ──►│◄── grace period ──►│
                       │   full features     │   full features    │  Free tier
                       │                     │   + renewal prompt │  quotas only
```

| Date | Purpose | Set By |
|---|---|---|
| `starts_at` | When the subscription becomes active | License server (on activation/renewal) |
| `expires_at` | When the paid period ends | License server (calculated from tier duration: 1 month, 1 year, lifetime) |
| `grace_until` | Last day the POS can operate with full features offline | `expires_at + 14 days` (per ADR #5) |

**Tier durations (configured per tier):**

| Tier | Default Duration | expires_at |
|---|---|---|
| **Free** | Lifetime | NULL (never expires) |
| **Pro** | 1 year from activation | `activated_at + 365 days` |
| **Premium** | 1 year from activation | `activated_at + 365 days` |
| **Enterprise** | Configurable (1–3 years) | Negotiated per contract |

**Expiration flow:**

1. `expires_at` approaches → POS shows "License expires in X days" in the admin panel
2. `expires_at` passes → POS enters **grace period** (`grace_until`), continues operating with full features
3. `grace_until` passes → POS reverts to **Free tier quotas** (1 store, 1 register). Data is preserved. Customer sees "License expired — upgrade to restore full features"
4. Customer renews → License server creates a new `subscriptions` row with fresh `starts_at`/`expires_at` → POS pulls the signed payload → full features restored

**Key expiration:**

`license_keys.expires_at` controls how long an unused key remains valid. If a customer buys a Pro license but doesn't activate it within (e.g.) 90 days, the key expires and shows as `'expired'` in the admin dashboard. The admin can extend or regenerate it.

### 5. Signing Flow

The license server holds an RSA-2048 private key. The corresponding public key (`oz-pos-updater.key.pub`) is **embedded in the OZ-POS binary** at build time.

**Signing (license server, on activation/renewal):**

```rust
fn sign_subscription(tenant: &Tenant, private_key: &RsaPrivateKey) -> SignedSubscription {
    let payload = serde_json::json!({
        "tenant_id": tenant.id,
        "tier_key": tenant.tier_key,
        "status": tenant.status,
        "expires_at": tenant.expires_at,
        "max_stores": tenant.max_stores,
        "max_pos_instances": tenant.max_pos_instances,
        "allowed_types": tenant.allowed_types,
        "issued_at": Utc::now().to_rfc3339(),
    });

    let payload_str = serde_json::to_string(&payload).unwrap();
    let signature = private_key.sign(
        RSA_PADDING_SCHEME,
        &sha256_hash(payload_str.as_bytes())
    );

    SignedSubscription {
        payload: payload_str,
        signature: base64_encode(&signature),
    }
}
```

**Renewal flow:**

1. POS detects `expires_at` approaching (within 30 days) → shows a banner: "License expires in X days. Renew now."
2. Customer contacts us to renew (or automated Stripe flow in v2).
3. Admin clicks "Renew" on the tenant in the dashboard → sets new `expires_at` → server creates a new `subscriptions` row and re-signs.
4. Next time POS syncs or explicitly calls `/api/v1/license/renew`, it pulls the updated signed subscription.
5. POS verifies the new signature → stores updated `tenant_subscription` locally → full features continue.

```rust
fn verify_subscription(sub: &SignedSubscription, public_key: &RsaPublicKey) -> Result<TenantSubscription> {
    let payload_hash = sha256_hash(sub.payload.as_bytes());
    let signature = base64_decode(&sub.signature)?;
    
    public_key.verify(RSA_PADDING_SCHEME, &payload_hash, &signature)
        .map_err(|_| CoreError::InvalidSubscriptionSignature)?;
    
    let subscription: TenantSubscription = serde_json::from_str(&sub.payload)?;
    Ok(subscription)
}
```

**Key rotation:** The private key is stored as an environment variable (`OZ_LICENSE_PRIVATE_KEY`) or mounted file. Public key is embedded in the binary via `include_str!()`. To rotate keys: generate a new key pair, update the environment variable on the license server, and ship a new POS binary with the updated public key. The old public key remains valid for existing subscriptions until they expire.

### 6. Admin Dashboard

A lightweight web UI for the developer to manage licenses. **Not** a full React SPA — server-rendered HTML with minimal JavaScript (HTMX or vanilla), served from the same axum binary.

```
┌─────────────────────────────────────────────────────┐
│  OZ-POS License Server           [admin@oz-pos.com] │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Dashboard                                          │
│  ┌──────────┬──────────┬──────────┬──────────┐     │
│  │ Active   │ Keys     │ Revenue  │ Trials   │     │
│  │ Tenants  │ Issued   │ (MTD)    │ Active   │     │
│  │   142    │   350    │ $4,200   │   18     │     │
│  └──────────┴──────────┴──────────┴──────────┘     │
│                                                     │
│  ── Tenants ────────────────────────────────        │
│  Search: [____________]  Tier: [All ▼]             │
│                                                     │
│  │ Name          │ Tier    │ Stores │ Status  │     │
│  │ Downtown Cafe │ Pro     │ 2/2    │ Active  │     │
│  │ Mall Bistro   │ Premium │ 5/5    │ Active  │     │
│  │ Food Truck X  │ Free    │ 1/1    │ Active  │     │
│  │ ...           │         │        │         │     │
│                                                     │
│  ── Generate Keys ──────────────────────────        │
│  Tier: [Pro ▼]  Count: [5]  Max Stores: [2]        │
│  [Generate]                                          │
│                                                     │
└─────────────────────────────────────────────────────┘
```

Dashboard pages:
- **Dashboard** — metrics overview (active tenants, keys issued, revenue estimate)
- **Tenants** — searchable list with filters (tier, status, country), detail view with full contact info, activation history, subscription timeline
- **Keys** — generate, view status, revoke, filter by expiration
- **Settings** — admin users management

### 7. Rate Limiting & Abuse Prevention

The activation endpoint is public (no auth) and must be protected:

| Control | Implementation |
|---|---|
| **Rate limiting** | 5 activation attempts per IP per hour |
| **Key brute-force** | 3 failed attempts per key → 15-minute cooldown |
| **Machine fingerprint** | `machine_id` stored; one key = one machine (transfer requires admin reset) |
| **API key for renew/status** | `tenant.api_key` required for renew and status endpoints (issued on activation) |

### 8. Docker Deployment

**Deployment model:** The license server runs on a VPS with a public IP (e.g., Hetzner, DigitalOcean, $20/month). POS clients and the admin dashboard both connect to the same domain. A reverse proxy (Caddy/nginx) handles HTTPS termination.

The POS binary embeds the server URL at build time:

```rust
// Embedded in oz-core or a build script
pub const LICENSE_SERVER_URL: &str = "https://license.oz-pos.com";
```

This URL is used for activation, renewal, and status checks. It can be overridden via environment variable (`OZ_LICENSE_SERVER_URL`) for testing or self-hosted deployments.

```
┌──────────────────────────────────────────────┐
│  VPS (public IP)                              │
│                                               │
│  Caddy/nginx (:443 HTTPS)                     │
│  ├── license.oz-pos.com/api/v1/* → :3100     │
│  └── license.oz-pos.com/admin/*  → :3100     │
│                                               │
│  ┌──────────────────┐  ┌──────────────────┐  │
│  │ license-server    │  │ license-db       │  │
│  │ (:3100, internal) │──│ (PostgreSQL)     │  │
│  │                   │  │ port 5432        │  │
│  │ RSA private key   │  │ internal only    │  │
│  │ Admin dashboard   │  └──────────────────┘  │
│  │ Activation API    │                        │
│  │ Rate limiting     │                        │
│  └──────────────────┘                        │
└──────────────────────────────────────────────┘
        ▲                          ▲
        │ POS clients              │ You (browser)
        │ activate/renew           │ admin dashboard
```

**Security notes:**
- `license-db` is NOT exposed to the internet — only `license-server` connects to it internally
- Only ports 80/443 are exposed publicly; port 3100 is internal to Docker
- `OZ_LICENSE_PRIVATE_KEY` is set via environment variable (never committed to Git)
- Admin password hash is set once via `OZ_ADMIN_PASSWORD_HASH` env var on first deploy
- Rate limiting on the activation endpoint prevents brute-force key guessing from the public internet

```yaml
# docker-compose.yml addition
services:
  license-server:
    build:
      context: .
      dockerfile: Dockerfile.license
    ports:
      - "3100:3100"
    environment:
      OZ_LICENSE_PORT: "3100"
      DATABASE_URL: "postgresql://ozpos:secret@license-db:5432/ozpos_license"
      OZ_LICENSE_PRIVATE_KEY: "${OZ_LICENSE_PRIVATE_KEY}"  # RSA private key (PEM)
      OZ_ADMIN_PASSWORD_HASH: "${OZ_ADMIN_PASSWORD_HASH}"  # Initial admin password
      RUST_LOG: "info"
    depends_on:
      license-db:
        condition: service_healthy
    restart: unless-stopped

  license-db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: ozpos
      POSTGRES_PASSWORD: "${PG_PASSWORD}"
      POSTGRES_DB: ozpos_license
    volumes:
      - license_pg_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ozpos -d ozpos_license"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  license_pg_data:
```

The license server runs on port **3100** (distinct from cloud-sync on 3099).

---

## Consequences

### Positive

- License validation is cryptographically enforced — customers cannot forge subscriptions.
- POS operates fully offline after activation (14-day grace per ADR #5).
- The license server is NOT critical-path for POS uptime — it's only needed during activation and renewal.
- Separate from cloud sync — compromise of sync server does not expose the private signing key.
- Admin dashboard gives the developer full visibility and control over licenses.

### Negative

- Requires hosting a PostgreSQL database + license server (two new services).
- License key distribution (emailing keys to customers) is a manual process in v1 — no automated purchase flow.
- Machine binding means license transfer requires admin intervention.
- If the license server is down during a renewal window, the customer may hit the 14-day grace limit.

### Mitigations

- The license server + DB are lightweight and can run on a $20/month VPS.
- Automated purchase flow (Stripe integration → auto-generate keys) can be added later.
- Renewal can happen any time during the 14-day grace window — plenty of time to resolve server issues.
- Admin can reset machine binding from the dashboard if a customer replaces their hardware.

---

## Implementation Checklist

- [ ] Create `apps/license-server/` crate (axum binary, same pattern as cloud-server).
- [ ] Add `Dockerfile.license` for multi-stage build.
- [ ] Implement `/api/v1/license/activate` — validates key, creates tenant, signs subscription.
- [ ] Implement `/api/v1/license/renew` — re-signs subscription with updated expiry.
- [ ] Implement `/api/v1/license/status/:tenant_id` — public status endpoint.
- [ ] Implement admin auth (email/password → session cookie).
- [ ] Implement admin dashboard (server-rendered HTML with askama templates).
- [ ] Implement key generation and management UI.
- [ ] Implement rate limiting middleware.
- [ ] Add `license-server` and `license-db` services to `docker-compose.yml`.
- [ ] Embed public key in POS binary via `include_str!()` build script.
- [ ] Implement `verify_subscription()` in `crates/oz-core` using the embedded public key.
- [ ] Write tests: activation, renewal, signature verification, rate limiting, admin auth, key revocation.
- [ ] Run `cargo clippy -p license-server -- -D warnings` and full test suite.

---

## Related

- ADR #4 — Store-First Tenancy & Workspace Type/Instance Architecture
- ADR #5 — Subscription Tier & Entitlement (defines `tenant_subscription` schema and `InstanceStatus`)
- `oz-pos-updater.key.pub` — Public key embedded in POS binary
- `apps/cloud-server/` — Cloud sync server (separate service, separate Docker container)
- `docker-compose.yml` — Updated with license-server + license-db services
