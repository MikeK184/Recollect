#!/usr/bin/env bash
# Default run checks document shape, the baseline fixture and the offline supply-chain
# pins (image digests, tool hashes, action SHAs); it neither compiles nor tests product
# code. Pass --code to also run Rust library tests and the web typecheck.
set -euo pipefail

run_code=0
for argument in "$@"; do
  case "$argument" in
    --code) run_code=1 ;;
    *) printf 'Usage: %s [--code]\n' "$0" >&2; exit 2 ;;
  esac
done

RECOLLECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$RECOLLECT_ROOT/docs/tools/lint_agent_governance.sh"
python3 -B -m unittest discover -s "$RECOLLECT_ROOT/docs/tools/tests" -p 'test_*.py' -v
python3 -B "$RECOLLECT_ROOT/scripts/memory_support_baseline.py" check
python3 -B -m unittest discover -s "$RECOLLECT_ROOT/scripts/tests" -p 'test_memory_support_baseline.py' -v
python3 -B "$RECOLLECT_ROOT/scripts/check-pins.py"
python3 -B -m unittest discover -s "$RECOLLECT_ROOT/scripts/tests" -p 'test_check_pins.py' -v
python3 -B -m unittest discover -s "$RECOLLECT_ROOT/scripts/tests" -p 'test_download_pins.py' -v

if [[ "$run_code" -eq 1 ]]; then
  (cd "$RECOLLECT_ROOT" && cargo test --workspace --lib)
  npm --prefix "$RECOLLECT_ROOT/web" run typecheck
fi
