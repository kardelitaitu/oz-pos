#!/bin/sh
set -e

# If running as root, ensure volume directory /data exists and is owned by ozpos,
# then drop privileges to the non-root ozpos user using gosu.
if [ "$(id -u)" = "0" ]; then
    mkdir -p /data
    chown -R ozpos:ozpos /data || true
    if command -v gosu >/dev/null 2>&1; then
        exec gosu ozpos "$@"
    else
        exec su -s /bin/sh ozpos -c "$*"
    fi
fi

exec "$@"
