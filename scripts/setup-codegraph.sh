#!/usr/bin/env bash
set -euo pipefail
umask 077

RECOLLECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RECOLLECT_CODEGRAPH_TOOLS="$RECOLLECT_ROOT/.codex/tools/codegraph"

if ! command -v node >/dev/null || ! command -v npm >/dev/null; then
  printf '%s\n' 'CodeGraph setup requires Node.js and npm on PATH.' >&2
  exit 1
fi
if [[ ! -d "$RECOLLECT_ROOT/references/cognee/.git" && ! -f "$RECOLLECT_ROOT/references/cognee/.git" ]]; then
  printf '%s\n' 'Expected the separate Cognee Git checkout at Recollect/references/cognee/.' >&2
  exit 1
fi

export TMPDIR="$RECOLLECT_ROOT/.cache/codegraph/tmp"
mkdir -p "$TMPDIR"
npm ci --prefix "$RECOLLECT_CODEGRAPH_TOOLS" \
  --cache "$RECOLLECT_ROOT/.cache/npm" --registry=https://registry.npmjs.org \
  --ignore-scripts --include=optional --no-audit --no-fund

"$RECOLLECT_CODEGRAPH_TOOLS/codegraph" --version
if [[ -f "$RECOLLECT_ROOT/.codegraph/codegraph.db" ]]; then
  "$RECOLLECT_CODEGRAPH_TOOLS/codegraph" sync
else
  # CLI init starts no watcher. Avoid its optional Git-hook installation when
  # an inherited setting (or WSL drive) would normally disable MCP watching.
  CODEGRAPH_NO_WATCH=0 CODEGRAPH_FORCE_WATCH=1 \
    "$RECOLLECT_CODEGRAPH_TOOLS/codegraph" init --yes
fi
"$RECOLLECT_CODEGRAPH_TOOLS/codegraph" status
