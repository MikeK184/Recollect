#!/usr/bin/env python3
"""Prepare pinned source-only organization inputs offline, without qualification.

Every CLI input is explicit. Production freeze/selection hashes are immutable;
independent tests can supply their own Pins through the Python API. This tool
uses no network, provider, database, process, environment configuration or retry.
Declared three-part scope tokens are fixture setup, not semantic identity.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
from typing import Any, Iterator

import frozen_source_corpus as loader
import score_frozen_source_corpus as scorer


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
FREEZE_SCHEMA = "recollect-revision4-predata-candidate-freeze-1"
SELECTION_SCHEMA = "recollect-real-development-selection-1"
FEED_SCHEMA = "recollect-native-organization-source-feed-1"
PREPARATION_SCHEMA = "recollect-native-organization-preparation-1"
FAILURE_MESSAGE = "organization preparation failed"
JSON_LIMITS = {"candidate": 256 * 1024, "selection": 256 * 1024, "recipe": 16 * 1024}
# Stream hashes without unbounded reads, including when a pinned file grows.
MAX_PINNED_FILE_BYTES = 16 * 1024 * 1024
MAX_PINNED_TOTAL_BYTES = 512 * 1024 * 1024
PRODUCERS = ("source-facts-4", "source-reconciliation-4")
FREEZE_FIELDS = {
    "additional_model_calls", "amendment", "extractors", "grouping", "heldout_bindings",
    "migration_sha256", "model_configuration", "private_corpus_bodies_opened_by_preparer",
    "purpose", "qualified", "recorded_at", "release_catalogue_seeded", "schema",
    "source_sha256", "supersedes_predata_freeze_sha256",
}
GROUPING_FIELDS = {
    "algorithm", "arms", "concurrency", "gds_parameters", "gds_version", "label_policy",
    "new_grid_trials", "parameters", "random_seed", "recipe", "representation",
    "selection_path", "selection_sha256",
}
SELECTION_FIELDS = {
    "schema", "input_mode", "development_source_cohort_sha256", "recipe_sha256",
    "development_corpus_sha256", "scorer_sha256", "trials", "selected", "parameters",
    "rule", "production_recipe_qualified", "development_result_sha256",
}


@dataclass(frozen=True)
class Pins:
    """Exact artifact hashes; API injection is only for independent toy fixtures."""

    candidate_freeze_sha256: str
    development_selection_sha256: str


PRODUCTION_PINS = Pins(
    "ffc4f4251823bde4060c2d1158482728b4913d955e508a1fef9b8cf0cb6c6431",
    "084c075eb5f0a300861e2c95cd6c75ad17506d2c2b49b342070dbab45457056d",
)


class PreparationError(ValueError):
    """Content-free whole-preparation failure."""


def _require(condition: bool) -> None:
    if not condition:
        raise PreparationError(FAILURE_MESSAGE)


def _object(value: Any, fields: set[str]) -> dict[str, Any]:
    _require(isinstance(value, dict) and set(value) == fields)
    return value


def _hash(value: Any) -> str:
    _require(isinstance(value, str) and loader.HASH_PATTERN.fullmatch(value) is not None)
    return value


def _encoded(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                       allow_nan=False) + "\n").encode("utf-8")


def _parts(path: str | Path) -> tuple[str, ...]:
    text = str(path) if isinstance(path, Path) else path
    _require(isinstance(text, str) and 0 < len(text.encode("utf-8")) <= 4096)
    _require("\0" not in text and not {".", ".."}.intersection(text.split("/")))
    candidate = Path(text)
    if candidate.is_absolute():
        candidate = candidate.relative_to(REPOSITORY_ROOT)
    _require(bool(candidate.parts) and not candidate.is_absolute())
    return candidate.parts


@contextmanager
def _parent(path: str | Path) -> Iterator[tuple[int, str]]:
    """Hold every parent by descriptor; a replaced symlink is never followed."""
    parts = _parts(path)
    descriptors = []
    try:
        # Walk even the absolute repository ancestors with O_NOFOLLOW.
        descriptor = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        descriptors.append(descriptor)
        for component in (*REPOSITORY_ROOT.parts[1:], *parts[:-1]):
            descriptor = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                                 dir_fd=descriptor)
            descriptors.append(descriptor)
        yield descriptor, parts[-1]
    finally:
        for descriptor in reversed(descriptors):
            os.close(descriptor)


def _file_digest(path: str | Path, *, maximum: int = MAX_PINNED_FILE_BYTES) -> tuple[str, int]:
    with _parent(path) as (parent, name):
        descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
        try:
            _require(stat.S_ISREG(os.fstat(descriptor).st_mode))
            digest, size = hashlib.sha256(), 0
            while True:
                raw = os.read(descriptor, min(64 * 1024, maximum - size + 1))
                if not raw:
                    return digest.hexdigest(), size
                size += len(raw)
                _require(size <= maximum)
                digest.update(raw)
        finally:
            os.close(descriptor)


def _read_json(path: str | Path, digest: str, kind: str) -> Any:
    _hash(digest)
    with _parent(path) as (parent, name):
        descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
        with os.fdopen(descriptor, "rb") as stream:
            _require(stat.S_ISREG(os.fstat(stream.fileno()).st_mode))
            raw = stream.read(JSON_LIMITS[kind] + 1)
    _require(len(raw) <= JSON_LIMITS[kind] and hashlib.sha256(raw).hexdigest() == digest)
    return loader._parse_json(raw)


def _parameters(value: Any) -> None:
    parameters = _object(value, set(scorer.PARAMETERS))
    _require(type(parameters["neighbors"]) is int)
    _require(all(type(parameters[key]) in (int, float) for key in ("minimum_cosine", "vector_scale")))
    _require(parameters == scorer.PARAMETERS)


def _candidate(value: Any, selection_path: str | Path, pins: Pins) -> None:
    candidate = _object(value, FREEZE_FIELDS)
    _require(candidate["schema"] == FREEZE_SCHEMA)
    extractors = _object(candidate["extractors"], {"direct", "reconciliation", "dto", "support_verifier"})
    for name, producer in (("direct", PRODUCERS[0]), ("reconciliation", PRODUCERS[1])):
        _require(extractors[name] == [producer, producer, "learning-quote-anchors-3"])
    _require(extractors["dto"] == "source-mentions-1" and extractors["support_verifier"] == "source-support-3")
    grouping = _object(candidate["grouping"], GROUPING_FIELDS)
    _parameters(grouping["parameters"])
    _require(grouping["arms"] == list(scorer.MODES))
    _require(_hash(grouping["selection_sha256"]) == pins.development_selection_sha256)
    _require(_parts(grouping["selection_path"]) == _parts(selection_path))
    _require(candidate["qualified"] is False and candidate["release_catalogue_seeded"] is False)
    _require(type(candidate["additional_model_calls"]) is int and candidate["additional_model_calls"] == 0)
    _require(type(grouping["new_grid_trials"]) is int and grouping["new_grid_trials"] == 0)
    # Only this public whitelist is read. It is not copied into source-feed or recipe.
    models = _object(candidate["model_configuration"], {
        "provider", "endpoint", "text_model", "embedding_model", "embedding_dimensions",
    })
    _require(models == {"provider": "openai", "endpoint": "https://api.openai.com/v1",
                        "text_model": "gpt-5.6-luna", "embedding_model": "text-embedding-3-large",
                        "embedding_dimensions": 3072})
    _require(type(models["embedding_dimensions"]) is int)
    seen, total = set(), 0
    for field, count in (("source_sha256", 198), ("migration_sha256", 64)):
        table = candidate[field]
        _require(isinstance(table, dict) and len(table) == count)
        for path, expected in table.items():
            parts = _parts(path)
            _require(not Path(path).is_absolute() and parts not in seen)
            seen.add(parts)
            actual, size = _file_digest(path, maximum=min(MAX_PINNED_FILE_BYTES, MAX_PINNED_TOTAL_BYTES - total))
            total += size
            _require(actual == _hash(expected))


def _selection(value: Any) -> None:
    selection = _object(value, SELECTION_FIELDS)
    _require(selection["schema"] == SELECTION_SCHEMA and selection["selected"] is True)
    _require(selection["production_recipe_qualified"] is False)
    _parameters(selection["parameters"])
    _require(type(selection["trials"]) is int and selection["trials"] == 27)
    for field in ("development_source_cohort_sha256", "recipe_sha256", "development_corpus_sha256", "scorer_sha256"):
        _hash(selection[field])
    results = selection["development_result_sha256"]
    _require(isinstance(results, list) and len(results) == selection["trials"])
    for digest in results:
        _hash(digest)


def _write_file(directory: int, name: str, raw: bytes) -> None:
    descriptor = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                         0o600, dir_fd=directory)
    with os.fdopen(descriptor, "wb") as stream:
        os.fchmod(stream.fileno(), 0o600)
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())


def _write_output(parent: int, name: str, files: dict[str, bytes]) -> None:
    os.mkdir(name, 0o700, dir_fd=parent)
    created = os.stat(name, dir_fd=parent, follow_symlinks=False)
    _require(stat.S_ISDIR(created.st_mode))
    descriptor = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)
    try:
        opened = os.fstat(descriptor)
        _require((created.st_dev, created.st_ino) == (opened.st_dev, opened.st_ino))
        _require(not os.listdir(descriptor))
        os.fchmod(descriptor, 0o700)
        os.fsync(parent)
        for filename, raw in files.items():
            _write_file(descriptor, filename, raw)
        os.fsync(descriptor)
        current = os.stat(name, dir_fd=parent, follow_symlinks=False)
        _require(stat.S_ISDIR(current.st_mode))
        _require((created.st_dev, created.st_ino) == (current.st_dev, current.st_ino))
    finally:
        os.close(descriptor)


def prepare(manifest: str | Path, *, manifest_sha256: str, candidate_freeze: str | Path,
            candidate_freeze_sha256: str, development_selection: str | Path,
            development_selection_sha256: str, producer: str, out: str | Path,
            pins: Pins = PRODUCTION_PINS) -> dict[str, Any]:
    """Admit all inputs before creating a new output; preserve owned partials on failure."""
    try:
        _require(candidate_freeze_sha256 == _hash(pins.candidate_freeze_sha256))
        _require(development_selection_sha256 == _hash(pins.development_selection_sha256))
        _require(producer in PRODUCERS)
        candidate = _read_json(candidate_freeze, candidate_freeze_sha256, "candidate")
        selection = _read_json(development_selection, development_selection_sha256, "selection")
        _candidate(candidate, development_selection, pins)
        _selection(selection)
        loaded = loader.load_frozen_corpus(manifest, manifest_sha256=manifest_sha256)
        summary = loaded.summary()
        recipe = {
            "schema": scorer.RECIPE_SCHEMA, "candidate_freeze_sha256": candidate_freeze_sha256,
            "development_selection_sha256": development_selection_sha256,
            "producer": {"prompt_label": producer, "schema_label": producer,
                         "adapter_revision": "learning-quote-anchors-3", "mapping_schema": "source-mentions-1"},
            "parameters": dict(scorer.PARAMETERS), "modes": list(scorer.MODES),
        }
        scorer._recipe(recipe)
        recipe_raw = _encoded(recipe)
        _require(len(recipe_raw) <= JSON_LIMITS["recipe"])
        recipe_hash = hashlib.sha256(recipe_raw).hexdigest()
        feed = {
            "schema": FEED_SCHEMA, "corpus_bindings": dict(loaded.file_sha256),
            "recipe_sha256": recipe_hash,
            "sources": [{
                "document_id": document["id"], "text_sha256": hashlib.sha256(document["text"].encode("utf-8")).hexdigest(),
                **{field: document[field] for field in ("partition", "language", "scope", "title", "text")},
            } for document in loaded.annotations["documents"]],
        }
        feed_raw = _encoded(feed)
        preparation = {
            "schema": PREPARATION_SCHEMA,
            "input_sha256": dict(loaded.file_sha256) | {
                "candidate_freeze": candidate_freeze_sha256,
                "development_selection": development_selection_sha256,
                "preparer": _file_digest(Path(__file__))[0],
                "loader": _file_digest(Path(loader.__file__))[0],
                "scorer": _file_digest(Path(scorer.__file__))[0],
            },
            "output_sha256": {"recipe": recipe_hash, "source_feed": hashlib.sha256(feed_raw).hexdigest()},
            "counts": summary["counts"],
            "complete_topic_gold_readiness": summary["complete_topic_gold_readiness"],
            "not_measured": dict(scorer.UNMEASURED),
        }
        files = {"recipe.json": recipe_raw, "source-feed.json": feed_raw,
                 "preparation.json": _encoded(preparation)}
        # Resolve and hold the parent only after all input validation and encoding.
        with _parent(out) as (parent, name):
            try:
                os.stat(name, dir_fd=parent, follow_symlinks=False)
            except FileNotFoundError:
                pass
            else:
                raise PreparationError(FAILURE_MESSAGE)
            _write_output(parent, name, files)
        # Return an aggregate-only receipt, including the final receipt file hash.
        return preparation | {"output_sha256": preparation["output_sha256"] | {
            "preparation": hashlib.sha256(files["preparation.json"]).hexdigest(),
        }}
    except (OSError, ValueError, TypeError, KeyError, RecursionError, OverflowError, RuntimeError):
        raise PreparationError(FAILURE_MESSAGE) from None


class _ArgumentParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        self.exit(2, "organization preparation arguments invalid\n")


def main(argv: list[str] | None = None) -> int:
    parser = _ArgumentParser(description=__doc__, allow_abbrev=False)
    for flag in ("manifest", "manifest-sha256", "candidate-freeze", "candidate-freeze-sha256",
                 "development-selection", "development-selection-sha256", "out"):
        parser.add_argument(f"--{flag}", required=True)
    parser.add_argument("--producer", required=True, choices=PRODUCERS)
    args = parser.parse_args(argv)
    try:
        result = prepare(**vars(args), pins=PRODUCTION_PINS)
    except PreparationError:
        print(FAILURE_MESSAGE, file=sys.stderr)
        return 1
    print(_encoded(result).decode("utf-8"), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
