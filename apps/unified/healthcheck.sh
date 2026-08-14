#!/bin/sh
set -e

# Aggregate healthcheck for the unified image (Docker HEALTHCHECK).
# Both functions must be healthy: PocketBase's /api/health and the cloud
# server's /health (which pings its DB and reports sync queue depth).
# Pings go straight to the app ports — not through caddy — so a broken
# proxy route cannot mask an unhealthy process.

if ! wget -qO- http://localhost:8080/api/health >/dev/null 2>&1; then
    echo "unified healthcheck: license (PocketBase) /api/health failed" >&2
    exit 1
fi

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
