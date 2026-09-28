#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

case "${1:-status}" in
  up)
    if [[ $# -gt 2 || ( $# -eq 2 && "$2" != --build ) ]]; then
      printf 'Usage: %s up [--build]\n' "$0" >&2
      exit 2
    fi
    ./scripts/setup-local.sh
    set -a
    source .env
    set +a
    python3 - <<'PY'
import os
from pathlib import Path
if Path(os.environ['RECOLLECT_CREDENTIAL_FILE']).resolve() != Path('.data/runtime/credentials.json').resolve():
    raise SystemExit('Custom account-file path: configure its Compose mount before using the root stack.')
if os.environ.get('RECOLLECT_PUBLIC_ORIGIN') != 'http://127.0.0.1:8787':
    raise SystemExit('Root stack uses http://127.0.0.1:8787; use a named installation for another origin.')
for key in ('RECOLLECT_MCP_CREDENTIALS_FILE', 'RECOLLECT_MCP_OUTBOX_DIR', 'RECOLLECT_ERASURE_MIRROR_CONFIG'):
    if os.environ.get(key):
        raise SystemExit('Custom provider/receipt/mirror paths require explicit container mounts; use a named installation or Compose override.')
PY
    if [[ "${2:-}" == --build ]]; then
      python3 - <<'PY'
import shutil
if shutil.disk_usage('.').free < 12 * 1024**3:
    raise SystemExit('Building requires at least 12 GiB free; no caches or data were deleted.')
PY
      ./scripts/docker.sh compose build api
    fi
    ./scripts/docker.sh compose stop --timeout 90 api worker
    ./scripts/check-local-port.sh
    ./scripts/docker.sh compose up -d --wait --wait-timeout 300 postgres neo4j
    ./scripts/docker.sh compose up --no-deps --force-recreate --abort-on-container-exit --exit-code-from migrate migrate
    ./scripts/docker.sh compose up -d --wait --wait-timeout 300 api worker
    printf 'Open http://127.0.0.1:8787. Login credentials are in .env.\n'
    ;;
  stop|down)
    ./scripts/docker.sh compose "$1" --timeout 90
    ;;
  status)
    ./scripts/docker.sh compose ps --all
    ;;
  logs)
    ./scripts/docker.sh compose logs --follow --tail 100 api worker
    ;;
  *)
    printf 'Usage: %s {up [--build]|stop|down|status|logs}\n' "$0" >&2
    exit 2
    ;;
esac
