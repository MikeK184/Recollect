#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# The image owns its default home. Personal host settings, authentication and
# customer workspaces are not mounted or forwarded to these synthetic fixtures.
./scripts/docker.sh build -t recollect-capture-hosts:local infra/capture-hosts
mkdir -p .cache/capture-hosts/{target,cargo,work}
run_fixture() {
./scripts/docker.sh run --rm --init "$@" \
  --label io.recollect.owner=session-capture-test \
  -v "$PWD/Cargo.toml:/workspace/Cargo.toml:ro" \
  -v "$PWD/Cargo.lock:/workspace/Cargo.lock:ro" \
  -v "$PWD/crates:/workspace/crates:ro" \
  -v "$PWD/.cache/capture-hosts/target:/workspace/target" \
  -v "$PWD/.cache/capture-hosts/cargo:/usr/local/cargo/registry" \
  -v "$PWD/.cache/capture-hosts/work:/workspace/.cache" \
  -e RECOLLECT_CAPTURE_HOST_FIXTURE=1 \
  recollect-capture-hosts:local \
  cargo test --locked -p recollect-agent --test capture_hosts "${RECOLLECT_HOST_TEST_ARGS[@]}"
}
RECOLLECT_HOST_TEST_ARGS=(--no-run)
run_fixture
RECOLLECT_HOST_TEST_ARGS=(--offline -- --ignored --nocapture --test-threads=1)
run_fixture --network none
