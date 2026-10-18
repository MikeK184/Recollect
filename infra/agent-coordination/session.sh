#!/usr/bin/env bash
set -euo pipefail
umask 077
# Only synthetic fixture traffic crosses this relay. The plugin still talks to
# loopback; no production endpoint or credential is involved.
socat TCP-LISTEN:18787,bind=127.0.0.1,reuseaddr,fork "TCP:host.docker.internal:${RECOLLECT_FIXTURE_PORT}" &
printf '%s' "$RECOLLECT_FIXTURE_KEYRING_PASSWORD" | gnome-keyring-daemon --unlock --components=secrets >/dev/null
printf 'export DBUS_SESSION_BUS_ADDRESS=%q\n' "$DBUS_SESSION_BUS_ADDRESS" > /run/recollect-fixture-bus
exec sleep infinity
