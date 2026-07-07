#!/usr/bin/env bash
set -euo pipefail

RECOLLECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$RECOLLECT_ROOT/docs/tools/lint_agent_governance.sh"
python3 -B -m unittest discover -s "$RECOLLECT_ROOT/docs/tools/tests" -p 'test_*.py' -v
