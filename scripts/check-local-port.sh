#!/usr/bin/env bash
set -euo pipefail
python3 - <<'PY'
import socket
with socket.socket() as probe:
    probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    try:
        probe.bind(('127.0.0.1', 8787))
    except OSError:
        raise SystemExit('Port 8787 is occupied. Stop the existing Recollect API and worker before switching runtimes.')
PY
