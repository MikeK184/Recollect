"""Independent source-feed toys only. No private corpus, gold or provider reads."""

from contextlib import ExitStack, redirect_stderr, redirect_stdout
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import socket
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import urllib.request


ROOT = Path(__file__).resolve().parents[2]
PROOF_ROOT = ROOT / ".cache/automatic-mapping/source-feed-preparer-proof-2026-10-18"
sys.path.insert(0, str(ROOT / "scripts"))
import prepare_frozen_organization as preparer

# Reuse only independently authored toy fixture constructors, never old artifacts.
SPEC = importlib.util.spec_from_file_location("source_feed_independent_toys", ROOT / "scripts/tests/test_score_frozen_source_corpus.py")
toys = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(toys)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n").encode("utf-8")


class PreparationTests(unittest.TestCase):
    def setUp(self):
        PROOF_ROOT.mkdir(mode=0o700, parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="independent-toy-", dir=PROOF_ROOT)
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.out = self.directory / "prepared"
        self.corpus = toys.corpus_fixture()
        self.manifest_path = self.directory / "manifest.json"
        self.freeze_corpus()
        self.selection_path = self.directory / "selection.json"
        self.selection = {
            "schema": preparer.SELECTION_SCHEMA, "input_mode": "independent-toy-numeric-selection",
            "development_source_cohort_sha256": sha(b"independent-cohort"),
            "recipe_sha256": sha(b"independent-recipe"),
            "development_corpus_sha256": sha(b"independent-corpus"),
            "scorer_sha256": sha(b"independent-scorer"), "trials": 27, "selected": True,
            "parameters": dict(preparer.scorer.PARAMETERS), "rule": "independently authored toy declaration",
            "production_recipe_qualified": False,
            "development_result_sha256": [sha(f"independent-result-{index}".encode()) for index in range(27)],
        }
        self.freeze_path = self.directory / "candidate.json"
        source_hashes, migration_hashes = {}, {}
        for table, count, prefix in ((source_hashes, 198, "source"), (migration_hashes, 64, "migration")):
            for index in range(count):
                path = self.directory / f"toy-{prefix}-{index}"
                raw = f"independent {prefix} {index}\n".encode()
                path.write_bytes(raw)
                table[str(path.relative_to(ROOT))] = sha(raw)
        self.candidate = {
            "schema": preparer.FREEZE_SCHEMA, "additional_model_calls": 0, "amendment": "independent toy",
            "extractors": {
                "direct": ["source-facts-4", "source-facts-4", "learning-quote-anchors-3"],
                "reconciliation": ["source-reconciliation-4", "source-reconciliation-4", "learning-quote-anchors-3"],
                "dto": "source-mentions-1", "support_verifier": "source-support-3",
            },
            "grouping": {
                "algorithm": "leiden", "arms": list(preparer.scorer.MODES), "concurrency": 1,
                "gds_parameters": {"concurrency": 1, "gamma": 1.0, "maxLevels": 10, "randomSeed": 42,
                                   "theta": 0.01, "tolerance": 0.0001},
                "gds_version": "independent-toy", "label_policy": "independent toy", "new_grid_trials": 0,
                "parameters": dict(preparer.scorer.PARAMETERS), "random_seed": 42,
                "recipe": "independent-toy", "representation": "independent-toy",
                "selection_path": str(self.selection_path.relative_to(ROOT)), "selection_sha256": "",
            },
            "heldout_bindings": {"independent_content_free_binding": sha(b"unused-toy")},
            "migration_sha256": migration_hashes,
            "model_configuration": {
                "provider": "openai", "endpoint": "https://api.openai.com/v1", "text_model": "gpt-5.6-luna",
                "embedding_model": "text-embedding-3-large", "embedding_dimensions": 3072,
            },
            "private_corpus_bodies_opened_by_preparer": False, "purpose": "independent toy",
            "qualified": False, "recorded_at": "independent-toy", "release_catalogue_seeded": False,
            "source_sha256": source_hashes, "supersedes_predata_freeze_sha256": sha(b"independent-prior-freeze"),
        }
        self.write_pins()

    def freeze_corpus(self):
        toys.author_pairs(self.corpus)
        files = {
            "corpus": encode(self.corpus), "guide": b"Independent toy guide only.\n",
            "source_hashes": encode({"schema": "independent-toy-source-hashes", "documents": {
                document["id"]: sha(document["text"].encode()) for document in self.corpus["documents"]}}),
            "interface": encode(toys.interface_fixture()),
        }
        self.manifest = {"schema": preparer.loader.MANIFEST_SCHEMA, "corpus_schema": self.corpus["schema"]}
        for kind, raw in files.items():
            path = self.directory / f"{kind}.json"
            path.write_bytes(raw)
            self.manifest[f"{kind}_path"] = str(path.relative_to(ROOT))
            self.manifest[f"{kind}_sha256"] = sha(raw)
        self.manifest_path.write_bytes(encode(self.manifest))
        self.manifest_hash = sha(self.manifest_path.read_bytes())

    def write_pins(self):
        self.selection_path.write_bytes(encode(self.selection))
        selection_hash = sha(self.selection_path.read_bytes())
        self.candidate["grouping"]["selection_sha256"] = selection_hash
        self.freeze_path.write_bytes(encode(self.candidate))
        self.pins = preparer.Pins(sha(self.freeze_path.read_bytes()), selection_hash)

    def parameters(self):
        return {
            "manifest": self.manifest_path, "manifest_sha256": self.manifest_hash,
            "candidate_freeze": self.freeze_path, "candidate_freeze_sha256": self.pins.candidate_freeze_sha256,
            "development_selection": self.selection_path,
            "development_selection_sha256": self.pins.development_selection_sha256,
            "producer": "source-facts-4", "out": self.out,
        }

    def prepare(self, **changes):
        return preparer.prepare(**(self.parameters() | changes), pins=self.pins)

    def reject(self, **changes):
        with self.assertRaisesRegex(preparer.PreparationError, "^organization preparation failed$"):
            self.prepare(**changes)
        self.assertFalse(self.out.exists())

    def argv(self):
        return [part for name, value in self.parameters().items() for part in (f"--{name.replace('_', '-')}", str(value))]

    def output(self, directory=None):
        return {path.name: path.read_bytes() for path in (directory or self.out).iterdir()}

    def test_exact_source_only_feed_and_both_producer_families(self):
        loaded = preparer.loader.load_frozen_corpus(self.manifest_path, manifest_sha256=self.manifest_hash)
        for producer in preparer.PRODUCERS:
            out = self.directory / producer
            receipt = self.prepare(producer=producer, out=out)
            files = self.output(out)
            self.assertEqual(set(files), {"recipe.json", "source-feed.json", "preparation.json"})
            recipe, feed, preparation = [json.loads(files[name]) for name in ("recipe.json", "source-feed.json", "preparation.json")]
            self.assertEqual(preparer.scorer._recipe(recipe), recipe)
            self.assertEqual(recipe["producer"], {"prompt_label": producer, "schema_label": producer,
                                                "adapter_revision": "learning-quote-anchors-3", "mapping_schema": "source-mentions-1"})
            self.assertEqual(set(feed), {"schema", "corpus_bindings", "recipe_sha256", "sources"})
            self.assertEqual(feed["schema"], preparer.FEED_SCHEMA)
            self.assertEqual(feed["corpus_bindings"], loaded.file_sha256)
            self.assertEqual(feed["recipe_sha256"], sha(files["recipe.json"]))
            self.assertEqual(len(feed["sources"]), len(self.corpus["documents"]))
            for source, document in zip(feed["sources"], self.corpus["documents"]):
                self.assertEqual(set(source), {"document_id", "text_sha256", "partition", "language", "scope", "title", "text"})
                self.assertEqual(source, {"document_id": document["id"], "text_sha256": sha(document["text"].encode()),
                                          **{key: document[key] for key in ("partition", "language", "scope", "title", "text")}})
            self.assertEqual(preparation["counts"], loaded.summary()["counts"])
            self.assertEqual(preparation["complete_topic_gold_readiness"], "NOT_READY")
            self.assertEqual(preparation["not_measured"], preparer.scorer.UNMEASURED)
            self.assertEqual(set(preparation), {"schema", "input_sha256", "output_sha256", "counts", "complete_topic_gold_readiness", "not_measured"})
            expected_inputs = loaded.file_sha256 | {
                "candidate_freeze": self.pins.candidate_freeze_sha256, "development_selection": self.pins.development_selection_sha256,
                "preparer": sha(Path(preparer.__file__).read_bytes()), "loader": sha(Path(preparer.loader.__file__).read_bytes()),
                "scorer": sha(Path(preparer.scorer.__file__).read_bytes()),
            }
            self.assertEqual(preparation["input_sha256"], expected_inputs)
            self.assertEqual(preparation["output_sha256"], {"recipe": sha(files["recipe.json"]), "source_feed": sha(files["source-feed.json"])})
            self.assertEqual(receipt["output_sha256"]["preparation"], sha(files["preparation.json"]))
            for sentinel in (b"TOPIC_SENTINEL", b"METADATA_SENTINEL", b"PAIR_REASON_SENTINEL", b"LABEL_SENTINEL", b"UNKNOWN_REASON_SENTINEL"):
                self.assertNotIn(sentinel, b"".join(files.values()))
            self.assertNotIn(b'"model_configuration"', b"".join(files.values()))

    def test_empty_unknown_unsupported_development_and_all_250_sources_preserved(self):
        for index in range(6, 250):
            self.corpus["documents"].append(toys.document(f"toy-{index}", "" if index % 2 else "Toy trace."))
        self.freeze_corpus()
        receipt = self.prepare()
        feed = json.loads((self.out / "source-feed.json").read_bytes())
        self.assertEqual(len(feed["sources"]), 250)
        self.assertEqual(receipt["counts"]["topic_pairs"], 31125)
        self.assertEqual({source["document_id"] for source in feed["sources"]}, {source["id"] for source in self.corpus["documents"]})
        self.assertEqual(sum(source["text"] == "" for source in feed["sources"]), 123)
        self.assertTrue(any(source["language"] == "fr" for source in feed["sources"]))
        self.assertTrue(any(source["partition"] == "development" for source in feed["sources"]))
        self.assertGreater(receipt["counts"]["topic_pairs_unknown"], 0)

    def test_ready_gold_still_cannot_qualify_recipe(self):
        self.corpus["documents"] = self.corpus["documents"][:2]
        self.freeze_corpus()
        receipt = self.prepare()
        self.assertEqual(receipt["complete_topic_gold_readiness"], "READY")
        self.assertEqual(set(receipt["not_measured"].values()), {"NOT_MEASURED"})

    def test_deterministic_bytes_and_no_environment_configuration(self):
        first = self.prepare()
        before = self.output()
        with mock.patch.dict(os.environ, {"OPENAI_API_KEY": "ENV_SECRET_SENTINEL", "MODEL": "ENV_MODEL_SENTINEL"}):
            second = self.prepare(out=self.directory / "second")
        self.assertEqual(first, second)
        self.assertEqual(before, self.output(self.directory / "second"))
        self.assertNotIn(b"ENV_", b"".join(before.values()))

    def test_zero_network_subprocess_provider_or_database_calls(self):
        with ExitStack() as stack:
            for target in ("socket.socket", "socket.create_connection", "urllib.request.urlopen", "subprocess.Popen",
                           "subprocess.run", "os.system", "os.popen", "sqlite3.connect"):
                stack.enter_context(mock.patch(target, side_effect=AssertionError("external call forbidden")))
            self.prepare()

    def test_immutable_production_pins_and_original_hash_drift_reject_before_loader(self):
        self.assertEqual(preparer.PRODUCTION_PINS, preparer.Pins(
            "ffc4f4251823bde4060c2d1158482728b4913d955e508a1fef9b8cf0cb6c6431",
            "084c075eb5f0a300861e2c95cd6c75ad17506d2c2b49b342070dbab45457056d"))
        with mock.patch.object(preparer.loader, "load_frozen_corpus", side_effect=AssertionError("must not open corpus")):
            with self.assertRaises(preparer.PreparationError):
                preparer.prepare(**self.parameters())
            self.reject(candidate_freeze_sha256="0" * 64)
            self.reject(development_selection_sha256="0" * 64)
            self.freeze_path.write_bytes(self.freeze_path.read_bytes() + b" ")
            self.reject()

    def test_every_source_and_migration_pin_is_checked(self):
        for field in ("source_sha256", "migration_sha256"):
            table = self.candidate[field]
            for path in (next(iter(table)), next(reversed(table))):
                target = ROOT / path
                original = target.read_bytes()
                target.write_bytes(original + b"drift")
                self.reject()
                target.write_bytes(original)
        for field in ("source_sha256", "migration_sha256"):
            original = copy.deepcopy(self.candidate[field])
            self.candidate[field].pop(next(iter(original)))
            self.write_pins()
            self.reject()
            self.candidate[field] = original
        self.write_pins()
        self.prepare()

    def test_selection_file_hash_and_bound_selection_path_drift(self):
        self.selection_path.write_bytes(self.selection_path.read_bytes() + b" ")
        self.reject()
        self.write_pins()
        self.candidate["grouping"]["selection_path"] = str((self.directory / "other.json").relative_to(ROOT))
        self.write_pins()
        self.reject()

    def test_fixed_tuple_modes_and_both_exact_producer_families(self):
        mutations = [
            lambda: self.selection["parameters"].update(neighbors=4),
            lambda: self.selection["parameters"].update(neighbors=True),
            lambda: self.selection.update(selected=False),
            lambda: self.candidate["grouping"]["parameters"].update(minimum_cosine=0.5),
            lambda: self.candidate["grouping"]["arms"].reverse(),
            lambda: self.candidate["extractors"]["direct"].__setitem__(2, "learning-quote-anchors-2"),
            lambda: self.candidate["extractors"]["reconciliation"].__setitem__(0, "source-reconciliation-3"),
            lambda: self.candidate["model_configuration"].update(secret="SECRET_SENTINEL"),
            lambda: self.selection.update(unexpected="SECRET_SENTINEL"),
        ]
        candidate, selection = copy.deepcopy(self.candidate), copy.deepcopy(self.selection)
        for mutation in mutations:
            self.candidate, self.selection = copy.deepcopy(candidate), copy.deepcopy(selection)
            mutation()
            self.write_pins()
            self.reject()
        self.candidate, self.selection = candidate, selection
        self.write_pins()
        self.reject(producer="source-facts-3")

    def test_bounded_json_duplicate_keys_depth_nonfinite_and_invalid_utf8(self):
        for field, path, limit in (("candidate", self.freeze_path, preparer.JSON_LIMITS["candidate"]),
                                   ("selection", self.selection_path, preparer.JSON_LIMITS["selection"])):
            original = path.read_bytes()
            invalids = [b" " * (limit + 1), b'{"nested":{"x":1,"x":2}}', b"[" * 40 + b"0" + b"]" * 40,
                        b'{"a":NaN}', b'{"a":Infinity}', b'{"a":1e999}', b"\xff"]
            for raw in invalids:
                with self.subTest(field=field, invalid=sha(raw)):
                    path.write_bytes(raw)
                    pins = preparer.Pins(sha(raw) if field == "candidate" else self.pins.candidate_freeze_sha256,
                                         sha(raw) if field == "selection" else self.pins.development_selection_sha256)
                    parameters = self.parameters() | {f"{'candidate_freeze' if field == 'candidate' else 'development_selection'}_sha256": sha(raw)}
                    with self.assertRaises(preparer.PreparationError):
                        preparer.prepare(**parameters, pins=pins)
                    self.assertFalse(self.out.exists())
            path.write_bytes(original)
        # Exercise the shared pre-parse allocation bound on a tiny bounded input.
        with mock.patch.object(preparer.loader, "MAX_JSON_VALUES", 3):
            self.reject()

    def test_input_paths_and_nonregular_files_rejected(self):
        self.reject(candidate_freeze=self.directory / ".." / "candidate.json")
        self.reject(candidate_freeze=Path("/tmp/SECRET_SENTINEL"))
        alias = self.directory / "candidate-link.json"
        alias.symlink_to(self.freeze_path)
        self.reject(candidate_freeze=alias)
        fifo = self.directory / "candidate-fifo"
        os.mkfifo(fifo, 0o600)
        self.reject(candidate_freeze=fifo)
        self.reject(candidate_freeze=self.directory)

    def test_loader_admission_failure_creates_no_outputs(self):
        path = ROOT / self.manifest["corpus_path"]
        raw = path.read_bytes()
        path.write_bytes(raw + b" ")
        self.reject()
        path.write_bytes(raw)
        self.reject(manifest_sha256="0" * 64)

    def test_restrictive_permissions_and_fsync(self):
        real_fsync = os.fsync
        with mock.patch.object(preparer.os, "fsync", wraps=real_fsync) as calls:
            self.prepare()
        self.assertEqual(stat.S_IMODE(self.out.stat().st_mode), 0o700)
        self.assertTrue(all(stat.S_IMODE(path.stat().st_mode) == 0o600 for path in self.out.iterdir()))
        self.assertEqual(calls.call_count, 5)

    def test_output_traversal_parent_target_symlink_overwrite_and_missing_parent_refused(self):
        for out in (self.directory / ".." / "escape", Path("/tmp/SECRET_SENTINEL"), ROOT,
                    self.directory / "missing" / "child", self.directory / "any" / ".." / "child"):
            self.reject(out=out)
        parent_alias = self.directory / "parent-link"
        parent_alias.symlink_to(self.directory, target_is_directory=True)
        self.reject(out=parent_alias / "child")
        self.out.symlink_to(self.directory / "missing-target", target_is_directory=True)
        self.reject()
        self.assertTrue(self.out.is_symlink())
        self.out.unlink()
        self.out.mkdir()
        marker = self.out / "must-preserve"
        marker.write_bytes(b"existing target sentinel")
        with self.assertRaises(preparer.PreparationError):
            self.prepare()
        self.assertEqual(marker.read_bytes(), b"existing target sentinel")
        self.assertEqual(list(self.out.iterdir()), [marker])

    def test_replaced_parent_symlink_is_never_followed(self):
        parent, moved, elsewhere = [self.directory / name for name in ("original-parent", "moved-parent", "elsewhere")]
        parent.mkdir()
        elsewhere.mkdir()
        real_writer = preparer._write_output

        def replace_parent(descriptor, name, files):
            parent.rename(moved)
            parent.symlink_to(elsewhere, target_is_directory=True)
            return real_writer(descriptor, name, files)

        with mock.patch.object(preparer, "_write_output", side_effect=replace_parent):
            self.prepare(out=parent / "prepared")
        self.assertEqual(list(elsewhere.iterdir()), [])
        self.assertEqual(set(self.output(moved / "prepared")), {"recipe.json", "source-feed.json", "preparation.json"})

    def test_replaced_real_target_is_not_chmodded_or_written(self):
        foreign = self.directory / "preexisting-directory"
        foreign.mkdir(mode=0o750)
        marker = foreign / "preexisting-marker"
        marker.write_bytes(b"must remain untouched")
        original_mode = stat.S_IMODE(foreign.stat().st_mode)
        displaced = self.directory / "displaced-owned-directory"
        real_open = os.open

        def replace_target(path, flags, mode=0o777, *, dir_fd=None):
            if path == self.out.name and flags & os.O_DIRECTORY and self.out.exists():
                self.out.rename(displaced)
                foreign.rename(self.out)
            return real_open(path, flags, mode, dir_fd=dir_fd)

        with mock.patch.object(preparer.os, "open", side_effect=replace_target):
            with self.assertRaises(preparer.PreparationError):
                self.prepare()
        self.assertEqual(stat.S_IMODE(self.out.stat().st_mode), original_mode)
        self.assertEqual({path.name for path in self.out.iterdir()}, {"preexisting-marker"})
        self.assertEqual((self.out / "preexisting-marker").read_bytes(), b"must remain untouched")
        self.assertEqual(list(displaced.iterdir()), [])

    def test_target_symlink_after_open_preserves_owned_partial_and_refuses_success(self):
        elsewhere = self.directory / "elsewhere"
        elsewhere.mkdir(mode=0o750)
        marker = elsewhere / "preexisting-marker"
        marker.write_bytes(b"must remain untouched")
        original_mode = stat.S_IMODE(elsewhere.stat().st_mode)
        displaced = self.directory / "displaced-owned-directory"
        real_writer = preparer._write_file

        def replace_target(directory, name, raw):
            if name == "recipe.json":
                self.out.rename(displaced)
                self.out.symlink_to(elsewhere, target_is_directory=True)
            return real_writer(directory, name, raw)

        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.object(preparer, "_write_file", side_effect=replace_target), \
                mock.patch.object(preparer, "PRODUCTION_PINS", self.pins), redirect_stdout(stdout), redirect_stderr(stderr):
            status = preparer.main(self.argv())
        self.assertEqual(status, 1)
        self.assertEqual(stdout.getvalue(), "")
        self.assertEqual(stderr.getvalue(), "organization preparation failed\n")
        self.assertTrue(self.out.is_symlink())
        self.assertEqual(stat.S_IMODE(elsewhere.stat().st_mode), original_mode)
        self.assertEqual(list(elsewhere.iterdir()), [marker])
        self.assertEqual(marker.read_bytes(), b"must remain untouched")
        self.assertEqual(set(self.output(displaced)), {"recipe.json", "source-feed.json", "preparation.json"})

    def test_late_failure_preserves_only_owned_partial_and_never_reports_success(self):
        original = preparer._write_file

        def fail_second(directory, name, raw):
            if name == "source-feed.json":
                raise OSError("SECRET_SENTINEL later disk failure")
            return original(directory, name, raw)

        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.object(preparer, "_write_file", side_effect=fail_second), \
                mock.patch.object(preparer, "PRODUCTION_PINS", self.pins), redirect_stdout(stdout), redirect_stderr(stderr):
            status = preparer.main(self.argv())
        self.assertEqual(status, 1)
        self.assertEqual(stdout.getvalue(), "")
        self.assertEqual(stderr.getvalue(), "organization preparation failed\n")
        self.assertEqual(set(self.output()), {"recipe.json"})
        with self.assertRaises(preparer.PreparationError):
            self.prepare()
        self.assertEqual(set(self.output()), {"recipe.json"})

    def test_cli_api_success_stdout_contains_only_aggregate_receipt(self):
        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.object(preparer, "PRODUCTION_PINS", self.pins), redirect_stdout(stdout), redirect_stderr(stderr):
            status = preparer.main(self.argv())
        self.assertEqual(status, 0)
        self.assertEqual(stderr.getvalue(), "")
        receipt = json.loads(stdout.getvalue())
        self.assertEqual(set(receipt), {"schema", "input_sha256", "output_sha256", "counts", "complete_topic_gold_readiness", "not_measured"})
        self.assertEqual(receipt["schema"], preparer.PREPARATION_SCHEMA)
        self.assertEqual(set(receipt["output_sha256"]), {"recipe", "source_feed", "preparation"})
        for sentinel in ("SENTINEL", "Élan", "source-feed-preparer-proof", str(self.directory), "scope", "topics"):
            self.assertNotIn(sentinel, stdout.getvalue() + stderr.getvalue())

    def test_real_cli_keeps_production_pins_and_generic_argument_errors(self):
        command = [sys.executable, "-B", str(ROOT / "scripts/prepare_frozen_organization.py")]
        failed = subprocess.run(command + self.argv(), capture_output=True, text=True, check=False)
        self.assertEqual(failed.returncode, 1)
        self.assertEqual(failed.stdout, "")
        self.assertEqual(failed.stderr, "organization preparation failed\n")
        for arguments in ([], ["--PRIVATE_SENTINEL"], self.argv() + ["--pins", "PRIVATE_SENTINEL"],
                          ["--producer", "PRIVATE_SENTINEL"], ["--man", "PRIVATE_SENTINEL"]):
            result = subprocess.run(command + arguments, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stdout, "")
            self.assertEqual(result.stderr, "organization preparation arguments invalid\n")
        self.assertFalse(self.out.exists())


if __name__ == "__main__":
    unittest.main()
