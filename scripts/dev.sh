#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
./scripts/check-local-port.sh
./scripts/setup-local.sh
set -a
source .env
set +a
if [[ -n "$(./scripts/docker.sh compose ps --status running -q api worker)" ]]; then
  printf 'Stop the container API and worker with ./scripts/stack.sh stop before native development.\n' >&2
  exit 1
fi
./scripts/docker.sh compose up -d --wait --wait-timeout 300 postgres neo4j
cargo run -p recollect-server -- migrate
npm --prefix web install --cache .cache/npm --no-audit --no-fund
mkdir -p .cache
./scripts/generate-api.sh
npm --prefix web run build
printf 'Open http://127.0.0.1:8787. Login credentials are in the ignored .env file.\n'
mkdir -p .cache/runtime
unset DATABASE_ADMIN_URL POSTGRES_PASSWORD RECOLLECT_DB_PASSWORD
env -u VAULT_TOKEN target/debug/recollect-server worker > .cache/runtime/worker.log 2>&1 &
RECOLLECT_WORKER_PID=$!
env -u VAULT_TOKEN target/debug/recollect-server serve &
RECOLLECT_API_PID=$!
cleanup() {
  kill -INT "$RECOLLECT_API_PID" "$RECOLLECT_WORKER_PID" 2>/dev/null || true
  wait "$RECOLLECT_API_PID" "$RECOLLECT_WORKER_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM
wait "$RECOLLECT_API_PID"
