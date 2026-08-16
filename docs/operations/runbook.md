# Operations Runbook — OZ-POS (unified Northflank deployment)

One Northflank service, one Docker image. Two functions behind one caddy
reverse proxy (single public port):

| Function | Process | Internal port | Data |
|----------|---------|---------------|------|
| Auth (license) | PocketBase + Go hooks | 8080 | PocketBase SQLite (`pb_data/data.db`) |
| Sync (cloud) | Rust axum | 3099 | Postgres (managed addon) |

This runbook covers the §11 reliability contract of `unify-auth-and-sync.md`:
Postgres PITR, PocketBase backup, restore drills, and alerting on retention
flatline, queue depth, webhook 5xx, and token-mint rate. It also documents the
metrics that make each incident observable.

---

## 1. Monitoring Surfaces

### `GET /health` (public, no auth)

JSON — the aggregate healthcheck target. `status` is `"ok"` when the DB ping
succeeds, `"degraded"` otherwise:

```json
{
  "status": "ok",
  "sync_queue_depth": 0,
  "db_latency_us": 1200,
  "last_sync_at": "...",
  "db_kind": "postgres"
}
```

`sync_queue_depth` is a **JSON field, not a Prometheus gauge** — alert on it by
polling `/health` (see §5). `db_kind` tells you which backend the server is on
(`sqlite` during the cutover window, `postgres` after).

### `GET /metrics` (Prometheus text)

All counters below are rendered here. `GET /api/sync/status` (JWT-authed) is
the per-tenant view: health, version, the tenant's pending queue depth, and
the tiered heartbeat interval.

### `apps/unified/healthcheck.sh` (container healthcheck)

Checks **both** processes, DB connectivity, and pending queue depth — not just
"port is open". A failing healthcheck is what the orchestrator uses to restart
or drain the container.

---

## 2. Metrics Reference

| Metric | Type | Meaning |
|--------|------|---------|
| `health_checks_total` / `health_check_failures_total` | counter | Health endpoint hits / DB-ping failures. A rising failure count = DB unreachable. |
| `health_db_latency_micros` | histogram | DB ping latency. |
| `sync_pushes_total{outcome}` | counter | Pushed items, by `accepted` / `conflict` / `rejected`. |
| `sync_anchor_expired_total` | counter | 410 `anchor_expired` responses (client `since` older than the pruned window). |
| `sync_pull_row_decode_failures_total` | counter | Rows that failed to decode during pull — **schema drift between server and store**. Any increase is a bug, not noise (SYNC-10). |
| `sync_push_duration_ms` / `sync_pull_duration_ms` / `sync_batch_size_bytes` | histogram | Sync latency + payload size. |
| `db_connection_contention_seconds{handler}` | histogram | DB lock acquisition time per handler. High p99 = pool starvation. |
| `prune_queue_deleted_total` | counter | `offline_queue` rows deleted by the hourly prune (90-day horizon). |
| `prune_sent_reports_deleted_total` | counter | `sent_reports` dedup claims deleted by the same prune. |
| `rate_limit_429_total{limiter="sync"}` | counter | 429s from the per-tenant sync limiters (push 100/min, pull 300/min, snapshot 50/min, status 300/min). |
| `rate_limit_429_total{limiter="token"}` | counter | 429s from the token-mint limiter (30/min per client IP). Sustained growth = brute force or a broken client. |
| `webhook_5xx_total` | counter | 5xx from the Stripe/Square webhook handlers. Any increase means real payment events are failing server-side and payment/plan state may be stale. |

The license server (Go) exposes no Prometheus metrics; its rate limiter is the
persisted 5/IP/hr bucket for activate/renew/status, observable only through
HTTP 429s in its access logs.

---

## 3. Incident Response

### 3.1 DB connection failure

- **Symptom:** `/health` → `"status": "degraded"`; `health_check_failures_total` increments; cloud-server logs DB errors.
- **Action:** Check the Postgres addon status on Northflank (the managed DB, not the container). Verify `DATABASE_URL` and that the app connects as the post-cutover role (§6.3). Check pool exhaustion (`db_connection_contention_seconds` p99 high, `OZ_DB_POOL_SIZE` too low).
- **Escalation:** > 2 min → on-call. Postgres itself unreachable → follow the addon's failover procedure; the sync function degrades (POS keeps working offline — the offline queue absorbs the outage by design).

### 3.2 Sync queue backlog

- **Symptom:** `sync_queue_depth` (from `/health`) stays elevated; pull latency (`sync_pull_duration_ms`) climbs.
- **Action:** Identify the tenant(s) via the sync API logs / `rate_limit_429_total{limiter="sync"}`. A small queue is normal (offline POS devices flush on reconnect); a *growing* queue with no 429s means pushes are failing server-side — check webhook `finalize_sale` path and DB writes.
- **Escalation:** `sync_queue_depth > 500` for 10 min → page on-call.

### 3.3 Webhook 5xx (payments)

- **Symptom:** `webhook_5xx_total` increases.
- **Action:** Webhooks are the payment-authenticity boundary — a 5xx means Stripe/Square events failed server-side. Check `STRIPE_WEBHOOK_SECRET` / `SQUARE_WEBHOOK_SIGNATURE_KEY` first (a misconfigured secret is the most common cause), then DB errors. Stripe redelivers with backoff, so the damage self-heals once the root cause is fixed.
- **Escalation:** any sustained increase over 15 min → on-call (payment/plan state may be stale).

### 3.4 Token-mint brute force

- **Symptom:** `rate_limit_429_total{limiter="token"}` increases (minting is rare in normal operation — once per terminal boot or JWT expiry).
- **Action:** A 429 cluster on `token` means someone is hammering `POST /api/v1/tokens` (admin key or client credentials). Check the mint logs for the client IP, verify no leaked `OZ_ADMIN_KEY`/`OZ_API_SECRET`, rotate secrets if there is any doubt.
- **Escalation:** sustained 429s for 15 min → on-call; block the offending IP at the proxy.

### 3.5 Tenant hammering sync

- **Symptom:** `rate_limit_429_total{limiter="sync"}` sustained (per-tenant limiters: push 100/min, pull 300/min, snapshot 50/min, status 300/min).
- **Action:** A tenant in a 429 loop is a buggy client or misconfiguration, not an attack. Identify the tenant from logs; check its `client_id`/`client_secret` and the JWT expiry/re-mint path. The limiter already protects the DB — no emergency action needed beyond contacting the tenant.
- **Escalation:** if one tenant's traffic degrades the shared Postgres pool, disable that tenant's sync temporarily (the offline queue absorbs it).

### 3.6 Retention flatline

- **Symptom:** `prune_queue_deleted_total` **and** `prune_sent_reports_deleted_total` stay flat while the queue has rows older than 90 days.
- **Action:** The hourly prune (`start_prune_loop_pg`) deletes `offline_queue` rows > 90 days in 500-row batches plus `sent_reports` claims at the same horizon. A flatline with old rows present means the loop died — check the cloud-server logs for the prune task, then restart the server (supervisord restarts it).
- **Escalation:** flatline for 7 days with old rows present → on-call; unbounded `offline_queue` growth is a disk + pull-latency risk.

### 3.7 Pull decode failures (schema drift)

- **Symptom:** `sync_pull_row_decode_failures_total` increases.
- **Action:** This is a **code bug**, not an ops issue — the server and the row decoder disagree on the `offline_queue` schema. Clients receive 5xx, not silently truncated pages. Revert the offending deploy.
- **Escalation:** any increase → on-call (every client pull fails).

### 3.8 High error rate

- **Symptom:** `webhook_5xx_total` + `health_check_failures_total` climbing, sync latency degraded.
- **Action:** Check the latest deploy for regressions; verify external dependencies (payment gateway, license server, Postgres addon).
- **Escalation:** roll back the last deploy if error rate doesn't recover in 5 min (the image is one deployable unit — rolling back restores both functions together).

### 3.9 RLS fail-closed surprise (queries return 0 rows)

- **Symptom:** sync/REST handlers return empty results or rejected writes for data that exists; not a crash.
- **Cause:** RLS is **enforced** post-cutover (§6.3). Every tenant-touching query must run inside a transaction whose first statement is `SET LOCAL oz.tenant_id = '<tenant>'` (the sync data layer does this automatically in `apps/cloud-server/src/sync_store.rs`). A zero-rows result means the GUC was not set — a missed tenant scope is failing closed *by design*.
- **Action:** Check the handler actually went through the tenant-scoped sync layer rather than a raw connection. Do **not** "fix" it by disabling RLS.
- **Escalation:** if a legitimate path is broken, revert that path's code change.

---

## 4. Backup & Restore

### 4.1 Postgres (managed addon) — PITR

The sync DB is the only data that scales and the only one that must never lose
a committed write. Rely on the managed addon's backup service, **not** on the
container:

- **Enable** the addon's automated backups (daily full snapshot) and **PITR**
  (WAL archiving). Keep ≥ 7 days of PITR window so a logical corruption can be
  rewound past its introduction.
- **Verify** the backup job runs on schedule (Northflank dashboard) and that
  the storage target is outside the app's own volume.
- **RPO ≤ 5 min** (PITR replay of WAL), **RTO ≤ 1 h** (restore to a new addon
  instance + repoint `DATABASE_URL`).

**Restore drill (monthly):** restore the latest snapshot + PITR replay to a
throwaway addon instance, run the verification query set (§4.4), then delete
the throwaway. A drill that has never been executed is not a backup strategy.

### 4.2 PocketBase SQLite (auth)

Low-traffic, but irreplaceable: `tenants`, `license_keys`, `subscriptions`,
`tenant_machines`. Two acceptable strategies per `unify-auth-and-sync.md`:

**Option A — litestream (continuous, recommended):** replicate
`/data/pb_data/data.db` to object storage continuously. Minimal config:

```yaml
# litestream.yml
dbs:
  - path: /data/pb_data/data.db
    replicas:
      - url: s3://<bucket>/pb-data
        retention: 72h
```

**Option B — nightly `VACUUM INTO` + off-machine copy (simpler):** `VACUUM
INTO` produces a consistent, compacted snapshot that is safe to take while
PocketBase is running:

```bash
# cron — every night 02:00
0 2 * * * bash /opt/oz/backup-pb.sh
```

```bash
# /opt/oz/backup-pb.sh
set -euo pipefail
SNAP="/backups/pb-data-$(date +%Y%m%d-%H%M%S).db"
sqlite3 /data/pb_data/data.db "VACUUM INTO '$SNAP'"
gzip -f "$SNAP"                                   # then ship to S3/GCS
find /backups -name 'pb-data-*.db.gz' -mtime +30 -delete
```

The generic `scripts/backup-db.sh` also works against any SQLite file
(integrity check → consistent `.backup` → gzip → retention): `bash
scripts/backup-db.sh /data/pb_data/data.db` with `BACKUP_DIR=/backups`.

**RPO ≤ 24 h** (nightly) or **≤ 5 min** (litestream), **RTO ≤ 1 h**.

### 4.3 Desktop POS SQLite (per-install)

Each POS keeps its own local SQLite store. The shipped scripts handle it:

- Backup: `bash scripts/backup-db.sh [db-path]` (integrity check, consistent
  `.backup`, gzip, 30-day retention; `BACKUP_DIR`/`RETENTION_DAYS` env).
- Restore: `bash scripts/restore-db.sh <backup-file> [db-path]` (verifies the
  backup, snapshots the current DB as `.pre-restore`, replaces, drops stale
  `-wal`/`-shm` sidecars, final smoke query).

### 4.4 Restore drill checklist (quarterly — both databases)

1. **Postgres:** restore the newest snapshot + WAL to a throwaway addon;
   verify with `SELECT count(*) FROM offline_queue; SELECT count(*) FROM
   sales;` and a tenant-scoped spot check (`SELECT ... WHERE tenant_id =
   '<tenant>'` with `SET LOCAL oz.tenant_id`); confirm row counts match the
   production addon's `pg_stat_user_tables`.
2. **PocketBase:** restore the newest backup into a scratch PocketBase;
   verify `tenants`, `license_keys`, `subscriptions`, `tenant_machines` row
   counts and that a sample `api_key_lookup` still resolves (hash lookup is
   data — a truncation shows up as a failed lookup).
3. **Time the drill.** RTO is "restore + verify + repoint" end-to-end; if it
   exceeds the target, fix the process before you need it.
4. **Record** the drill result (date, RTO observed, any deviation).

---

## 5. Alerting

Rules below assume a Prometheus scraping `/metrics` on the cloud server and a
`/health` poller for queue depth (JSON, not a gauge — poll with curl + jq on a
cron, or a json_exporter scrape).

| Alert | Expression | For | Severity |
|-------|-----------|-----|----------|
| Retention flatline | `increase(prune_queue_deleted_total[7d]) == 0` **and** `increase(prune_sent_reports_deleted_total[7d]) == 0` | 7d | warning (prune loop dead; verify old rows actually exist first) |
| Queue depth growing | `/health` → `sync_queue_depth > 100` | 5 min | warning |
| Queue depth critical | `/health` → `sync_queue_depth > 500` | 10 min | page |
| Webhook 5xx | `increase(webhook_5xx_total[15m]) > 0` | 15 min | warning (payment events failing server-side) |
| Token-mint brute force | `increase(rate_limit_429_total{limiter="token"}[15m]) > 0` | 15 min | warning (minting is rare; 429s = attacker or broken client) |
| Sync tenant abuse | `increase(rate_limit_429_total{limiter="sync"}[5m]) > 0` | 5 min | info → warning if sustained |
| Health degraded | `increase(health_check_failures_total[5m]) > 0` | 5 min | page |
| Pull decode failures | `increase(sync_pull_row_decode_failures_total[1h]) > 0` | 1 h | page (schema drift — every client pull fails) |
| DB contention | p99 `db_connection_contention_seconds` > 1 s | 10 min | warning (pool starvation → raise `OZ_DB_POOL_SIZE` or add capacity) |

License-server 429s (5/IP/hr activate/renew/status) have no metric — alert
from access-log error counts (e.g. Prometheus mtail / Loki) if the auth
function comes under attack.

---

## 6. Deployment & Lifecycle

### 6.1 Process supervision

Supervisord runs both functions in one container and **restarts each
independently**; it forwards SIGTERM so rolling deploys shut down gracefully
(drain in-flight sync, flush WAL). `docker-entrypoint.sh` fixes volume
ownership before exec'ing supervisord. The container healthcheck
(`apps/unified/healthcheck.sh`) gates traffic on both functions + DB.

### 6.2 Startup secrets (fail closed)

In production, `OZ_PRODUCTION=1` must be set. It makes the server **refuse to
start** if `OZ_API_SECRET` or `OZ_ADMIN_KEY` is unset (no dev-secret
fallback, no open token mint) and implies `OZ_DB_REQUIRE_TLS=1` (startup
fails if `DATABASE_URL` lacks `sslmode=require`). Keep all three in the
Northflank secret store, never in the image.

### 6.3 RLS cutover (one-time deploy step)

Tenant isolation ships in two halves; both must be true before multi-tenant
data is exposed:

1. **Schema side (already shipped):** the generated PG migration enables RLS
   + a `tenant_isolation` policy on the 15 tenant tables, and the sync data
   layer opens every transaction with `SET LOCAL oz.tenant_id`.
2. **Cutover (must be run):** as the DB owner, run
   `psql "$DATABASE_URL" -f scripts/rls-cutover.sql` — it creates the
   restricted `oz_app` role, grants DML (including the non-RLS webhook
   tables `processed_webhooks` / `payments`), creates the
   `oz_webhook_resolver` role (NOLOGIN BYPASSRLS — used by the webhook
   handlers for their pre-tenant resolution reads via tx-scoped `SET LOCAL
   ROLE`, so it never needs a password), and `FORCE ROW LEVEL SECURITY`s
   the 15 tables so the table-owner bypass no longer applies. Then enable
   login on the role and **point `DATABASE_URL` at `oz_app`** **and set
   `OZ_APPLY_SCHEMA=0`** — without it, startup re-applies `PG_INIT` (full
   DDL) and fails with `permission denied for schema public`, because
   `oz_app` only has DML grants. The schema is applied once by the
   migration tool as the owner; the app then boots without touching DDL.
   From then on, a missed `WHERE tenant_id = ?` returns zero rows instead
   of leaking. Reversible: `ALTER TABLE ... NO FORCE ROW LEVEL SECURITY`
   on all 15 + `DROP ROLE oz_app` + `DROP ROLE oz_webhook_resolver`.

### 6.4 Rate limits (self-protection — do not weaken)

| Surface | Limit |
|---------|-------|
| `/api/sync/push` | 100/min per tenant |
| `/api/sync/pull` | 300/min per tenant |
| `/api/sync/snapshot` | 50/min per tenant |
| `/api/sync/status` | 300/min per tenant |
| `/api/v1/tokens` | 30/min per client IP |
| License activate/renew/status | 5/hr per IP (persisted to SQLite — survives restarts) |

The sync limiter and snapshot cache are **per-process** (in-memory); scaling
past one sync instance requires moving both to a shared store (Redis) — see
the growth path in `unify-auth-and-sync.md`.

---

## 7. Disk Space Management

### Log Growth Cap (50 MB per service)

Every service in the Compose stack runs the `json-file` log driver with
`max-size: "10m"` and `max-file: "5"` (set in `docker-compose.yml` and
the prod/pg overrides) — so each service holds **at most 50 MB of logs
(5 × 10 MB) regardless of uptime**. Unbounded log growth that fills the
host disk is no longer possible.

### Check Current Usage

```bash
# Overall Docker disk usage — images, containers, volumes, build cache
docker system df

# Per-container / per-image / per-volume breakdown
docker system df -v

# Host disk (the real limit)
df -h
```

To confirm a service's log rotation is applied (its log file should stay
well under 50 MB):

```bash
docker inspect -f '{{.LogPath}}' oz-pos-pos-cloud-server-1
ls -lh "$(docker inspect -f '{{.LogPath}}' oz-pos-pos-cloud-server-1)"
```

### Prune Policy (safe cron)

**Weekly cron — safe to automate.** Reclaims dangling images, build cache,
and stopped containers. Never touches volumes:

```bash
# crontab -e (root) — every Sunday 03:00, reclaim > 7 days old
0 3 * * 0 docker system prune -f --filter "until=168h"
```

Manual `docker compose down` stops containers without removing volumes,
so an abandoned stack's volumes accumulate until pruned manually.

**Volume pruning is deliberate, manual, and never in cron** — the data
volumes (`oz_cloud_data`, `pb_data`, `redis_data`, `oz_pg_data`) hold the
actual databases:

```bash
# Inspect before deleting anything
docker volume ls
docker volume inspect oz-pos_oz_cloud_data

# Manual cleanup of orphaned volumes only
docker volume prune
```

> ⚠️ **Never** put `docker system prune --volumes` or `docker volume prune`
> in cron — it deletes the SQLite / PocketBase / PostgreSQL data volumes
> and their backups are not automatically restored.

### Incident: Disk Space Pressure

- **Symptom:** `df -h` shows > 85% used; `docker system df` reports large
  reclaimable build cache; services slow or fail to write
- **Action:** `docker system prune -f --filter "until=24h"`; identify the
  biggest consumer with `docker system df -v`; verify log rotation is
  applied (`docker inspect ... .LogPath`)
- **Escalation:** If a data volume (`oz_cloud_data`/`pb_data`) is the
  growth source, run `scripts/backup-db.sh` first, then investigate the
  DB size — never delete the volume as a shortcut

---

## 8. Unified Northflank Deployment (live config)

> **Status:** live since 2026-08-16. One service serves both auth
> (PocketBase) and sync (Rust) behind a single caddy, replacing the two
> standalone services (`oz-pos-license-service` + `oz-sync`).

### Service

| Setting | Value |
|---------|-------|
| Service name | `oz-cloud` |
| Public URL | `https://oz--cloud--76cyv4d6bn54.code.run` |
| Dockerfile | `Dockerfile.unified` (repo root) |
| Port | `80` (caddy; routes to :8080 PocketBase / :3099 Rust) |
| Volume | single volume at `/data` (Northflank free tier = 1 volume) |
| Build trigger | push/merge to `main` (CI builds from the branch) |

**Single-volume layout (DOCKER-11):**

| Function | Data path |
|----------|-----------|
| Sync (Rust SQLite) | `/data/oz-pos.db` |
| Auth (PocketBase) | `/data/pb_data/` (`serve --dir=/data/pb_data`) |

Both live under `/data` so one persistent volume covers the whole service.
The old `pb_data:/pb/pb_data` mount from the standalone license service no
longer exists — migrating that data requires a PocketBase backup → restore
(see `unify-auth-and-sync.md` §Phase 3.5).

### Environment variables

| Variable | Value / source | Notes |
|----------|----------------|-------|
| `OZ_LICENSE_PRIVATE_KEY` | RSA PEM | required — Go license server exits without it |
| `OZ_API_SECRET` | `openssl rand -hex 32` | required when `OZ_PRODUCTION=1` |
| `OZ_ADMIN_KEY` | random string | required when `OZ_PRODUCTION=1`; gates token mint |
| `OZ_PRODUCTION` | `1` | fail-closed boot: refuses to start if either secret is unset; implies `OZ_DB_REQUIRE_TLS=1` |
| `OZ_ENFORCE_PLANS` | `1` | reject free-plan sync (403 plan_required) |
| `OZ_CORS_ORIGINS` | optional | extra origins beyond the default allowlist |
| `PADDLE_WEBHOOK_SECRET` | optional | Paddle Billing webhook provisioning |
| `PADDLE_PRICE_TIERS` | optional | `price_id:tier_key` map |
| `OZ_SMTP_*`, `OZ_DISCORD_WEBHOOK`, `OZ_WEB_ALLOWED_ORIGINS` | optional | if configured |

> ⚠️ **Do not set `OZ_PRODUCTION=1` unless both `OZ_API_SECRET` and
> `OZ_ADMIN_KEY` are set** — startup fails fast by design (no dev-secret
> fallback, no open token mint). Without `OZ_PRODUCTION`, the service runs
> in dev mode: `/api/v1/tokens` mints freely.

### Verification checklist (post-deploy)

```bash
BASE="https://oz--cloud--76cyv4d6bn54.code.run"
curl -s "$BASE/health"                                  # sync pill → 200 ok
curl -s "$BASE/api/health"                              # auth pill → 200
curl -s -X POST "$BASE/api/v1/license/activate" \
  -H 'Content-Type: application/json' -d '{}'           # PocketBase 400 (not 404)
curl -s -o /dev/null -w '%{http_code}' "$BASE/_/"       # admin UI → 200
curl -s -o /dev/null -w '%{http_code}' -X POST \
  "$BASE/api/v1/paddle/webhook"                          # 503 not-configured (not 404)
```

Also: create the PocketBase superuser via the `/_/` first-boot installer
link (or shell: `pocketbase superuser upsert EMAIL PASS`).

### App-side URL references (the 5 hardcoded spots)

All point at the unified host; each also has an env-var override:

| File | Change | Override |
|------|--------|----------|
| `crates/oz-core/src/license_verification.rs` | `LICENSE_SERVER_URL` const | `OZ_LICENSE_SERVER_URL` |
| `apps/desktop-client/tauri.conf.json` | CSP `connect-src` | — |
| `apps/tablet-client/tauri.conf.json` | CSP `connect-src` | — |
| `ui/src/features/auth/LicenseActivationScreen.tsx` | `AUTH_SERVICE_URL` fallback | `VITE_AUTH_SERVICE_URL` |
| `ui/src/features/auth/__tests__/LicenseActivationScreen.test.tsx` | pinned URL | — |

The **sync server URL** is per-install user config: Settings → Cloud Sync
→ enter `https://oz--cloud--76cyv4d6bn54.code.run`. Unlike auth, it is
stored in the local DB (never compiled in).
