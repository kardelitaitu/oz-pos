# VPS Migration Guide — Zero-Downtime Server Migration

**ADR:** [#11](../decisions/2026-07-13-zero-downtime-vps-migration.md)
**Status:** Implemented (2026-07-15)

This guide covers migrating the `oz-cloud-server` to a new VPS or hosting
provider with zero manual reconfiguration on POS terminals.

---

## Strategy Overview

```
Layer 1 — DNS (Primary, same domain)
  Old VPS → [copy DB + DNS update] → New VPS
  Clients resolve automatically on next DNS TTL expiry.

Layer 2 — Server Auto-Redirect (Secondary, domain change)
  Old server returns {"error":"server_migrated","new_url":"https://new.com"}
  Client detects it, updates local settings, reconnects automatically.

Layer 3 — Manual UI (Safety net)
  Settings → Cloud Sync → Server URL input field.
  For terminals offline during the migration window.
```

---

## Scenario A: Same Domain (Layer 1 — DNS)

Use this when the sync server domain stays the same (e.g., `sync.ozpos.com`).

### Steps

1. **Deploy new server** on the target VPS:
   ```bash
   # On the new VPS
   docker-compose up -d   # or however you deploy
   ```

2. **Copy the SQLite database** from the old VPS:
   ```bash
   # On the old VPS
   scp /data/oz-pos.db user@new-vps:/data/oz-pos.db
   ```

3. **Update DNS** at your domain provider (Cloudflare, Route53, etc.):
   - Change the `A`/`AAAA` record for `sync.ozpos.com` to the new VPS IP
   - Set TTL to 60 seconds during the migration

4. **Verify** the new server is responding:
   ```bash
   curl https://sync.ozpos.com/health
   # {"status":"ok","version":"0.0.9",...}
   ```

5. **Wait** for DNS propagation (typically 5-60 minutes with low TTL).
   Clients automatically resolve to the new IP on their next sync checkin.

6. **Shut down** the old server after confirming all clients are connecting
   to the new IP (check logs for zero traffic over 24 hours).

---

## Scenario B: Domain Change (Layer 2 — Auto-Redirect)

Use this when moving to a completely different domain (e.g.,
`sync.ozpos.com` → `pos-cloud.com`).

### Steps

1. **Deploy the new server** normally:
   ```bash
   docker-compose up -d
   ```

2. **Switch the old server to redirect-only mode:**
   ```bash
   # On the old VPS, set these env vars and restart:
   OZ_REDIRECT_ONLY=true
   OZ_SYNC_REDIRECT_URL=https://pos-cloud.com
   ```
   This starts a minimal server (~5 MB RAM) that only returns the
   migration redirect. No database, no pruning, no metrics.

3. **Verify** the redirect is working:
   ```bash
   curl -v https://sync.ozpos.com/api/sync/push -d '[]' -H 'Content-Type: application/json'
   # HTTP 421 Misdirected Request
   # {"error":"server_migrated","new_url":"https://pos-cloud.com"}
   ```

4. **POS clients auto-update:** When a terminal connects to the old server:
   - The server returns HTTP 421 with the `server_migrated` JSON
   - The client's sync daemon detects it, updates `sync_server_url` in the local DB
   - The next sync cycle connects to the new URL automatically
   - The migration is logged: `"server migrated — local config updated"`

5. **Monitor** the old server logs — once sync traffic drops to zero
   (typically 24-72 hours for active terminals), the redirect has served
   its purpose.

6. **Keep the old server running** for 15-30 days to catch terminals that
   were offline during the migration window. After that, shut it down.

---

## Scenario C: Terminals Offline During Migration (Layer 3 — Manual)

For terminals that were powered off or offline during the entire
migration window, the store manager can manually update the URL:

1. Open OZ-POS → **Settings** → **Cloud Sync**
2. Enter the new server URL in the **Server URL** field
3. Click **Save**
4. Click **Sync Now** to verify connectivity

---

## Environment Variables Reference

| Variable | Purpose | Required |
|----------|---------|----------|
| `OZ_REDIRECT_ONLY` | Run in redirect-only mode (ADR #11). Set to `true`. | For old server in Scenario B |
| `OZ_SYNC_REDIRECT_URL` | New server URL for migration redirect. | Yes (with `OZ_REDIRECT_ONLY`) |
| `OZ_API_PORT` | HTTP server listen port (default: 3099) | No |
| `OZ_DB_PATH` | Path to SQLite database (default: `oz-pos.db`) | For normal mode only |

---

## Rollback

If the new server has issues:

### Scenario A (DNS)
- Update the DNS record back to the old VPS IP
- Clients will revert on next DNS TTL expiry

### Scenario B (Redirect)
- Stop the new server
- On the old server, remove `OZ_REDIRECT_ONLY` and `OZ_SYNC_REDIRECT_URL`
- Restart the old server in normal mode
- Clients that already migrated will get a connection error and fall back
  to Layer 3 (manual UI) — or you can temporarily set up a reverse redirect
  on the old server back to itself

---

## Monitoring

During migration, watch the old server logs for:

```
server migrated — local config updated    # Client auto-updated successfully
sync daemon started                       # Daemon connects to new URL
```

And on the old server, watch for declining sync traffic:

```bash
# Count unique client IPs still hitting the old server
tail -f /var/log/oz-cloud-server.log | grep "server migrated" | wc -l
```

---

## Related

- [ADR #11: VPS Migration Strategy](../decisions/2026-07-13-zero-downtime-vps-migration.md)
- [ADR #10: Sync Performance Strategy](../decisions/2026-07-13-sync-performance-compression-batching.md)
- `apps/cloud-server/src/redirect.rs` — Redirect middleware
- `platform/sync/src/transport.rs` — Client-side redirect detection
- `platform/sync/src/daemon.rs` — Auto-update handler
