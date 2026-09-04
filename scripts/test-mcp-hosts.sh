#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - <<'PY'
import shutil
if shutil.disk_usage('.').free < 12 * 1024**3:
    raise SystemExit('The owned host fixture needs at least 12 GiB free for compilation and live databases. Free rebuildable project caches before running it.')
PY
./scripts/docker.sh build -t recollect-capture-hosts:local infra/capture-hosts
./scripts/docker.sh build -t recollect-mcp-hosts:local infra/mcp-hosts
mkdir -p .cache/mcp-hosts/{target,cargo,work}
fixture="recollect-mcp-hosts-$(python3 -c 'import uuid; print(uuid.uuid4().hex[:12])')"
fixture_env="$PWD/.cache/mcp-hosts/$fixture.env"
python3 - "$fixture_env" <<'PY'
import os, sys, uuid
values = {k:uuid.uuid4().hex for k in ('POSTGRES_PASSWORD','RECOLLECT_DB_PASSWORD','RECOLLECT_OWNER_PASSWORD','NEO4J_PASSWORD','RECOLLECT_FIXTURE_KEYRING_PASSWORD')}
values.update(POSTGRES_USER='recollect_admin',POSTGRES_DB='recollect',RECOLLECT_MCP_HOST_FIXTURE='1',RECOLLECT_TEST_STATE_DIR='/workspace/.cache',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0')
values['DATABASE_URL']='postgres://recollect_app:'+values['RECOLLECT_DB_PASSWORD']+'@fixture-db:5432/recollect'
values['DATABASE_ADMIN_URL']='postgres://recollect_admin:'+values['POSTGRES_PASSWORD']+'@fixture-db:5432/recollect'
values['NEO4J_AUTH']='neo4j/'+values['NEO4J_PASSWORD']
values['NEO4J_URL']='http://fixture-graph:7474'
with os.fdopen(os.open(sys.argv[1],os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),'w') as f:
    for k,v in values.items(): f.write(k+'='+v+'\n')
with os.fdopen(os.open(sys.argv[1]+'.graph',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600),'w') as f:
    f.write('NEO4J_AUTH='+values['NEO4J_AUTH']+'\n')
PY
run_fixture() {
  ./scripts/docker.sh run --rm --init "$@" \
    --label io.recollect.owner=mcp-host-test \
    -v "$PWD/Cargo.toml:/workspace/Cargo.toml:ro" \
    -v "$PWD/Cargo.lock:/workspace/Cargo.lock:ro" \
    -v "$PWD/crates:/workspace/crates:ro" \
    -v "$PWD/.cache/mcp-hosts/target:/workspace/target" \
    -v "$PWD/.cache/capture-hosts/cargo:/usr/local/cargo/registry" \
    -v "$PWD/.cache/mcp-hosts/work:/workspace/.cache" \
    --env-file "$fixture_env" recollect-mcp-hosts:local "${fixture_command[@]}"
}
# Fetch/build separately; the actual hosts later run on an internal network with
# only their owned databases and loopback synthetic model/API endpoints.
fixture_command=(bash -c 'cargo build --locked -p recollect-agent --bins && cargo build --locked -p recollect-mcp-runtime --example mcp-fixture && cargo test --locked -p recollect-server --test platform --no-run')
run_fixture
./scripts/docker.sh network create --internal --label io.recollect.owner=mcp-host-test "$fixture" > /dev/null
cleanup() {
  ./scripts/docker.sh rm -fv "$fixture-db" "$fixture-graph" > /dev/null 2>&1 || true
  ./scripts/docker.sh network rm "$fixture" > /dev/null 2>&1 || true
}
trap cleanup EXIT
./scripts/docker.sh run -d --name "$fixture-db" --network "$fixture" --network-alias fixture-db \
  --label io.recollect.owner=mcp-host-test --env-file "$fixture_env" \
  -v "$PWD/infra/postgres-init.sh:/docker-entrypoint-initdb.d/recollect.sh:ro" \
  pgvector/pgvector:pg17 > /dev/null
for attempt in {1..60}; do
  if ./scripts/docker.sh exec "$fixture-db" pg_isready -U recollect_admin -d recollect > /dev/null 2>&1; then break; fi
  sleep 1
done
./scripts/docker.sh exec "$fixture-db" pg_isready -U recollect_admin -d recollect > /dev/null
# The production harness checks its analytical ownership journal at teardown.
# This pinned graph image includes its matching GDS plugin. Its entrypoint treats
# every NEO4J_* variable as configuration, so pass only the graph's own inputs.
./scripts/docker.sh run -d --name "$fixture-graph" --network "$fixture" --network-alias fixture-graph \
  --label io.recollect.owner=mcp-host-test --env-file "$fixture_env.graph" \
  --memory 3g --cpus 2 \
  -e 'NEO4J_PLUGINS=["graph-data-science"]' \
  -e 'NEO4J_dbms_security_procedures_unrestricted=gds.*' \
  -e NEO4J_server_memory_heap_initial__size=512m \
  -e NEO4J_server_memory_heap_max__size=1G \
  -e NEO4J_server_memory_pagecache_size=512m \
  neo4j:2026.08 > /dev/null
for attempt in {1..180}; do
  if ./scripts/docker.sh exec "$fixture-graph" wget -q --spider http://localhost:7474 > /dev/null 2>&1; then break; fi
  if [[ "$(./scripts/docker.sh inspect --format '{{.State.Running}}' "$fixture-graph")" != true ]]; then
    printf 'Owned graph fixture exited before readiness.\n' >&2
    exit 1
  fi
  sleep 1
done
./scripts/docker.sh exec "$fixture-graph" wget -q --spider http://localhost:7474
fixture_command=(dbus-run-session -- recollect-test-session bash -c 'cargo test --locked --offline -p recollect-server --test platform mcp_actual_hosts_native_tools_and_fresh_context -- --ignored --nocapture --test-threads=1 && cargo test --locked --offline -p recollect-server --test platform mcp_agent_native_scope_capture_and_managed_receipts -- --ignored --nocapture --test-threads=1 && cargo test --locked --offline -p recollect-server --test platform mcp_local_native_runner_and_cli_execute_on_the_paired_device -- --ignored --nocapture --test-threads=1 && cargo test --locked --offline -p recollect-server --test platform mcp_private_native_runner_executes_stdio_and_http_with_exact_registration -- --ignored --nocapture --test-threads=1')
run_fixture --network "$fixture"
