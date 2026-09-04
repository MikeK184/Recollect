#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Explicit owned fixture paths only. Never source shell Vault credentials.
if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo 'Usage: ./scripts/test-mcp-vault.sh OWNED_LOCAL_FIXTURE [AUTHORIZED_ENTERPRISE_FIXTURE]' >&2
  exit 2
fi
export RECOLLECT_TEST_VAULT_LOCAL="$1"
RECOLLECT_VAULT_TEST_ARGS=(--ignored --test-threads=1)
if [[ $# == 2 ]]; then
  export RECOLLECT_TEST_VAULT_ENTERPRISE="$2"
else
  RECOLLECT_VAULT_TEST_ARGS+=(--skip vault_live_enterprise)
fi
cargo build -p recollect-mcp-runtime --example mcp-fixture
cargo test -p recollect-mcp-runtime --lib vault_live -- "${RECOLLECT_VAULT_TEST_ARGS[@]}"
