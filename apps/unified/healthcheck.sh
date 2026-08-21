#!/bin/sh
set -e

# Aggregate healthcheck for the unified image (Docker HEALTHCHECK).
# All four layers must be healthy:
#   1. license server (PocketBase) /api/health responds,
#   2. its SMTP sender-identity probe passes (relay reachable + sender
#      verified) — N consecutive failures fail the container,
#   3. its Paddle webhook secret is configured (a rotated/removed secret
#      would make every webhook answer 503) — N consecutive misses fail
#      the container,
#   4. the cloud server's /health (DB ping + sync queue depth).
# Pings go straight to the app ports — not through caddy — so a broken
# proxy route cannot mask an unhealthy process.
#
# Tested by apps/unified/test-healthcheck.sh (fake-wget harness).

# ── N-consecutive counter helpers ─────────────────────────────────────
# Both gates below fail the container only after N CONSECUTIVE bad probes
# (so a transient hiccup doesn't flap it); any good probe resets the
# counter. Each gate keeps its own state file and threshold.

# count_consecutive <state-file> <max-fails> <gate-description>
# Increments the counter in state-file; exits 1 when max-fails is reached.
count_consecutive() {
    state_file="$1"
    max_fails="$2"
    gate_desc="$3"
    fails="$(cat "$state_file" 2>/dev/null || echo 0)"
    case "$fails" in
        *[!0-9]* | '') fails=0 ;;
    esac
    fails=$((fails + 1))
    echo "$fails" > "$state_file"
    echo "unified healthcheck: $gate_desc (consecutive failure $fails/$max_fails)" >&2
    if [ "$fails" -ge "$max_fails" ]; then
        echo "unified healthcheck: failing container after $max_fails consecutive $gate_desc" >&2
        exit 1
    fi
}

# reset_consecutive <state-file>
reset_consecutive() {
    rm -f "$1"
}

# ── 1 + 2 + 3: license server /api/health ────────────────────────────
license_health="$(wget -qO- http://localhost:8080/api/health 2>/dev/null)" || {
    echo "unified healthcheck: license (PocketBase) /api/health failed" >&2
    exit 1
}

# ── 2: SMTP sender-identity gate ──────────────────────────────────────
# The probe is a STATUS field — a broken relay does NOT fail the HTTP
# check (only a DB outage does) — so it needs its own gate here. SMTP that
# is not configured at all is skipped (request-otp answers 503 by design).
max_smtp_fails="${OZ_HEALTH_SMTP_MAX_FAILS:-3}"
smtp_fail_state="/tmp/oz-health-smtp-fails"

smtp_block="$(printf '%s' "$license_health" | grep -o '"smtp":{[^}]*}' || true)"
if [ -n "$smtp_block" ]; then
    smtp_configured="$(printf '%s' "$smtp_block" | grep -c '"configured":true' || true)"
    smtp_verified="$(printf '%s' "$smtp_block" | grep -c '"verified":true' || true)"

    if [ "$smtp_configured" -eq 1 ] && [ "$smtp_verified" -ne 1 ]; then
        count_consecutive "$smtp_fail_state" "$max_smtp_fails" "SMTP sender identity not verified"
    else
        reset_consecutive "$smtp_fail_state"
    fi
fi

# ── 3: Paddle webhook secret gate ─────────────────────────────────────
# Unlike SMTP, Paddle has no supported "not configured" mode — the webhook
# endpoint is always mounted and the boot gate requires the secret — so a
# missing secret_configured (e.g. the secret was rotated out from under the
# running service) counts as a failure after N consecutive probes.
# Payloads without the paddle block (pre-field images) are skipped.
max_paddle_fails="${OZ_HEALTH_PADDLE_MAX_FAILS:-3}"
paddle_fail_state="/tmp/oz-health-paddle-fails"

paddle_block="$(printf '%s' "$license_health" | grep -o '"paddle":{[^}]*}' || true)"
if [ -n "$paddle_block" ]; then
    paddle_secret="$(printf '%s' "$paddle_block" | grep -c '"secret_configured":true' || true)"

    if [ "$paddle_secret" -ne 1 ]; then
        count_consecutive "$paddle_fail_state" "$max_paddle_fails" "Paddle webhook secret not configured"
    else
        reset_consecutive "$paddle_fail_state"
    fi
fi

# ── 4: cloud server /health ───────────────────────────────────────────
# Retry up to 3 times with 2s delay — the Rust server may still be
# starting (schema migration on first PostgreSQL boot).
sync_health=""
for i in 1 2 3; do
    sync_health="$(wget -qO- http://localhost:3099/health 2>/dev/null)" && break
    echo "unified healthcheck: sync attempt $i/3 failed, retrying..." >&2
    sleep 2
done
if [ -z "$sync_health" ]; then
    echo "unified healthcheck: sync (cloud server) /health failed after 3 retries" >&2
    exit 1
fi

case "$sync_health" in
    *'"status":"ok"'*) ;;
    *)
        echo "unified healthcheck: sync health not ok: $sync_health" >&2
        exit 1
        ;;
esac

exit 0
