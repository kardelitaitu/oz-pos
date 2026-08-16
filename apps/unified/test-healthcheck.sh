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

# run_health <license-payload> [license-down] [max-fails]
# Runs healthcheck.sh with the given fake payload and prints its exit code.
run_health() {
    payload="$1"
    down="${2:-0}"
    maxfails="${3:-}"
    code=0
    if [ -n "$maxfails" ]; then
        FAKE_LICENSE_HEALTH="$payload" FAKE_LICENSE_DOWN="$down" \
            OZ_HEALTH_SMTP_MAX_FAILS="$maxfails" PATH="$SHIM:$PATH" sh "$HERE/healthcheck.sh" || code=$?
    else
        FAKE_LICENSE_HEALTH="$payload" FAKE_LICENSE_DOWN="$down" \
            PATH="$SHIM:$PATH" sh "$HERE/healthcheck.sh" || code=$?
    fi
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

echo
echo "== $PASS passed, $FAIL failed =="
[ "$FAIL" -eq 0 ]
