#!/usr/bin/env bash
set -euo pipefail
# This is the disposable image's home and session, never the user's keychain.
printf '%s' "$RECOLLECT_FIXTURE_KEYRING_PASSWORD" | gnome-keyring-daemon --unlock --components=secrets > /dev/null
exec "$@"
