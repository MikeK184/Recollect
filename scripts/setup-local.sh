#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - <<'PY'
from pathlib import Path
import json
import os
import re
import secrets
import shlex
import shutil
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
# Share only the app's state, artifacts and journal with its containers, using
# the current user's UID/GID so native and container writes stay compatible.
text = p.read_text()
def value(key, default=None):
    match = re.search(r'^' + key + r'=(.*)$', text, re.MULTILINE)
    if not match:
        return default
    parts = shlex.split(match[1], comments=True)
    return parts[0] if parts else ''

runtime = Path('.data/runtime')
runtime.mkdir(exist_ok=True, mode=0o700)
for key, default in [('RECOLLECT_ARTIFACT_DIR', '.data/artifacts'),
                     ('RECOLLECT_ERASURE_JOURNAL', '.data/erasure-journal')]:
    Path(value(key, default)).mkdir(parents=True, exist_ok=True)
updates = {}
for key, current in [('RECOLLECT_LOCAL_UID', os.getuid()), ('RECOLLECT_LOCAL_GID', os.getgid())]:
    if value(key) is None:
        updates[key] = str(current)
# The root compose.yaml is an override of the shared product services; make plain
# `docker compose` commands load both files, as ./scripts/stack.sh does explicitly.
if value('COMPOSE_FILE') is None:
    updates['COMPOSE_FILE'] = 'infra/product/compose.yaml:compose.yaml'
credentials = value('RECOLLECT_CREDENTIAL_FILE', '.data/credentials.json')
if Path(credentials) == Path('.data/credentials.json'):
    old, new = Path(credentials), runtime / 'credentials.json'
    def same_accounts(a, b):
        # The API hashes a legacy file when it starts (ADR 0024), so a copy that
        # differs only by hashing the same credential IDs is not a conflict.
        if a.read_bytes() == b.read_bytes():
            return True
        try:
            left, right = json.loads(a.read_text()), json.loads(b.read_text())
        except ValueError:
            return False
        hashed = lambda value: isinstance(value, str) and value.startswith('$argon2id$')
        return (isinstance(left, dict) and isinstance(right, dict) and left.keys() == right.keys()
                and all(left[k] == right[k] or hashed(left[k]) or hashed(right[k]) for k in left))
    if old.exists() and new.exists() and not same_accounts(old, new):
        raise SystemExit('Both legacy and runtime account files exist with different contents; reconcile them before switching runtimes.')
    if old.exists() and not new.exists():
        shutil.copyfile(old, new)
        new.chmod(0o600)
    updates['RECOLLECT_CREDENTIAL_FILE'] = str(new)
for key, current in updates.items():
    if re.search(r'^' + key + '=', text, re.MULTILINE):
        text = re.sub(r'^' + key + r'=.*$', key + '=' + current, text, flags=re.MULTILINE)
    else:
        text = text.rstrip('\n') + '\n' + key + '=' + current + '\n'
if updates:
    p.write_text(text)
    p.chmod(0o600)
PY
