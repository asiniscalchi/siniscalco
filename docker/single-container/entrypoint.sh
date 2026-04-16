#!/usr/bin/env bash
set -Eeuo pipefail

# Platforms like Render set PORT for the external listener.
# Capture it for nginx, then reset PORT for the backend.
LISTEN_PORT="${PORT:-80}"
sed -i "s/listen 80/listen ${LISTEN_PORT}/" /etc/nginx/nginx.conf
export PORT=3000

backend &
backend_pid=$!

nginx -g "daemon off;" &
nginx_pid=$!

terminate() {
    kill -TERM "$backend_pid" "$nginx_pid" 2>/dev/null || true
    wait "$backend_pid" "$nginx_pid" 2>/dev/null || true
}

trap terminate INT TERM

set +e
wait -n "$backend_pid" "$nginx_pid"
status=$?
terminate
exit "$status"
