#!/usr/bin/env bash
set -Eeuo pipefail

# NGINX_PORT controls the externally-facing listener.
# Falls back to PORT (set by Render and similar platforms), then to 80.
export NGINX_PORT="${NGINX_PORT:-${PORT:-80}}"
# Backend always listens on its own fixed port.
export PORT=3000

envsubst '${NGINX_PORT}' < /etc/nginx/nginx.conf.template > /etc/nginx/nginx.conf

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
