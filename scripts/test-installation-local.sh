#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# No Docker daemon, paid provider, personal keychain or system trust-store mutation.
cargo build --locked -p recollect-agent --bins
cargo test --locked -p recollect-mcp-runtime --test agent_tls --no-run
cargo test --locked -p recollect-server --test operations_offline
python3 -B -m unittest discover -s scripts/tests -p 'test_installation.py' -v
python3 -B -m unittest discover -s scripts/tests -p 'test_native_tls.py' -v
python3 -B -m unittest discover -s scripts/tests -p 'test_caddy.py' -v
