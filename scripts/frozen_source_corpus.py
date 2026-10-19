#!/usr/bin/env python3
"""Offline, additive admission of frozen annotations; never evaluates a model.

Usage::

    python3 -B scripts/frozen_source_corpus.py MANIFEST --sha256 MANIFEST_SHA256

``load_frozen_corpus(path, manifest_sha256=...)`` returns raw annotations and
typed topic pairs for a future authorized caller. This is neither native-origin
admission nor an integration with any existing scorer. Scope tokens and identity
preconditions remain annotation data, never canonical identity proof.

The explicit JSON manifest has exactly these fields (all hashes are lowercase
raw-file SHA-256): ``schema`` = ``recollect-frozen-source-corpus-manifest-1``,
``corpus_path``, ``corpus_sha256``, ``corpus_schema``, ``guide_path``,
``guide_sha256``, ``source_hashes_path``, ``source_hashes_sha256``,
``interface_path``, and ``interface_sha256``. Bound paths are repository-relative;
the manifest itself may be an absolute or repository-relative path. All resolved
files must stay inside this repository; there is no discovery or network I/O.
The manifest's nonempty corpus schema must match both the interface declaration
and corpus; the interface version, structural fields and enums remain strict.
CLI gold readiness describes annotation coverage only. Producer or topic-recipe
qualification is always explicitly ``NOT_MEASURED`` by this offline loader.

Limits reject the whole input, never select a prefix: 250 documents, 32 MiB
corpus, 2 MiB per source/guide, 256 KiB hash table, 64 KiB interface, 16 KiB
manifest, depth 32 and 1,000,000 JSON values per file. Each document admits at most
128 mentions, 128 aliases, 256 relations and 256 topics. Strings are bounded by
the source limit; identifiers by 256 bytes. Empty source means exactly ``""``.
The CLI emits only counts, raw-file bindings and topic readiness; all failures
use generic errors without paths, source IDs, labels or annotation content.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import itertools
import json
from pathlib import Path
import re
import sys
from typing import Any


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MANIFEST_SCHEMA = "recollect-frozen-source-corpus-manifest-1"
INTERFACE_SCHEMA = "recollect-unseen-organization-interface-descriptor-1"
MAX_DOCUMENTS = 250
MAX_SOURCE_BYTES = 2 * 1024 * 1024
MAX_JSON_DEPTH = 32
MAX_JSON_VALUES = 1_000_000
FILE_LIMITS = {
    "manifest": 16 * 1024,
    "corpus": 32 * 1024 * 1024,
    "guide": MAX_SOURCE_BYTES,
    "source_hashes": 256 * 1024,
    "interface": 64 * 1024,
}
KINDS = {"person", "technology", "service", "environment", "repository", "concept", "setting"}
ALIAS_DISPOSITIONS = {
    "supported_equivalence", "rejected_equivalence", "incompatible_scope",
    "unresolved_equivalence",
}
RELATION_DISPOSITIONS = {"supported_navigation", "contradicted_navigation"}
RELATIONS = {
    "uses", "configured_by", "depends_on", "located_in", "implements",
    "related_to", "does_not_use", "has_owner",
}
IDENTITY_DISPOSITIONS = {"must_match", "must_differ", "unresolved"}
TOPIC_DISPOSITIONS = {
    "related_ prefix", "unrelated", "unrelated_empty",
    "cross_Brain_no_membership_or_count", "unknown_topic_coverage",
}
TOP_LEVEL = {
    "schema": "string", "metadata": "object", "documents": "Document[]",
    "topic_pairs": "TopicPair[]", "identity_controls": "IdentityControl[]",
    "topic_pair_metadata": "object[]",
}
DOCUMENT_FIELDS = {
    "id", "title", "partition", "language", "scope", "text", "mentions",
    "aliases", "relations", "topics", "metadata",
}
DOCUMENT_DECLARATIONS = {
    "id": "string", "title": "string", "partition": "development|held_out",
    "language": "string",
    "scope": "three slash-separated symbolic fields: Brain/repository/environment",
    "text": "string exact immutable UTF-8 source", "mentions": "Mention[]",
    "aliases": "Alias[]", "relations": "Relation[]", "topics": "string[]",
    "metadata": "annotation metadata, including explicit topic_annotation_state=complete|unknown",
}
RECORD_FIELDS = {
    "Mention": ["localKey:string", "kind:Kind", "byteStart:integer", "byteEnd:integer", "exactQuote:string"],
    "Alias": ["leftKey:string", "rightKey:string", "disposition:AliasDisposition", "byteStart:integer", "byteEnd:integer", "exactEvidence:string"],
    "Relation": ["fromKey:string", "toKey:string", "relation:RelationKind", "disposition:RelationDisposition", "byteStart:integer", "byteEnd:integer", "exactEvidence:string"],
    "TopicPair": ["leftDocumentId:string", "rightDocumentId:string", "disposition:TopicPairDisposition"],
}
HASH_PATTERN = re.compile(r"[0-9a-f]{64}\Z")
MENTION_KEY_PATTERN = re.compile(r"[A-Za-z0-9_.-]{1,80}\Z")


class CorpusValidationError(ValueError):
    """Content-free admission failure. Never include rejected input in messages."""


def _require(condition: bool) -> None:
    if not condition:
        raise CorpusValidationError("frozen corpus validation failed")


def _string(value: Any, *, nonempty: bool = True, limit: int = 256) -> str:
    _require(isinstance(value, str))
    try:
        size = len(value.encode("utf-8"))
    except UnicodeError:
        raise CorpusValidationError("frozen corpus validation failed") from None
    _require(size <= limit and (bool(value) or not nonempty))
    return value


def _object(value: Any, keys: set[str] | None = None) -> dict[str, Any]:
    _require(isinstance(value, dict))
    if keys is not None:
        _require(set(value) == keys)
    return value


def _array(value: Any, maximum: int) -> list[Any]:
    _require(isinstance(value, list) and len(value) <= maximum)
    return value


def _record(value: Any, length: int) -> list[Any]:
    _require(isinstance(value, list) and len(value) == length)
    return value


def _json_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        _require(key not in result)
        result[key] = value
    return result


def _invalid_constant(_: str) -> None:
    raise CorpusValidationError("frozen corpus validation failed")


def _preflight_json(text: str) -> None:
    """Bound valid JSON allocation before parsing, without accepting syntax.

    Count value tokens and containers outside strings; a colon subtracts its
    preceding object-key token. The stdlib parser still decides all syntax and
    the post-parse walk enforces exact depth/value limits. One transient key
    token is allowed before its colon so valid objects at the limit can pass.
    """
    count = depth = 0
    quoted = escaped = scalar = False
    for char in text:
        if quoted:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
            continue
        if char in " \t\r\n":
            scalar = False
            continue
        if char == '"':
            quoted = True
            scalar = False
            count += 1
        elif char in "[{":
            depth += 1
            count += 1
            scalar = False
        elif char in "]}":
            depth -= 1
            scalar = False
        elif char == ":":
            count -= 1
            scalar = False
        elif char == ",":
            scalar = False
        elif not scalar:
            count += 1
            scalar = True
        _require(count <= MAX_JSON_VALUES + 1 and depth <= MAX_JSON_DEPTH + 1)


def _parse_json(raw: bytes) -> Any:
    try:
        decoded = raw.decode("utf-8")
        _preflight_json(decoded)
        value = json.loads(decoded, object_pairs_hook=_json_pairs,
                           parse_constant=_invalid_constant)
        stack = [(value, 0)]
        count = 0
        while stack:
            item, depth = stack.pop()
            count += 1
            _require(depth <= MAX_JSON_DEPTH and count <= MAX_JSON_VALUES)
            if isinstance(item, str):
                _string(item, nonempty=False, limit=MAX_SOURCE_BYTES)
            elif isinstance(item, dict):
                for key, child in item.items():
                    _string(key, limit=256)
                    stack.append((child, depth + 1))
            elif isinstance(item, list):
                stack.extend((child, depth + 1) for child in item)
            elif isinstance(item, float):
                # JSON exponents can overflow even without NaN/Infinity literals.
                _require(float("-inf") < item < float("inf"))
        return value
    except (UnicodeError, ValueError, RecursionError, OverflowError):
        raise CorpusValidationError("frozen corpus validation failed") from None


def _path(value: Any, *, manifest: bool = False) -> Path:
    text = _string(str(value) if isinstance(value, Path) else value, limit=4096)
    candidate = Path(text)
    _require(".." not in candidate.parts and (manifest or not candidate.is_absolute()))
    try:
        resolved = (REPOSITORY_ROOT / candidate).resolve(strict=True)
        _require(resolved.is_relative_to(REPOSITORY_ROOT) and resolved.is_file())
    except (OSError, ValueError, RuntimeError):
        raise CorpusValidationError("frozen corpus validation failed") from None
    return resolved


def _read_pinned(path: Path, digest: Any, kind: str) -> bytes:
    _require(isinstance(digest, str) and HASH_PATTERN.fullmatch(digest) is not None)
    try:
        with path.open("rb") as stream:
            raw = stream.read(FILE_LIMITS[kind] + 1)
        _require(len(raw) <= FILE_LIMITS[kind])
        _require(hashlib.sha256(raw).hexdigest() == digest)
        raw.decode("utf-8")
    except (OSError, UnicodeError):
        raise CorpusValidationError("frozen corpus validation failed") from None
    return raw


def _validate_interface(value: Any, corpus_schema: str) -> None:
    interface = _object(value)
    _require(set(interface) <= {
        "schema", "corpus_schema_version", "scope", "top_level_required_keys",
        "Document", "Mention", "Alias", "Relation", "TopicPair", "enums",
        "source_hash_manifest", "revision_metadata_compatibility", "encoding_rules",
        "identity_eligibility_preconditions", "topic_coverage_state", "unknown_pair_semantics",
    })
    _require(interface.get("schema") == INTERFACE_SCHEMA)
    _require(corpus_schema == _string(interface.get("corpus_schema_version")))
    _require(interface.get("top_level_required_keys") == TOP_LEVEL)
    _require(interface.get("Document") == DOCUMENT_DECLARATIONS)
    for key, expected in RECORD_FIELDS.items():
        _require(interface.get(key) == expected)
    enums = _object(interface.get("enums"), {
        "Kind", "AliasDisposition", "RelationDisposition", "RelationKind",
        "TopicPairDisposition", "IdentityDisposition",
    })
    for name, expected in (
        ("Kind", KINDS), ("AliasDisposition", ALIAS_DISPOSITIONS),
        ("RelationDisposition", RELATION_DISPOSITIONS), ("RelationKind", RELATIONS),
        ("TopicPairDisposition", TOPIC_DISPOSITIONS),
        ("IdentityDisposition", IDENTITY_DISPOSITIONS),
    ):
        declared = _array(enums[name], len(expected))
        _require(all(isinstance(item, str) for item in declared))
        _require(set(declared) == expected and len(declared) == len(expected))
    _require(interface.get("source_hash_manifest") == {
        "schema": "string",
        "documents": "map document ID -> lowercase SHA256 hex of exact UTF-8 text",
    })
    coverage = _object(interface.get("topic_coverage_state"))
    _require(coverage.get("location") == "documents[].metadata.topic_annotation_state")
    _require(coverage.get("enum") == ["complete", "unknown"])
    unknown = _object(interface.get("unknown_pair_semantics"))
    _require(unknown.get("disposition") == "unknown_topic_coverage")
    _require("truth" in unknown and unknown["truth"] is None)


def _span(text: str, start: Any, end: Any, quote: Any) -> tuple[int, int]:
    _require(type(start) is int and type(end) is int)
    encoded = text.encode("utf-8")
    _require(0 <= start < end <= len(encoded))
    _string(quote, limit=MAX_SOURCE_BYTES)
    try:
        _require(encoded[:start].decode("utf-8") is not None)
        selected = encoded[start:end].decode("utf-8")
    except UnicodeError:
        raise CorpusValidationError("frozen corpus validation failed") from None
    _require(selected == quote)
    return start, end


def _document(value: Any) -> tuple[dict[str, Any], dict[str, tuple[int, int]]]:
    document = _object(value, DOCUMENT_FIELDS)
    _string(document["id"])
    _string(document["title"], nonempty=False, limit=4096)
    _require(document["partition"] in ("development", "held_out"))
    _string(document["language"])
    scope = _string(document["scope"])
    _require(len(scope.split("/")) == 3 and all(scope.split("/")))
    text = _string(document["text"], nonempty=False, limit=MAX_SOURCE_BYTES)
    metadata = _object(document["metadata"])
    _require(metadata.get("topic_annotation_state") in ("complete", "unknown"))
    topics = _array(document["topics"], 256)
    for topic in topics:
        _string(topic)
    _require(len(set(topics)) == len(topics))
    mentions: dict[str, tuple[int, int]] = {}
    occurrences: set[tuple[Any, ...]] = set()
    for row in _array(document["mentions"], 128):
        key, kind, start, end, quote = _record(row, 5)
        _require(isinstance(key, str) and MENTION_KEY_PATTERN.fullmatch(key) is not None)
        _require(key not in mentions and isinstance(kind, str) and kind in KINDS)
        span = _span(text, start, end, quote)
        occurrence = (kind, start, end, quote)
        _require(occurrence not in occurrences)
        occurrences.add(occurrence)
        mentions[key] = span
    for field, length, maximum in (("aliases", 6, 128), ("relations", 7, 256)):
        seen: set[tuple[Any, ...]] = set()
        for raw in _array(document[field], maximum):
            row = _record(raw, length)
            left, right = row[:2]
            _require(isinstance(left, str) and isinstance(right, str))
            _require(left != right and left in mentions and right in mentions)
            if field == "aliases":
                disposition, start, end, quote = row[2:]
                _require(isinstance(disposition, str) and disposition in ALIAS_DISPOSITIONS)
            else:
                relation, disposition, start, end, quote = row[2:]
                _require(isinstance(relation, str) and relation in RELATIONS)
                _require(isinstance(disposition, str) and disposition in RELATION_DISPOSITIONS)
            first, last = _span(text, start, end, quote)
            _require(all(first <= mentions[key][0] and mentions[key][1] <= last
                         for key in (left, right)))
            duplicate_key = tuple(row)
            if field == "aliases":
                duplicate_key = (*sorted((left, right)), *row[2:])
            _require(duplicate_key not in seen)
            seen.add(duplicate_key)
    _document_metadata(document, mentions)
    return document, mentions


def _document_metadata(document: dict[str, Any], mentions: dict[str, tuple[int, int]]) -> None:
    """Validate declared optional reference surfaces; preserve opaque metadata."""
    metadata = document["metadata"]
    if "ambiguities" in metadata:
        for ambiguity in _array(metadata["ambiguities"], 1024):
            _object(ambiguity)
    if "coverage" in metadata:
        coverage = _object(metadata["coverage"])
        for key, allowed in (
            ("explicit_alias_clauses", {"complete", "complete_no_explicit_alias_clause"}),
            ("explicit_relation_clauses", {"complete", "complete_no_explicit_expressible_binary_clause"}),
        ):
            if key in coverage:
                _require(isinstance(coverage[key], str) and coverage[key] in allowed)
        for key in ("unlisted_combinatorial_pairs", "topic_gold"):
            if key in coverage:
                _string(coverage[key], limit=MAX_SOURCE_BYTES)
    if "disposition_ledger" not in metadata:
        return
    for raw in _array(metadata["disposition_ledger"], 1024):
        row = _object(raw)
        required = {"kind", "disposition", "evidence_quote", "occurrence", "assertion_eligibility"}
        _require(required <= set(row))
        _require(row["kind"] in ("alias", "relation"))
        _string(row["assertion_eligibility"], nonempty=False, limit=MAX_SOURCE_BYTES)
        evidence = _string(row["evidence_quote"], limit=MAX_SOURCE_BYTES)
        occurrence = row["occurrence"]
        _require(type(occurrence) is int and 1 <= occurrence <= MAX_SOURCE_BYTES)
        # Explicit occurrence is 1-based and never repaired to another quote.
        start = -1
        for _ in range(occurrence):
            start = document["text"].find(evidence, start + 1)
            _require(start >= 0)
        byte_start = len(document["text"][:start].encode("utf-8"))
        byte_end = byte_start + len(evidence.encode("utf-8"))
        if row["kind"] == "alias":
            _require({"left", "right"} <= set(row))
            left, right = row["left"], row["right"]
            target = [left, right, row["disposition"], byte_start, byte_end, evidence]
            candidates = document["aliases"]
        else:
            _require({"from", "to", "relation"} <= set(row))
            left, right = row["from"], row["to"]
            target = [left, right, row["relation"], row["disposition"], byte_start, byte_end, evidence]
            candidates = document["relations"]
        _require(isinstance(left, str) and isinstance(right, str))
        _require(left in mentions and right in mentions)
        _require(target in candidates)
        _require(all(byte_start <= mentions[key][0] and mentions[key][1] <= byte_end
                     for key in (left, right)))


@dataclass(frozen=True)
class TopicPair:
    left_document_id: str
    right_document_id: str
    disposition: str
    truth: bool | None


@dataclass(frozen=True)
class FrozenCorpus:
    """Validated raw annotations and explicit three-value topic truth."""

    annotations: dict[str, Any]
    topic_pairs: tuple[TopicPair, ...]
    file_sha256: dict[str, str]

    def summary(self) -> dict[str, Any]:
        """Return content-free admission evidence, without scores or identities."""
        docs = self.annotations["documents"]
        unknown_sources = sum(doc["metadata"]["topic_annotation_state"] == "unknown" for doc in docs)
        unknown_pairs = sum(pair.truth is None for pair in self.topic_pairs)
        return {
            "schema": "recollect-frozen-source-corpus-validation-1",
            "file_sha256": dict(self.file_sha256),
            "counts": {
                "sources": len(docs),
                "mentions": sum(len(doc["mentions"]) for doc in docs),
                "aliases": sum(len(doc["aliases"]) for doc in docs),
                "relations": sum(len(doc["relations"]) for doc in docs),
                "identity_controls": len(self.annotations["identity_controls"]),
                "topic_sources_known": len(docs) - unknown_sources,
                "topic_sources_unknown": unknown_sources,
                "topic_pairs": len(self.topic_pairs),
                "topic_pairs_positive": sum(pair.truth is True for pair in self.topic_pairs),
                "topic_pairs_negative": sum(pair.truth is False for pair in self.topic_pairs),
                "topic_pairs_unknown": unknown_pairs,
            },
            "complete_topic_gold_readiness": "NOT_READY" if unknown_sources or unknown_pairs else "READY",
            "producer_or_topic_recipe_qualification": "NOT_MEASURED",
        }


def _validate_annotations(corpus: Any, source_hashes: Any, corpus_schema: str) -> tuple[TopicPair, ...]:
    corpus = _object(corpus, set(TOP_LEVEL))
    _require(corpus["schema"] == corpus_schema)
    _object(corpus["metadata"])
    documents = _array(corpus["documents"], MAX_DOCUMENTS)
    _require(bool(documents))
    by_id: dict[str, dict[str, Any]] = {}
    mention_maps: dict[str, dict[str, tuple[int, int]]] = {}
    for raw in documents:
        document, mentions = _document(raw)
        key = document["id"]
        _require(key not in by_id)
        by_id[key] = document
        mention_maps[key] = mentions
    hashes = _object(source_hashes, {"schema", "documents"})
    _string(hashes["schema"])
    source_map = _object(hashes["documents"])
    _require(set(source_map) == set(by_id))
    for key, document in by_id.items():
        _require(isinstance(source_map[key], str) and HASH_PATTERN.fullmatch(source_map[key]) is not None)
        _require(source_map[key] == hashlib.sha256(document["text"].encode("utf-8")).hexdigest())

    expected_pairs = set(itertools.combinations(sorted(by_id), 2))
    seen_pairs: set[tuple[str, str]] = set()
    topic_pairs: list[TopicPair] = []
    for row in _array(corpus["topic_pairs"], MAX_DOCUMENTS * (MAX_DOCUMENTS - 1) // 2):
        left, right, disposition = _record(row, 3)
        _require(isinstance(left, str) and isinstance(right, str) and left != right)
        _require(left in by_id and right in by_id)
        pair = tuple(sorted((left, right)))
        _require(pair not in seen_pairs)
        seen_pairs.add(pair)
        _string(disposition)
        positive = disposition.startswith("related_") and len(disposition) > len("related_")
        _require(positive or disposition in TOPIC_DISPOSITIONS - {"related_ prefix"})
        a, b = by_id[left], by_id[right]
        if a["scope"].split("/")[0] != b["scope"].split("/")[0]:
            _require(disposition == "cross_Brain_no_membership_or_count")
        elif a["text"] == "" or b["text"] == "":
            _require(disposition == "unrelated_empty")
        else:
            _require(disposition not in {"unrelated_empty", "cross_Brain_no_membership_or_count"})
            if any(doc["metadata"]["topic_annotation_state"] == "unknown" for doc in (a, b)):
                _require(disposition == "unknown_topic_coverage")
        truth = None if disposition == "unknown_topic_coverage" else positive
        topic_pairs.append(TopicPair(left, right, disposition, truth))
    _require(seen_pairs == expected_pairs)

    identity_ids: set[str] = set()
    identity_pairs: set[tuple[tuple[str, str], tuple[str, str]]] = set()
    for raw in _array(corpus["identity_controls"], MAX_DOCUMENTS * 128):
        control = _object(raw)
        required = {"id", "left", "right", "disposition", "reason", "validation_boundary"}
        _require(required <= set(control) <= required | {"eligibility_preconditions"})
        key = _string(control["id"])
        _require(key not in identity_ids)
        identity_ids.add(key)
        _require(isinstance(control["disposition"], str) and control["disposition"] in IDENTITY_DISPOSITIONS)
        _string(control["reason"], nonempty=False, limit=MAX_SOURCE_BYTES)
        _string(control["validation_boundary"], nonempty=False, limit=MAX_SOURCE_BYTES)
        if "eligibility_preconditions" in control:
            preconditions = _object(control["eligibility_preconditions"], {
                "current_contribution_requires", "withdrawn_source_may_contribute_current_support",
                "identity_semantics",
            })
            _string(preconditions["current_contribution_requires"], limit=MAX_SOURCE_BYTES)
            _require(type(preconditions["withdrawn_source_may_contribute_current_support"]) is bool)
            _string(preconditions["identity_semantics"], limit=MAX_SOURCE_BYTES)
        endpoints = []
        for side in ("left", "right"):
            endpoint = _object(control[side], {"document_id", "mention_key"})
            doc_id = _string(endpoint["document_id"])
            mention_key = _string(endpoint["mention_key"])
            _require(doc_id in mention_maps and mention_key in mention_maps[doc_id])
            endpoints.append((doc_id, mention_key))
        _require(endpoints[0] != endpoints[1])
        pair = tuple(sorted(endpoints))
        _require(pair not in identity_pairs)
        identity_pairs.add(pair)
    metadata_rows = _array(corpus["topic_pair_metadata"], len(expected_pairs))
    _require(len(metadata_rows) == len(topic_pairs))
    for pair, raw in zip(topic_pairs, metadata_rows):
        metadata = _object(raw, {
            "left", "right", "disposition", "reason", "shared_topics",
            "cross_brain_boundary", "same_repository_scope_token", "same_environment_scope_token",
            "label_usefulness", "unsupported_language_in_pair", "lifecycle_scenario_note",
            "topic_truth_status", "scored_topic_positive_or_negative", "topic_coverage_reason",
        })
        _require(metadata["left"] == pair.left_document_id and metadata["right"] == pair.right_document_id)
        _require(metadata["disposition"] == pair.disposition)
        _string(metadata["reason"], nonempty=False, limit=MAX_SOURCE_BYTES)
        _string(metadata["label_usefulness"], nonempty=False, limit=MAX_SOURCE_BYTES)
        left, right = by_id[pair.left_document_id], by_id[pair.right_document_id]
        left_scope, right_scope = left["scope"].split("/"), right["scope"].split("/")
        for key, expected in (
            ("cross_brain_boundary", left_scope[0] != right_scope[0]),
            ("same_repository_scope_token", left_scope[1] == right_scope[1]),
            ("same_environment_scope_token", left_scope[2] == right_scope[2]),
            ("scored_topic_positive_or_negative", pair.truth is not None),
        ):
            _require(type(metadata[key]) is bool and metadata[key] == expected)
        _require(type(metadata["unsupported_language_in_pair"]) is bool)
        topics = _array(metadata["shared_topics"], 256)
        for topic in topics:
            _string(topic)
        _require(len(topics) == len(set(topics)))
        _require(set(topics) == set(left["topics"]) & set(right["topics"]))
        _require(metadata["topic_truth_status"] == ("unknown" if pair.truth is None else "known"))
        for key in ("lifecycle_scenario_note", "topic_coverage_reason"):
            if metadata[key] is not None:
                _string(metadata[key], limit=MAX_SOURCE_BYTES)
        if pair.truth is None:
            _string(metadata["topic_coverage_reason"], limit=MAX_SOURCE_BYTES)
    if "annotation_coverage" in corpus["metadata"]:
        coverage = _object(corpus["metadata"]["annotation_coverage"])
        if "unlisted_endpoint_pairs" in coverage:
            _string(coverage["unlisted_endpoint_pairs"], limit=MAX_SOURCE_BYTES)
    return tuple(topic_pairs)


def load_frozen_corpus(manifest_path: str | Path, *, manifest_sha256: str) -> FrozenCorpus:
    """Validate one explicitly pinned local corpus. Never invoke providers/scorers."""
    try:
        path = _path(manifest_path, manifest=True)
        manifest = _object(_parse_json(_read_pinned(path, manifest_sha256, "manifest")))
        keys = {"schema", "corpus_schema"} | {
            f"{kind}_{suffix}" for kind in ("corpus", "guide", "source_hashes", "interface")
            for suffix in ("path", "sha256")
        }
        _require(set(manifest) == keys and manifest["schema"] == MANIFEST_SCHEMA)
        corpus_schema = _string(manifest["corpus_schema"])
        paths = {kind: _path(manifest[f"{kind}_path"])
                 for kind in ("corpus", "guide", "source_hashes", "interface")}
        _require(len({path, *paths.values()}) == 5)
        raw = {kind: _read_pinned(file, manifest[f"{kind}_sha256"], kind)
               for kind, file in paths.items()}
        _validate_interface(_parse_json(raw["interface"]), corpus_schema)
        corpus = _parse_json(raw["corpus"])
        pairs = _validate_annotations(corpus, _parse_json(raw["source_hashes"]), corpus_schema)
        bindings = {"manifest": manifest_sha256} | {
            kind: manifest[f"{kind}_sha256"] for kind in paths
        }
        return FrozenCorpus(corpus, pairs, bindings)
    except (CorpusValidationError, OSError, TypeError, ValueError, KeyError, RecursionError):
        raise CorpusValidationError("frozen corpus validation failed") from None


class _ArgumentParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        self.exit(2, "frozen corpus arguments invalid\n")


def main(argv: list[str] | None = None) -> int:
    parser = _ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("manifest")
    parser.add_argument("--sha256", required=True, help="exact raw manifest SHA-256")
    args = parser.parse_args(argv)
    try:
        loaded = load_frozen_corpus(args.manifest, manifest_sha256=args.sha256)
    except CorpusValidationError:
        print("frozen corpus validation failed", file=sys.stderr)
        return 1
    print(json.dumps(loaded.summary(), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
