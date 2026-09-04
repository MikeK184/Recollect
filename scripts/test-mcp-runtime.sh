#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p recollect-mcp-runtime --example mcp-fixture
# Ephemeral synthetic markers; never load real provider or database credentials.
export RECOLLECT_MCP_SECRET_TEST="$(python3 -c 'import uuid; print(uuid.uuid4())')"
export RECOLLECT_MCP_SECRET_TEST_NEXT="$(python3 -c 'import uuid; print(uuid.uuid4())')"
export RECOLLECT_MCP_AMBIENT_TEST=must_not_reach_child
export RECOLLECT_DEVICE_PROFILE=must_not_reach_child
cargo test -p recollect-mcp-runtime --test transports -- --ignored --test-threads=1
