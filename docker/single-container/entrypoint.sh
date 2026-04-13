#!/usr/bin/env bash
set -Eeuo pipefail

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
