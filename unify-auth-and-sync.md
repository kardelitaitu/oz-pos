# Unify Auth & Sync to Northflank

> Co-locate the license (auth) server and cloud (sync) server into one
> Northflank deployment: one Docker image running both functions, two
> databases coupled by the HS256 JWT identity, sync data on Postgres.

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

## 2. Target State (One Deployment, Two Functions)

```
┌──────────────────────────────────────────────────────────┐
│  One Northflank service, one Docker image                │
│  caddy (single port, path routing)                       │
│                                                          │
│  Auth function (PocketBase + Go hooks — kept)            │
│    /api/v1/license/activate|renew|status                 │
│    RSA-2048 subscription signing                         │
│    web_users auth (future website)                       │
│    admin UI /_/                                          │
│    DB: PocketBase SQLite (low traffic)                   │
│                                                          │
│  Sync function (Rust axum — kept)                        │
│    /api/sync/push|pull|snapshot|status                   │
│    /api/v1/tokens (HS256 JWT mint)                       │
│    /api/webhooks/stripe|square                           │
│    REST API + plan gating + rate limiting + prune        │
│    DB: Postgres (managed addon — futureproof)            │
│                                                          │
│  Identity: HS256 JWT (tenant_id + terminal_id)           │
└──────────────────────────────────────────────────────────┘
```

### What this buys

| Benefit | Detail |
|---------|--------|
| **One deploy** | One image, one Northflank service, one reverse proxy |
| **No rewrite** | Both mature codebases are kept; only packaging + the Postgres path change |
| **Futureproof sync** | Sync data on Postgres (pooling, replicas, backups) — the part that scales |
| **Cheap auth** | License + website users stay on PocketBase/SQLite (low traffic, bounded) |
| **One identity** | The HS256 JWT's `tenant_id` is the single contract between the two functions |

### Honest constraints

- Two databases, not one. They are coupled by the JWT `tenant_id` (and the
  Stripe/Square webhooks mirror plan state into the sync DB), not by a shared
  table. If cross-database queries become necessary, that is the signal to
  merge auth data into Postgres.
- PocketBase remains single-node SQLite; it holds only low-traffic data.

---

## 3. Data Layout (Two Databases)

### Auth DB — PocketBase SQLite (unchanged)

```
license_keys, tenants, subscriptions, tenant_machines
web_users (future website — per website-plan.md)
```

### Sync DB — Postgres (the cloud server's 92 tables, unchanged)

```
offline_queue, products, tax_rates, users, terminals,
tenant_plans, tenant_subscription, stripe_customers,
processed_webhooks, payments, sales, sale_lines, …
```

### Identity contract

PocketBase `tenants.id` is the canonical tenant ID. The sync server's JWT mint
embeds it as the `tenant_id` claim, and every sync/REST row is scoped by that
string. The two databases never need to share a table — they share the
identifier.

### Plan gating (explicit coupling)

The sync function enforces "free → no sync" from its own `tenant_plans`,
updated by the Stripe/Square webhooks. The license server's `subscriptions`
remains the source of truth for the signed payload and tier key. These are
kept in step operationally today; unifying them into one table is a later,
optional step, not a prerequisite for one deploy.

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
2. Keep the HS256 JWT mint (`POST /api/v1/tokens`) and its auth middleware in
   the Rust sync server (session) — unchanged.
3. Use the JWT's `tenant_id` as the single identity contract: PocketBase
   `tenants.id` is the canonical ID, the JWT carries it, and the sync DB
   scopes rows by it. The two credentials coexist without one shared table.

---

## 5. Sync Protocol (Unchanged)

The Rust cloud server exposes four sync endpoints, all behind the oz-api JWT
(`auth_middleware`), per-tenant rate limiting, and optional plan gating
(`OZ_ENFORCE_PLANS`). The sync engine stays in the Rust server exactly as-is —
the POS client does not change. Only its database and its deployment move.

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

### Anti-spike & retention (must be preserved)

The sync function carries two DB-protection loops that are load-bearing and
must survive the SQLite → Postgres move — they are not optional:

| Mechanism | Today | Behavior |
|-----------|-------|----------|
| **Per-tenant rate limiter** (`rate_limit.rs`) | In-memory token bucket — DB-agnostic, already wired on both backends | push 100/min · pull 300/min · snapshot 50/min · status 300/min; `429` + `Retry-After`; stale-bucket cleanup every 60 s |
| **Retention / prune loop** (`prune.rs`) | Hourly — SQLite-specific, wired only on the SQLite branch today | archive `stock_movements` > 90 days (ledger rollup); delete `offline_queue` rows > 90 days regardless of status, in 500-row cursor batches; `incremental_vacuum(50)` per batch; `PRUNE_QUEUE_DELETED_TOTAL` counter |

Phase 1 must port the prune loop to Postgres — the batched DELETE is portable,
`incremental_vacuum` becomes autovacuum/VACUUM — and re-wire it onto the
Postgres branch, which today starts neither it nor the report-sender loop.
The rate limiter stays as-is: in-memory, per-process, DB-agnostic.

### Key Change: co-locate, don't merge

| Aspect | Change |
|--------|--------|
| Sync handlers | Unchanged (Rust) |
| License handlers | Unchanged (Go/PocketBase) |
| Sync database | SQLite → Postgres (managed addon) |
| License database | PocketBase SQLite (unchanged) |
| Deployment | Two Northflank services → one image, one service |
| Identity | HS256 JWT `tenant_id` (already the contract) |

---

## 6. Migration Plan

### Phase 1: Finish the Postgres backend (Weeks 1-2)

| Step | Action |
|------|--------|
| 1.1 | Port the full 92-table migration set to Postgres DDL |
| 1.2 | Make oz-api REST handlers run on Postgres (drop the in-memory SQLite fallback) |
| 1.3 | Enable DB TLS and raise the pool above the 8-connection default |
| 1.4 | Migrate live sync data from SQLite to Postgres; verify row counts + checksums |
| 1.5 | Re-wire the background loops (prune, report sender, rate-limit cleanup) onto Postgres — today the prune + report loops run only on the SQLite branch |

### Phase 2: Combine into one image (Week 2)

| Step | Action |
|------|--------|
| 2.1 | Multi-stage Dockerfile building both binaries (unify libc) |
| 2.2 | Supervisor entrypoint running PocketBase (8080) + Rust (3099) |
| 2.3 | caddy reverse proxy on one port with path routing |
| 2.4 | One volume (pb_data + sync data), one aggregate healthcheck |

### Phase 3: Deploy one service (Week 3)

| Step | Action |
|------|--------|
| 3.1 | Provision the managed Postgres addon on Northflank |
| 3.2 | Deploy the combined image; blue-green beside the two old services |
| 3.3 | Point the POS at the single URL; keep redirect mode (421) for stragglers |
| 3.4 | Decommission the two old Northflank services |

---

## 7. Code Changes Required

### Rust (cloud server — Postgres path)

| File | Change |
|------|--------|
| `crates/oz-core/migrations/` | Add Postgres DDL for the full schema |
| `apps/cloud-server/src/db.rs` | Run full migrations on Postgres; drop the 2-table stub |
| `apps/cloud-server/src/main.rs` | Wire oz-api to Postgres (drop the in-memory SQLite fallback) **and re-start the prune + report-sender loops on the Postgres branch** |
| `apps/cloud-server/src/prune.rs` | Port the retention loop to Postgres (batched DELETE + autovacuum; keep `PRUNE_QUEUE_DELETED_TOTAL`) |
| `apps/cloud-server/src/email.rs` | Port the report-sender loop off `rusqlite::Connection` onto the Postgres pool |
| `apps/cloud-server/src/rate_limit.rs` | Unchanged — in-memory and DB-agnostic; limits preserved |

### Packaging (new)

| File | Change |
|------|--------|
| `Dockerfile.unified` | Build both binaries into one image |
| `apps/unified/supervisord.conf` | Run PocketBase + Rust under one supervisor |
| `apps/unified/Caddyfile` | One port, path-routed to 8080 / 3099 |

### Go (license server) + Rust (POS app)

| File | Change |
|------|--------|
| `apps/license-server/*` | Unchanged (kept as the auth function) |
| `crates/oz-core/src/sync_client.rs` | Unchanged — same endpoints + token mint |
| `platform/startup/src/rate_sync.rs` | Point server URL defaults at the single service URL |

---

## 8. Environment Variables

### Auth function (PocketBase)

```bash
PB_DATA_DIR=/data/pb_data
PB_URL=https://api.oz-pos.com
OZ_LICENSE_PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----..."
# SMTP for web_users email (future website)
SMTP_HOST=... SMTP_PORT=587 SMTP_USERNAME=... SMTP_PASSWORD=...
```

### Sync function (Rust)

```bash
DATABASE_URL=postgres://user:pass@host:5432/ozpos
OZ_API_SECRET=...                # HS256 JWT signing secret
OZ_ADMIN_KEY=...                 # gates POST /api/v1/tokens
OZ_ENFORCE_PLANS=1               # reject free-plan sync (403 plan_required)
STRIPE_WEBHOOK_SECRET=whsec_...
SQUARE_WEBHOOK_SIGNATURE_KEY=...
SQUARE_WEBHOOK_URL=https://.../api/webhooks/square
OZ_API_PORT=3099
RUST_LOG=info
```

The two functions share one volume and one secret store; they communicate only
through the HS256 JWT, never directly.

---

## 9. API Endpoints (One Deployment)

### Auth function (PocketBase — kept)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/license/activate` | POST | Activate a key, issue the tenant `api_key`, sign the subscription |
| `/api/v1/license/renew` | POST | Renew a subscription |
| `/api/v1/license/status` | POST | Check status / revoke a machine (Bearer `api_key`) |
| `/api/collections/web_users/auth-*` | POST | Website auth (future, per website-plan.md) |

### Sync function (Rust — kept)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/sync/push` | POST | Accept `OfflineQueueItem`s, insert with existing IDs, per-item outcomes |
| `/api/sync/pull` | POST | Replay `offline_queue` changes since a timestamp, cursor-paginated |
| `/api/sync/snapshot` | GET | Reference-data baseline (products, tax rates, users) |
| `/api/sync/status` | GET | Health, version, pending queue depth, heartbeat interval |
| `/api/v1/tokens` | POST | Mint HS256 JWT (`X-Admin-Key` / client credentials / dev) |
| `/api/webhooks/stripe` | POST | Subscription → plan; payment → `finalize_sale` |
| `/api/webhooks/square` | POST | Payment → `finalize_sale` |
| `/health` · `/metrics` | GET | Health + Prometheus metrics (incl. retention counter) |

---

## 10. Backward Compatibility

### Transition Period (~3 weeks)

| Client | Before | During Transition | After |
|--------|--------|-------------------|-------|
| POS App | Two URLs (license + sync) | One URL (reverse proxy), old services still up | One URL |
| Sync | Rust (SQLite) | Rust (Postgres, in parallel) | Rust (Postgres) |
| License | Go/PocketBase | Go/PocketBase | Go/PocketBase |

No protocol changes: the POS keeps the same HS256 JWT mint and the same
push/pull/snapshot/status calls. The existing redirect mode
(`OZ_REDIRECT_ONLY` + `OZ_SYNC_REDIRECT_URL`, HTTP 421) covers stragglers.

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

Lock to an explicit allowlist before serving the website:

| Origin | Purpose |
|--------|---------|
| `https://oz-pos.com` | Website (global) |
| `https://id.oz-pos.com` | Website (Indonesia) |
| `http://localhost:4321` | Website (dev) |
| `tauri://localhost` | POS app (Tauri) |

### Security Headers

| Header | Value | Purpose |
|--------|-------|---------|
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-Frame-Options` | `DENY` | Prevent clickjacking |
| `Strict-Transport-Security` | `max-age=31536000` | Force HTTPS |
| `Content-Security-Policy` | `default-src 'self'` | Restrict resources |

### Hardening

- Run Postgres with TLS and a dedicated low-privilege role.
- Store `OZ_API_SECRET`, `OZ_ADMIN_KEY`, and `OZ_LICENSE_PRIVATE_KEY` in
  Northflank secrets, never in the image.
- Lock down PocketBase's `/_/` admin UI (`DisableSignUp` + IP allowlist).
- The reverse proxy is the single public entry point; both app ports stay
  internal to the container.

---

## 12. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Postgres path is a 2-table stub today | Phase 1 completes it before traffic moves |
| Prune/report loops are SQLite-only (`incremental_vacuum`, `rusqlite::Connection`) | Ported + re-wired in Phase 1 (step 1.5) before cutover; retention counter confirms aging |
| In-memory rate limiter is per-process | Fine for one node; move to a shared store (Redis) only if the sync function is scaled out |
| Data loss during SQLite → Postgres | Back up SQLite, verify row counts + checksums, replay window |
| Two databases drift | JWT `tenant_id` is the single identity; webhooks mirror plan state; reconcile if a cross-DB query appears |
| Supervisor / reverse proxy failure | Aggregate healthcheck; keep the two processes independently restartable |
| POS breakage | No protocol change; redirect mode (421) catches stragglers |
| PocketBase single-node | Holds only low-traffic auth data; migrate web_users to a hosted provider if it outgrows |
| Sync conflicts | Version-based optimistic concurrency (existing) |

---

## 13. Timeline

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| **Phase 1**: Finish Postgres backend | 1-2 weeks | Full schema + oz-api on Postgres + data migration |
| **Phase 2**: Combine into one image | 1 week | Dockerfile + supervisor + reverse proxy |
| **Phase 3**: Deploy one service | 1 week | Northflank service + Postgres addon + cutover |
| **Total** | ~3-4 weeks | One deployment, two functions |

---

## 14. Success Criteria

| Criterion | How to Verify |
|-----------|---------------|
| One deployment | One Northflank service, one image, one public URL |
| Two functions | PocketBase (auth) + Rust (sync) both healthy in the one container |
| Sync on Postgres | All sync/REST reads and writes hit Postgres; SQLite fallback gone |
| License works | activate/renew/status behave as today |
| Sync works | POS pushes/pulls via the unchanged contract |
| Webhooks work | Stripe/Square update plans and enqueue `finalize_sale` |
| One identity | JWT `tenant_id` scopes every sync row to the canonical tenant |
| No data loss | Row counts + checksums match after the SQLite → Postgres migration |
| Anti-spike loops survive | Rate limits still return `429`; hourly prune ages `offline_queue`/`stock_movements` on Postgres and the retention counter increments |
