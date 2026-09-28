#!/usr/bin/env python3
"""Focused setup-preservation and resolved Compose boundary checks; no live writes."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent


class RootCompose(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=ROOT / '.cache', prefix='root-compose-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'scripts').mkdir()
        shutil.copy(ROOT / 'scripts/setup-local.sh', self.root / 'scripts/setup-local.sh')
        shutil.copy(ROOT / 'compose.yaml', self.root / 'compose.yaml')

    def setup_local(self):
        return subprocess.run(['bash', str(self.root / 'scripts/setup-local.sh')],
                              capture_output=True, text=True, check=True)

    def test_setup_preserves_existing_credentials_and_is_repeatable(self):
        self.setup_local()
        env = self.root / '.env'
        original = env.read_bytes()
        self.setup_local()
        self.assertEqual(env.read_bytes(), original)
        self.assertEqual(env.stat().st_mode & 0o777, 0o600)
        self.assertTrue((self.root / '.data/artifacts').is_dir())
        self.assertTrue((self.root / '.data/erasure-journal').is_dir())

    def test_legacy_account_file_survives_and_conflicts_are_refused(self):
        (self.root / '.data').mkdir()
        old = self.root / '.data/credentials.json'
        content = json.dumps({'fixture-account': 'synthetic-test-value'})
        old.write_text(content)
        self.setup_local()
        new = self.root / '.data/runtime/credentials.json'
        self.assertEqual(new.read_text(), content)
        self.assertEqual(old.read_text(), content)
        self.assertEqual(new.stat().st_mode & 0o777, 0o600)
        env = self.root / '.env'
        env.write_text(env.read_text().replace('.data/runtime/credentials.json', '.data/credentials.json'))
        new.write_text('{}')
        with self.assertRaises(subprocess.CalledProcessError):
            self.setup_local()
        self.assertEqual(old.read_text(), content)
        self.assertIn('RECOLLECT_CREDENTIAL_FILE=.data/credentials.json', env.read_text())

    def test_compose_is_one_preserving_stack_with_runtime_secret_boundaries(self):
        self.setup_local()
        env = {k: v for k, v in os.environ.items() if k in
               ('PATH', 'HOME', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH')}
        # A root credential in the host must never enter the application model.
        env['VAULT_TOKEN'] = 'synthetic-forbidden-input'
        result = subprocess.run([str(ROOT / 'scripts/docker.sh'), 'compose',
                                 '--project-directory', str(self.root),
                                 '--env-file', str(self.root / '.env'),
                                 '-f', str(self.root / 'compose.yaml'), 'config', '--format', 'json'],
                                env=env, capture_output=True, text=True, check=True)
        config = json.loads(result.stdout)
        self.assertEqual(config['name'], 'recollect')
        services = config['services']
        self.assertEqual(set(services), {'postgres', 'neo4j', 'migrate', 'api', 'worker'})
        self.assertEqual(config['volumes']['postgres_data']['name'], 'recollect_postgres_data')
        for role in ('api', 'worker'):
            service = services[role]
            runtime = service['environment']
            for forbidden in ('DATABASE_ADMIN_URL', 'POSTGRES_PASSWORD', 'RECOLLECT_DB_PASSWORD', 'VAULT_TOKEN'):
                self.assertNotIn(forbidden, runtime)
            self.assertIsNone(runtime.get('RECOLLECT_OIDC_ISSUER'))
            self.assertEqual(service['depends_on']['migrate']['condition'], 'service_completed_successfully')
            self.assertEqual(service['depends_on']['neo4j']['condition'], 'service_healthy')
            sources = {mount['source'] for mount in service['volumes']}
            self.assertEqual(sources, {str(self.root / '.data' / suffix)
                                      for suffix in ('runtime', 'artifacts', 'erasure-journal')})
            self.assertGreater(int(service['mem_limit']), 0)
            self.assertEqual(service['stop_grace_period'], '1m30s')
        self.assertIn('DATABASE_ADMIN_URL', services['migrate']['environment'])
        self.assertEqual(services['api']['ports'][0]['host_ip'], '127.0.0.1')
        self.assertFalse(services['worker'].get('ports'))


if __name__ == '__main__':
    unittest.main(verbosity=2)
