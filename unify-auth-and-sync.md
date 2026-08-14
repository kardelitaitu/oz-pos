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
| **Per-tenant rate limiter** (`rate_limit.rs`) | In-memory token bucket — DB-agnostic, already wired on both backends | push 100/min · pull 300/min · snapshot 50/min · status 300/min; token mint 30/min per IP; `429` + `Retry-After`; stale-bucket cleanup every 60 s |
| **Retention / prune loop** (`prune.rs`) | Hourly, now on **both** backends — `offline_queue` retention ported to Postgres (`start_prune_loop_pg`); `archive_stock_movements` + `incremental_vacuum` remain SQLite-only | delete `offline_queue` rows > 90 days regardless of status, in 500-row cursor batches (SQLite: `incremental_vacuum(50)` per batch; PG: autovacuum reclaims space); `archive_stock_movements` ledger rollup (SQLite only); `PRUNE_QUEUE_DELETED_TOTAL` counter |

Phase 1.5 has ported the offline-queue retention half of the prune loop to
Postgres (batched `DELETE ... id = ANY($1)`; `incremental_vacuum` becomes
autovacuum) and re-wired it onto the Postgres branch, and the report-sender
email loop now runs on Postgres too (`email_pg.rs` — settings + the full
analytics bundle, reusing the shared scheduler/filter/builder logic from
`oz_core`). Remaining on SQLite: the `archive_stock_movements` ledger rollup
only (deliberate — Postgres reclaims space via autovacuum, and the stock
rollup has no PG port). The rate limiter stays as-is: in-memory,
per-process, DB-agnostic.

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
| 1.1 | Port the full 92-table migration set to Postgres DDL — **done**: `scripts/generate-pg-migration.py` → `20260813_init.pg.sql` (type-mapped, FK-order topo-sorted, 4 triggers → plpgsql); applied + verified idempotent on Postgres 16 (92 tables, 121 indexes, 4 triggers) |
| 1.2 | Make oz-api REST handlers run on Postgres (drop the in-memory SQLite fallback) — **done**: the sync function (`SyncStore`, via `SyncState.pg`), the REST surface (`crates/oz-api/src/pg.rs`, via `AppState.pg` — products, categories, tax_rates, users, plans, sales, terminals, token client-credentials), **and** the Stripe/Square webhook layer (dedup, `finalize_sale` enqueue, payment lookup, stripe-customer mapping, plan writes) all read/write Postgres on the cloud branch; the report-sender loop remains on SQLite |
| 1.3 | Raise the pool above the 8-connection default — **done**: `OZ_DB_POOL_SIZE` (default 20); TLS via rustls already wired |
| 1.4 | Migrate live sync data from SQLite to Postgres; verify row counts + checksums — **done**: `apps/cloud-server/src/bin/migrate_sqlite_to_pg.rs` copies the cloud surface (FK-topo-sorted from `pg_constraint`, `ON CONFLICT DO NOTHING` so re-runs are safe, per-table row-count + FNV-1a checksum verification); verified end-to-end + idempotent re-run against live Postgres |
| 1.5 | Re-wire the background loops onto Postgres — **done**: prune (offline-queue retention) via `start_prune_loop_pg`, **and** the report-sender email loop via `email_pg::start_report_sender_loop_pg` (settings + analytics bundle on Postgres). Only the `archive_stock_movements` stock rollup remains SQLite-only (autovacuum supersedes it on PG). Rate-limit cleanup is DB-agnostic. |

#### Phase 1.2 — data-layer design (the remaining core)

`Store` cannot be rewritten to Postgres: it is the SQLite data layer for the
whole POS (desktop + tablet + cloud), a borrow-wrapper over
`&rusqlite::Connection` with ~45 domain modules, and the cloud server is only
one of its three consumers. Phase 1.2 therefore adds a **parallel async
Postgres data layer** for the cloud server and leaves `Store` untouched:

| Decision | Choice |
|----------|--------|
| Abstraction | `apps/cloud-server/src/sync_store.rs` — a `SyncStore` enum (`Sqlite` / `Postgres`) with one async surface over `deadpool_postgres::Pool` (no `blocking` feature; stay async). **Done** for the sync function. |
| Surface | Only what the cloud server uses: `offline_queue` (push/pull/prune), `tenant_plans` (get/set), `processed_webhooks` (dedup), stripe/square webhook tables, products/tax_rates/users (snapshot **and** REST CRUD), sales (REST create/get/status), terminals + token client-credentials, SMTP + report schedule (email loop). **Implemented**: `offline_queue` push/pull + counts, `tenant_plans` get/set, products/categories/tax_rates/users/plans/sales/terminals/tokens REST, prune retention, webhook dedup + `finalize_sale` + stripe-customer mapping, **and** the email/report loop (`email_pg.rs` — SMTP config, report schedule, dedup key, store name, and the full 10-query analytics bundle on Postgres). |
| SQL | Write Postgres SQL directly (`$1` params, `now() AT TIME ZONE 'UTC'`, `ON CONFLICT`); no dialect shim |
| State | `SyncState` **and** `oz_api::AppState` hold `pg: Option<Pool>` (threaded through `build_router`); the Postgres branch passes `Some(pool)`, the SQLite branch `None`. `CloudServerState` also carries the pool so `/health` reports the real Postgres queue depth. |
| REST (oz-api) | **done** — `crates/oz-api/src/pg.rs` implements the REST surface against `deadpool_postgres::Pool`; every handler dispatches on `AppState::pg` (Postgres path vs. the untouched SQLite `Store` path). The Postgres branch no longer serves the empty in-memory SQLite. |
| Transactions | `tokio_postgres` `client.transaction()` for multi-row writes; `batch_execute` for the (already-done) migration |

Port order (highest value/risk first): **sync function** (offline_queue + plan
gating) — **done** → **prune loop, offline-queue retention** (1.5) — **done** →
**REST handlers** (1.2 tail) — **done** → **webhook dedup + `finalize_sale`**
— **done** (all webhook DB access is backend-aware; PG integration test
covers dedup, payment lookup, enqueue, tenant resolution, and plan upgrade)
→ **email loop** (1.5) — **done** (settings + analytics bundle on Postgres;
`pg_integration_email_loop_reads_postgres` covers revenue, heatmap, category
breakdown, low stock, and config round-trips) → **data migration** (1.4) —
**done** (binary + `pg_integration_migrate_and_verify`). Each landed
step has a skip-if-no-PG integration test (the established pattern) while the
SQLite branch keeps its full coverage.

### Phase 2: Combine into one image (Week 2) — complete

Status: **done** — `Dockerfile.unified`, `apps/unified/supervisord.conf`,
`apps/unified/Caddyfile`, `apps/unified/docker-entrypoint.sh`,
`apps/unified/healthcheck.sh`. Sync runs on SQLite until Phase 1 lands
(`DATABASE_URL` switches to Postgres with no packaging change).

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
| `crates/oz-core/migrations/` | **done** — `20260813_init.pg.sql` + `generate-pg-migration.py`; exposed as `oz_core::migrations::PG_INIT`. The generator now emits `ON CONFLICT DO NOTHING` on every seed INSERT, so re-applying `PG_INIT` on an existing volume is idempotent (verified live — cloud-server restarts work) |
| `apps/cloud-server/src/db.rs` | **done** — `connect_postgres` applies `PG_INIT` (full schema) instead of the 2-table stub; pool sized via `OZ_DB_POOL_SIZE` |
| `apps/cloud-server/src/main.rs` | **done** — the oz-api router receives the Postgres pool (`AppState.pg`), the prune loop starts on the Postgres branch (`start_prune_loop_pg`), the report-sender loop starts on the Postgres branch (`email_pg::start_report_sender_loop_pg`), and `/health` reports the real Postgres queue depth / `db: "postgres"` |
| `crates/oz-api/src/pg.rs` | **done** — the REST Postgres data layer (products, categories, tax_rates, users, plans, sales, terminals, token client-credentials); `PgError` → HTTP mapping; `SUM(...)::bigint` cast for stock (Postgres `SUM(bigint)` returns `numeric`); skip-if-no-PG integration test |
| `apps/cloud-server/src/webhooks.rs` | **done** — webhook dedup (`processed_webhooks` upsert + exists), payment → sale lookup, `finalize_sale` enqueue, stripe-customer mapping, and tenant-plan writes are backend-aware (`CloudServerState.pg`); PG integration test |
| `apps/cloud-server/src/prune.rs` | **done** — offline-queue retention ported to Postgres (`start_prune_loop_pg`, batched `DELETE ... id = ANY($1)`, autovacuum, `PRUNE_QUEUE_DELETED_TOTAL`); `archive_stock_movements` rollup remains SQLite-only |
| `apps/cloud-server/src/email.rs` | SQLite loop unchanged (desktop client path) |
| `apps/cloud-server/src/email_pg.rs` | **done** — Postgres report-sender loop: `settings` read/write (SMTP config + schedule + dedup key + store name) and the full 10-query analytics bundle (`daily/weekly/monthly revenue`, `top_products`, `hourly_heatmap`, `category_breakdown`, `low_stock`, `active_stock_alerts`, `category_popularity`, `category_forecast`) against `deadpool_postgres::Pool`, reusing `oz_core`'s shared scheduler/filter/`ReportEmailBuilder`; `should_send_scheduled_with_last_sent` extracted in `oz-core` so the two loops share one cadence/dedup implementation; skip-if-no-PG integration test |
| `apps/cloud-server/src/bin/migrate_sqlite_to_pg.rs` | **done** — Phase 1.4 cutover tool: copies the cloud surface from a SQLite file to Postgres (schema applied idempotently, rows via `ON CONFLICT DO NOTHING`), FK-topo-sorted copy order from `pg_constraint`, per-table row-count + FNV-1a checksum verification; `--dry-run`; unit tests + skip-if-no-PG integration test |
| `apps/cloud-server/src/rate_limit.rs` | Unchanged — in-memory and DB-agnostic; limits preserved |

### Packaging (new)

| File | Change |
|------|--------|
| `Dockerfile.unified` | **done** — build both binaries into one image |
| `apps/unified/supervisord.conf` | **done** — run PocketBase + Rust under one supervisor |
| `apps/unified/Caddyfile` | **done** — one port, path-routed to 8080 / 3099 |
| `apps/unified/docker-entrypoint.sh` | **done** — volume ownership fix + exec supervisord |
| `apps/unified/healthcheck.sh` | **done** — aggregate healthcheck (both functions + DB ping) |

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
DATABASE_URL=postgres://user:pass@host:5432/ozpos?sslmode=require  # require = encrypted via rustls (disable/prefer for local dev)
OZ_PRODUCTION=1                  # fail startup if OZ_API_SECRET/OZ_ADMIN_KEY unset; also implies OZ_DB_REQUIRE_TLS
OZ_DB_REQUIRE_TLS=1              # fail startup if DATABASE_URL omits sslmode=require (implied by OZ_PRODUCTION=1)
OZ_DB_POOL_SIZE=20               # max Postgres pool connections (positive integer; ignored for SQLite)
OZ_API_SECRET=...                # HS256 JWT signing secret — required when OZ_PRODUCTION=1 (unset falls back to the hard-coded dev secret; JWTs become forgeable)
OZ_ADMIN_KEY=...                 # gates POST /api/v1/tokens — required when OZ_PRODUCTION=1 (unset = open token mint, dev mode)
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

## 11. Security, Reliability & Growth

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

- **Run Postgres with TLS** and a dedicated low-privilege role. The pool uses
  rustls, and `sslmode=require` is enforced at startup — via
  `OZ_DB_REQUIRE_TLS=1`, or automatically under `OZ_PRODUCTION=1` — so there
  is no silent plaintext fallback. The dedicated low-privilege role is still
  to be added.
- **Fail startup in prod if `OZ_API_SECRET` or `OZ_ADMIN_KEY` is unset.**
  `OZ_PRODUCTION=1` enforces both at startup — without it the JWT secret
  falls back to the hard-coded `oz-pos-dev-secret-change-in-production`
  (forgeable JWTs) and the token mint opens to anyone.
- **Rate-limit `/api/v1/tokens`** — done: 30/min per client IP (keyed on
  `X-Forwarded-For`/`X-Real-IP`). License `/status` is still unthrottled.
- **Hash the tenant `api_key` at rest** — done: `tenants.api_key` now stores a
  bcrypt hash and `tenants.api_key_lookup` (hidden, uniquely indexed) stores a
  hex SHA-256 for O(1) lookup. `/renew` + `/status` resolve via
  `findTenantByAPIKey`; legacy plaintext rows are lazily migrated on their
  first successful auth. A re-activation without the key rotates it (the key
  is write-only). The legacy body-`api_key` fallback is still a deprecation
  item (it leaks the secret into CDN/access logs).
- **Keep the license server's 5/IP/hr activate/renew limiter** — it persists
  state to SQLite so restarts don't reset brute-force state.
- Store `OZ_API_SECRET`, `OZ_ADMIN_KEY`, and `OZ_LICENSE_PRIVATE_KEY` in
  Northflank secrets, never in the image.
- Lock down PocketBase's `/_/` admin UI (`DisableSignUp` + IP allowlist).
- The reverse proxy is the single public entry point; both app ports stay
  internal to the container.

### Reliability & operations

| Area | Requirement |
|------|-------------|
| Backups | Managed Postgres PITR **and** the PocketBase SQLite file (litestream or nightly `VACUUM INTO`); define RPO/RTO and run a restore drill |
| Aggregate healthcheck | Must check both processes, DB connectivity, and pending queue depth — not just "port is open" |
| Restart policy | Supervisor restarts each function independently; forward SIGTERM for graceful shutdown during rolling deploys |
| Alerting | Thresholds on: retention counter flatline, `offline_queue` depth, webhook 5xx, and token-mint rate |

### Growth path

| When | Action |
|------|--------|
| A feature needs a cross-DB join (e.g. plan gate must read the license tier directly) | Merge the auth collections into Postgres; retire PocketBase SQLite |
| The sync function is scaled beyond one instance | Move the rate limiter **and** the 5-min snapshot cache out of process memory into a shared store (Redis) — both are per-process today |
| Schema changes ship | Adopt a Postgres migration tool (sqlx/refinery) so DDL is versioned, not ad-hoc `batch_execute` |
| Cross-tenant isolation must be provable | Add Postgres Row-Level Security so a missed `WHERE tenant_id = ?` fails closed instead of leaking rows |
| A second tenant brings the same product SKU or username | `products.sku` and `users.username` are globally `UNIQUE` today (fine for the single `default` tenant). First multi-tenant rollout must switch to `UNIQUE(tenant_id, sku)` / `UNIQUE(tenant_id, username)` and thread `tenant_id` through the by-SKU REST lookups — otherwise the second tenant's sync push fails with a 500 |

---

## 11.5 Post-Implementation Review (2026-08-15)

Full review of the merged Phase 1 code (commits `8d565711` → `dd6a362c`): every
REST handler, the sync store, prune, webhooks, the email/analytics port, the
1.4 migration tool, the generated schema, and the plan doc's claims.

### Fixed in this review

| Finding | Fix | Test |
|---------|-----|------|
| `adjust_stock` read the previous quantity **outside** the transaction — concurrent adjustments to one SKU could lose updates (SQLite's single-writer semantics had masked it) | Whole read-modify-write moved inside the tx with the product row locked (`SELECT … FOR UPDATE`), serializing per-SKU adjustments | `pg_integration_concurrent_adjust_stock` — 20 concurrent `-1` adjustments must land as 20 ledger rows with final qty exactly `start − 20` |
| `update_sale_status` validated against a stale status — two concurrent transitions could both pass the state-machine check and double-apply | Compare-and-swap `UPDATE … WHERE id = $1 AND status = $2`; the loser re-reads and reports the current state | `pg_integration_concurrent_sale_status_transition` — exactly one of two concurrent `Pending→Active` wins; `version` bumps exactly once |
| `offline_queue` pulls filter `(tenant_id, created_at)` but no such index existed — every poll sorted the tenant's whole queue | Added `idx_offline_queue_tenant_created` to both schemas (regenerated `20260813_init.pg.sql` via the generator; index-surface guard bumped 121 → 122) | Existing `init_sql_creates_complete_schema_surface` guard |
| Stale comment claimed "the oz-api router requires SQLite" on the PG branch | Rewritten: REST handlers dispatch on `state.pg`; the in-memory SQLite is a never-written fallback | — |

### Verified clean (parity, no change needed)

- Every data-touching REST handler dispatches on `state.pg`; the in-memory SQLite fallback is never written in production PG mode, and `/health` reads the real Postgres queue depth.
- Analytics date math matches the SQLite engine: both interpret the stored UTC wall-clock; weekly grouping is Monday-first on both sides (`DATE(d, '-6 days', 'weekday 1')` ≡ `date_trunc('week', …)`).
- Snapshot column parity (`price_updated_at` defaults to `''` in both schemas); `track_serial`/`is_active` 0/1 BIGINT reads consistent.
- SMTP at rest: the PG loop decrypts via `oz_core::crypto` with the same static-key domain separation; plaintext legacy seeds degrade gracefully.
- No `unwrap()`/`expect()` in production cloud-server code; every client-supplied id stays parameter-bound (hostile-id prune tests).
- Migration tool: FK-topological copy order from `pg_constraint`, `ON CONFLICT DO NOTHING`, row-count + FNV-1a checksum verification, idempotent re-run verified live.
- `db.rs` TLS enforcement (`sslmode=require` fail-closed) and startup secret checks intact.

### Deferred (documented, not fixed)

1. Global `UNIQUE(sku)` / `UNIQUE(username)` — see the new growth-path row above.
2. Analytics scans use `created_at::date` (a cast, so `idx_sales_created_at` can't serve them). Fine for a background daily job; add `(status, (created_at::date))` if the bundle grows.
3. Postgres Row-Level Security — already listed in the growth path; becomes mandatory with real multi-tenancy.

### Test plan (what proves the port is right)

| Area | Test | Backend |
|------|------|---------|
| REST round-trip (product/tax/user/plan/sale/terminal, oversell, state machine) | `pg_integration_rest_roundtrip` (oz-api) | live PG |
| Concurrency (stock, status transitions) | the two `pg_integration_concurrent_*` tests | live PG |
| Sync store push/pull/snapshot/plan | `pg_integration_sync_store_*` (cloud-server) | live PG |
| Webhooks dedup, tenant resolution, plan writes | `pg_integration_webhooks_read_write_postgres` | live PG |
| Prune retention batching + hostile-id-as-data | `pg_integration_prune_*` + unit tests | live PG + SQLite |
| Email analytics bundle + settings | `pg_integration_email_loop_reads_postgres` | live PG |
| SQLite → Postgres cutover | migration bin unit tests + live-PG integration incl. idempotent re-run | live PG |
| CI gate | Postgres service container added to `ci.yml` (`rust-test-fast` + `rust-test-apps`) with `OZ_TEST_PG_URL`, so the skip-if-unreachable tests can no longer silently skip in CI | CI |

All tests run with `OZ_TEST_PG_URL` set against one Postgres 16 and coexist
(unique namespaces + cleanup); `cargo test -p oz-api` 138, `oz-cloud-server`
158+2, `oz-core --lib` 1773, migration bin 5.

---

## 12. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Postgres path was a 2-table stub | **Resolved** — full 92-table schema + sync function + prune retention + REST handlers + webhooks + the report-sender email loop + the 1.4 migration tool all run on Postgres; the only remaining SQLite surface is the `archive_stock_movements` stock rollup (deliberate — autovacuum supersedes it) |
| Prune/report loops are SQLite-only (`incremental_vacuum`, `rusqlite::Connection`) | Offline-queue retention is ported + re-wired on Postgres (step 1.5) and the report-sender loop runs on Postgres (`email_pg`); only the `archive_stock_movements` rollup remains SQLite-only (autovacuum supersedes it on PG); retention counter confirms aging |
| In-memory rate limiter is per-process | Fine for one node; move to a shared store (Redis) only if the sync function is scaled out |
| Snapshot cache is also in-memory | Same — Redis/shared store on scale-out |
| Dev-secret fallback / open token mint in prod | Fail startup if `OZ_API_SECRET` / `OZ_ADMIN_KEY` unset |
| Postgres pool was `NoTls` | Now rustls; startup fails unless `DATABASE_URL` sets `sslmode=require` (`OZ_DB_REQUIRE_TLS=1` or `OZ_PRODUCTION=1`) |
| `api_key` stored plaintext | Done: bcrypt + SHA-256 lookup; legacy rows migrated lazily |
| PocketBase SQLite has no backup | litestream / nightly `VACUUM INTO` + restore drill |
| Data loss during SQLite → Postgres | Back up SQLite, verify row counts + checksums, replay window |
| Two databases drift | JWT `tenant_id` is the single identity; webhooks mirror plan state; reconcile if a cross-DB query appears |
| Supervisor / reverse proxy failure | Aggregate healthcheck; keep the two processes independently restartable |
| POS breakage | No protocol change; redirect mode (421) catches stragglers |
| PocketBase single-node | Holds only low-traffic auth data; migrate web_users to a hosted provider if it outgrows |
| Sync conflicts | Version-based optimistic concurrency (existing) |
| Lost stock updates under concurrency (read-outside-tx) | Fixed: `adjust_stock` locks the product row (`FOR UPDATE`) inside the tx; concurrent-adjust test proves no lost updates |
| Double-applied sale status transitions under concurrency | Fixed: compare-and-swap `UPDATE … WHERE status = $current`; concurrent-transition test proves exactly one winner |
| PG integration tests silently skip in CI | Fixed: Postgres service container + `OZ_TEST_PG_URL` in `ci.yml` (`rust-test-fast`, `rust-test-apps`) |

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
| Sync on Postgres | Sync, prune, REST, webhook, and email/report reads/writes hit Postgres; the in-memory SQLite REST fallback is gone — the only remaining SQLite surface is the `archive_stock_movements` rollup (deliberate; autovacuum supersedes it) |
| License works | activate/renew/status behave as today |
| Sync works | POS pushes/pulls via the unchanged contract |
| Webhooks work | Stripe/Square update plans and enqueue `finalize_sale` |
| One identity | JWT `tenant_id` scopes every sync row to the canonical tenant |
| No data loss | Row counts + checksums match after the SQLite → Postgres migration |
| Anti-spike loops survive | Rate limits still return `429`; the hourly prune ages `offline_queue` on Postgres and the retention counter increments; the report-sender loop runs on Postgres; the `stock_movements` rollup remains SQLite-only (deliberate) |
| Secrets enforced | `OZ_PRODUCTION=1` fails startup unless `OZ_API_SECRET` and `OZ_ADMIN_KEY` are set (no dev fallback / open mint) |
| Backups verified | A restore drill recovers both Postgres and PocketBase SQLite to a known point |
| Alerts fire | A flatlined retention counter or growing queue depth pages a human |
