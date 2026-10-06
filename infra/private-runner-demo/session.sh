#!/usr/bin/env bash
set -euo pipefail
umask 077
# A lab-only TLS relay reaches the existing local installation without changing it.
socat OPENSSL-LISTEN:9443,bind=127.0.0.1,reuseaddr,fork,cert=/run/recollect/tls/relay.pem,verify=0 TCP:host.docker.internal:8787 &
# This Secret Service belongs to the disposable Ubuntu container, not the host.
printf '%s' "$RECOLLECT_DEMO_KEYRING_PASSWORD" | gnome-keyring-daemon --unlock --components=secrets >/dev/null
printf 'export DBUS_SESSION_BUS_ADDRESS=%q\n' "$DBUS_SESSION_BUS_ADDRESS" > /run/recollect/bus.env
if [[ ! -f /srv/recollect-demo/hello.txt ]]; then
    printf 'Ubuntu private-runner filesystem proof\nThis file exists inside the demo container only.\n' > /srv/recollect-demo/hello.txt
    chmod 555 /srv/recollect-demo
    chmod 444 /srv/recollect-demo/hello.txt
fi
while [[ ! -f /run/recollect/runner-id ]]; do sleep 1; done
exec recollect-agent private-runner "$(cat /run/recollect/runner-id)" /root/.local/share/recollect/receipts
