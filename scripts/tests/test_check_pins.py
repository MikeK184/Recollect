"""Offline supply-chain pin checker: accepted forms, rejected forms and the repository itself."""
import contextlib
import importlib.util
import io
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('check_pins', ROOT / 'scripts' / 'check-pins.py')
pins = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pins)

D = 'sha256:' + 'a' * 64
SHA = 'd23441a48e516b6c34aea4fa41551a30e30af803'


def messages(problems):
    return [message for _, message in problems]


class DockerfileTests(unittest.TestCase):
    def test_pinned_local_alias_and_scratch_references_pass(self):
        checked, problems = pins.check_dockerfile(f'''# comment
FROM rust:1.94-slim-bookworm@{D} AS server
FROM --platform=linux/amd64 localhost:5000/team/node:22@{D} AS web
FROM scratch AS empty
FROM server AS again
FROM recollect-capture-hosts:local
FROM ${{BASE:-recollect-base:local}}
COPY --from=server /a /b
COPY --from=0 /a /b
COPY --from=again /a /b
''')
        self.assertEqual(problems, [])
        self.assertEqual(checked, 4)

    def test_tag_only_tagless_and_malformed_pins_fail(self):
        _, problems = pins.check_dockerfile(
            f'FROM ubuntu:24.04\nFROM node@{D}\nFROM debian:12@sha256:abc\nFROM ubuntu:24.04@{D.upper()}\nFROM ${{BASE}}\n')
        self.assertEqual([number for number, _ in problems], [1, 2, 3, 4, 5])
        self.assertIn('no @sha256 digest pin', problems[0][1])
        self.assertIn('tag missing or digest malformed', problems[1][1])

    def test_continuation_and_flags_still_find_the_image(self):
        _, problems = pins.check_dockerfile('FROM \\\n  --platform=linux/arm64 \\\n  alpine:3 AS x\nRUN true\n')
        self.assertEqual(problems and problems[0][0], 1)

    def test_copy_from_external_image_must_be_pinned(self):
        _, problems = pins.check_dockerfile(f'FROM a:1@{D}\nCOPY --from=tool:1 /x /x\nCOPY --from=tool:1@{D} /y /y\n')
        self.assertEqual([number for number, _ in problems], [2])


class YamlTests(unittest.TestCase):
    def test_image_values_follow_the_same_rule(self):
        text = (f'services:\n  a:\n    image: pgvector/pgvector:pg17@{D}\n  b:\n    image: "neo4j:2026.08@{D}"  # c\n'
                '  c:\n    image: ${RECOLLECT_IMAGE:-recollect-product:local}\n  d:\n    image: recollect-dev:local\n'
                '  e:\n    image: neo4j:2026.08\n  f:\n    image: caddy\n  g:\n    # image: ignored\n    container:\n      image: ${X}\n')
        checked, problems = pins.check_yaml(text, actions=False)
        self.assertEqual([number for number, _ in problems], [11, 13, 17])
        self.assertEqual(checked, 7)

    def test_uses_requires_a_full_commit_sha_only_for_actions_files(self):
        text = (f'steps:\n  - uses: actions/checkout@{SHA} # v6\n  - uses: actions/setup-node@v6\n'
                f'  - uses: owner/repo/sub/path@{SHA}\n  - uses: owner/repo@main\n  - uses: owner/repo@{SHA[:7]}\n'
                '  - uses: ./local-action\n  - uses: docker://alpine:3\n  - uses: "${{ matrix.action }}"\n')
        _, problems = pins.check_yaml(text, actions=True)
        self.assertEqual([number for number, _ in problems], [3, 5, 6, 8, 9])
        _, problems = pins.check_yaml(text, actions=False)
        self.assertEqual(problems, [])


class ScriptTests(unittest.TestCase):
    pattern = pins.script_pattern(pins.THIRD_PARTY_IMAGES)

    def test_known_images_need_a_digest(self):
        checked, problems = pins.check_script(
            f'docker run pgvector/pgvector:pg17@{D}\ndocker run neo4j:2026.08\n'
            'image="docker.io/library/ubuntu:24.04"\nx=ghcr.io/dexidp/dex:v2.45.1 dex\n', self.pattern)
        self.assertEqual([number for number, _ in problems], [2, 3, 4])
        self.assertEqual(checked, 4)

    def test_hosts_urls_paths_and_node_modules_are_not_images(self):
        checked, problems = pins.check_script(
            'url = "http://neo4j:7474"\nDB=postgres://u@postgres:5432/x\nimport fs from "node:fs"\n'
            'cd ./node:22\nname = "recollect-pgvector:local"\nauth = "neo4j:" + password\n', self.pattern)
        self.assertEqual((checked, problems), (0, []))

    def test_dockerfile_and_yaml_names_extend_the_known_images(self):
        names = pins.collect_names([('dockerfile', f'FROM golang:1.23@{D} AS build\nFROM build\nFROM x:local\n'),
                                    ('yaml', f'image: ghcr.io/acme/tool:1@{D}\nimage: ${{V}}\n')])
        self.assertEqual(names, {'golang', 'ghcr.io/acme/tool'})
        pattern = pins.script_pattern(names)
        _, problems = pins.check_script('docker run golang:1.23\n', pattern)
        self.assertEqual(len(problems), 1)


class TreeTests(unittest.TestCase):
    def build(self, root, files):
        for name, text in files.items():
            path = Path(root, name)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)

    def test_tree_reports_each_rule_and_ignores_pruned_and_test_paths(self):
        with tempfile.TemporaryDirectory() as temp:
            self.build(temp, {
                'infra/app/Dockerfile': 'FROM ubuntu:24.04\n',
                'infra/compose.yaml': 'services:\n  a:\n    image: neo4j:5\n',
                '.github/workflows/ci.yml': 'jobs:\n  a:\n    steps:\n      - uses: actions/checkout@v4\n',
                'scripts/start.sh': 'docker run pgvector/pgvector:pg17\n',
                'scripts/ok.sh': f'docker run pgvector/pgvector:pg17@{D}\n',
                'scripts/tests/fixture.py': 'docker run pgvector/pgvector:pg17\n',
                'node_modules/pkg/Dockerfile': 'FROM ubuntu:24.04\n',
                '.cache/x/Dockerfile': 'FROM ubuntu:24.04\n',
                'docs/readme.md': 'FROM ubuntu:24.04\n',
            })
            counts, findings = pins.check_tree(temp)
        self.assertEqual(sorted((path, line) for path, line, _ in findings),
                         [('.github/workflows/ci.yml', 4), ('infra/app/Dockerfile', 1),
                          ('infra/compose.yaml', 3), ('scripts/start.sh', 1)])
        self.assertEqual(counts, {'dockerfile': 1, 'yaml': 2, 'script': 2})

    @unittest.skipUnless(shutil.which('git'), 'git is required')
    def test_git_listing_skips_ignored_files_and_includes_untracked_ones(self):
        with tempfile.TemporaryDirectory() as temp:
            subprocess.run(['git', 'init', '-q', temp], check=True)
            self.build(temp, {'.gitignore': 'ignored/\n', 'ignored/Dockerfile': 'FROM ubuntu:24.04\n',
                              'new/Dockerfile': 'FROM ubuntu:24.04\n'})
            _, findings = pins.check_tree(temp)
        self.assertEqual([path for path, _, _ in findings], ['new/Dockerfile'])

    def test_command_line_exit_status(self):
        with tempfile.TemporaryDirectory() as temp:
            self.build(temp, {'infra/Dockerfile': f'FROM ubuntu:24.04@{D}\n'})
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                self.assertEqual(pins.main(['--root', temp]), 0)
            self.assertIn('check-pins: OK', out.getvalue())
            self.build(temp, {'infra/Dockerfile': 'FROM ubuntu:24.04\n'})
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                self.assertEqual(pins.main(['--root', temp]), 1)
            self.assertIn('infra/Dockerfile:1:', err.getvalue())

    def test_repository_references_are_pinned(self):
        counts, findings = pins.check_tree(ROOT)
        self.assertEqual(findings, [])
        self.assertGreater(counts['dockerfile'], 0)
        self.assertGreater(counts['yaml'], 0)
        self.assertGreater(counts['script'], 0)


if __name__ == '__main__':
    unittest.main()
