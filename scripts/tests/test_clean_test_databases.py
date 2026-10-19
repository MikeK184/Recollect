"""Only databases the test harnesses create may ever be listed or dropped."""
import contextlib
import importlib.util
import io
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('clean_test_databases', ROOT / 'scripts' / 'clean-test-databases.py')
cleaner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cleaner)

HEX = '0123456789abcdef0123456789abcdef'


class NameFilterTests(unittest.TestCase):
    def test_accepts_exactly_the_harness_names(self):
        self.assertTrue(cleaner.is_owned(f'recollect_test_{HEX}'))
        self.assertTrue(cleaner.is_owned(f'recollect_ui_{HEX}'))

    def test_rejects_the_installed_brain_and_other_fixtures(self):
        for name in ['recollect', 'postgres', 'template1', 'recollect_admin',
                     f'recollect_test_brain_del_{HEX}', f'recollect_lease_rehearsal_{HEX}',
                     f'recollect_prod_{HEX}', f'my_recollect_test_{HEX}']:
            self.assertFalse(cleaner.is_owned(name), name)

    def test_rejects_malformed_suffixes(self):
        for suffix in [HEX[:-1], HEX + '0', HEX.upper(), HEX[:-1] + 'g', '', f'{HEX}_extra', f' {HEX}']:
            self.assertFalse(cleaner.is_owned('recollect_test_' + suffix), suffix)
            self.assertFalse(cleaner.is_owned('recollect_ui_' + suffix), suffix)

    def test_rejects_trailing_newline_and_injection(self):
        for name in [f'recollect_test_{HEX}\n', f'recollect_test_{HEX};DROP DATABASE recollect',
                     f'recollect_test_{HEX}"', f'"recollect_test_{HEX}', None, 5]:
            self.assertFalse(cleaner.is_owned(name), repr(name))

    def test_owned_names_filters_sorts_and_deduplicates(self):
        other = 'f' * 32
        names = [f'recollect_ui_{HEX}', 'recollect', f'recollect_test_{other}',
                 f'recollect_ui_{HEX}', f'recollect_test_{HEX}\n']
        self.assertEqual(cleaner.owned_names(names),
                         [f'recollect_test_{other}', f'recollect_ui_{HEX}'])


class DropTests(unittest.TestCase):
    def test_drop_never_sends_a_foreign_name_to_the_server(self):
        sent = []

        def fake(arguments, stdin=None):
            sent.append(stdin)
            return mock.Mock(returncode=0, stderr='', stdout='')

        with mock.patch.object(cleaner, 'psql', fake):
            dropped, failed = cleaner.drop([f'recollect_test_{HEX}', 'recollect', f'recollect_ui_{HEX}\n'])
        self.assertEqual((dropped, failed), ([f'recollect_test_{HEX}'], []))
        script = ''.join(sent)
        self.assertIn(f'DROP DATABASE IF EXISTS "recollect_test_{HEX}";', script)
        self.assertNotIn('"recollect"', script)
        self.assertNotIn('FORCE', script)

    def test_statement_failures_are_reported_not_hidden(self):
        busy = f'recollect_test_{HEX}'
        quiet = f'recollect_ui_{HEX}'
        error = f'ERROR:  database "{busy}" is being accessed by other users'
        with mock.patch.object(cleaner, 'psql', lambda a, stdin=None: mock.Mock(returncode=0, stderr=error, stdout='')):
            dropped, failed = cleaner.drop([busy, quiet])
        self.assertEqual((dropped, failed), ([quiet], [busy]))

    def test_dry_run_drops_nothing(self):
        output = io.StringIO()
        with mock.patch.object(cleaner, 'listing', return_value=([f'recollect_test_{HEX}'], [])), \
                mock.patch.object(cleaner, 'drop') as drop, contextlib.redirect_stdout(output):
            self.assertEqual(cleaner.main([]), 0)
        drop.assert_not_called()
        self.assertIn('would drop: ' + f'recollect_test_{HEX}', output.getvalue())


if __name__ == '__main__':
    unittest.main()
