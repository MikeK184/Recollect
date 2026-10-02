#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
set -a
source .env
set +a
# The disposable database gets a disposable owner credential too. Browser
# failure artifacts must never contain the installation owner's password.
export RECOLLECT_OWNER_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(32))')"
# Provider tests opt in explicitly. Ordinary UI fixtures never spend the
# installation's provider allowance just because a new Brain has managed defaults.
if [[ "${RECOLLECT_TEST_OPENAI:-0}" != 1 ]]; then
  unset OPENAI_API_KEY
fi
if [[ $# == 0 ]]; then
  for RECOLLECT_UI_FILE in web/tests/*.spec.ts; do
    ./scripts/test-ui.sh "${RECOLLECT_UI_FILE#web/}"
  done
  exit 0
fi
RECOLLECT_UI_NEEDS_WORKER="${RECOLLECT_TEST_MODEL_WORKER:-0}"
RECOLLECT_UI_MCP_FIXTURE=0
RECOLLECT_UI_MCP_RUNTIME=0
for RECOLLECT_UI_SPEC in "$@"; do
  if [[ "${RECOLLECT_UI_SPEC##*/}" == mcp.spec.ts ]]; then
    RECOLLECT_UI_MCP_FIXTURE=1
  fi
  if [[ "${RECOLLECT_UI_SPEC##*/}" == mcp-runtime.spec.ts ]]; then
    RECOLLECT_UI_MCP_RUNTIME=1
    RECOLLECT_UI_NEEDS_WORKER=1
  fi
  if [[ "${RECOLLECT_UI_SPEC##*/}" == public-benchmark.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == mcp-direct.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == brain-deletion.spec.ts ]]; then
    RECOLLECT_UI_NEEDS_WORKER=1
  fi
  # Recall reads processed canonical chunks while the browser test is running.
  if [[ "${RECOLLECT_UI_SPEC##*/}" == desktop.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == recall.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == recall-graph.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == investigation.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == graph.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == graph-chrome.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == graph-combined.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == graph-analytics.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == graph-debug.spec.ts || "${RECOLLECT_UI_SPEC##*/}" == knowledge-lineage.spec.ts ]]; then
    RECOLLECT_UI_NEEDS_WORKER=1
  fi
done
if [[ "${RECOLLECT_TEST_DEX:-}" == 1 ]]; then
  export RECOLLECT_OIDC_ISSUER=http://127.0.0.1:5556/dex
  export RECOLLECT_OIDC_CLIENT_ID=recollect-test
  export RECOLLECT_OIDC_CLIENT_SECRET="$DEX_CLIENT_SECRET"
  export RECOLLECT_OIDC_SCOPES='profile groups'
fi
RECOLLECT_TEST_DB="recollect_ui_$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
./scripts/docker.sh compose exec -T postgres createdb -U recollect_admin "$RECOLLECT_TEST_DB"
RECOLLECT_UI_PID=''
RECOLLECT_UI_WORKER_PID=''
cleanup() {
  local RECOLLECT_UI_EXIT_STATUS=$?
  if [[ "$RECOLLECT_UI_EXIT_STATUS" != 0 && -d .cache/ui-test-results ]]; then
    mkdir -p ".cache/ui-failures/$RECOLLECT_TEST_DB"
    cp -R .cache/ui-test-results/. ".cache/ui-failures/$RECOLLECT_TEST_DB/"
    printf 'Browser failure artifacts: .cache/ui-failures/%s\n' "$RECOLLECT_TEST_DB" >&2
  fi
  if [[ -n "$RECOLLECT_UI_WORKER_PID" ]]; then
    # Background children may inherit ignored SIGINT before Tokio initializes.
    # TERM also uses the server's durable drain path once its handler is ready.
    kill -TERM "$RECOLLECT_UI_WORKER_PID" 2>/dev/null || true
    wait "$RECOLLECT_UI_WORKER_PID" 2>/dev/null || true
  fi
  if [[ -n "$RECOLLECT_UI_PID" ]]; then
    kill -TERM "$RECOLLECT_UI_PID" 2>/dev/null || true
    wait "$RECOLLECT_UI_PID" 2>/dev/null || true
  fi
  if [[ "${RECOLLECT_PUBLIC_SEMANTIC:-0}" == 1 && "$RECOLLECT_UI_EXIT_STATUS" != 0 ]]; then
    printf 'Semantic benchmark failed; retained owned database and private artifacts for request reconciliation: %s\n' "$RECOLLECT_TEST_DB" >&2
    return 0
  fi
  if ! env -u VAULT_TOKEN target/debug/recollect-server privacy-reconcile >> .cache/ui/cleanup.log 2>&1; then
    printf 'Owned UI database and journals retained because privacy/analytics cleanup is uncertain: %s\n' "$RECOLLECT_TEST_DB" >&2
    return 1
  fi
  if [[ "$(./scripts/docker.sh compose exec -T postgres psql -U recollect_admin -d "$RECOLLECT_TEST_DB" -Atqc 'SELECT count(*) FROM analytics_attempts WHERE cleaned_at IS NULL')" != 0 ]]; then
    printf 'Owned UI database retained because analytical attempts still need cleanup: %s\n' "$RECOLLECT_TEST_DB" >&2
    return 1
  fi
  if ! ./scripts/docker.sh compose exec -T postgres psql -U recollect_admin -d "$RECOLLECT_TEST_DB" -Atqc "SELECT coalesce(json_agg(id),'[]') FROM brains" | node scripts/cleanup-ui-graph.mjs; then
    printf 'Owned UI database retained because graph cleanup did not complete: %s\n' "$RECOLLECT_TEST_DB" >&2
    return 1
  fi
  ./scripts/docker.sh compose exec -T postgres dropdb -U recollect_admin --force "$RECOLLECT_TEST_DB"
  rm -f ".cache/ui/$RECOLLECT_TEST_DB-credentials.json"
  rm -rf ".cache/ui/$RECOLLECT_TEST_DB-artifacts"
  rm -rf ".cache/ui/$RECOLLECT_TEST_DB-erasure-journal"
  rm -f ".cache/ui/$RECOLLECT_TEST_DB-mcp-runtime.json"
}
trap cleanup EXIT
export DATABASE_URL="${DATABASE_URL%/*}/$RECOLLECT_TEST_DB"
export DATABASE_ADMIN_URL="${DATABASE_ADMIN_URL%/*}/$RECOLLECT_TEST_DB"
export RECOLLECT_BIND=127.0.0.1:8788
export RECOLLECT_PUBLIC_ORIGIN=http://127.0.0.1:8788
export RECOLLECT_UI_TEST_ORIGIN="$RECOLLECT_PUBLIC_ORIGIN"
export RECOLLECT_CREDENTIAL_FILE=".cache/ui/$RECOLLECT_TEST_DB-credentials.json"
export RECOLLECT_ARTIFACT_DIR="$PWD/.cache/ui/$RECOLLECT_TEST_DB-artifacts"
export RECOLLECT_ERASURE_JOURNAL="$PWD/.cache/ui/$RECOLLECT_TEST_DB-erasure-journal"
export RECOLLECT_STATIC_DIR="$PWD/.cache/ui/web-dist"
cargo build -q -p recollect-server -p recollect-agent
./scripts/generate-api.sh
npm --prefix web run build -- --outDir ../.cache/ui/web-dist
env -u VAULT_TOKEN target/debug/recollect-server migrate
if [[ "$RECOLLECT_UI_MCP_FIXTURE" == 1 ]]; then
  env -u VAULT_TOKEN target/debug/recollect-server mcp-definition-import crates/server/tests/fixtures/mcp-catalogue.json
fi
if [[ "$RECOLLECT_UI_MCP_RUNTIME" == 1 ]]; then
  cargo build -q -p recollect-mcp-runtime --example mcp-fixture
  export RECOLLECT_UI_MCP_MARKER="$RECOLLECT_ARTIFACT_DIR/desktop-effect-proof.txt"
  mkdir -p "$RECOLLECT_ARTIFACT_DIR"
  python3 - "$PWD" ".cache/ui/$RECOLLECT_TEST_DB-mcp-runtime.json" <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
definition = json.loads((root / 'crates/server/tests/fixtures/mcp-runtime.json').read_text())
definition['command'] = str(root / 'target/debug/examples/mcp-fixture')
pathlib.Path(sys.argv[2]).write_text(json.dumps(definition))
PY
  env -u VAULT_TOKEN target/debug/recollect-server mcp-definition-import ".cache/ui/$RECOLLECT_TEST_DB-mcp-runtime.json"
fi
mkdir -p .cache/ui
env -u VAULT_TOKEN target/debug/recollect-server serve > .cache/ui/server.log 2>&1 &
RECOLLECT_UI_PID=$!
RECOLLECT_UI_READY=0
for attempt in {1..180}; do
  if curl -fsS "$RECOLLECT_UI_TEST_ORIGIN/health/live" >/dev/null 2>&1; then RECOLLECT_UI_READY=1; break; fi
  if ! kill -0 "$RECOLLECT_UI_PID" 2>/dev/null; then break; fi
  sleep 1
done
[[ "$RECOLLECT_UI_READY" == 1 ]] || { printf 'Owned UI server did not become ready; inspect .cache/ui/server.log\n' >&2; exit 1; }
if [[ "$RECOLLECT_UI_NEEDS_WORKER" == 1 ]]; then
  env -u VAULT_TOKEN target/debug/recollect-server worker > .cache/ui/worker.log 2>&1 &
  RECOLLECT_UI_WORKER_PID=$!
fi
env -u VAULT_TOKEN npm --prefix web run test:ui -- "$@"
if [[ -z "$RECOLLECT_UI_WORKER_PID" ]]; then
  env -u VAULT_TOKEN target/debug/recollect-server worker > .cache/ui/worker.log 2>&1 &
  RECOLLECT_UI_WORKER_PID=$!
fi
# macOS can delay a newly started native binary before main; use the same
# bounded startup allowance as the HTTP fixture, and still require an empty queue.
RECOLLECT_UI_DRAIN_DEADLINE=$((SECONDS + 180))
while (( SECONDS < RECOLLECT_UI_DRAIN_DEADLINE )); do
  RECOLLECT_PENDING="$(./scripts/docker.sh compose exec -T postgres psql -U recollect_admin -d "$RECOLLECT_TEST_DB" -Atqc "SELECT count(*) FROM jobs WHERE state IN ('queued','running')")"
  if [[ "$RECOLLECT_PENDING" == 0 ]]; then break; fi
  if ! kill -0 "$RECOLLECT_UI_WORKER_PID" 2>/dev/null; then break; fi
  sleep 0.5
done
[[ "$RECOLLECT_PENDING" == 0 ]] || { printf 'Native worker did not drain queued UI work\n' >&2; exit 1; }
printf 'Native worker drained the persisted UI queue.\n'
