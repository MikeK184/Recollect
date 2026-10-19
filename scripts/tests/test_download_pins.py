"""Pinned SHA-256 verification for downloaded developer tools (age, Enola, rust-analyzer).

No network: urlopen and curl are replaced, and every archive here is synthetic. The
property under test is that a mismatching download is rejected before anything is
extracted or executed, and that a matching one still installs.
"""
import gzip
import hashlib
import importlib.util
import io
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
ROOT = SCRIPTS.parent
HEX64 = re.compile(r'^[0-9a-f]{64}$')


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


tools = load('setup_recovery_tools', 'setup-recovery-tools.py')
enola = load('setup_enola', 'setup-enola.py')


def tarball(members):
    """Gzip tarball of {name: (bytes, mode)}."""
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode='w:gz') as archive:
        for name, (data, mode) in members.items():
            info = tarfile.TarInfo(name)
            info.size, info.mode = len(data), mode
            archive.addfile(info, io.BytesIO(data))
    return buffer.getvalue()


def script_printing(text):
    return f'#!/bin/sh\nprintf "%s\\n" \'{text}\'\n'.encode()


class PinTableTests(unittest.TestCase):
    def test_tables_are_complete_well_formed_hex(self):
        platforms = {('darwin', 'amd64'), ('darwin', 'arm64'), ('linux', 'amd64'), ('linux', 'arm64')}
        for table in (tools.SHA256, enola.SHA256):
            self.assertEqual(set(table), platforms)
            for value in table.values():
                self.assertRegex(value, HEX64)
        self.assertEqual(len(set(tools.SHA256.values())), 4)
        self.assertEqual(len(set(enola.SHA256.values())), 4)

    def test_age_pins_match_the_product_dockerfile(self):
        dockerfile = (ROOT / 'infra/product/Dockerfile').read_text()
        self.assertIn(f'ARG AGE_VERSION=v{tools.VERSION}\n', dockerfile)
        for arch, key in (('AMD64', 'amd64'), ('ARM64', 'arm64')):
            self.assertIn(f'ARG AGE_SHA256_{arch}={tools.SHA256[("linux", key)]}\n', dockerfile)

    def test_age_version_matches_the_installed_tool_glob(self):
        self.assertIn(f'age-v{tools.VERSION}-*/age', (SCRIPTS / 'update-local.py').read_text())
        self.assertIn(f'age-v{tools.VERSION}-', (ROOT / 'docs/runbooks/recovery.md').read_text())

    def test_rust_analyzer_pin_is_well_formed(self):
        script = (SCRIPTS / 'setup-rust-navigation.sh').read_text()
        self.assertRegex(script, r'(?m)^RECOLLECT_RUST_ANALYZER_SHA256=[0-9a-f]{64}$')


class AgeSetupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        for patcher in (patch.object(tools, 'ROOT', self.root),
                        patch.object(tools.platform, 'system', return_value='Darwin'),
                        patch.object(tools.platform, 'machine', return_value='arm64')):
            patcher.start()
            self.addCleanup(patcher.stop)

    def cache(self):
        return self.root / '.cache' / 'recovery-tools'

    def test_mismatching_download_is_rejected_before_extraction_or_execution(self):
        urls = []
        archive = tarball({'age/age': (script_printing('v1.3.2'), 0o755),
                           'age/age-keygen': (script_printing('v1.3.2'), 0o755)})

        def serve(url, timeout):
            urls.append(url)
            return io.BytesIO(archive)

        with patch.object(tools.urllib.request, 'urlopen', serve), \
                patch.object(tools.tarfile, 'open', side_effect=AssertionError('extracted')), \
                patch.object(tools.subprocess, 'check_output', side_effect=AssertionError('executed')):
            with self.assertRaises(tools.ChecksumError):
                tools.install()
        self.assertEqual(urls, ['https://github.com/FiloSottile/age/releases/download/v1.3.2/'
                                'age-v1.3.2-darwin-arm64.tar.gz'])
        self.assertEqual(list(self.cache().iterdir()), [])

    def test_oversized_download_is_rejected(self):
        with patch.object(tools, 'LIMIT', 8), \
                patch.object(tools.urllib.request, 'urlopen', lambda url, timeout: io.BytesIO(b'x' * 64)), \
                patch.object(tools.tarfile, 'open', side_effect=AssertionError('extracted')):
            with self.assertRaisesRegex(ValueError, 'size bound'):
                tools.install()
        self.assertEqual(list(self.cache().iterdir()), [])

    def test_matching_download_installs_both_tools(self):
        archive = tarball({'age/age': (script_printing('v1.3.2'), 0o755),
                           'age/age-keygen': (script_printing('v1.3.2'), 0o755)})
        table = {**tools.SHA256, ('darwin', 'arm64'): hashlib.sha256(archive).hexdigest()}
        with patch.object(tools, 'SHA256', table), \
                patch.object(tools.urllib.request, 'urlopen', lambda url, timeout: io.BytesIO(archive)):
            location = tools.install()
        self.assertEqual(location, self.cache() / 'age-v1.3.2-darwin-arm64')
        for name in ('age', 'age-keygen'):
            self.assertTrue(os.access(location / name, os.X_OK))


class EnolaSetupTests(unittest.TestCase):
    asset = f'enola-{enola.VERSION}-linux-amd64'

    def archive(self):
        manifest = b'{"version":"0.4.19","extractor_version":"fixture"}'
        return tarball({self.asset: (b"#!/bin/sh\nprintf '%s' '" + manifest + b"'\n", 0o755),
                        'LICENSE': (b'license', 0o644), 'NOTICE': (b'notice', 0o644)})

    def test_mismatching_download_is_rejected_before_extraction_or_execution(self):
        archive = self.archive()
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            with patch.object(enola.urllib.request, 'urlopen', lambda request, timeout: io.BytesIO(archive)), \
                    patch.object(enola.tarfile, 'open', side_effect=AssertionError('extracted')), \
                    patch.object(enola.subprocess, 'run', side_effect=AssertionError('executed')):
                with self.assertRaisesRegex(SystemExit, 'pinned SHA-256'):
                    enola.install(folder, 'linux', 'amd64')
            self.assertEqual(list(folder.iterdir()), [])

    def test_requested_url_is_the_release_asset(self):
        requested = []

        def serve(request, timeout):
            requested.append(request.full_url)
            return io.BytesIO(b'')

        with tempfile.TemporaryDirectory() as temp, patch.object(enola.urllib.request, 'urlopen', serve):
            with self.assertRaises(SystemExit):
                enola.install(Path(temp), 'darwin', 'arm64')
        self.assertEqual(requested, [f'https://github.com/enola-labs/enola/releases/download/v{enola.VERSION}/'
                                     f'enola-{enola.VERSION}-darwin-arm64.tar.gz'])

    def test_matching_download_installs_and_reports_version(self):
        archive = self.archive()
        table = {**enola.SHA256, ('linux', 'amd64'): hashlib.sha256(archive).hexdigest()}
        with tempfile.TemporaryDirectory() as temp, patch.object(enola, 'SHA256', table), \
                patch.object(enola.urllib.request, 'urlopen', lambda request, timeout: io.BytesIO(archive)):
            result = enola.install(Path(temp), 'linux', 'amd64')
            self.assertEqual(result['version'], '0.4.19')
            self.assertEqual(stat.S_IMODE((Path(temp) / 'enola').stat().st_mode), 0o700)
            self.assertEqual((Path(temp) / 'LICENSE').read_bytes(), b'license')


@unittest.skipUnless(shutil.which('bash') and shutil.which('shasum') and shutil.which('gzip'),
                     'bash, shasum and gzip are required')
class RustAnalyzerSetupTests(unittest.TestCase):
    """Run a copy of setup-rust-navigation.sh with shimmed curl, uname and toolchain."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        (self.root / 'scripts').mkdir()
        self.shims = self.root / 'shims'
        self.shims.mkdir()
        self.script = self.root / 'scripts' / 'setup-rust-navigation.sh'
        self.source = (SCRIPTS / 'setup-rust-navigation.sh').read_text()

    def shim(self, name, body):
        path = self.shims / name
        path.write_text('#!/bin/sh\n' + body)
        path.chmod(0o755)

    def run_script(self, served, pinned):
        """Serve `served` as the download; pin `pinned` (None keeps the repository's value)."""
        (self.root / 'served.bin').write_bytes(served)
        source = self.source
        if pinned:
            source = re.sub(r'(?m)^RECOLLECT_RUST_ANALYZER_SHA256=.*$',
                            'RECOLLECT_RUST_ANALYZER_SHA256=' + pinned, source)
        self.script.write_text(source)
        self.shim('uname', 'case "$1" in -s) echo Darwin;; -m) echo arm64;; esac\n')
        self.shim('curl', 'while [ $# -gt 0 ]; do if [ "$1" = --output ]; then out="$2"; fi; shift; done\n'
                          f'cp "{self.root}/served.bin" "$out"\n')
        self.shim('cargo', 'echo cargo reached >&2; exit 1\n')
        for name in ('rustc', 'node', 'npm'):
            self.shim(name, 'exit 0\n')
        env = {'PATH': f'{self.shims}:/usr/bin:/bin', 'HOME': str(self.root)}
        return subprocess.run(['bash', str(self.script)], env=env, capture_output=True, text=True, timeout=60)

    def installed(self):
        return self.root / '.codex/tools/rust-analyzer/installed/bin'

    def test_mismatching_download_is_rejected_before_decompression(self):
        fake = gzip.compress(script_printing('rust-analyzer fake'))
        result = self.run_script(fake, None)
        self.assertEqual(result.returncode, 1)
        self.assertIn('pinned SHA-256', result.stderr)
        self.assertNotIn('cargo reached', result.stderr)
        self.assertEqual(list(self.installed().iterdir()), [])
        self.assertFalse((self.root / '.cache/rust-navigation/rust-analyzer.gz').exists())

    def test_matching_download_is_installed_before_the_build_steps(self):
        fake = gzip.compress(script_printing('rust-analyzer fake'))
        result = self.run_script(fake, hashlib.sha256(fake).hexdigest())
        self.assertIn('rust-analyzer fake', result.stdout)
        self.assertIn('cargo reached', result.stderr)
        self.assertTrue(os.access(self.installed() / 'rust-analyzer', os.X_OK))


if __name__ == '__main__':
    unittest.main()
