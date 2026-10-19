#!/usr/bin/env python3
"""List, and with --drop remove, disposable test databases that failed runs left behind.

The platform suite (crates/server/tests/platform.rs) names its databases
recollect_test_<32 hex> and scripts/test-ui.sh names its own recollect_ui_<32 hex>.
Both keep the database of a failed run for diagnosis and drop it on success, so
failures accumulate on the Compose PostgreSQL. Only those exact names are ever
listed or dropped. The installed Brain's database, other fixtures
(recollect_test_brain_del_*, recollect_lease_rehearsal_*) and anything else on the
server are never touched.

  python3 scripts/clean-test-databases.py          # dry run: list what --drop would remove
  python3 scripts/clean-test-databases.py --drop   # drop the idle ones

A database with a live session belongs to a running test, so it is listed as busy
and is never dropped (no FORCE); rerun after that run finishes. Stop concurrent
test runs first if you want everything gone.
"""
import argparse
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
# Same compose files ./scripts/setup-local.sh records in .env as COMPOSE_FILE.
DEFAULT_COMPOSE_FILE = 'infra/product/compose.yaml:compose.yaml'
OWNED = re.compile(r'recollect_(?:test|ui)_[0-9a-f]{32}')
# The same expression, evaluated by the server so the listing never transfers
# unrelated names. It is a pre-filter only: every name is re-checked with OWNED.
SQL_PATTERN = '^recollect_(test|ui)_[0-9a-f]{32}$'
BATCH = 50


def is_owned(name):
    """True only for a database this repository's test harnesses created.

    fullmatch, not a `$` anchor: a trailing newline must not slip through.
    """
    return isinstance(name, str) and OWNED.fullmatch(name) is not None


def owned_names(names):
    return sorted({name for name in names if is_owned(name)})


def psql(arguments, stdin=None):
    environment = dict(os.environ)
    environment.setdefault('COMPOSE_FILE', DEFAULT_COMPOSE_FILE)
    command = [str(ROOT / 'scripts/docker.sh'), 'compose', 'exec', '-T', 'postgres',
               'psql', '-X', '-U', 'recollect_admin', '-d', 'postgres', *arguments]
    return subprocess.run(command, cwd=ROOT, env=environment, input=stdin,
                          capture_output=True, text=True)


def listing():
    """Return (idle, busy) owned database names. Raises SystemExit if psql fails."""
    result = psql(['-Atq', '-F', '|', '-c', (
        'SELECT d.datname, (SELECT count(*) FROM pg_stat_activity a WHERE a.datname=d.datname) '
        f"FROM pg_database d WHERE d.datname ~ '{SQL_PATTERN}' ORDER BY d.datname")])
    if result.returncode != 0:
        reason = result.stderr.strip().splitlines()[-1:] or ['no error output']
        sys.exit(f'Could not list databases through ./scripts/docker.sh compose exec postgres: {reason[0]}')
    idle, busy = [], []
    for line in result.stdout.splitlines():
        name, _, sessions = line.partition('|')
        if not is_owned(name):
            continue
        (busy if sessions.strip() != '0' else idle).append(name)
    return idle, busy


def drop(names):
    """Drop each name without FORCE; return (dropped, failed)."""
    dropped, failed = [], []
    for start in range(0, len(names), BATCH):
        batch = [name for name in names[start:start + BATCH] if is_owned(name)]
        # Quoted although validated: the statement text never carries other input.
        script = ''.join(f'DROP DATABASE IF EXISTS "{name}";\n' for name in batch)
        result = psql(['-q', '-v', 'ON_ERROR_STOP=0'], stdin=script)
        failures = set(re.findall(r'database "(recollect_(?:test|ui)_[0-9a-f]{32})"', result.stderr))
        failed.extend(name for name in batch if name in failures)
        dropped.extend(name for name in batch if name not in failures)
    return dropped, failed


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    parser.add_argument('--drop', action='store_true',
                        help='drop the idle matching databases (default: only list them)')
    arguments = parser.parse_args(argv)
    idle, busy = listing()
    print(f'{len(idle)} idle and {len(busy)} busy disposable test databases '
          f'({SQL_PATTERN})')
    for name in busy:
        print(f'  busy (live session, kept): {name}')
    if not arguments.drop:
        for name in idle:
            print(f'  would drop: {name}')
        print('Dry run: nothing was dropped. Pass --drop to remove the idle databases.')
        return 0
    dropped, failed = drop(idle)
    print(f'Dropped {len(dropped)} databases; {len(failed)} could not be dropped.')
    for name in failed:
        print(f'  not dropped: {name}')
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
