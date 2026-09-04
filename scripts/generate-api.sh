#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .cache
cargo run -q -p recollect-server -- openapi > .cache/openapi.json
python3 - <<'PY'
import collections, json
with open('.cache/openapi.json') as source:
    schema = json.load(source)
ids = [op['operationId'] for path in schema['paths'].values()
       for op in path.values() if isinstance(op, dict) and 'operationId' in op]
duplicates = [name for name, count in collections.Counter(ids).items() if count > 1]
if duplicates:
    raise SystemExit('Duplicate OpenAPI operation IDs: ' + ', '.join(duplicates))
print(f'Validated {len(ids)} unique API operations')
PY
npm --prefix web run generate
