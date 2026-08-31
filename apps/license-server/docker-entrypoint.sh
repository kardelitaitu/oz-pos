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
        # P7: su-exec passes "$@" verbatim, but `su -c "$*"` collapses
        # args into one string and breaks quoted-argument CMDS. The shim
        # `-c 'exec "$@"' -- "$@"` re-executes the original command with
        # each argument preserved as a separate word.
        exec su -s /bin/sh pb -c 'exec "$@"' -- "$@"
    fi
fi

exec "$@"
