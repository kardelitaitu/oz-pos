#!/bin/sh
set -e

# Aggregate healthcheck for the unified image (Docker HEALTHCHECK).
# All three layers must be healthy:
#   1. license server (PocketBase) /api/health responds,
#   2. its SMTP sender-identity probe passes (relay reachable + sender
#      verified) — N consecutive failures fail the container,
#   3. the cloud server's /health (DB ping + sync queue depth).
# Pings go straight to the app ports — not through caddy — so a broken
# proxy route cannot mask an unhealthy process.
#
# Tested by apps/unified/test-healthcheck.sh (fake-wget harness).

# ── 1 + 2: license server /api/health ────────────────────────────────
license_health="$(wget -qO- http://localhost:8080/api/health 2>/dev/null)" || {
    echo "unified healthcheck: license (PocketBase) /api/health failed" >&2
    exit 1
}

# The SMTP sender-identity probe is a STATUS field — a broken relay does
# NOT fail the HTTP check (only a DB outage does) — so it needs its own
# gate here. Fail the container only after N CONSECUTIVE failing probes
# (default 3) so a transient relay hiccup doesn't flap it; each successful
# probe resets the counter. SMTP that is not configured at all is skipped
# (request-otp answers 503 by design then).
max_smtp_fails="${OZ_HEALTH_SMTP_MAX_FAILS:-3}"
smtp_fail_state="/tmp/oz-health-smtp-fails"

smtp_block="$(printf '%s' "$license_health" | grep -o '"smtp":{[^}]*}' || true)"
if [ -n "$smtp_block" ]; then
    smtp_configured="$(printf '%s' "$smtp_block" | grep -c '"configured":true' || true)"
    smtp_verified="$(printf '%s' "$smtp_block" | grep -c '"verified":true' || true)"

    if [ "$smtp_configured" -eq 1 ] && [ "$smtp_verified" -ne 1 ]; then
        smtp_fails="$(cat "$smtp_fail_state" 2>/dev/null || echo 0)"
        case "$smtp_fails" in
            *[!0-9]* | '') smtp_fails=0 ;;
        esac
        smtp_fails=$((smtp_fails + 1))
        echo "$smtp_fails" > "$smtp_fail_state"
        echo "unified healthcheck: SMTP sender identity not verified (consecutive failure $smtp_fails/$max_smtp_fails)" >&2
        if [ "$smtp_fails" -ge "$max_smtp_fails" ]; then
            echo "unified healthcheck: failing container after $max_smtp_fails consecutive SMTP probe failures" >&2
            exit 1
        fi
    else
        rm -f "$smtp_fail_state"
    fi
fi

# ── 3: cloud server /health ──────────────────────────────────────────
sync_health="$(wget -qO- http://localhost:3099/health 2>/dev/null)" || {
    echo "unified healthcheck: sync (cloud server) /health failed" >&2
    exit 1
}

case "$sync_health" in
    *'"status":"ok"'*) ;;
    *)
        echo "unified healthcheck: sync health not ok: $sync_health" >&2
        exit 1
        ;;
esac

exit 0
