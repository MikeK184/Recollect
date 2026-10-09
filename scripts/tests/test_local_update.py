"""Local update ownership and CLI safety controls; no live update or model calls."""
import contextlib
import importlib.util
import io
from pathlib import Path
import sys
import tempfile
import subprocess
import os
import json
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import root_installation
import install

spec = importlib.util.spec_from_file_location('local_update', Path(__file__).resolve().parents[1] / 'update-local.py')
update = importlib.util.module_from_spec(spec)
spec.loader.exec_module(update)


class LocalUpdateProof(unittest.TestCase):
    def test_stack_targets_owned_root_despite_foreign_compose_overrides(self):
        with tempfile.TemporaryDirectory(dir=install.ROOT / '.cache') as temp:
            root = Path(temp)
            scripts = root / 'scripts'
            scripts.mkdir()
            stack = scripts / 'stack.sh'
            stack.write_text((install.ROOT / 'scripts/stack.sh').read_text())
            for name in ('setup-local.sh', 'check-local-port.sh'):
                (scripts / name).write_text('#!/bin/sh\nexit 0\n')
            (scripts / 'docker.sh').write_text('#!/usr/bin/env python3\nimport json,sys\nfrom pathlib import Path\nwith Path("commands.jsonl").open("a") as output: output.write(json.dumps(sys.argv[1:])+"\\n")\n')
            for script in scripts.iterdir():
                script.chmod(0o700)
            (root / '.env').write_text('RECOLLECT_CREDENTIAL_FILE=.data/runtime/credentials.json\nRECOLLECT_PUBLIC_ORIGIN=http://127.0.0.1:8787\nCOMPOSE_PROJECT_NAME=foreign\nCOMPOSE_FILE=/foreign.yaml\n')
            (root / 'compose.yaml').write_text('name: recollect\nservices: {}\n')
            env = {k:v for k,v in os.environ.items() if not k.startswith('RECOLLECT_')}
            env.update({'COMPOSE_PROJECT_NAME':'neighbor','COMPOSE_FILE':'/neighbor.yaml'})
            result = subprocess.run([str(stack), 'up'], env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            commands = [json.loads(line) for line in (root / 'commands.jsonl').read_text().splitlines()]
            self.assertEqual(len(commands), 4)
            for command in commands:
                self.assertEqual(command[:9], ['compose','--project-directory',str(root),'--project-name','recollect','--env-file',str(root/'.env'),'-f',str(root/'compose.yaml')])
                self.assertNotIn('foreign', command)
                self.assertNotIn('neighbor', command)

    def test_help_has_no_checkpoint_or_build_effects(self):
        with patch.object(sys, 'argv', ['update-local.py', '--help']), \
                patch.object(root_installation, 'register') as register, \
                patch.object(update, 'checkpoint') as checkpoint, \
                contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(SystemExit) as exited:
                update.main()
            self.assertEqual(exited.exception.code, 0)
            register.assert_not_called()
            checkpoint.assert_not_called()

    def test_failed_checkpoint_prevents_build(self):
        with patch.object(sys, 'argv', ['update-local.py']), \
                patch.object(update, 'update_lock', return_value=contextlib.nullcontext()), \
                patch.object(root_installation, 'register', return_value={}), \
                patch.object(update, 'checkpoint', side_effect=ValueError('fixture failure')), \
                patch.object(update.subprocess, 'run') as command, \
                contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(update.main(), 1)
            command.assert_not_called()

    def test_concurrent_update_is_rejected(self):
        with tempfile.TemporaryDirectory(dir=install.ROOT / '.cache') as temp, \
                patch.object(install, 'ROOT', Path(temp)):
            with update.update_lock():
                with self.assertRaises(ValueError):
                    with update.update_lock():
                        self.fail('Concurrent update acquired the same lock')
            with update.update_lock():
                pass

    def test_ready_old_build_or_schema_cannot_complete_update(self):
        expected = {'revision':'new-dirty','built_at':'2026-10-08T12:00:00Z'}
        for ready in [
            {'ready':True,'schema_current':True,'build':{'revision':'old','built_at':'old'},'migration':'041_openrouter_provider'},
            {'ready':True,'schema_current':True,'build':expected,'migration':'035_plugin_workflow'},
            {'ready':True,'schema_current':False,'build':expected,'migration':'041_openrouter_provider'},
        ]:
            with self.subTest(ready=ready), \
                    patch.object(update.urllib.request, 'urlopen', return_value=contextlib.nullcontext(io.StringIO(__import__('json').dumps(ready)))), \
                    patch.object(update.subprocess, 'run') as command:
                with self.assertRaises(ValueError):
                    update.verify_runtime(expected)
                command.assert_not_called()

    def test_root_identity_cannot_adopt_a_foreign_project_or_image(self):
        good = {'name':root_installation.NAME,'project':'recollect','workspace':str(install.ROOT),
                'root':True,'mode':'personal','origin':'http://127.0.0.1:8787','checkpoint_image':'sha256:'+'a'*64}
        root_installation.validate(good)
        for field,value in [('project','neighbor'),('workspace','/tmp/other'),('checkpoint_image','latest'),('root',False)]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                root_installation.validate({**good,field:value})


if __name__ == '__main__':
    unittest.main()
