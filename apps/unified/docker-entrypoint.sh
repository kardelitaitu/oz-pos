#!/bin/sh
set -e

# supervisord runs as root so caddy can bind the privileged :80 port; the
# app processes drop to the non-root `ozpos` user. Fix volume ownership
# here in case a volume was created by an earlier root-based image.
if [ "$(id -u)" = "0" ]; then
    mkdir -p /data /pb/pb_data
    chown -R ozpos:ozpos /data /pb || true
fi

exec "$@"
