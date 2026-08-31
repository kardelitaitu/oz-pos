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
        # P7: gosu/su-exec pass "$@" verbatim, but `su -c "$*"` collapses
        # args into one string and breaks quoted-argument CMDS. The shim
        # `-c 'exec "$@"' -- "$@"` re-executes the original command with
        # each argument preserved as a separate word.
        exec su -s /bin/sh ozpos -c 'exec "$@"' -- "$@"
    fi
fi

exec "$@"
