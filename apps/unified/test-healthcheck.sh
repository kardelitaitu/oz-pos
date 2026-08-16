#!/bin/sh
set -e

# Tests for healthcheck.sh. Runs the real script against a fake `wget`
# that serves canned /api/health payloads, so the SMTP counter logic
# (N consecutive failures, reset on success, skip when not configured)
# is exercised without a live stack.
#
# Usage: sh apps/unified/test-healthcheck.sh

HERE="$(cd "$(dirname "$0")" && pwd)"
SHIM="$(mktemp -d)"
trap 'rm -rf "$SHIM"' EXIT

cat > "$SHIM/wget" <<'EOF'
#!/bin/sh
case "$*" in
    *:8080*)
        [ "${FAKE_LICENSE_DOWN:-0}" = "1" ] && { echo "license down" >&2; exit 1; }
        printf '%s' "$FAKE_LICENSE_HEALTH"
        ;;
    *:3099*)
        printf '%s' "${FAKE_SYNC_HEALTH:-{\"status\":\"ok\"}}"
        ;;
    *)
        echo "unexpected wget args: $*" >&2
        exit 1
        ;;
esac
EOF
chmod +x "$SHIM/wget"

STATE=/tmp/oz-health-smtp-fails
rm -f "$STATE"

PASS=0
FAIL=0

# run_health <license-payload> [license-down] [smtp-max-fails] [paddle-max-fails]
# Runs healthcheck.sh with the given fake payload and prints its exit code.
run_health() {
    payload="$1"
    down="${2:-0}"
    code=0
    if [ -n "${3:-}" ]; then export OZ_HEALTH_SMTP_MAX_FAILS="$3"; else unset OZ_HEALTH_SMTP_MAX_FAILS; fi
    if [ -n "${4:-}" ]; then export OZ_HEALTH_PADDLE_MAX_FAILS="$4"; else unset OZ_HEALTH_PADDLE_MAX_FAILS; fi
    FAKE_LICENSE_HEALTH="$payload" FAKE_LICENSE_DOWN="$down" \
        PATH="$SHIM:$PATH" sh "$HERE/healthcheck.sh" || code=$?
    echo "$code"
}

# expect <name> <expected-exit> <actual-exit>
expect() {
    if [ "$2" -eq "$3" ]; then
        PASS=$((PASS + 1))
        echo "PASS: $1"
    else
        FAIL=$((FAIL + 1))
        echo "FAIL: $1 (expected exit $2, got $3)"
    fi
}

VERIFIED='{"status":"ok","smtp":{"configured":true,"error":"","verified":true}}'
BROKEN='{"status":"ok","smtp":{"configured":true,"error":"relay rejected sender","verified":false}}'
NOT_CONFIGURED='{"status":"ok","smtp":{"configured":false,"verified":false,"error":""}}'
OLD_IMAGE='{"status":"ok","db_connected":true}'
# Full payloads (smtp ok) with a paddle block present / secret missing.
PADDLE_OK='{"status":"ok","smtp":{"configured":true,"error":"","verified":true},"paddle":{"secret_configured":true,"price_tiers_configured":true,"price_tiers_mappings":2,"error":""}}'
PADDLE_MISSING='{"status":"ok","smtp":{"configured":true,"error":"","verified":true},"paddle":{"secret_configured":false,"price_tiers_configured":true,"price_tiers_mappings":2,"error":""}}'
# Broken SMTP + healthy paddle (for the counter-independence test).
PADDLE_OK_WITH_BROKEN_SMTP='{"status":"ok","smtp":{"configured":true,"error":"relay rejected sender","verified":false},"paddle":{"secret_configured":true,"price_tiers_configured":true,"price_tiers_mappings":2,"error":""}}'

# 1. Verified sender -> healthy, no state file.
rm -f "$STATE"
expect "verified sender is healthy" 0 "$(run_health "$VERIFIED")"
[ ! -f "$STATE" ] && echo "PASS: state file cleared on success" || { echo "FAIL: state file should not exist"; FAIL=$((FAIL + 1)); }

# 2. Broken relay: sub-threshold runs stay healthy, Nth run fails the container.
rm -f "$STATE"
expect "broken relay run 1 (sub-threshold)" 0 "$(run_health "$BROKEN")"
expect "broken relay run 2 (sub-threshold)" 0 "$(run_health "$BROKEN")"
expect "broken relay run 3 (threshold reached)" 1 "$(run_health "$BROKEN")"

# 3. A recovery resets the counter; the next failure starts over at 1.
expect "recovery is healthy" 0 "$(run_health "$VERIFIED")"
[ ! -f "$STATE" ] && echo "PASS: recovery cleared the counter" || { echo "FAIL: counter should reset on recovery"; FAIL=$((FAIL + 1)); }
expect "broken again run 1 (counter restarted)" 0 "$(run_health "$BROKEN")"

# 4. SMTP not configured -> always healthy, counter not touched.
rm -f "$STATE"
expect "not-configured run 1" 0 "$(run_health "$NOT_CONFIGURED")"
expect "not-configured run 2" 0 "$(run_health "$NOT_CONFIGURED")"
[ ! -f "$STATE" ] && echo "PASS: unconfigured SMTP never counts failures" || { echo "FAIL: state file should not exist"; FAIL=$((FAIL + 1)); }

# 5. Old image without the smtp block -> healthy (backward compatible).
expect "old payload without smtp block" 0 "$(run_health "$OLD_IMAGE")"

# 6. License endpoint down -> fails immediately (unchanged behavior).
expect "license endpoint down" 1 "$(run_health "" 1)"

# 7. OZ_HEALTH_SMTP_MAX_FAILS=1 -> first broken probe fails immediately.
rm -f "$STATE"
expect "max-fails=1 fails on first broken probe" 1 "$(run_health "$BROKEN" 0 1)"

# ── Paddle secret gate ────────────────────────────────────────────────
PSTATE=/tmp/oz-health-paddle-fails
rm -f "$STATE" "$PSTATE"

# 8. Secret present -> healthy, no paddle state file.
expect "paddle secret present is healthy" 0 "$(run_health "$PADDLE_OK")"
[ ! -f "$PSTATE" ] && echo "PASS: paddle state file cleared on success" || { echo "FAIL: paddle state file should not exist"; FAIL=$((FAIL + 1)); }

# 9. Secret missing: sub-threshold runs stay healthy, Nth run fails.
expect "paddle missing run 1 (sub-threshold)" 0 "$(run_health "$PADDLE_MISSING")"
expect "paddle missing run 2 (sub-threshold)" 0 "$(run_health "$PADDLE_MISSING")"
expect "paddle missing run 3 (threshold reached)" 1 "$(run_health "$PADDLE_MISSING")"

# 10. Recovery resets the paddle counter.
expect "paddle recovery is healthy" 0 "$(run_health "$PADDLE_OK")"
[ ! -f "$PSTATE" ] && echo "PASS: paddle recovery cleared the counter" || { echo "FAIL: paddle counter should reset on recovery"; FAIL=$((FAIL + 1)); }

# 11. OZ_HEALTH_PADDLE_MAX_FAILS=1 -> fails on the first miss.
rm -f "$PSTATE"
expect "paddle max-fails=1 fails on first miss" 1 "$(run_health "$PADDLE_MISSING" 0 0 1)"

# 12. Gate counters are independent: broken SMTP + ok paddle (and the
#     reverse) must not trip the other gate's state.
rm -f "$STATE" "$PSTATE"
expect "broken smtp + ok paddle run 1" 0 "$(run_health "$PADDLE_OK_WITH_BROKEN_SMTP")"
[ -f "$STATE" ] && [ ! -f "$PSTATE" ] && echo "PASS: only the SMTP counter incremented" || { echo "FAIL: gate counters should be independent"; FAIL=$((FAIL + 1)); }
expect "ok smtp + missing paddle run 1" 0 "$(run_health "$PADDLE_MISSING")"
[ -f "$PSTATE" ] && [ ! -f "$STATE" ] && echo "PASS: only the paddle counter incremented" || { echo "FAIL: gate counters should be independent"; FAIL=$((FAIL + 1)); }

# 13. Old payload without the paddle block -> healthy (backward compatible).
expect "old payload without paddle block" 0 "$(run_health "$OLD_IMAGE")"

echo
echo "== $PASS passed, $FAIL failed =="
[ "$FAIL" -eq 0 ]
