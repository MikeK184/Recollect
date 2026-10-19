#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
set -a
source .env
set +a
# Deeply nested native-worker futures overflow the 2 MiB default test-thread
# stack; the root .cargo/config.toml sets the same floor for bare `cargo test`.
export RUST_MIN_STACK="${RUST_MIN_STACK:-8388608}"
# Extra arguments (test-name filters, --exact) pass through to the test binary.
skips=(
  --skip oidc_live --skip mcp_actual_hosts_native_tools_and_fresh_context
  --skip mcp_agent_native_scope_capture_and_managed_receipts
  --skip mcp_local_native_runner_and_cli_execute_on_the_paired_device
  --skip mcp_private_native_runner_executes_stdio_and_http_with_exact_registration
  --skip mcp_plugin_
)
# The two encrypted-recovery cases need an explicit disposable SFTP endpoint.
# Without one they are reported as not run; RECOLLECT_TEST_REQUIRE_RECOVERY=1
# keeps the strict early refusal for a complete pre-release run.
if [[ ! -f "${RECOLLECT_TEST_RECOVERY_FIXTURE:-}" ]]; then
  if [[ "${RECOLLECT_TEST_REQUIRE_RECOVERY:-0}" == 1 ]]; then
    printf 'Set RECOLLECT_TEST_RECOVERY_FIXTURE to the owned SFTP fixture JSON; see docs/runbooks/local-development.md.\n' >&2
    exit 2
  fi
  sftp_cases=(
    remote_erasure_failure_denies_locally_and_retries_one_encrypted_entry
    recovery_analytics_ownership_is_mirrored_before_native_work_and_retained_for_restore
  )
  for sftp_case in "${sftp_cases[@]}"; do
    skips+=(--skip "$sftp_case")
  done
  not_run="NOT RUN: ${sftp_cases[0]}, ${sftp_cases[1]} (no SFTP fixture; set RECOLLECT_TEST_RECOVERY_FIXTURE or RECOLLECT_TEST_REQUIRE_RECOVERY=1)"
  printf '%s\n' "$not_run"
  # Repeat it after the long run so a passing summary cannot hide the gap.
  trap 'printf "%s\n" "$not_run"' EXIT
fi
cargo build --locked -p recollect-mcp-runtime --example mcp-fixture
# Actual coding hosts and cross-process native credentials need their dedicated
# home, Secret Service and internal network. Run scripts/test-mcp-hosts.sh for
# those native/packaged-plugin proofs; ordinary platform runs must not wait on a macOS prompt.
cargo test -p recollect-server --test platform -- --ignored --test-threads=1 \
  "${skips[@]}" "$@"
