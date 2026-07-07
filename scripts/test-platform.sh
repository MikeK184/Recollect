#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
set -a
source .env
set +a
# Actual coding hosts and cross-process native credentials need their dedicated
# home, Secret Service and internal network. Run scripts/test-mcp-hosts.sh for
# those four proofs; ordinary platform runs must not wait on a macOS prompt.
cargo test -p recollect-server --test platform -- --ignored --test-threads=1 \
  --skip oidc_live --skip mcp_actual_hosts_native_tools_and_fresh_context \
  --skip mcp_agent_native_scope_capture_and_managed_receipts \
  --skip mcp_local_native_runner_and_cli_execute_on_the_paired_device \
  --skip mcp_private_native_runner_executes_stdio_and_http_with_exact_registration
