#!/usr/bin/env bash
set -euo pipefail

RECOLLECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec python3 -B "$RECOLLECT_ROOT/docs/tools/lint_agent_governance.py" "$@"
