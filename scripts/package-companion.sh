#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$#" != 1 ]]; then
  printf 'Usage: ./scripts/package-companion.sh .cache/packages/NAME\n' >&2
  exit 1
fi
python3 - <<'PY'
import shutil
if shutil.disk_usage('.').free < 12 * 1024**3:
    raise SystemExit('The native release build requires at least 12 GiB free; no data or caches were removed.')
PY
RECOLLECT_PACKAGE_DIR="$(python3 - "$1" <<'PY'
from pathlib import Path
import sys
root, path = Path.cwd().resolve(), Path(sys.argv[1]).resolve()
if root not in path.parents or path.exists():
    raise SystemExit('Choose a new output directory inside Recollect; existing bundles are preserved.')
print(path)
PY
)"
cargo build --locked --release -p recollect-agent --bins
mkdir -p "$RECOLLECT_PACKAGE_DIR"
for binary in recollect-agent recollect-mcp-runner recollect-mcp-bridge; do
  install -m 755 "target/release/$binary" "$RECOLLECT_PACKAGE_DIR/$binary"
done
rustc -vV > "$RECOLLECT_PACKAGE_DIR/build-host.txt"
printf 'Native companion and both sibling helpers packaged in %s\n' "$RECOLLECT_PACKAGE_DIR"
