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
