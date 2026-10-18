#!/usr/bin/env python3
"""Score explicitly pinned, recorded organization outcomes using frozen annotations.

This additive offline tool never produces outcomes, fits a recipe or admits native
origins. All five mode records describe observations only. Supported endpoint
annotation agreement is distinct from coverage of nonpositive/ambiguous gold.
Unlisted endpoints are not semantic negatives. Even perfect annotation agreement
leaves canonical identity, alias acceptance, provenance and qualification unmeasured.

All six input flags are required: --manifest/--manifest-sha256,
--recipe/--recipe-sha256, --outcomes/--outcomes-sha256. Optional --details creates
an exclusive 0600 file inside the repository, with IDs and numeric diagnostics
only. Stdout is aggregate-only. No network, process, provider or database is used.
"""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import math
import os
from pathlib import Path
import re
import sys
from typing import Any
import uuid

from frozen_source_corpus import (
    CorpusValidationError, FrozenCorpus, HASH_PATTERN, KINDS, MAX_DOCUMENTS,
    MENTION_KEY_PATTERN, RELATIONS, REPOSITORY_ROOT, _parse_json,
    load_frozen_corpus,
)


RECIPE_SCHEMA = "recollect-offline-organization-recipe-1"
OUTCOMES_SCHEMA = "recollect-offline-organization-outcomes-1"
REPORT_SCHEMA = "recollect-offline-organization-score-1"
FILE_LIMITS = {"recipe": 16 * 1024, "outcomes": 96 * 1024 * 1024}
MAX_CANDIDATE_BYTES = 256 * 1024
MAX_GROUPS = 10_000
MAX_MEMBERSHIPS = 50_000
MODES = ("native_hybrid", "native_weighted", "entity_overlap", "metadata", "eligible_vector")
PARAMETERS = {"neighbors": 3, "minimum_cosine": 0.65, "vector_scale": 0.25}
EXTRACTION_STATES = (
    "validated", "absent", "unsupported_language", "missing", "not_attempted", "rejected", "failed",
)
MODE_STATES = (
    "assigned", "unassigned", "ineligible", "unsupported", "rejected", "failed", "missing", "not_attempted",
)
CANDIDATE_FAILURES = (
    "InvalidJson", "InvalidSchema", "ForeignInput", "UnsupportedLanguage", "InputLimit",
    "InvalidKey", "InvalidSpan", "InvalidConfidence", "InvalidEndpoint", "IncompatibleAlias",
)
ENDPOINT_CLASSIFICATIONS = (
    "supported_only_predictions", "known_nonpositive_disposition_hits",
    "ambiguous_disposition_predictions", "unlisted_predictions",
)
NONPOSITIVE_DISPOSITIONS = {
    "aliases": ("rejected_equivalence", "incompatible_scope", "unresolved_equivalence"),
    "relations": ("contradicted_navigation",),
}
UNMEASURED = {
    key: "NOT_MEASURED" for key in (
        "canonical_identity", "accepted_alias_dispositions", "native_origin", "label_usefulness",
        "producer_qualification", "recipe_qualification",
    )
}
UUID_PATTERN = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\Z")
FAILURE_MESSAGE = "offline outcome evaluation failed"


class OutcomeValidationError(ValueError):
    """A content-free whole-artifact validation failure."""


def _require(condition: bool) -> None:
    if not condition:
        raise OutcomeValidationError(FAILURE_MESSAGE)


def _string(value: Any, limit: int = 256) -> str:
    _require(isinstance(value, str))
    _require(0 < len(value.encode("utf-8")) <= limit)
    return value


def _object(value: Any, keys: set[str]) -> dict[str, Any]:
    _require(isinstance(value, dict) and set(value) == keys)
    return value


def _array(value: Any, maximum: int) -> list[Any]:
    _require(isinstance(value, list) and len(value) <= maximum)
    return value


def _hash(value: Any) -> str:
    _require(isinstance(value, str) and HASH_PATTERN.fullmatch(value) is not None)
    return value


def _uuid(value: Any) -> str:
    _require(isinstance(value, str) and UUID_PATTERN.fullmatch(value) is not None)
    _require(str(uuid.UUID(value)) == value)
    return value


def _encoded(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                       allow_nan=False) + "\n").encode("utf-8")


def _read_pinned(path: str | Path, digest: str, kind: str) -> Any:
    _hash(digest)
    candidate = Path(_string(str(path), 4096))
    _require(".." not in candidate.parts)
    resolved = (REPOSITORY_ROOT / candidate).resolve(strict=True)
    _require(resolved.is_relative_to(REPOSITORY_ROOT) and resolved.is_file())
    with resolved.open("rb") as stream:
        raw = stream.read(FILE_LIMITS[kind] + 1)
    _require(len(raw) <= FILE_LIMITS[kind] and hashlib.sha256(raw).hexdigest() == digest)
    # Reuse the loader's bounded duplicate-key/depth/value/finite JSON parser.
    return _parse_json(raw)


def _parameters(value: Any) -> None:
    params = _object(value, set(PARAMETERS))
    _require(type(params["neighbors"]) is int)
    _require(all(type(params[key]) in (int, float) for key in ("minimum_cosine", "vector_scale")))
    _require(params == PARAMETERS)


def _recipe(value: Any) -> dict[str, Any]:
    recipe = _object(value, {
        "schema", "candidate_freeze_sha256", "development_selection_sha256", "producer", "parameters", "modes",
    })
    _require(recipe["schema"] == RECIPE_SCHEMA)
    _hash(recipe["candidate_freeze_sha256"])
    _hash(recipe["development_selection_sha256"])
    producer = _object(recipe["producer"], {"prompt_label", "schema_label", "adapter_revision", "mapping_schema"})
    _require(producer["prompt_label"] in ("source-facts-4", "source-reconciliation-4"))
    _require(producer["schema_label"] == producer["prompt_label"])
    _require(producer["adapter_revision"] == "learning-quote-anchors-3")
    _require(producer["mapping_schema"] == "source-mentions-1")
    _parameters(recipe["parameters"])
    _require(recipe["modes"] == list(MODES))
    return recipe


def _span(value: Any, source: bytes, *, mention: bool = False) -> tuple[int, int, str]:
    span = _object(value, {"byte_start", "byte_end", "quote"})
    start, end = span["byte_start"], span["byte_end"]
    quote = _string(span["quote"], 256 if mention else 2 * 1024 * 1024)
    _require(bool(quote.strip()) and type(start) is int and type(end) is int)
    _require(0 <= start < end <= len(source))
    _require(source[:start].decode("utf-8") is not None)
    _require(source[start:end].decode("utf-8") == quote)
    return start, end, quote


def _candidates(value: Any, version: str, document: dict[str, Any], producer: dict[str, Any]) -> None:
    candidates = _object(value, {
        "schema", "adapter_revision", "source_version_id", "language", "mentions", "aliases", "relations",
    })
    # The limit applies to the compact serialized DTO, not its surrounding outcome.
    _require(len(_encoded(candidates)) - 1 <= MAX_CANDIDATE_BYTES)
    _require(candidates["schema"] == producer["mapping_schema"])
    _require(candidates["adapter_revision"] == producer["adapter_revision"])
    _require(_uuid(candidates["source_version_id"]) == version and candidates["language"] == "en")
    source = document["text"].encode("utf-8")
    endpoints: dict[str, tuple[str, int, int, str]] = {}
    occurrences: set[tuple[str, int, int, str]] = set()
    for value in _array(candidates["mentions"], 128):
        mention = _object(value, {"key", "kind", "span", "confidence"})
        key, kind = _string(mention["key"]), _string(mention["kind"])
        _require(MENTION_KEY_PATTERN.fullmatch(key) is not None and key not in endpoints and kind in KINDS)
        signature = (kind, *_span(mention["span"], source, mention=True))
        _require(signature not in occurrences)
        occurrences.add(signature)
        endpoints[key] = signature
        confidence = mention["confidence"]
        _require(type(confidence) in (int, float) and math.isfinite(confidence) and 0 <= confidence <= 1)
    for field, maximum in (("aliases", 128), ("relations", 256)):
        for raw in _array(candidates[field], maximum):
            keys = {"left", "right", "span"} if field == "aliases" else {"from", "to", "relation", "span"}
            edge = _object(raw, keys)
            left = _string(edge["left" if field == "aliases" else "from"])
            right = _string(edge["right" if field == "aliases" else "to"])
            _require(left != right and left in endpoints and right in endpoints)
            start, end, _ = _span(edge["span"], source)
            _require(all(start <= endpoints[key][1] and endpoints[key][2] <= end for key in (left, right)))
            if field == "aliases":
                _require(endpoints[left][0] == endpoints[right][0])
            else:
                _require(_string(edge["relation"]) in RELATIONS)
            # Identical aliases/relations are legal native DTO emissions. They
            # remain repeated observations and are penalized by multiset scoring.


def _sources(value: Any, documents: dict[str, dict[str, Any]], recipe: dict[str, Any]) -> dict[str, Any]:
    records: dict[str, Any] = {}
    versions: set[str] = set()
    for raw in _array(value, MAX_DOCUMENTS):
        source = _object(raw, {"document_id", "text_sha256", "partition", "source_version_id", "extraction"})
        key = _string(source["document_id"])
        _require(key in documents and key not in records)
        document = documents[key]
        _require(_hash(source["text_sha256"]) == hashlib.sha256(document["text"].encode("utf-8")).hexdigest())
        _require(source["partition"] == document["partition"])
        version = source["source_version_id"]
        if version is not None:
            _uuid(version)
            _require(version not in versions)
            versions.add(version)
        extraction = _object(source["extraction"], {"state", "failure", "candidates"})
        state = _string(extraction["state"])
        _require(state in EXTRACTION_STATES)
        if state == "validated":
            _require(version is not None and extraction["failure"] is None)
            _candidates(extraction["candidates"], version, document, recipe["producer"])
        else:
            _require(extraction["candidates"] is None)
            if state == "rejected":
                _require(extraction["failure"] in CANDIDATE_FAILURES)
            elif state == "failed":
                _require(extraction["failure"] == "UpstreamFailure")
            else:
                _require(extraction["failure"] is None)
        records[key] = source
    _require(set(records) == set(documents))
    return records


def _mode(value: Any, documents: dict[str, Any], sources: dict[str, Any]) -> dict[str, Any]:
    mode = _object(value, {"name", "parameters", "sources", "groups"})
    _require(_string(mode["name"]) in MODES)
    _parameters(mode["parameters"])
    by_source: dict[str, Any] = {}
    source_memberships = 0
    for raw in _array(mode["sources"], MAX_DOCUMENTS):
        row = _object(raw, {"document_id", "state", "ner_feature_available", "vector_feature_available", "groups"})
        key = _string(row["document_id"])
        _require(key in documents and key not in by_source)
        _require(_string(row["state"]) in MODE_STATES)
        _require(type(row["ner_feature_available"]) is bool and type(row["vector_feature_available"]) is bool)
        _require(not row["ner_feature_available"] or sources[key]["extraction"]["state"] == "validated")
        ids = [_string(group) for group in _array(row["groups"], MAX_GROUPS)]
        _require(len(set(ids)) == len(ids) and bool(ids) == (row["state"] == "assigned"))
        source_memberships += len(ids)
        _require(source_memberships <= MAX_MEMBERSHIPS)
        by_source[key] = row
    _require(set(by_source) == set(documents))
    groups: dict[str, set[str]] = {}
    memberships = 0
    for raw in _array(mode["groups"], MAX_GROUPS):
        group = _object(raw, {"id", "members"})
        key = _string(group["id"])
        _require(key not in groups)
        members = [_string(member) for member in _array(group["members"], MAX_DOCUMENTS)]
        _require(bool(members) and len(set(members)) == len(members) and set(members) <= set(documents))
        memberships += len(members)
        _require(memberships <= MAX_MEMBERSHIPS)
        groups[key] = set(members)
    observed = {(source, group) for source, row in by_source.items() for group in row["groups"]}
    declared = {(source, group) for group, members in groups.items() for source in members}
    _require(observed == declared)
    return {"name": mode["name"], "sources": by_source, "groups": groups}


def _admit(value: Any, frozen: FrozenCorpus, recipe: dict[str, Any], recipe_sha256: str) -> tuple[dict[str, Any], dict[str, Any]]:
    outcomes = _object(value, {"schema", "corpus_bindings", "recipe_sha256", "sources", "modes"})
    _require(outcomes["schema"] == OUTCOMES_SCHEMA)
    bindings = _object(outcomes["corpus_bindings"], set(frozen.file_sha256))
    _require(all(_hash(bindings[key]) == digest for key, digest in frozen.file_sha256.items()))
    _require(_hash(outcomes["recipe_sha256"]) == recipe_sha256)
    documents = {doc["id"]: doc for doc in frozen.annotations["documents"]}
    sources = _sources(outcomes["sources"], documents, recipe)
    modes: dict[str, Any] = {}
    for raw in _array(outcomes["modes"], len(MODES)):
        mode = _mode(raw, documents, sources)
        _require(mode["name"] not in modes)
        modes[mode["name"]] = mode
    _require(set(modes) == set(MODES))
    return sources, modes


def _agreement(expected: Counter, predicted: Counter, *, match: bool = True, endpoint: bool = False) -> dict[str, Any]:
    tp = sum((expected & predicted).values()) if match else 0
    return _metrics(tp, sum(predicted.values()) - tp, sum(expected.values()) - tp, endpoint=endpoint)


def _metrics(tp: int, fp: int, fn: int, *, endpoint: bool = False) -> dict[str, Any]:
    return {
        "tp": tp, "unmatched_strict_gold_fp" if endpoint else "fp": fp, "fn": fn,
        "precision": tp / (tp + fp) if tp + fp else None,
        "recall": tp / (tp + fn) if tp + fn else None,
        "f1": 2 * tp / (2 * tp + fp + fn) if 2 * tp + fp + fn else None,
    }


def _mention_signatures(document: dict[str, Any], candidates: dict[str, Any] | None) -> tuple[dict, dict]:
    gold = {key: (kind, start, end, quote) for key, kind, start, end, quote in document["mentions"]}
    predicted = {} if candidates is None else {
        m["key"]: (m["kind"], m["span"]["byte_start"], m["span"]["byte_end"], m["span"]["quote"])
        for m in candidates["mentions"]
    }
    return gold, predicted


def _edge_signature(left: tuple, right: tuple, span: tuple, relation: str | None) -> tuple:
    return (*sorted((left, right)), *span) if relation is None else (left, right, relation, *span)


def _endpoint_score(document: dict[str, Any], candidates: dict[str, Any] | None, field: str,
                    gold_mentions: dict, predicted_mentions: dict) -> dict[str, Any]:
    supported = "supported_equivalence" if field == "aliases" else "supported_navigation"
    gold = Counter()
    positives = Counter()
    dispositions: dict[tuple, set[str]] = {}
    for row in document[field]:
        left, right = gold_mentions[row[0]], gold_mentions[row[1]]
        relation = None if field == "aliases" else row[2]
        disposition = row[2] if field == "aliases" else row[3]
        signature = _edge_signature(left, right, tuple(row[-3:]), relation)
        gold[signature] += 1
        dispositions.setdefault(signature, set()).add(disposition)
        if disposition == supported:
            positives[signature] += 1
    predicted = Counter()
    for row in [] if candidates is None else candidates[field]:
        left = predicted_mentions[row["left" if field == "aliases" else "from"]]
        right = predicted_mentions[row["right" if field == "aliases" else "to"]]
        span = row["span"]
        predicted[_edge_signature(left, right, (span["byte_start"], span["byte_end"], span["quote"]),
                                  None if field == "aliases" else row["relation"])] += 1
    classification = Counter({name: 0 for name in ENDPOINT_CLASSIFICATIONS})
    nonpositive = Counter({name: 0 for name in NONPOSITIVE_DISPOSITIONS[field]})
    for signature, count in predicted.items():
        declared = dispositions.get(signature, set())
        if not declared:
            classification["unlisted_predictions"] += count
        elif len(declared) > 1:
            classification["ambiguous_disposition_predictions"] += count
        elif supported in declared:
            classification["supported_only_predictions"] += count
        else:
            classification["known_nonpositive_disposition_hits"] += count
            nonpositive[next(iter(declared))] += count
    match = document["language"] == "en"
    primary = _agreement(positives, predicted, match=match, endpoint=True)
    all_gold = _agreement(gold, predicted, match=match, endpoint=True)
    return {
        "supported_annotation_agreement": primary,
        "supported_endpoint_recall": primary["recall"],
        "all_disposition_endpoint_coverage": {
            "matched": all_gold["tp"], "gold": sum(gold.values()), "predictions": sum(predicted.values()),
            "recall": all_gold["recall"],
        },
        "classification": dict(classification), "known_nonpositive_by_disposition": dict(nonpositive),
        "ambiguous_gold_signatures": sum(len(value) > 1 for value in dispositions.values()),
    }


def _sum_metrics(rows: list[dict[str, Any]], *, endpoint: bool = False) -> dict[str, Any]:
    fp = "unmatched_strict_gold_fp" if endpoint else "fp"
    return _metrics(sum(row["tp"] for row in rows), sum(row[fp] for row in rows), sum(row["fn"] for row in rows), endpoint=endpoint)


def _coverage(selected: set[str], universe: set[str]) -> dict[str, Any]:
    return {"count": len(selected & universe), "total": len(universe),
            "coverage": len(selected & universe) / len(universe) if universe else None}


def _aggregate_extraction(rows: list[dict[str, Any]]) -> dict[str, Any]:
    summary: dict[str, Any] = {
        "sources": len(rows), "states": {state: sum(row["state"] == state for row in rows) for state in EXTRACTION_STATES},
        "failures": {failure: sum(row["failure"] == failure for row in rows) for failure in (*CANDIDATE_FAILURES, "UpstreamFailure")},
        "mentions": _sum_metrics([row["mentions"] for row in rows]),
        "non_english_sources": sum(not row["english"] for row in rows),
        "non_english_proven_abstentions": sum(row["proven_abstention"] for row in rows),
    }
    for field in ("aliases", "relations"):
        endpoints = [row[field] for row in rows]
        primary = _sum_metrics([row["supported_annotation_agreement"] for row in endpoints], endpoint=True)
        all_gold = [row["all_disposition_endpoint_coverage"] for row in endpoints]
        matched, gold, predicted = (sum(row[key] for row in all_gold) for key in ("matched", "gold", "predictions"))
        summary[field] = {
            "supported_annotation_agreement": primary, "supported_endpoint_recall": primary["recall"],
            "all_disposition_endpoint_coverage": {"matched": matched, "gold": gold, "predictions": predicted,
                                                 "recall": matched / gold if gold else None},
            "classification": {key: sum(row["classification"][key] for row in endpoints)
                               for key in ENDPOINT_CLASSIFICATIONS},
            "known_nonpositive_by_disposition": {key: sum(row["known_nonpositive_by_disposition"][key] for row in endpoints)
                                                 for key in NONPOSITIVE_DISPOSITIONS[field]},
            "ambiguous_gold_signatures": sum(row["ambiguous_gold_signatures"] for row in endpoints),
        }
    return summary


def _score_mode(mode: dict[str, Any], frozen: FrozenCorpus) -> tuple[dict[str, Any], dict[str, Any]]:
    documents = {doc["id"]: doc for doc in frozen.annotations["documents"]}
    by_source, groups = mode["sources"], mode["groups"]
    assignments = {key: set(row["groups"]) for key, row in by_source.items()}
    positive_sources: set[str] = set()
    recovered_sources: set[str] = set()
    pair_rows = []
    tp = fp = fn = tn = unknown_predictions = cross_brain_failures = 0
    for pair in frozen.topic_pairs:
        left, right = pair.left_document_id, pair.right_document_id
        shared_count = len(assignments[left] & assignments[right])
        related = bool(shared_count)
        cross_brain = documents[left]["scope"].split("/")[0] != documents[right]["scope"].split("/")[0]
        if pair.truth is True:
            positive_sources.update((left, right))
            if related:
                tp += 1
                recovered_sources.update((left, right))
            else:
                fn += 1
        elif pair.truth is False:
            fp += related
            tn += not related
        else:
            unknown_predictions += related
        cross_brain_failures += cross_brain and related
        pair_rows.append({"left_document_id": left, "right_document_id": right, "truth": pair.truth,
                          "predicted_related": related, "shared_group_count": shared_count,
                          "cross_brain_related_failure": cross_brain and related})
    assigned = {key for key, ids in assignments.items() if ids}
    nonempty = {key for key, doc in documents.items() if doc["text"] != ""}
    unknown = {key for key, doc in documents.items() if doc["metadata"]["topic_annotation_state"] == "unknown"}
    multi = set().union(*(members for members in groups.values() if len(members) > 1))
    summary = {
        "topic_annotation_agreement": _metrics(tp, fp, fn) | {"tn": tn},
        "topic_pairs": len(frozen.topic_pairs),
        "topic_pairs_known": sum(pair.truth is not None for pair in frozen.topic_pairs),
        "topic_pairs_unknown": sum(pair.truth is None for pair in frozen.topic_pairs),
        "unknown_related_predictions": unknown_predictions,
        "cross_brain_related_pair_failures": cross_brain_failures,
        "source_states": {state: sum(row["state"] == state for row in by_source.values()) for state in MODE_STATES},
        "all_source_assignment": _coverage(assigned, set(documents)),
        "nonempty_source_assignment": _coverage(assigned, nonempty),
        "known_positive_source_assignment": _coverage(assigned, positive_sources),
        "known_positive_sources_with_recovered_pair": _coverage(recovered_sources, positive_sources),
        "multi_source_group_coverage": _coverage(multi, set(documents)),
        "unknown_source_assignments": _coverage(assigned, unknown),
        "groups": len(groups), "singleton_groups": sum(len(members) == 1 for members in groups.values()),
        "multi_source_groups": sum(len(members) > 1 for members in groups.values()),
        "memberships": sum(len(ids) for ids in assignments.values()),
        "declared_ner_feature_sources": sum(row["ner_feature_available"] for row in by_source.values()),
        "declared_vector_feature_sources": sum(row["vector_feature_available"] for row in by_source.values()),
    }
    details = {"sources": [by_source[key] for key in sorted(by_source)], "pairs": pair_rows,
               "groups": [{"id": key, "members": sorted(groups[key])} for key in sorted(groups)]}
    return summary, details


def evaluate(manifest: str | Path, *, manifest_sha256: str, recipe: str | Path,
             recipe_sha256: str, outcomes: str | Path, outcomes_sha256: str) -> tuple[dict[str, Any], dict[str, Any]]:
    """Validate all artifacts before returning aggregate and text-free ID details."""
    try:
        frozen = load_frozen_corpus(manifest, manifest_sha256=manifest_sha256)
        frozen_recipe = _recipe(_read_pinned(recipe, recipe_sha256, "recipe"))
        sources, modes = _admit(_read_pinned(outcomes, outcomes_sha256, "outcomes"), frozen, frozen_recipe, recipe_sha256)
        source_rows = []
        for document in frozen.annotations["documents"]:
            extraction = sources[document["id"]]["extraction"]
            candidates = extraction["candidates"]
            gold, predicted = _mention_signatures(document, candidates)
            english = document["language"] == "en"
            state, failure = extraction["state"], extraction["failure"]
            source_rows.append({
                "document_id": document["id"], "partition": document["partition"], "english": english,
                "state": state, "failure": failure,
                "proven_abstention": not english and (state in ("absent", "unsupported_language")
                                                      or state == "rejected" and failure == "UnsupportedLanguage"),
                "mentions": _agreement(Counter(gold.values()), Counter(predicted.values()), match=english),
                **{field: _endpoint_score(document, candidates, field, gold, predicted) for field in ("aliases", "relations")},
            })
        summary = {
            "schema": REPORT_SCHEMA,
            "input_sha256": dict(frozen.file_sha256) | {
                "recipe": recipe_sha256, "outcomes": outcomes_sha256,
                "scorer": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            },
            "recipe_bindings": {key: frozen_recipe[key] for key in ("candidate_freeze_sha256", "development_selection_sha256")},
            "counts": frozen.summary()["counts"],
            "complete_topic_gold_readiness": frozen.summary()["complete_topic_gold_readiness"],
            "not_measured": dict(UNMEASURED),
            "extraction": _aggregate_extraction(source_rows),
            "partitions": {partition: _aggregate_extraction([row for row in source_rows if row["partition"] == partition])
                           for partition in ("development", "held_out")},
            "modes": {},
        }
        details: dict[str, Any] = {"schema": REPORT_SCHEMA, "input_sha256": summary["input_sha256"],
                                   "not_measured": dict(UNMEASURED),
                                   "sources": sorted(source_rows, key=lambda row: row["document_id"]), "modes": {}}
        for name in MODES:
            summary["modes"][name], details["modes"][name] = _score_mode(modes[name], frozen)
        return summary, details
    except (CorpusValidationError, OSError, TypeError, ValueError, KeyError, RuntimeError, RecursionError, OverflowError):
        raise OutcomeValidationError(FAILURE_MESSAGE) from None


def write_details(path: str | Path, details: dict[str, Any]) -> None:
    """Use directory descriptors to reject symlinks, traversal and overwrite races."""
    descriptor = None
    try:
        candidate = Path(_string(str(path), 4096))
        _require(".." not in candidate.parts)
        if candidate.is_absolute():
            candidate = candidate.relative_to(REPOSITORY_ROOT)
        _require(bool(candidate.parts))
        descriptor = os.open(REPOSITORY_ROOT, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        for part in candidate.parts[:-1]:
            next_descriptor = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        output = os.open(candidate.name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                         0o600, dir_fd=descriptor)
        with os.fdopen(output, "wb") as stream:
            os.fchmod(stream.fileno(), 0o600)
            stream.write(_encoded(details))
            stream.flush()
            os.fsync(stream.fileno())
    except (OSError, TypeError, ValueError, RuntimeError):
        raise OutcomeValidationError(FAILURE_MESSAGE) from None
    finally:
        if descriptor is not None:
            os.close(descriptor)


class _ArgumentParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        self.exit(2, "offline outcome arguments invalid\n")


class _Once(argparse.Action):
    def __call__(self, parser, namespace, values, option_string=None):
        if getattr(namespace, self.dest) is not None:
            parser.error("repeated option")
        setattr(namespace, self.dest, values)


def main(argv: list[str] | None = None) -> int:
    parser = _ArgumentParser(description=__doc__, allow_abbrev=False)
    for name in ("manifest", "manifest-sha256", "recipe", "recipe-sha256", "outcomes", "outcomes-sha256"):
        parser.add_argument(f"--{name}", required=True, action=_Once)
    parser.add_argument("--details", action=_Once)
    args = parser.parse_args(argv)
    try:
        summary, details = evaluate(args.manifest, manifest_sha256=args.manifest_sha256,
                                    recipe=args.recipe, recipe_sha256=args.recipe_sha256,
                                    outcomes=args.outcomes, outcomes_sha256=args.outcomes_sha256)
        if args.details is not None:
            write_details(args.details, details)
    except OutcomeValidationError:
        print(FAILURE_MESSAGE, file=sys.stderr)
        return 1
    print(_encoded(summary).decode("utf-8"), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
