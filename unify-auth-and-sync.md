# Unify Auth & Sync to Northflank

> Merge the license server and cloud server into a single PocketBase-based backend on Northflank.

---

## 1. Current State (Two Servers)

```
┌────────────────────────────────────────────────────────┐
│  License Server (Go) — PocketBase + Go hooks           │
│  Northflank · DB: PocketBase SQLite                    │
│  Auth: tenant api_key (persistent)                     │
│                                                        │
│  Routes:   POST /api/v1/license/activate               │
│            POST /api/v1/license/renew                  │
│            POST /api/v1/license/status                 │
│  Signing:  RSA-2048 subscription payloads              │
│                                                        │
│  Collections: license_keys, tenants,                   │
│               subscriptions, tenant_machines           │
└────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────┐
│  Cloud Server (Rust) — axum + SQLite                   │
│  Northflank · DB: SQLite (own file)                    │
│  Auth: HS256 JWT (oz-api, POST /api/v1/tokens)         │
│                                                        │
│  Routes:   POST /api/sync/push                         │
│            POST /api/sync/pull                         │
│            GET  /api/sync/snapshot                     │
│            GET  /api/sync/status                       │
│            REST API (products, sales, …)               │
│            POST /api/webhooks/stripe                   │
│            POST /api/webhooks/square                   │
│                                                        │
│  Tables (cloud-relevant): offline_queue, products,     │
│      tax_rates, users, terminals, tenant_plans,        │
│      tenant_subscription, stripe_customers,            │
│      processed_webhooks, payments, sales               │
└────────────────────────────────────────────────────────┘
```

### Problems

| Problem | Impact |
|---------|--------|
| **Two databases** | License data (PocketBase) and sync data (SQLite) are isolated — the sync plan gate reads its own `tenant_plans` copy, not the license server's tier |
| **Two credential systems** | The license server authenticates with a persistent tenant `api_key`; the cloud server mints short-lived HS256 JWTs via `POST /api/v1/tokens`. The POS holds both. |
| **No shared tenant registry** | The license server owns `tenants`; the cloud server has no `tenants` table — tenant identity is a loose `tenant_id` string column on many tables (default `'default'`), so the two can silently disagree |
| **Duplicated plan state** | The cloud DB's `tenant_plans` + `tenant_subscription` (with `signed_payload`/`signature`/`api_key` columns) mirror the license server's `subscriptions` — two sources of truth kept in sync manually |
| **Two deployments** | Two Northflank services to maintain, monitor, update |
| **No shared state** | "Free tier → no sync" is enforced from the cloud server's own `tenant_plans`, not from the license server's tier |

---

## 2. Target State (Single PocketBase Server)

```
┌──────────────────────────────────────────────────────────────┐
│   Unified Server (PocketBase + Go hooks)                     │
│   Northflank (single service)                                │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐   │
│   │   PocketBase Core                                    │   │
│   │   - Built-in user auth (email/password, OTP, OAuth)  │   │
│   │   - Built-in admin UI                                │   │
│   │   - Built-in REST API (per collection)               │   │
│   │   - Built-in SMTP                                    │   │
│   │   - SQLite database                                  │   │
│   └──────────────────────────────────────────────────────┘   │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐   │
│   │   Custom Go Hooks (existing license server code)     │   │
│   │   - POST /api/v1/license/activate                    │   │
│   │   - POST /api/v1/license/renew                       │   │
│   │   - POST /api/v1/license/status                      │   │
│   │   - RSA signing of subscription payloads             │   │
│   └──────────────────────────────────────────────────────┘   │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐   │
│   │   New Go Hooks (sync endpoints, ported from Rust)    │   │
│   │   - POST /api/sync/push                              │   │
│   │   - POST /api/sync/pull                              │   │
│   │   - GET  /api/sync/status                            │   │
│   │   - GET  /api/sync/snapshot                          │   │
│   │   - POST /api/v1/tokens (JWT minting)                │   │
│   └──────────────────────────────────────────────────────┘   │
│                                                              │
│   Collections:                                               │
│   - users (PocketBase built-in)                              │
│   - license_keys                                             │
│   - tenants                                                  │
│   - subscriptions                                            │
│   - tenant_machines                                          │
│   - sync_queue (new)                                         │
│   - products (new)                                           │
│   - orders (new)                                             │
│   - ... (other POS data)                                     │
└──────────────────────────────────────────────────────────────┘
            │
            │  Single database, single auth, single deployment
            │
     ┌──────┴──────┐
     ▼             ▼
 POS App        Website
 (JWT from      (PocketBase
  /api/v1/tokens)  auth API)
```

### Benefits

| Benefit | Detail |
|---------|--------|
| **Single database** | License, subscription, sync data all in one place |
| **Single auth** | PocketBase users for website, API keys for POS — same database |
| **Single deployment** | One Northflank service, one database volume |
| **Shared tenant state** | "Free tier → no sync" enforced in one place |
| **Consistent data** | License and sync queries hit the same database |
| **Easier scaling** | One service to scale, one database to backup |

---

## 3. Unified Collections Schema

### Existing Collections (Keep)

```sql
-- PocketBase built-in (already exists)
-- users: id, email, password, verified, created, updated

-- From license server (already exists)
license_keys:    id, key, tier_key, status, expires_at, activated_at,
                 activated_by, revoked_at, notes, created, updated

tenants:         id, email, phone, api_key, status, created, updated

subscriptions:   id, tenant_id, tier_key, status, starts_at, expires_at,
                 grace_until, signed_payload, signature, created, updated

tenant_machines: id, tenant_id, first_seen_at, last_seen_at,
                 machine_id, revoked_at
```

### New Collections (Add)

```sql
-- Sync queue: pending offline items from POS apps
sync_queue (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL,         -- → tenants.id
    terminal_id TEXT NOT NULL,         -- which POS terminal
    table_name  TEXT NOT NULL,         -- e.g. "products", "orders"
    operation   TEXT NOT NULL,         -- "insert", "update", "delete"
    record_id   TEXT NOT NULL,         -- affected row ID
    payload     TEXT NOT NULL,         -- JSON diff/patch
    status      TEXT NOT NULL DEFAULT 'pending',  -- pending/synced/failed
    created_at  TEXT NOT NULL,
    synced_at   TEXT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
)

-- Terminal registration (from cloud server)
terminals (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL,
    name        TEXT NOT NULL,         -- "Register 1", "Kitchen Display"
    device_type TEXT NOT NULL,         -- "desktop", "tablet"
    api_key     TEXT NOT NULL UNIQUE,  -- per-terminal JWT signing key
    status      TEXT NOT NULL DEFAULT 'active',
    created_at  TEXT NOT NULL,
    last_seen   TEXT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
)

-- Product catalog (synced from POS)
products (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    sku         TEXT,
    price       INTEGER NOT NULL,     -- minor units (i64)
    category    TEXT,
    stock       INTEGER DEFAULT 0,
    status      TEXT NOT NULL DEFAULT 'active',
    version     INTEGER NOT NULL DEFAULT 1,  -- optimistic concurrency
    updated_at  TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
)

-- Orders (synced from POS)
orders (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL,
    terminal_id TEXT NOT NULL,
    customer_id TEXT,
    total       INTEGER NOT NULL,     -- minor units
    status      TEXT NOT NULL DEFAULT 'open',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
)

-- Add tier_key to tenants for fast plan checks
-- (already exists in tenants table from license server)
```

---

## 4. Unified Auth Flow

### The two credentials that exist today

The two servers use two *different* credentials, and they serve different
purposes — one is durable identity, the other a short-lived session:

| Credential | Issued by | Lifetime | Carries | Used for |
|-----------|-----------|----------|---------|----------|
| **Tenant `api_key`** | License server, `POST /api/v1/license/activate` | Persistent — no expiry, revoked explicitly | Tenant identity (stored on the `tenants` record) | License activation, renewal, status (`/api/v1/license/*`) |
| **HS256 JWT** | Cloud server, `POST /api/v1/tokens` (oz-api) | ~24 h (`exp` claim) | `tenant_id`, `terminal_id` scoping | Sync (`/api/sync/*`) and the REST API |

They are complementary, not competing. The `api_key` answers *"is this the
registered tenant?"*; the JWT answers *"is this an active session scoped to
that tenant and terminal?"*.

### Tenant `api_key` (identity — license server)

Issued on first activation, stored on the `tenants` record, and sent as
`Authorization: Bearer <api_key>`:

```
POS App first boot → user enters license key + email + machine_id
    │
    ▼
POST /api/v1/license/activate
    │
    ├── Go hook validates the key against license_keys
    ├── Finds or creates the tenant by email
    ├── Issues api_key (stored on the tenants record)
    ├── Signs the subscription payload with RSA-2048
    └── Returns { tenant_id, api_key, signed_payload, signature }

POS App subsequent boots
    │
    ├── Reads stored tenant_id + api_key from local SQLite
    └── POST /api/v1/license/status  { Authorization: Bearer <api_key> }
```

The `api_key` is not a JWT — it is a persistent secret verified directly
against the `tenants` collection. It never expires; it is revoked only on
explicit tenant action, subscription cancel + grace expiry, or an admin
security action.

### HS256 JWT (session — cloud server)

Minted by `POST /api/v1/tokens`, signed with `OZ_API_SECRET`, carrying `sub`,
`jti`, `exp`, `iat`, `tenant_id`, and `terminal_id` claims. The mint itself is
authorised one of three ways (ADR sync-auth-hardening): terminal client
credentials (`client_id` + `client_secret`), an `X-Admin-Key` header gated by
`OZ_ADMIN_KEY`, or — in dev only — an open mint when neither is configured.

```
POS App (before sync) → POST /api/v1/tokens
    │                    (client credentials | X-Admin-Key | open dev mint)
    ▼
oz-api create_token_scoped()
    │
    ├── Claims: tenant_id, terminal_id, exp = now + 24h
    └── Returns { token, expires_at, token_id }

POS App sync cycle
    │
    ├── POST /api/sync/push      { Authorization: Bearer <jwt> }
    ├── POST /api/sync/pull      { Authorization: Bearer <jwt> }
    ├── GET  /api/sync/snapshot  { Authorization: Bearer <jwt> }
    └── GET  /api/sync/status    { Authorization: Bearer <jwt> }
```

`auth_middleware` validates the JWT and injects its claims; the sync handlers
take `tenant_id` from the claims (never the request body) and the plan gate
(`OZ_ENFORCE_PLANS`) reads the same claim. On a structured 401
(`token_expired`) the client re-mints and retries once; `invalid_token` /
`missing_token` are surfaced as configuration errors.

### Refresh model

| Credential | Refresh strategy |
|-----------|------------------|
| `api_key` (identity) | No refresh — valid until revoked |
| JWT (session) | Re-mint at `POST /api/v1/tokens` on `token_expired` (no refresh token; minting is cheap) |

### Recommendation: keep both — unify only where they are verified

Do **not** collapse the two credentials into one:

- Making the `api_key` a JWT (or the JWT permanent) loses the short lifetime,
  the `terminal_id` scoping, and the expiry-based invalidation that sync and
  plan gating rely on.
- Making sync/REST use the raw `api_key` would send the long-lived identity
  secret on every data-plane request and lose the per-terminal scoping the
  JWT claims carry.

The merge should therefore:

1. Keep `api_key` on `tenants` for `/api/v1/license/*` (identity + activation).
2. Keep the HS256 JWT mint (`POST /api/v1/tokens`) and `auth_middleware` for
   `/api/sync/*` and the REST API (session), ported to Go against PocketBase.
3. Share one tenant record so both credentials resolve to the same tenant —
   that shared record is the actual unification; the two credential types
   coexist on top of it.

### Future: website users (separate concern, not yet built)

A merchant-facing website would add PocketBase's built-in `users` collection
(email/password or OTP) as a *third* client type. That is independent of the
POS credentials above, out of scope for this merge, and does not replace or
link to the tenant `api_key`.

---

## 5. Sync Protocol (Migrated to PocketBase)

The Rust cloud server exposes four sync endpoints, all behind the oz-api JWT
(`auth_middleware`), per-tenant rate limiting, and optional plan gating
(`OZ_ENFORCE_PLANS`). The migration to PocketBase must preserve these exact
contracts rather than redesign them.

| Endpoint | Method | Contract |
|----------|--------|----------|
| `/api/sync/push` | POST | Accept a `Vec<OfflineQueueItem>`; insert each into `offline_queue` with its client-generated ID; tenant taken from JWT claims (never the body); per-item `Accepted` / `Rejected{reason}` (non-UUID id, duplicate id, db error) |
| `/api/sync/pull` | POST | Replay `offline_queue` rows for the tenant changed since `since`, ordered `created_at, id`, cursor-paginated (500/page); `410 anchor_expired` if `since` is older than the pruned window |
| `/api/sync/snapshot` | GET | Reference-data baseline — products, tax rates, users — scoped to the JWT tenant; 5-min in-memory cache; `pin_hash` never serialised (SYNC-06); query failures return 5xx, never an empty success (SYNC-09) |
| `/api/sync/status` | GET | Health, version, pending queue depth, tiered heartbeat interval |

### Push Flow

```
POS App → POST /api/sync/push
    │
    │  Headers: Authorization: Bearer <jwt from POST /api/v1/tokens>
    │  Body: [ { id, action, payload, status, retry_count, last_error,
    │           created_at, synced_at } ]        // Vec<OfflineQueueItem>
    │
    ▼
axum: push_handler (auth → plan gate → rate limit → handler)
    │
    ├── tenant_id = claims.tenant_id (never the request body)
    ├── For each item:
    │   ├── Reject non-UUID ids (defense-in-depth for the prune DELETE path)
    │   ├── INSERT INTO offline_queue (existing client-generated id)
    │   └── Duplicate id (UNIQUE) → Rejected { reason: "duplicate id" }
    └── Return PushResponse { results: [Accepted | Rejected] }
```

Push does NOT write to `products`/`sales` tables. The offline queue is a
pending-action ledger; reference data flows through the snapshot, and payment
events flow in through the Stripe/Square webhooks (as `finalize_sale` actions).

### Pull Flow (queue replay)

```
POS App → POST /api/sync/pull
    │
    │  Headers: Authorization: Bearer <jwt>
    │  Body: { since?: "2026-08-01T00:00:00Z", cursor?: "created_at|id" }
    │
    ▼
axum: pull_handler
    │
    ├── tenant_id = claims.tenant_id
    ├── If since is older than the oldest retained row → 410 anchor_expired
    ├── SELECT ... FROM offline_queue
    │     WHERE tenant_id = ? AND created_at >= since
    │     ORDER BY created_at ASC, id ASC LIMIT 501
    ├── Truncate to 500; if a 501st row existed, return next_cursor
    └── Return PullResponse { items, next_cursor }
```

### Snapshot (reference data, not the queue)

```
POS App → GET /api/sync/snapshot   (Authorization: Bearer <jwt>)
    │
    ▼
axum: snapshot_handler
    │
    ├── tenant_id = claims.tenant_id
    ├── Serve cached JSON if younger than 5 min
    ├── SELECT products, tax_rates, users WHERE tenant_id = ?
    ├── users: pin_hash deliberately omitted (SYNC-06)
    └── Return { products: [...], tax_rates: [...], users: [...] }
```

### Key Change: PocketBase as Source of Truth

| Before (Cloud Server, actual) | After (PocketBase) |
|-------------------------------|--------------------|
| SQLite tables (`offline_queue`, `products`, `tax_rates`, `users`, `terminals`, `tenant_plans`, …) | PocketBase collections |
| HS256 JWT minted by `POST /api/v1/tokens` (oz-api), gated by `OZ_ADMIN_KEY` | Same JWT minting (Go hook or PocketBase auth), same client contract |
| axum handlers in Rust | Go hooks, semantics preserved |
| Tenant identity = loose `tenant_id` string column (no `tenants` table) | Unified `tenants` collection |
| Stripe/Square webhooks in Rust (plan gating + `finalize_sale`) | Ported to Go hooks (or left in the Rust server until cutover) |

---

## 6. Migration Plan

### Phase 1: Add Sync Collections to PocketBase (Week 1)

| Step | Action |
|------|--------|
| 1.1 | Add `sync_queue`, `terminals`, `products`, `orders` collections to `pb_schema.json` |
| 1.2 | Add Go hooks for sync push/pull endpoints |
| 1.3 | Test sync with existing POS app (backward compatible) |
| 1.4 | Deploy to Northflank (replace license server) |

### Phase 2: Migrate POS App Auth (Week 2)

| Step | Action |
|------|--------|
| 2.1 | Update `sync_client.rs` to use PocketBase auth endpoints |
| 2.2 | Change token storage: API key → PocketBase JWT |
| 2.3 | Add token refresh logic (call `/auth-refresh` before expiry) |
| 2.4 | Keep backward compatibility: support old JWT during transition |

### Phase 3: Migrate Cloud Server Data (Week 3)

| Step | Action |
|------|--------|
| 3.1 | Export data from cloud server SQLite |
| 3.2 | Import into PocketBase collections |
| 3.3 | Verify data integrity |
| 3.4 | Update cloud server env vars to point to new PocketBase |

### Phase 4: Decommission Cloud Server (Week 4)

| Step | Action |
|------|--------|
| 4.1 | Stop cloud server on Northflank |
| 4.2 | Remove cloud server deployment |
| 4.3 | Update POS app default server URL |
| 4.4 | Monitor for issues |

---

## 7. Code Changes Required

### Rust (POS App)

| File | Change |
|------|--------|
| `crates/oz-core/src/sync_client.rs` | Point at the unified server URL — no protocol change needed if the four contracts (push/pull/status/snapshot) are preserved |
| `crates/oz-core/src/sync_client.rs` | Decide auth: keep HS256 JWT minting via `POST /api/v1/tokens`, or switch to a PocketBase-issued token |
| `platform/startup/src/rate_sync.rs` | Update server URL defaults |

### Go (License Server → Unified Server)

| File | Change |
|------|--------|
| `apps/license-server/main.go` | Add sync route handlers |
| `apps/license-server/pb_schema.json` | Add sync collections |
| `apps/license-server/sync_push.go` | New: push handler (insert into offline-queue collection) |
| `apps/license-server/sync_pull.go` | New: pull handler (cursor-paginated queue replay) |
| `apps/license-server/sync_snapshot.go` | New: snapshot handler (products / tax rates / users) |
| `apps/license-server/sync_status.go` | New: status handler (health + heartbeat interval) |
| `apps/license-server/sync_auth.go` | New: JWT verification for sync |

### TypeScript (Website)

| File | Change |
|------|--------|
| `website/src/components/AuthForm.tsx` | Point to unified PocketBase URL |
| `website/src/pages/[locale]/account.astro` | Query license + subscription from same DB |

---

## 8. Environment Variables

### Unified Server (Northflank)

```bash
# PocketBase
PB_DATA_DIR=/data/pb_data
PB_URL=https://license.oz-pos.com

# RSA signing (existing)
OZ_LICENSE_PRIVATE_KEY-----BEGIN RSA PRIVATE KEY-----...

# Paddle (new)
PADDLE_VENDOR_ID=12345
PADDLE_CLIENT_TOKEN=xxxxx
PADDLE_WEBHOOK_SECRET=xxxxx

# SMTP (for email verification)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=noreply@oz-pos.com
SMTP_PASSWORD=xxxxx

# Sync config
OZ_SYNC_MAX_ITEMS_PER_PUSH=100
OZ_SYNC_SNAPSHOT_CACHE_TTL=300

# Rate limiting
OZ_RATE_LIMIT_WINDOW=60
OZ_RATE_LIMIT_MAX=100
```

---

## 9. API Endpoints (Unified)

### License Endpoints (Existing)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/license/activate` | POST | Activate license key |
| `/api/v1/license/renew` | POST | Renew subscription |
| `/api/v1/license/status` | POST | Check license status |

### Sync Endpoints (Ported from Cloud Server)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/sync/push` | POST | Accept `OfflineQueueItem`s, insert with existing IDs, return per-item outcomes |
| `/api/sync/pull` | POST | Replay `offline_queue` changes since a timestamp, cursor-paginated |
| `/api/sync/snapshot` | GET | Reference-data baseline (products, tax rates, users) |
| `/api/sync/status` | GET | Health, version, pending queue depth, heartbeat interval |
| `/api/v1/tokens` | POST | Mint HS256 JWT (gated by `OZ_ADMIN_KEY` when set) |

### Auth Endpoints (PocketBase Built-in)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/collections/users/auth-with-password` | POST | Website login |
| `/api/collections/users/auth-with-otp` | POST | Website OTP login |
| `/api/collections/users/auth-refresh` | POST | Refresh website session |
| `/api/collections/tenants/auth-with-api-key` | POST | POS app login |

### Paddle Webhook (New)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/paddle/webhook` | POST | Handle Paddle payment events |

---

## 10. Backward Compatibility

### Transition Period (4 weeks)

| Client | Before | During Transition | After |
|--------|--------|-------------------|-------|
| POS App v0.0.25 | Old cloud server JWT | Both JWTs accepted | PocketBase JWT only |
| Website | N/A | PocketBase auth | PocketBase auth |
| License Server | Go hooks only | Go hooks + sync | Unified server |

### Version Detection

```rust
// POS app can detect which server it's talking to
fn detect_server_version(base_url: &str) -> ServerVersion {
    // The unified server exposes the license endpoints alongside sync.
    if reqwest::get(format!("{}/api/v1/license/status", base_url)).is_ok() {
        ServerVersion::Unified
    } else {
        ServerVersion::Legacy
    }
}
```

---

## 11. CORS & Security

### CORS Configuration

The unified server must allow cross-origin requests from:

| Origin | Purpose |
|--------|---------|
| `https://oz-pos.com` | Website (global) |
| `https://id.oz-pos.com` | Website (Indonesia) |
| `http://localhost:4321` | Website (dev) |
| `tauri://localhost` | POS app (Tauri) |

```go
// In main.go — CORS layer
cors := cors.New(cors.Options{
    AllowedOrigins:   []string{"https://oz-pos.com", "https://id.oz-pos.com", "http://localhost:4321"},
    AllowedMethods:   []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
    AllowedHeaders:   []string{"Authorization", "Content-Type", "X-Admin-Key"},
    AllowCredentials: true,
    MaxAge:           3600,
})
```

### Security Headers

| Header | Value | Purpose |
|--------|-------|---------|
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-Frame-Options` | `DENY` | Prevent clickjacking |
| `Strict-Transport-Security` | `max-age=31536000` | Force HTTPS |
| `Content-Security-Policy` | `default-src 'self'` | Restrict resources |

### PocketBase Admin UI

PocketBase has a built-in admin UI at `/_/` (e.g. `https://license.oz-pos.com/_/`).
Access should be restricted:

```go
// In main.go — disable admin UI in production
app.OnBeforeServe().Bind(func(e *core.ServeEvent) error {
    e.App.Settings().DisableSignUp = true  // disable public registration
    return nil
})
```

Or restrict via IP allowlist on Northflank.

---

## 12. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Data loss during migration | Export backup before migration, verify checksums |
| Downtime during switchover | Blue-green deploy: new server runs alongside old for 1 week |
| POS app breaks with new auth | Keep old auth endpoint alive during transition |
| PocketBase performance | SQLite handles ~100 concurrent users fine for this scale |
| Sync conflicts | Version-based optimistic concurrency (existing) |

---

## 13. Timeline

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| **Phase 1**: Add sync to PocketBase | 1 week | Sync collections + Go hooks + deploy |
| **Phase 2**: Migrate POS app auth | 1 week | Rust changes + token refresh |
| **Phase 3**: Migrate data | 1 week | Data export/import + verification |
| **Phase 4**: Decommission old server | 1 week | Cleanup + monitoring |
| **Total** | ~4 weeks | Fully unified backend |

---

## 14. Success Criteria

| Criterion | How to Verify |
|-----------|---------------|
| Single database | All data in one PocketBase instance |
| Single auth | Same credentials work on website + POS app |
| Sync works | POS app pushes/pulls data successfully |
| License works | Activation, renewal, validation all pass |
| Website auth works | Login, register, account page functional |
| No data loss | Migration checksums match |
| Performance | Sync latency < 500ms for 100 items |
