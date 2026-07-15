# VPS Migration Guide — Zero-Downtime Server Migration

**ADR:** [#11](../decisions/2026-07-13-zero-downtime-vps-migration.md)
**Status:** Implemented (2026-07-15)
**Target audience:** DevOps / system administrators

This guide walks through migrating `oz-cloud-server` to a new VPS with zero
manual reconfiguration on POS terminals. Every step is labelled with which
server to run it on.

---

## Quick Reference: The 3-Layer Strategy

```
Layer 1 — DNS (Primary, same domain)
  Clients connect via domain (e.g. sync.ozpos.com).
  Update the A record → clients follow automatically on next DNS TTL expiry.

Layer 2 — Server Auto-Redirect (Secondary, domain change)
  Old server returns {"error":"server_migrated","new_url":"https://new.com"}
  Client auto-updates its local sync_server_url and reconnects.

Layer 3 — Manual UI (Safety net, always available)
  Settings → Cloud Sync → Server URL input field.
```

---

## Scenario A: Same Domain (Layer 1 — DNS)

Use when the sync server domain stays the same (e.g., `sync.ozpos.com`).

### On the New Server

**1. Deploy the cloud server**

```bash
# Build and run with Docker
docker build -f Dockerfile.server -t oz-pos-cloud:latest .
docker run -d \
  --name oz-cloud-server \
  -p 3099:3099 \
  -v oz-data:/data \
  -e OZ_API_SECRET=<your-secret> \
  -e RUST_LOG=info \
  oz-pos-cloud:latest
```

> **Note:** If running without Docker, the binary is built via `cargo build --package oz-cloud-server --release` and produces `target/release/oz-cloud-server`. Defaults: `OZ_API_PORT=3099`, `OZ_DB_PATH=/data/oz-pos.db`.

**2. Verify the new server is healthy**

```bash
curl https://<new-vps-ip>:3099/health
# Expected: {"status":"ok","version":"0.0.9","uptime_seconds":12,...}
```

**3. Copy the database from the old server**

```bash
# On the old server, copy the SQLite DB to the new server.
# If you used a custom OZ_DB_PATH, use that path instead.
scp /data/oz-pos.db user@<new-vps-ip>:/data/oz-pos.db
```

If using a managed PostgreSQL backend (`DATABASE_URL`), point the new server
to the same database instance — no file copy needed.

**4. Confirm the database is intact on the new server**

```bash
# On the new server
sqlite3 /data/oz-pos.db "SELECT COUNT(*) FROM offline_queue;"
# Should match the count from the old server
```

### On the Old Server

**5. Update DNS**

Go to your domain provider (Cloudflare, Route53, etc.) and:
- Change the `A`/`AAAA` record for `sync.ozpos.com` to the new VPS IP address
- Set TTL to **60 seconds** during the migration window
- Save the old IP — you'll need it for rollback

**6. Verify DNS propagation**

```bash
# Run from multiple locations to check propagation
dig sync.ozpos.com +short
# Should return the new VPS IP

# Or test with curl once propagated:
curl https://sync.ozpos.com/health
# Should return 200 from the new server
```

**7. Monitor old server traffic**

```bash
# On the old server — watch for declining sync requests
tail -f /var/log/oz-cloud-server.log | grep "POST /api/sync"
```

Once traffic drops to zero (typically within 1–6 hours with a 60s TTL),
all active terminals have migrated.

**8. Shut down the old server**

```bash
# After confirming zero traffic for 24 hours
docker stop oz-cloud-server && docker rm oz-cloud-server
```

> ⚠️ **Keep a snapshot/backup of the old database** before shutting down, in case
> rollback is needed. See the Rollback section below.

---

## Scenario B: Domain Change (Layer 2 — Auto-Redirect)

Use when moving to a completely different domain (e.g.,
`sync.ozpos.com` → `pos-cloud.com`).

### On the New Server

**1. Deploy the new server**

```bash
docker build -f Dockerfile.server -t oz-pos-cloud:latest .
docker run -d \
  --name oz-cloud-server \
  -p 3099:3099 \
  -v oz-data:/data \
  -e OZ_API_SECRET=<your-secret> \
  oz-pos-cloud:latest
```

**2. Verify health**

```bash
curl https://<new-vps-ip>:3099/health
# {"status":"ok","version":"0.0.9",...}
```

**3. Copy the database from the old server**

```bash
# Copy the SQLite DB. If the old server used a custom OZ_DB_PATH,
# use that path instead of /data/oz-pos.db.
scp user@<old-vps-ip>:/data/oz-pos.db /data/oz-pos.db
```

### On the Old Server

**4. Switch to redirect-only mode**

Stop the old server running in normal mode and restart it in redirect-only
mode. This starts a minimal service (~5 MB RAM) that only returns the
migration redirect — no database, no pruning, no metrics.

```bash
# Stop the normal server
docker stop oz-cloud-server
docker rm oz-cloud-server

# Start in redirect-only mode
docker run -d \
  --name oz-cloud-redirect \
  -p 3099:3099 \
  -e OZ_REDIRECT_ONLY=true \
  -e OZ_SYNC_REDIRECT_URL=https://pos-cloud.com \
  -e RUST_LOG=info \
  oz-pos-cloud:latest
```

> **Without Docker:** Set the env vars and run the binary directly:
> ```bash
> export OZ_REDIRECT_ONLY=true
> export OZ_SYNC_REDIRECT_URL=https://pos-cloud.com
> export OZ_API_PORT=3099
> ./oz-cloud-server
> ```

**5. Verify the redirect is working**

```bash
# Test push endpoint
curl -v https://sync.ozpos.com/api/sync/push \
  -d '[]' \
  -H 'Content-Type: application/json'
# Expected: HTTP 421 Misdirected Request
# Body: {"error":"server_migrated","new_url":"https://pos-cloud.com"}

# Test pull endpoint
curl -v https://sync.ozpos.com/api/sync/pull \
  -d '{"since":null}' \
  -H 'Content-Type: application/json'
# Expected: Same HTTP 421 response

# Test snapshot endpoint (used after anchor expiry)
curl -v https://sync.ozpos.com/api/sync/snapshot
# Expected: Same HTTP 421 response
```

**6. What happens on each POS terminal (automatic)**

When a terminal connects to the old server during its next sync cycle:

1. The old server returns **HTTP 421** with the redirect JSON
2. The terminal's sync daemon detects `"server_migrated"` and:
   - Calls `Settings::set_sync_server_url()` to update the local SQLite DB
   - Logs: `"server migrated — local config updated"`
3. The **next sync cycle** automatically connects to `https://pos-cloud.com`
4. The terminal continues normal operation — no interruption, no staff action

This works for all three sync paths: push, pull, and snapshot (anchor-expiry
recovery). Every transport method detects the redirect.

**7. Monitor migration progress**

```bash
# On the old server — count how many terminals have migrated
tail -f /var/log/oz-cloud-server.log | grep "server migrated"
# Each line = one terminal that auto-updated

# Also watch traffic declining
tail -f /var/log/oz-cloud-server.log | grep "POST /api/sync"
```

**8. Keep the old server running**

Keep it in redirect-only mode for **15–30 days** to catch terminals that
were powered off or offline during the migration window. The redirect-only
mode is extremely lightweight — no database, no sync processing, just the
HTTP 421 response.

**9. Shut down the old server**

```bash
# After the migration window (15-30 days) with zero remaining traffic
docker stop oz-cloud-redirect && docker rm oz-cloud-redirect
```

---

## Edge Case: Terminals Offline During Migration (Layer 3 — Manual)

For any terminal that was powered off or offline during the entire
migration window, a store manager can restore connectivity manually:

1. Open OZ-POS → **Settings** → **Cloud Sync**
2. Enter the new server URL in the **Server URL** field
3. Click **Save**
4. Click **Sync Now** to verify connectivity

The terminal immediately connects to the new server on the next cycle.
This is the safety net — it always works regardless of Layers 1 and 2.

---

## Environment Variables Reference

| Variable | Default | Purpose |
|----------|---------|---------|
| `OZ_DB_PATH` | `/data/oz-pos.db` | Path to the SQLite database file |
| `OZ_API_PORT` | `3099` | HTTP server listen port |
| `OZ_REDIRECT_ONLY` | (unset) | Run in redirect-only mode. Requires `OZ_SYNC_REDIRECT_URL`. |
| `OZ_SYNC_REDIRECT_URL` | (unset) | New server URL for migration redirect. |
| `OZ_API_SECRET` | (required) | JWT signing secret for API authentication |
| `RUST_LOG` | `info` | Log level filter (`debug`, `info`, `warn`, `error`) |

---

## Rollback

### Scenario A (DNS rollback)

1. Update the DNS `A` record back to the old VPS IP address
2. Set TTL to 60s for fast propagation
3. Clients revert on their next DNS TTL expiry
4. Once all traffic returns to the old server, shut down the new server

### Scenario B (Redirect rollback)

**If the new server has issues and you need to revert:**

1. Stop the new server:
   ```bash
   docker stop oz-cloud-server && docker rm oz-cloud-server
   ```

2. On the old server, switch back to normal mode:
   ```bash
   docker stop oz-cloud-redirect && docker rm oz-cloud-redirect
   docker run -d \
     --name oz-cloud-server \
     -p 3099:3099 \
     -v oz-data:/data \
     -e OZ_API_SECRET=<your-secret> \
     oz-pos-cloud:latest
   ```

3. **Terminals that already migrated:** These now try to connect to the new
   server (which is down). They'll get connection errors. On their next
   sync cycle, they'll log the error and retry with backoff. Two options:

   **Option A — Reverse redirect on the new VPS:**
   ```bash
   # On the new VPS, run the cloud server in redirect-only mode
   # pointing back to the old domain:
   docker stop oz-cloud-server && docker rm oz-cloud-server
   docker run -d \
     --name oz-cloud-redirect \
     -p 3099:3099 \
     -e OZ_REDIRECT_ONLY=true \
     -e OZ_SYNC_REDIRECT_URL=https://sync.ozpos.com \
     -e RUST_LOG=info \
     oz-pos-cloud:latest
   ```

   **Option B — Manual UI on each terminal:**

---

## Verification Checklist

Use this checklist during every migration to confirm each step completed:

```
□ New server deployed and /health returns 200
□ Database copied and row counts match
□ (Scenario A) DNS updated, TTL set to 60s
□ (Scenario A) DNS propagated — curl to domain hits new server
□ (Scenario B) Old server in redirect-only mode
□ (Scenario B) curl to old server returns HTTP 421 + migration JSON
□ (Scenario B) curl to old server /api/sync/push returns redirect
□ (Scenario B) curl to old server /api/sync/pull returns redirect
□ (Scenario B) curl to old server /api/sync/snapshot returns redirect
□ Old server logs show "server migrated — local config updated" entries
□ Old server sync traffic declining
□ Old server shut down after migration window (15-30 days)
□ Rollback plan documented and tested
```

---

## Related

- [ADR #11: VPS Migration Strategy](../decisions/2026-07-13-zero-downtime-vps-migration.md)
- [ADR #10: Sync Performance Strategy](../decisions/2026-07-13-sync-performance-compression-batching.md)
- `apps/cloud-server/src/redirect.rs` — Redirect middleware
- `apps/cloud-server/src/main.rs` — `OZ_REDIRECT_ONLY` mode
- `platform/sync/src/transport.rs` — Client-side `parse_server_migrated()`
- `platform/sync/src/daemon.rs` — Auto URL-update handler
- `Dockerfile.server` — Cloud server Docker build
