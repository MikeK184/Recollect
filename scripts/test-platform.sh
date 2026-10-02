#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
set -a
source .env
set +a
# Recovery cases require an explicit disposable SFTP endpoint. Refuse early
# instead of failing these cases after the rest of the suite has run.
if [[ ! -f "${RECOLLECT_TEST_RECOVERY_FIXTURE:-}" ]]; then
  printf 'Set RECOLLECT_TEST_RECOVERY_FIXTURE to the owned SFTP fixture JSON; see docs/runbooks/local-development.md.\n' >&2
  exit 2
fi
cargo build --locked -p recollect-mcp-runtime --example mcp-fixture
# Actual coding hosts and cross-process native credentials need their dedicated
# home, Secret Service and internal network. Run scripts/test-mcp-hosts.sh for
# those native/packaged-plugin proofs; ordinary platform runs must not wait on a macOS prompt.
cargo test -p recollect-server --test platform -- --ignored --test-threads=1 \
  --skip oidc_live --skip mcp_actual_hosts_native_tools_and_fresh_context \
  --skip mcp_agent_native_scope_capture_and_managed_receipts \
  --skip mcp_local_native_runner_and_cli_execute_on_the_paired_device \
  --skip mcp_private_native_runner_executes_stdio_and_http_with_exact_registration \
  --skip mcp_plugin_
