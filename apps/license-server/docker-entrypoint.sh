#!/bin/sh
set -e

# If running as root, ensure the volume directory /pb/pb_data exists and is
# owned by the non-root `pb` user, then drop privileges to that user using
# su-exec. Running as root is required only for this ownership fix (e.g. a
# pb_data volume created by an older root-based image); PocketBase itself
# must never serve HTTP as root (DOCKER-01).
if [ "$(id -u)" = "0" ]; then
    mkdir -p /pb/pb_data
    chown -R pb:pb /pb/pb_data || true
    if command -v su-exec >/dev/null 2>&1; then
        exec su-exec pb "$@"
    else
        exec su -s /bin/sh pb -c "$*"
    fi
fi

exec "$@"
