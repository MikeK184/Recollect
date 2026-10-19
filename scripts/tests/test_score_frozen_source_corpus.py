"""Independent, newly authored offline evaluator toys. Never open private data."""

from contextlib import redirect_stderr, redirect_stdout
import copy
import hashlib
import importlib.util
import io
import itertools
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import uuid


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location("score_frozen_source_corpus", ROOT / "scripts/score_frozen_source_corpus.py")
scorer = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = scorer
SPEC.loader.exec_module(scorer)
loader = sys.modules["frozen_source_corpus"]
PROOF_ROOT = ROOT / ".cache/automatic-mapping/offline-outcome-evaluator-2026-10-18"
CORPUS_SCHEMA = "independent-offline-evaluator-toy-1"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n").encode("utf-8")


def span(text, quote, occurrence=1):
    start = -1
    for _ in range(occurrence):
        start = text.index(quote, start + 1)
    first = len(text[:start].encode("utf-8"))
    return [first, first + len(quote.encode("utf-8")), quote]


def document(key, text, *, language="en", brain="north", unknown=False):
    return {
        "id": key, "title": "TITLE_SENTINEL_DO_NOT_PRINT", "partition": "held_out", "language": language,
        "scope": f"{brain}/archive/stage", "text": text, "mentions": [], "aliases": [], "relations": [],
        "topics": [], "metadata": {"topic_annotation_state": "unknown" if unknown else "complete"},
    }


def author_pairs(corpus):
    pairs, metadata = [], []
    for a, b in itertools.combinations(corpus["documents"], 2):
        sa, sb = a["scope"].split("/"), b["scope"].split("/")
        if sa[0] != sb[0]:
            disposition = "cross_Brain_no_membership_or_count"
        elif a["text"] == "" or b["text"] == "":
            disposition = "unrelated_empty"
        elif any(d["metadata"]["topic_annotation_state"] == "unknown" for d in (a, b)):
            disposition = "unknown_topic_coverage"
        elif {a["id"], b["id"]} == {"ID_SENTINEL_A", "B"}:
            disposition = "related_independent_route"
        else:
            disposition = "unrelated"
        unknown = disposition == "unknown_topic_coverage"
        pairs.append([a["id"], b["id"], disposition])
        metadata.append({
            "left": a["id"], "right": b["id"], "disposition": disposition,
            "reason": "PAIR_REASON_SENTINEL", "shared_topics": sorted(set(a["topics"]) & set(b["topics"])),
            "cross_brain_boundary": sa[0] != sb[0], "same_repository_scope_token": sa[1] == sb[1],
            "same_environment_scope_token": sa[2] == sb[2], "label_usefulness": "LABEL_SENTINEL",
            "unsupported_language_in_pair": a["language"] != "en" or b["language"] != "en",
            "lifecycle_scenario_note": None, "topic_truth_status": "unknown" if unknown else "known",
            "scored_topic_positive_or_negative": not unknown,
            "topic_coverage_reason": "UNKNOWN_REASON_SENTINEL" if unknown else None,
        })
    corpus["topic_pairs"], corpus["topic_pair_metadata"] = pairs, metadata


def corpus_fixture():
    a = document("ID_SENTINEL_A", "🪶 Élan aka Elan. Élan uses Cobalt. Elan waits.")
    text = a["text"]
    a["mentions"] = [
        ["accent1", "service", *span(text, "Élan")],
        ["plain1", "service", *span(text, "Elan")],
        ["accent2", "service", *span(text, "Élan", 2)],
        ["cobalt", "technology", *span(text, "Cobalt")],
        ["plain2", "service", *span(text, "Elan", 2)],
    ]
    a["aliases"] = [
        ["accent1", "plain1", "supported_equivalence", *span(text, "Élan aka Elan.")],
        ["plain1", "accent2", "rejected_equivalence", *span(text, "Elan. Élan uses Cobalt.")],
        ["accent1", "accent2", "incompatible_scope", *span(text, "Élan aka Elan. Élan uses Cobalt.")],
        ["plain1", "plain2", "unresolved_equivalence", *span(text, "Elan. Élan uses Cobalt. Elan waits.")],
    ]
    a["relations"] = [
        ["accent2", "cobalt", "uses", "supported_navigation", *span(text, "Élan uses Cobalt.")],
        ["accent1", "cobalt", "uses", "contradicted_navigation", *span(text, "Élan aka Elan. Élan uses Cobalt.")],
    ]
    a["topics"] = ["TOPIC_SENTINEL"]
    b = document("B", "Vesper route.")
    b["mentions"] = [["vesper", "service", *span(b["text"], "Vesper")]]
    b["partition"] = "development"
    other = copy.deepcopy(a)
    other["id"], other["scope"] = "X", "south/archive/stage"
    corpus = {
        "schema": CORPUS_SCHEMA, "metadata": {"note": "METADATA_SENTINEL"},
        "documents": [a, b, document("E", ""), other, document("U", "Juniper traces.", unknown=True),
                      document("N", "La brise souffle.", language="fr")],
        "topic_pairs": [], "topic_pair_metadata": [], "identity_controls": [],
    }
    author_pairs(corpus)
    return corpus


def interface_fixture():
    # Structural declarations only. No import of any old corpus fixture body.
    return {
        "schema": loader.INTERFACE_SCHEMA, "corpus_schema_version": CORPUS_SCHEMA,
        "top_level_required_keys": dict(loader.TOP_LEVEL), "Document": dict(loader.DOCUMENT_DECLARATIONS),
        **{key: list(value) for key, value in loader.RECORD_FIELDS.items()},
        "enums": {key: sorted(value) for key, value in (
            ("Kind", loader.KINDS), ("AliasDisposition", loader.ALIAS_DISPOSITIONS),
            ("RelationDisposition", loader.RELATION_DISPOSITIONS), ("RelationKind", loader.RELATIONS),
            ("TopicPairDisposition", loader.TOPIC_DISPOSITIONS), ("IdentityDisposition", loader.IDENTITY_DISPOSITIONS),
        )},
        "source_hash_manifest": {"schema": "string", "documents": "map document ID -> lowercase SHA256 hex of exact UTF-8 text"},
        "topic_coverage_state": {"location": "documents[].metadata.topic_annotation_state", "enum": ["complete", "unknown"]},
        "unknown_pair_semantics": {"disposition": "unknown_topic_coverage", "truth": None},
    }


def recipe_fixture():
    return {
        "schema": scorer.RECIPE_SCHEMA, "candidate_freeze_sha256": sha(b"independent frozen candidate"),
        "development_selection_sha256": sha(b"independent development selection"),
        "producer": {"prompt_label": "source-facts-4", "schema_label": "source-facts-4",
                     "adapter_revision": "learning-quote-anchors-3", "mapping_schema": "source-mentions-1"},
        "parameters": dict(scorer.PARAMETERS), "modes": list(scorer.MODES),
    }


def dto(document, version):
    def edge(row, alias):
        start, end, quote = row[-3:]
        return {**({"left": row[0], "right": row[1]} if alias else {"from": row[0], "to": row[1], "relation": row[2]}),
                "span": {"byte_start": start, "byte_end": end, "quote": quote}}
    return {
        "schema": "source-mentions-1", "adapter_revision": "learning-quote-anchors-3",
        "source_version_id": version, "language": "en",
        "mentions": [{"key": key, "kind": kind, "span": {"byte_start": start, "byte_end": end, "quote": quote}, "confidence": 0.8}
                     for key, kind, start, end, quote in document["mentions"]],
        "aliases": [edge(row, True) for row in document["aliases"] if row[2] == "supported_equivalence"],
        "relations": [edge(row, False) for row in document["relations"] if row[3] == "supported_navigation"],
    }


class EvaluatorTests(unittest.TestCase):
    def setUp(self):
        PROOF_ROOT.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="independent-toy-", dir=PROOF_ROOT)
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.corpus, self.recipe = corpus_fixture(), recipe_fixture()
        self.manifest_path = self.directory / "manifest.json"
        self.recipe_path, self.outcomes_path = self.directory / "recipe.json", self.directory / "outcomes.json"
        self.freeze_corpus()
        self.outcomes = self.outcomes_fixture()
        self.write_outcomes()

    def freeze_corpus(self):
        author_pairs(self.corpus)
        files = {
            "corpus": encode(self.corpus), "guide": b"Independent toy guide; literal evidence only.\n",
            "source_hashes": encode({"schema": "independent-text-hashes-1", "documents": {
                d["id"]: sha(d["text"].encode("utf-8")) for d in self.corpus["documents"]}}),
            "interface": encode(interface_fixture()),
        }
        self.manifest = {"schema": loader.MANIFEST_SCHEMA, "corpus_schema": CORPUS_SCHEMA}
        for kind, raw in files.items():
            path = self.directory / f"{kind}.json"
            path.write_bytes(raw)
            self.manifest[f"{kind}_path"], self.manifest[f"{kind}_sha256"] = str(path.relative_to(ROOT)), sha(raw)
        raw = encode(self.manifest)
        self.manifest_path.write_bytes(raw)
        self.manifest_hash = sha(raw)
        self.recipe_path.write_bytes(encode(self.recipe))
        self.recipe_hash = sha(self.recipe_path.read_bytes())

    def outcomes_fixture(self):
        sources = []
        for index, doc in enumerate(self.corpus["documents"]):
            version = str(uuid.UUID(int=index + 17))
            extraction = {"state": "validated", "failure": None, "candidates": dto(doc, version)}
            if doc["language"] != "en":
                extraction = {"state": "unsupported_language", "failure": None, "candidates": None}
            sources.append({"document_id": doc["id"], "text_sha256": sha(doc["text"].encode("utf-8")),
                            "partition": doc["partition"], "source_version_id": version, "extraction": extraction})
        modes = []
        known = {d["id"] for d in self.corpus["documents"]}
        for name in scorer.MODES:
            rows, groups = [], []
            for doc in self.corpus["documents"]:
                key = doc["id"]
                ids = ["route"] if key in {"ID_SENTINEL_A", "B"} else (["isolated"] if key == "X" else [])
                state = "assigned" if ids else ("unsupported" if key == "N" else "missing" if key == "U" else "unassigned")
                rows.append({"document_id": key, "state": state, "ner_feature_available": doc["language"] == "en",
                             "vector_feature_available": True, "groups": ids})
            if {"ID_SENTINEL_A", "B"} <= known:
                groups.append({"id": "route", "members": ["ID_SENTINEL_A", "B"]})
            if "X" in known:
                groups.append({"id": "isolated", "members": ["X"]})
            modes.append({"name": name, "parameters": dict(scorer.PARAMETERS), "sources": rows, "groups": groups})
        return {"schema": scorer.OUTCOMES_SCHEMA,
                "corpus_bindings": {"manifest": self.manifest_hash} | {kind: self.manifest[f"{kind}_sha256"] for kind in ("corpus", "guide", "source_hashes", "interface")},
                "recipe_sha256": self.recipe_hash, "sources": sources, "modes": modes}

    def write_outcomes(self):
        self.outcomes_path.write_bytes(encode(self.outcomes))
        self.outcomes_hash = sha(self.outcomes_path.read_bytes())

    def write_recipe(self):
        self.recipe_path.write_bytes(encode(self.recipe))
        self.recipe_hash = sha(self.recipe_path.read_bytes())
        self.outcomes["recipe_sha256"] = self.recipe_hash
        self.write_outcomes()

    def evaluate(self):
        self.write_outcomes()
        return scorer.evaluate(self.manifest_path, manifest_sha256=self.manifest_hash,
                               recipe=self.recipe_path, recipe_sha256=self.recipe_hash,
                               outcomes=self.outcomes_path, outcomes_sha256=self.outcomes_hash)

    def reject(self):
        with self.assertRaisesRegex(scorer.OutcomeValidationError, "^offline outcome evaluation failed$"):
            self.evaluate()

    def mutate_reject(self, mutate):
        original = copy.deepcopy(self.outcomes)
        mutate(self.outcomes)
        self.reject()
        self.outcomes = original

    def candidates(self, index=0):
        return self.outcomes["sources"][index]["extraction"]["candidates"]

    def source_result(self, key="ID_SENTINEL_A"):
        return next(row for row in self.evaluate()[1]["sources"] if row["document_id"] == key)

    def args(self):
        return ["--manifest", str(self.manifest_path), "--manifest-sha256", self.manifest_hash,
                "--recipe", str(self.recipe_path), "--recipe-sha256", self.recipe_hash,
                "--outcomes", str(self.outcomes_path), "--outcomes-sha256", self.outcomes_hash]

    def test_exact_unicode_and_perfect_supported_scores_never_qualify(self):
        summary, details = self.evaluate()
        self.assertEqual(summary["extraction"]["mentions"], scorer._metrics(11, 0, 0))
        for field in ("aliases", "relations"):
            self.assertEqual(summary["extraction"][field]["supported_annotation_agreement"], scorer._metrics(2, 0, 0, endpoint=True))
        self.assertEqual(summary["counts"]["sources"], 6)
        self.assertEqual(summary["counts"]["topic_pairs"], 15)
        self.assertEqual(summary["complete_topic_gold_readiness"], "NOT_READY")
        self.assertEqual(set(summary["not_measured"].values()), {"NOT_MEASURED"})
        self.assertEqual(summary["not_measured"], details["not_measured"])
        self.assertNotIn("SENTINEL", json.dumps(summary))
        for sentinel in ("Élan", "Cobalt", "TOPIC_SENTINEL", "TITLE_SENTINEL", "METADATA_SENTINEL", "PAIR_REASON_SENTINEL"):
            self.assertNotIn(sentinel, json.dumps(details, ensure_ascii=False))
        for name in scorer.MODES:
            self.assertEqual(summary["modes"][name]["topic_annotation_agreement"]["recall"], 1)

    def test_local_key_renaming_and_alias_reversal_do_not_change_agreement(self):
        original = self.evaluate()[0]
        for source in self.outcomes["sources"]:
            candidates = source["extraction"]["candidates"]
            if candidates is None:
                continue
            rename = {m["key"]: f"new_{index}" for index, m in enumerate(candidates["mentions"])}
            for mention in candidates["mentions"]:
                mention["key"] = rename[mention["key"]]
            for edge in candidates["aliases"]:
                edge["left"], edge["right"] = rename[edge["right"]], rename[edge["left"]]
            for edge in candidates["relations"]:
                edge["from"], edge["to"] = rename[edge["from"]], rename[edge["to"]]
        result = self.evaluate()[0]
        original.pop("input_sha256")
        result.pop("input_sha256")
        self.assertEqual(result, original)

    def test_wrong_kind_and_reversed_relation_are_strict_unmatched_predictions(self):
        candidates = self.candidates()
        candidates["mentions"][3]["kind"] = "concept"
        candidates["relations"][0]["from"], candidates["relations"][0]["to"] = "cobalt", "accent2"
        result = self.source_result()
        self.assertEqual((result["mentions"]["tp"], result["mentions"]["fp"], result["mentions"]["fn"]), (4, 1, 1))
        self.assertEqual(result["relations"]["supported_annotation_agreement"], scorer._metrics(0, 1, 1, endpoint=True))
        self.assertEqual(result["relations"]["classification"]["unlisted_predictions"], 1)

    def test_repeated_endpoint_emissions_are_admitted_and_multiset_penalized(self):
        for field in ("aliases", "relations"):
            self.candidates()[field] *= 3
        result = self.source_result()
        for field in ("aliases", "relations"):
            self.assertEqual(result[field]["supported_annotation_agreement"], scorer._metrics(1, 2, 0, endpoint=True))
            self.assertEqual(result[field]["all_disposition_endpoint_coverage"]["matched"], 1)

    def test_all_nonpositive_dispositions_and_unlisted_pairs_have_separate_diagnostics(self):
        doc = self.corpus["documents"][0]
        all_dto = copy.deepcopy(doc)
        for row in all_dto["aliases"]:
            row[2] = "supported_equivalence"
        for row in all_dto["relations"]:
            row[3] = "supported_navigation"
        candidates = dto(all_dto, self.outcomes["sources"][0]["source_version_id"])
        candidates["aliases"].append({"left": "accent2", "right": "plain2", "span": {
            "byte_start": 0, "byte_end": len(doc["text"].encode()), "quote": doc["text"]}})
        candidates["relations"].append({"from": "cobalt", "to": "plain2", "relation": "located_in", "span": {
            "byte_start": 0, "byte_end": len(doc["text"].encode()), "quote": doc["text"]}})
        self.outcomes["sources"][0]["extraction"]["candidates"] = candidates
        result = self.source_result()
        aliases, relations = result["aliases"], result["relations"]
        self.assertEqual(aliases["supported_annotation_agreement"], scorer._metrics(1, 4, 0, endpoint=True))
        self.assertEqual(aliases["all_disposition_endpoint_coverage"], {"matched": 4, "gold": 4, "predictions": 5, "recall": 1})
        self.assertEqual(aliases["known_nonpositive_by_disposition"], {
            "rejected_equivalence": 1, "incompatible_scope": 1, "unresolved_equivalence": 1})
        self.assertEqual(aliases["classification"]["unlisted_predictions"], 1)
        self.assertEqual(relations["supported_annotation_agreement"], scorer._metrics(1, 2, 0, endpoint=True))
        self.assertEqual(relations["known_nonpositive_by_disposition"], {"contradicted_navigation": 1})
        self.assertEqual(relations["classification"]["unlisted_predictions"], 1)

    def test_rejected_and_contradicted_emissions_earn_no_primary_true_positives(self):
        doc = copy.deepcopy(self.corpus["documents"][0])
        doc["aliases"] = [doc["aliases"][1]]
        doc["aliases"][0][2] = "supported_equivalence"
        doc["relations"] = [doc["relations"][1]]
        doc["relations"][0][3] = "supported_navigation"
        self.outcomes["sources"][0]["extraction"]["candidates"] = dto(doc, self.outcomes["sources"][0]["source_version_id"])
        result = self.source_result()
        for field in ("aliases", "relations"):
            self.assertEqual(result[field]["supported_annotation_agreement"], scorer._metrics(0, 1, 1, endpoint=True))
            self.assertEqual(result[field]["all_disposition_endpoint_coverage"]["matched"], 1)
            self.assertEqual(result[field]["classification"]["known_nonpositive_disposition_hits"], 1)

    def test_zero_emission_and_empty_partition_keep_all_diagnostic_zero_fields(self):
        self.corpus["documents"] = [document("only", "")]
        self.freeze_corpus()
        self.outcomes = self.outcomes_fixture()
        summary, _ = self.evaluate()
        for bucket in (summary["extraction"], summary["partitions"]["development"]):
            for field in ("aliases", "relations"):
                self.assertEqual(bucket[field]["classification"], {
                    "supported_only_predictions": 0, "known_nonpositive_disposition_hits": 0,
                    "ambiguous_disposition_predictions": 0, "unlisted_predictions": 0})
                self.assertEqual(set(bucket[field]["known_nonpositive_by_disposition"].values()), {0})

    def test_ambiguous_dispositions_are_flagged_without_selecting_a_class(self):
        row = list(self.corpus["documents"][0]["aliases"][0])
        row[2] = "rejected_equivalence"
        self.corpus["documents"][0]["aliases"].append(row)
        self.freeze_corpus()
        self.outcomes = self.outcomes_fixture()
        result = self.source_result()["aliases"]
        self.assertEqual(result["ambiguous_gold_signatures"], 1)
        self.assertEqual(result["classification"]["ambiguous_disposition_predictions"], 1)
        self.assertEqual(result["classification"]["supported_only_predictions"], 0)
        self.assertEqual(result["classification"]["known_nonpositive_disposition_hits"], 0)
        self.assertEqual(result["all_disposition_endpoint_coverage"]["gold"], 5)
        self.assertEqual(result["all_disposition_endpoint_coverage"]["matched"], 1)

    def test_overlaps_are_admitted_but_wrong_physical_occurrences_do_not_match(self):
        candidates = self.candidates()
        # "lan" overlaps the second physical Élan occurrence without duplicating it.
        second = span(self.corpus["documents"][0]["text"], "Élan", 2)
        candidates["mentions"].append({"key": "overlap", "kind": "concept", "confidence": 1,
            "span": {"byte_start": second[0] + 2, "byte_end": second[1], "quote": "lan"}})
        # Remove the first occurrence. The same surface on the second occurrence
        # remains an exact valid prediction but cannot recover the first gold row.
        candidates["mentions"].pop(0)
        candidates["aliases"] = []
        result = self.source_result()["mentions"]
        self.assertEqual(result, scorer._metrics(4, 1, 1))

    def test_wrong_occurrence_endpoint_with_exact_enclosing_evidence_is_unlisted(self):
        doc = self.corpus["documents"][0]
        edge = self.candidates()["relations"][0]
        edge["from"] = "accent1"
        edge["span"] = {"byte_start": 0, "byte_end": len(doc["text"].encode()), "quote": doc["text"]}
        result = self.source_result()["relations"]
        self.assertEqual(result["supported_annotation_agreement"], scorer._metrics(0, 1, 1, endpoint=True))
        self.assertEqual(result["classification"]["unlisted_predictions"], 1)

    def test_all_missing_error_and_unexpected_english_abstention_states_retain_fns(self):
        original = copy.deepcopy(self.outcomes)
        for state in scorer.EXTRACTION_STATES[1:]:
            with self.subTest(state=state):
                self.outcomes = copy.deepcopy(original)
                source = self.outcomes["sources"][0]
                source["extraction"] = {"state": state, "failure": "InvalidSpan" if state == "rejected" else "UpstreamFailure" if state == "failed" else None,
                                        "candidates": None}
                for mode in self.outcomes["modes"]:
                    mode["sources"][0]["ner_feature_available"] = False
                result = self.source_result()
                self.assertEqual(result["mentions"], scorer._metrics(0, 0, 5))
                self.assertEqual(result["aliases"]["supported_annotation_agreement"], scorer._metrics(0, 0, 1, endpoint=True))
                self.assertFalse(result["proven_abstention"])

    def test_nonenglish_english_labelled_predictions_are_penalized_and_failures_do_not_prove_abstention(self):
        doc = self.corpus["documents"][5]
        doc["mentions"] = [["breeze", "concept", *span(doc["text"], "brise")]]
        self.freeze_corpus()
        self.outcomes = self.outcomes_fixture()
        source = self.outcomes["sources"][5]
        source["extraction"] = {"state": "validated", "failure": None, "candidates": dto(doc, source["source_version_id"])}
        result = self.evaluate()[1]["sources"][3]
        self.assertEqual(result["document_id"], "N")
        self.assertEqual(result["mentions"], scorer._metrics(0, 1, 1))
        for state, failure, abstention in (("absent", None, True), ("unsupported_language", None, True),
                                           ("rejected", "UnsupportedLanguage", True), ("missing", None, False),
                                           ("failed", "UpstreamFailure", False), ("rejected", "InvalidSpan", False)):
            source["extraction"] = {"state": state, "failure": failure, "candidates": None}
            result = self.evaluate()[1]["sources"][3]
            self.assertEqual(result["mentions"]["fn"], 1)
            self.assertEqual(result["proven_abstention"], abstention)

    def test_all_failure_enum_values_admit_only_rejected_state(self):
        source = self.outcomes["sources"][0]
        for mode in self.outcomes["modes"]:
            mode["sources"][0]["ner_feature_available"] = False
        for failure in scorer.CANDIDATE_FAILURES:
            source["extraction"] = {"state": "rejected", "failure": failure, "candidates": None}
            self.assertEqual(self.evaluate()[0]["extraction"]["states"]["rejected"], 1)
        for state, failure in (("rejected", "UpstreamFailure"), ("failed", "InvalidSpan"), ("missing", "InvalidSpan"),
                               ("rejected", None), ("absent", "UnsupportedLanguage")):
            source["extraction"] = {"state": state, "failure": failure, "candidates": None}
            self.reject()

    def test_unknown_pairs_and_sources_remain_full_cohort_and_cross_brain_is_a_scored_failure(self):
        mode = self.outcomes["modes"][0]
        mode["sources"][3]["groups"] = ["route"]
        mode["sources"][4]["groups"] = ["route"]
        mode["sources"][4]["state"] = "assigned"
        mode["groups"] = [{"id": "route", "members": ["ID_SENTINEL_A", "B", "X", "U"]}]
        summary, details = self.evaluate()
        result = summary["modes"][scorer.MODES[0]]
        self.assertEqual((result["topic_pairs"], result["topic_pairs_known"], result["topic_pairs_unknown"]), (15, 12, 3))
        self.assertEqual(result["unknown_related_predictions"], 2)
        self.assertEqual(result["cross_brain_related_pair_failures"], 3)
        self.assertEqual(result["topic_annotation_agreement"], scorer._metrics(1, 3, 0) | {"tn": 8})
        self.assertEqual(result["unknown_source_assignments"], {"count": 1, "total": 1, "coverage": 1})
        self.assertEqual(len(details["modes"][scorer.MODES[0]]["pairs"]), 15)
        self.assertEqual(summary["counts"]["sources"], 6)

    def test_singleton_assignment_is_distinct_from_multisource_and_recovered_positive_coverage(self):
        mode = self.outcomes["modes"][0]
        mode["groups"] = [{"id": "a", "members": ["ID_SENTINEL_A"]}, {"id": "b", "members": ["B"]}, {"id": "isolated", "members": ["X"]}]
        mode["sources"][0]["groups"], mode["sources"][1]["groups"] = ["a"], ["b"]
        result = self.evaluate()[0]["modes"][scorer.MODES[0]]
        self.assertEqual(result["all_source_assignment"], {"count": 3, "total": 6, "coverage": 0.5})
        self.assertEqual(result["nonempty_source_assignment"]["total"], 5)
        self.assertEqual(result["known_positive_source_assignment"]["coverage"], 1)
        self.assertEqual(result["known_positive_sources_with_recovered_pair"]["coverage"], 0)
        self.assertEqual(result["multi_source_group_coverage"]["coverage"], 0)
        self.assertEqual(result["singleton_groups"], 3)
        self.assertEqual(result["topic_annotation_agreement"]["fn"], 1)

    def test_all_mode_error_states_are_admitted_without_dropping_known_positives(self):
        original = copy.deepcopy(self.outcomes)
        for state in scorer.MODE_STATES[1:]:
            with self.subTest(state=state):
                self.outcomes = copy.deepcopy(original)
                mode = self.outcomes["modes"][0]
                mode["sources"][0]["state"], mode["sources"][0]["groups"] = state, []
                mode["groups"][0]["members"] = ["B"]
                result = self.evaluate()[0]["modes"][scorer.MODES[0]]
                self.assertEqual(result["topic_annotation_agreement"]["fn"], 1)
                self.assertGreaterEqual(result["source_states"][state], 1)

    def test_full_250_source_31125_pair_toy_and_null_zero_denominators(self):
        self.corpus["documents"] = [document(f"toy-{i}", "") for i in range(250)]
        self.freeze_corpus()
        self.outcomes = self.outcomes_fixture()
        summary, details = self.evaluate()
        self.assertEqual((summary["counts"]["sources"], summary["counts"]["topic_pairs"]), (250, 31_125))
        self.assertEqual(summary["complete_topic_gold_readiness"], "READY")
        self.assertEqual(summary["extraction"]["mentions"], scorer._metrics(0, 0, 0))
        for mode in scorer.MODES:
            result = summary["modes"][mode]
            self.assertEqual(result["topic_annotation_agreement"], scorer._metrics(0, 0, 0) | {"tn": 31_125})
            self.assertEqual(result["all_source_assignment"]["total"], 250)
            self.assertIsNone(result["nonempty_source_assignment"]["coverage"])
            self.assertEqual(len(details["modes"][mode]["pairs"]), 31_125)
        # Allowed maximum shared memberships must not multiply group IDs into
        # every pair diagnostic: 6,225,000 intersections remain 31,125 counts.
        mode = self.outcomes["modes"][0]
        ids = [f"overlap-{i}" for i in range(200)]
        members = [doc["id"] for doc in self.corpus["documents"]]
        for source in mode["sources"]:
            source["state"], source["groups"] = "assigned", list(ids)
        mode["groups"] = [{"id": key, "members": list(members)} for key in ids]
        frozen = loader.load_frozen_corpus(self.manifest_path, manifest_sha256=self.manifest_hash)
        admitted = scorer._mode(mode, {doc["id"]: doc for doc in self.corpus["documents"]},
                                {source["document_id"]: source for source in self.outcomes["sources"]})
        result, bounded = scorer._score_mode(admitted, frozen)
        self.assertEqual(result["memberships"], 50_000)
        self.assertTrue(all(row["shared_group_count"] == 200 and "shared_groups" not in row for row in bounded["pairs"]))
        self.assertLess(len(encode(bounded)), 10 * 1024 * 1024)

    def test_complete_topic_readiness_is_only_loader_coverage_and_does_not_qualify(self):
        self.corpus["documents"][4]["metadata"]["topic_annotation_state"] = "complete"
        self.freeze_corpus()
        self.outcomes = self.outcomes_fixture()
        summary, _ = self.evaluate()
        self.assertEqual(summary["complete_topic_gold_readiness"], "READY")
        self.assertEqual(set(summary["not_measured"].values()), {"NOT_MEASURED"})

    def test_all_bindings_hashes_partitions_and_uuid_corruption_reject_whole_artifact(self):
        mutations = [
            lambda o: o["corpus_bindings"].__setitem__("manifest", "0" * 64),
            lambda o: o["corpus_bindings"].__setitem__("guide", "F" * 64),
            lambda o: o.__setitem__("recipe_sha256", "0" * 64),
            lambda o: o["sources"][0].__setitem__("text_sha256", "0" * 64),
            lambda o: o["sources"][0].__setitem__("partition", "development"),
            lambda o: o["sources"][0].__setitem__("source_version_id", None),
            lambda o: o["sources"][0].__setitem__("source_version_id", "A0000000-0000-0000-0000-000000000000"),
            lambda o: o["sources"][1].__setitem__("source_version_id", o["sources"][0]["source_version_id"]),
            lambda o: o["sources"][0]["extraction"]["candidates"].__setitem__("source_version_id", str(uuid.UUID(int=999))),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                self.mutate_reject(mutate)

    def test_physical_omission_duplicates_foreign_ids_and_membership_corruption_reject(self):
        mutations = [
            lambda o: o["sources"].pop(), lambda o: o["sources"].append(copy.deepcopy(o["sources"][0])),
            lambda o: o["sources"][0].__setitem__("document_id", "foreign"),
            lambda o: o["modes"].pop(), lambda o: o["modes"].append(copy.deepcopy(o["modes"][0])),
            lambda o: o["modes"][0]["sources"].pop(),
            lambda o: o["modes"][0]["sources"][0].__setitem__("groups", ["foreign"]),
            lambda o: o["modes"][0]["sources"][0].__setitem__("groups", ["route", "route"]),
            lambda o: o["modes"][0]["groups"][0]["members"].pop(),
            lambda o: o["modes"][0]["groups"][0]["members"].append("foreign"),
            lambda o: o["modes"][0]["groups"][0]["members"].append("B"),
            lambda o: o["modes"][0]["groups"][0].__setitem__("members", []),
            lambda o: o["modes"][0]["groups"].append(copy.deepcopy(o["modes"][0]["groups"][0])),
            lambda o: o["modes"][0]["sources"][0].__setitem__("state", "missing"),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                self.mutate_reject(mutate)

    def test_strict_shapes_unknown_fields_enums_and_booleans_reject(self):
        mutations = [
            lambda o: o.__setitem__("native_origin", True),
            lambda o: o["sources"][0].__setitem__("origin_verified", True),
            lambda o: o["sources"][0]["extraction"].__setitem__("state", "completed"),
            lambda o: o["sources"][0]["extraction"].__setitem__("failure", "UpstreamFailure"),
            lambda o: o["sources"][0]["extraction"]["candidates"].__setitem__("unknown", 1),
            lambda o: o["sources"][0]["extraction"]["candidates"].__setitem__("language", "fr"),
            lambda o: o["sources"][0]["extraction"]["candidates"].__setitem__("schema", "new-schema"),
            lambda o: o["sources"][0]["extraction"]["candidates"].__setitem__("adapter_revision", "learning-quote-anchors-2"),
            lambda o: o["sources"][0]["extraction"]["candidates"]["mentions"][0].__setitem__("kind", "instance"),
            lambda o: o["sources"][0]["extraction"]["candidates"]["aliases"][0].__setitem__("accepted", True),
            lambda o: o["sources"][0]["extraction"]["candidates"]["relations"][0].__setitem__("relation", "operational_truth"),
            lambda o: o["modes"][0]["sources"][0].__setitem__("vector_feature_available", 1),
            lambda o: o["modes"][0]["sources"][0].__setitem__("ner_feature_available", "true"),
            lambda o: o["modes"][0]["groups"][0].__setitem__("label", "LABEL_SENTINEL"),
            lambda o: o["modes"][0]["sources"][5].__setitem__("ner_feature_available", True),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                self.mutate_reject(mutate)

    def test_candidate_utf8_bounds_local_keys_duplicate_occurrences_and_endpoint_enclosure(self):
        mutations = [
            lambda c: c["mentions"][0]["span"].__setitem__("byte_start", True),
            lambda c: c["mentions"][0]["span"].__setitem__("byte_start", -1),
            lambda c: c["mentions"][0]["span"].__setitem__("byte_start", 6),
            lambda c: c["mentions"][0]["span"].__setitem__("byte_end", 10_000),
            lambda c: c["mentions"][0]["span"].__setitem__("quote", "Elan"),
            lambda c: c["mentions"][0].__setitem__("key", "invalid key"),
            lambda c: c["mentions"][0].__setitem__("key", "k" * 81),
            lambda c: c["mentions"][0].__setitem__("key", c["mentions"][1]["key"]),
            lambda c: c["mentions"].append({**copy.deepcopy(c["mentions"][0]), "key": "other"}),
            lambda c: c["aliases"][0].__setitem__("right", "foreign"),
            lambda c: c["aliases"][0].__setitem__("right", "accent1"),
            lambda c: c["aliases"][0].__setitem__("right", "cobalt"),
            lambda c: c["relations"][0].__setitem__("span", copy.deepcopy(c["mentions"][3]["span"])),
            lambda c: c["mentions"][0].__setitem__("span", {"byte_start": 4, "byte_end": 5, "quote": " "}),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                original = copy.deepcopy(self.outcomes)
                mutate(self.candidates())
                self.reject()
                self.outcomes = original

    def test_finite_confidence_numbers_and_exact_frozen_parameters_in_all_modes(self):
        for value in (True, -0.01, 1.01, float("nan"), float("inf"), float("-inf"), 1e300):
            with self.subTest(confidence=value):
                original = copy.deepcopy(self.outcomes)
                self.candidates()["mentions"][0]["confidence"] = value
                self.reject()
                self.outcomes = original
        for index in range(5):
            for key, value in (("neighbors", True), ("neighbors", 4), ("minimum_cosine", 0.64), ("vector_scale", 1.0)):
                self.mutate_reject(lambda o: o["modes"][index]["parameters"].__setitem__(key, value))

    def test_recipe_schema_labels_hashes_parameters_order_and_unknown_fields(self):
        original = copy.deepcopy(self.recipe)
        mutations = [
            lambda r: r.__setitem__("schema", "other"), lambda r: r.__setitem__("candidate_freeze_sha256", "A" * 64),
            lambda r: r.__setitem__("development_selection_sha256", True),
            lambda r: r["producer"].__setitem__("prompt_label", "source-facts-3"),
            lambda r: r["producer"].__setitem__("schema_label", "source-reconciliation-4"),
            lambda r: r["producer"].__setitem__("adapter_revision", "learning-quote-anchors-2"),
            lambda r: r["producer"].__setitem__("mapping_schema", "source-mentions-2"),
            lambda r: r["parameters"].__setitem__("neighbors", True),
            lambda r: r["parameters"].__setitem__("vector_scale", 0.3),
            lambda r: r["modes"].reverse(), lambda r: r.__setitem__("qualified", True),
        ]
        for mutate in mutations:
            self.recipe = copy.deepcopy(original)
            mutate(self.recipe)
            self.write_recipe()
            self.reject()
        self.recipe = copy.deepcopy(original)
        self.recipe["producer"]["prompt_label"] = self.recipe["producer"]["schema_label"] = "source-reconciliation-4"
        self.write_recipe()
        self.assertEqual(self.evaluate()[0]["counts"]["sources"], 6)

    def test_nested_duplicate_json_keys_nonfinite_invalid_utf8_and_surrogates(self):
        raw = encode(self.outcomes)
        bad = [raw.replace(b'"confidence": 0.8', b'"confidence":0.8,"confidence":0.9', 1),
               raw.replace(b'"confidence": 0.8', b'"confidence":NaN', 1),
               raw.replace(b'"confidence": 0.8', b'"confidence":1e999', 1),
               raw.replace(b'"confidence": 0.8', b'"confidence":Infinity', 1),
               b'{"x":"\\ud800"}', b"\xff"]
        for data in bad:
            self.outcomes_path.write_bytes(data)
            with self.assertRaises(scorer.OutcomeValidationError):
                scorer.evaluate(self.manifest_path, manifest_sha256=self.manifest_hash, recipe=self.recipe_path,
                                recipe_sha256=self.recipe_hash, outcomes=self.outcomes_path, outcomes_sha256=sha(data))
        self.recipe_path.write_bytes(encode(self.recipe).replace(b'"neighbors": 3', b'"neighbors":3,"neighbors":3', 1))
        self.recipe_hash = sha(self.recipe_path.read_bytes())
        self.outcomes["recipe_sha256"] = self.recipe_hash
        self.reject()

    def test_limits_candidate_counts_dto_bytes_ids_and_250_source_overflow(self):
        for field, maximum in (("mentions", 128), ("aliases", 128), ("relations", 256)):
            original = copy.deepcopy(self.outcomes)
            self.candidates()[field] = [copy.deepcopy(self.candidates()[field][0]) for _ in range(maximum + 1)]
            self.reject()
            self.outcomes = original
        self.mutate_reject(lambda o: o["modes"][0]["groups"][0].__setitem__("id", "é" * 129))
        self.mutate_reject(lambda o: o["sources"].extend([copy.deepcopy(o["sources"][0])] * 245))
        self.mutate_reject(lambda o: o["modes"][0]["sources"].extend([copy.deepcopy(o["modes"][0]["sources"][0])] * 245))
        # Legal repeated relations can still exceed the native compact DTO cap.
        doc = self.corpus["documents"][0]
        doc["text"] += "x" * 1200
        self.freeze_corpus()
        self.outcomes = self.outcomes_fixture()
        edge = self.candidates()["relations"][0]
        edge["span"] = {"byte_start": 0, "byte_end": len(doc["text"].encode()), "quote": doc["text"]}
        self.candidates()["relations"] = [copy.deepcopy(edge) for _ in range(256)]
        self.reject()

    def test_native_mention_quote_cap_is_utf8_bytes(self):
        doc = self.corpus["documents"][0]
        doc["text"] += "é" * 129
        self.freeze_corpus()
        self.outcomes = self.outcomes_fixture()
        first, last, quote = span(doc["text"], "é" * 129)
        self.candidates()["mentions"].append({"key": "long", "kind": "concept", "confidence": 0.5,
                                            "span": {"byte_start": first, "byte_end": last, "quote": quote}})
        self.reject()

    def test_exact_group_and_membership_limits_and_overflow(self):
        docs = {d["id"]: d for d in self.corpus["documents"]}
        sources = {s["document_id"]: s for s in self.outcomes["sources"]}
        mode = self.outcomes["modes"][0]
        ids = [f"group-{i}" for i in range(10_000)]
        for row in mode["sources"]:
            row["groups"] = list(ids) if row["document_id"] != "N" else []
            row["state"] = "assigned" if row["groups"] else "unassigned"
            row["ner_feature_available"] = False
        members = [key for key in docs if key != "N"]
        mode["groups"] = [{"id": key, "members": list(members)} for key in ids]
        self.assertEqual(len(scorer._mode(mode, docs, sources)["groups"]), 10_000)
        mode["groups"].append({"id": "too-many", "members": ["N"]})
        with self.assertRaises(scorer.OutcomeValidationError):
            scorer._mode(mode, docs, sources)
        mode["groups"].pop()
        mode["sources"][-1]["groups"], mode["sources"][-1]["state"] = list(ids), "assigned"
        for group in mode["groups"]:
            group["members"].append("N")
        with self.assertRaises(scorer.OutcomeValidationError):
            scorer._mode(mode, docs, sources)

    def test_recipe_outcome_size_and_shared_json_depth_value_preflight_bounds(self):
        for kind in ("recipe", "outcomes"):
            path = self.directory / f"oversize-{kind}.json"
            with path.open("wb") as stream:
                stream.seek(scorer.FILE_LIMITS[kind])
                stream.write(b" ")
            with mock.patch.object(scorer, "_parse_json", side_effect=AssertionError("parser allocation forbidden")):
                with self.assertRaises(scorer.OutcomeValidationError):
                    scorer._read_pinned(path, "0" * 64, kind)
        for raw in (b"[" * 34 + b"0" + b"]" * 34, b"[" + b"0," * 1_000_002 + b"0]"):
            self.outcomes_path.write_bytes(raw)
            with mock.patch.object(loader.json, "loads", side_effect=AssertionError("parser allocation forbidden")):
                with self.assertRaises(loader.CorpusValidationError):
                    scorer._read_pinned(self.outcomes_path, sha(raw), "outcomes")

    def test_every_cli_pin_detects_raw_drift(self):
        for path in (self.manifest_path, self.recipe_path, self.outcomes_path):
            raw = path.read_bytes()
            path.write_bytes(raw + b" ")
            out, err = io.StringIO(), io.StringIO()
            with redirect_stdout(out), redirect_stderr(err):
                self.assertEqual(scorer.main(self.args()), 1)
            self.assertEqual(out.getvalue(), "")
            self.assertEqual(err.getvalue(), "offline outcome evaluation failed\n")
            path.write_bytes(raw)

    def test_success_is_deterministic_aggregate_only_and_cli_is_sentinel_safe(self):
        outputs = []
        for _ in range(2):
            out, err = io.StringIO(), io.StringIO()
            with redirect_stdout(out), redirect_stderr(err):
                self.assertEqual(scorer.main(self.args()), 0)
            outputs.append(out.getvalue())
            self.assertEqual(err.getvalue(), "")
            self.assertNotIn("SENTINEL", out.getvalue())
        self.assertEqual(outputs[0], outputs[1])
        self.assertEqual(json.loads(outputs[0])["input_sha256"]["scorer"], sha(Path(scorer.__file__).read_bytes()))
        for arguments in ((self.args() + ["--UNSUPPORTED_SENTINEL"]), [],
                          self.args() + ["--recipe", "PATH_SENTINEL"], ["@PATH_SENTINEL"]):
            out, err = io.StringIO(), io.StringIO()
            with redirect_stdout(out), redirect_stderr(err), self.assertRaises(SystemExit) as caught:
                scorer.main(arguments)
            self.assertEqual(caught.exception.code, 2)
            self.assertEqual(out.getvalue(), "")
            self.assertEqual(err.getvalue(), "offline outcome arguments invalid\n")
        arguments = self.args()
        arguments[1] = "PATH_SENTINEL"
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            self.assertEqual(scorer.main(arguments), 1)
        self.assertEqual(out.getvalue(), "")
        self.assertEqual(err.getvalue(), "offline outcome evaluation failed\n")

    def test_details_are_exclusive_0600_and_never_copy_source_or_gold_text(self):
        target = self.directory / "details.json"
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            self.assertEqual(scorer.main(self.args() + ["--details", str(target)]), 0)
        self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o600)
        raw = target.read_bytes()
        details = json.loads(raw)
        self.assertEqual(len(details["sources"]), 6)
        for forbidden in ("🪶", "Élan", "Cobalt", "TITLE_SENTINEL", "TOPIC_SENTINEL", "PAIR_REASON_SENTINEL", "LABEL_SENTINEL"):
            self.assertNotIn(forbidden.encode(), raw)
        with self.assertRaises(scorer.OutcomeValidationError):
            scorer.write_details(target, details)
        self.assertEqual(target.read_bytes(), raw)

    def test_details_symlinks_parent_symlinks_escape_and_output_failure_are_generic(self):
        summary, details = self.evaluate()
        target = self.directory / "existing.json"
        target.write_bytes(b"PRESERVE_SENTINEL")
        link = self.directory / "link.json"
        link.symlink_to(target)
        parent = self.directory / "linked-directory"
        parent.symlink_to(self.directory, target_is_directory=True)
        forbidden = [link, parent / "new.json", self.directory / ".." / "escape.json", Path("/tmp/outside-SENTINEL.json")]
        for path in forbidden:
            with self.assertRaises(scorer.OutcomeValidationError):
                scorer.write_details(path, details)
        self.assertEqual(target.read_bytes(), b"PRESERVE_SENTINEL")
        self.assertFalse((self.directory / "new.json").exists())
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            self.assertEqual(scorer.main(self.args() + ["--details", str(link)]), 1)
        self.assertEqual(out.getvalue(), "")
        self.assertEqual(err.getvalue(), "offline outcome evaluation failed\n")
        self.assertEqual(summary["schema"], scorer.REPORT_SCHEMA)

    def test_inputs_cannot_escape_repository(self):
        for path in (Path("/etc/passwd"), Path("../foreign.json"), Path("https://example.invalid/nope")):
            with self.assertRaises((scorer.OutcomeValidationError, OSError)):
                scorer._read_pinned(path, "0" * 64, "recipe")

    def test_evaluator_needs_no_network_process_provider_or_native_engine(self):
        with mock.patch("socket.socket", side_effect=AssertionError("network forbidden")), \
             mock.patch("subprocess.Popen", side_effect=AssertionError("process forbidden")), \
             mock.patch("os.system", side_effect=AssertionError("process forbidden")):
            summary, _ = self.evaluate()
            self.assertEqual(summary["counts"]["sources"], 6)

    def test_actual_cli_process_with_independent_toys(self):
        result = subprocess.run([sys.executable, "-B", str(ROOT / "scripts/score_frozen_source_corpus.py"), *self.args()],
                                capture_output=True, text=True, check=False, timeout=30)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, "")
        self.assertNotIn("SENTINEL", result.stdout)
        self.assertEqual(json.loads(result.stdout)["counts"]["topic_pairs"], 15)


if __name__ == "__main__":
    unittest.main()
