#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - <<'PY'
from pathlib import Path
import secrets
p = Path('.env')
if not p.exists():
    db, app, owner, graph = (secrets.token_urlsafe(32) for _ in range(4))
    text = f'''POSTGRES_PASSWORD={db}
RECOLLECT_DB_PASSWORD={app}
DATABASE_ADMIN_URL=postgres://recollect_admin:{db}@127.0.0.1:55432/recollect
DATABASE_URL=postgres://recollect_app:{app}@127.0.0.1:55432/recollect
RECOLLECT_OWNER_USERNAME=owner
RECOLLECT_OWNER_PASSWORD={owner}
RECOLLECT_BIND=127.0.0.1:8787
RECOLLECT_PUBLIC_ORIGIN=http://127.0.0.1:8787
RECOLLECT_STATIC_DIR=web/dist
NEO4J_URL=http://127.0.0.1:57474
NEO4J_USERNAME=neo4j
NEO4J_PASSWORD={graph}
'''
    with p.open('x') as f:
        p.chmod(0o600)
        f.write(text)
    print('Created ignored .env. Owner login is stored there; existing credentials are never overwritten.')
else:
    print('Using existing .env; credentials unchanged.')
Path('.data').mkdir(exist_ok=True)
PY
