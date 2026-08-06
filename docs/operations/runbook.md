# Operations Runbook — OZ-POS

## Incident Response

### 1. DB Connection Failure
- **Symptom:** `/health` returns `"status": "degraded"`, `health_check_failures_total` increments
- **Action:** Check SQLite file permissions, disk space. Restart cloud-server.
- **Escalation:** If > 2 min, notify on-call engineer.

### 2. Sync Queue Backlog (> 100 items)
- **Symptom:** `sync_queue_depth` metrics spike, sync latency increases
- **Action:** Check network connectivity. Verify API tokens haven't expired. Increase sync frequency temporarily.
- **Escalation:** If > 500 items after 10 min, page on-call.

### 3. High Error Rate (> 5%)
- **Symptom:** `error_rate` metric above 5% for 5 minutes
- **Action:** Check latest deploy for regressions. Verify external dependencies (payment gateway, license server).
- **Escalation:** Roll back last deploy if error rate doesn't recover in 5 min.

### 4. Rate Limit Abuse
- **Symptom:** `rate_limit_hits_total` spikes for specific tenant
- **Action:** Review tenant activity. Contact tenant if legitimate. Block if malicious.
- **Escalation:** If affecting other tenants, temporarily disable offending tenant's sync.

## Backup & Restore

- **Backup:** SQLite `.backup` command daily. Store off-machine (S3/GCS).
- **Restore:** Replace `oz-pos.db` with backup. Restart application.
- **Testing:** Monthly restore test to verify backup integrity.

---

## Disk Space Management

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
