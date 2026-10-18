"""Independent toy annotation admission tests; no private data or providers."""

from contextlib import redirect_stderr, redirect_stdout
import copy
import hashlib
import importlib.util
import io
import itertools
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("frozen_source_corpus", ROOT / "scripts/frozen_source_corpus.py")
loader = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = loader
SPEC.loader.exec_module(loader)

CORPUS_SCHEMA = "recollect-unseen-organization-heldout-2-reviewed-revision-1-topic-coverage-amendment-1"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n").encode("utf-8")


def interface_fixture():
    # Generic declarations authored inline; no corpus instances or cache reads.
    return {
        "schema": "recollect-unseen-organization-interface-descriptor-1",
        "corpus_schema_version": CORPUS_SCHEMA,
        "top_level_required_keys": {
            "schema": "string", "metadata": "object", "documents": "Document[]",
            "topic_pairs": "TopicPair[]", "identity_controls": "IdentityControl[]",
            "topic_pair_metadata": "object[]",
        },
        "Document": {
            "id": "string", "title": "string", "partition": "development|held_out",
            "language": "string", "scope": "three slash-separated symbolic fields: Brain/repository/environment",
            "text": "string exact immutable UTF-8 source", "mentions": "Mention[]",
            "aliases": "Alias[]", "relations": "Relation[]", "topics": "string[]",
            "metadata": "annotation metadata, including explicit topic_annotation_state=complete|unknown",
        },
        "Mention": ["localKey:string", "kind:Kind", "byteStart:integer", "byteEnd:integer", "exactQuote:string"],
        "Alias": ["leftKey:string", "rightKey:string", "disposition:AliasDisposition", "byteStart:integer", "byteEnd:integer", "exactEvidence:string"],
        "Relation": ["fromKey:string", "toKey:string", "relation:RelationKind", "disposition:RelationDisposition", "byteStart:integer", "byteEnd:integer", "exactEvidence:string"],
        "TopicPair": ["leftDocumentId:string", "rightDocumentId:string", "disposition:TopicPairDisposition"],
        "enums": {
            "Kind": ["person", "technology", "service", "environment", "repository", "concept", "setting"],
            "AliasDisposition": ["supported_equivalence", "rejected_equivalence", "incompatible_scope", "unresolved_equivalence"],
            "RelationDisposition": ["supported_navigation", "contradicted_navigation"],
            "RelationKind": ["uses", "configured_by", "depends_on", "located_in", "implements", "related_to", "does_not_use", "has_owner"],
            "TopicPairDisposition": ["related_ prefix", "unrelated", "unrelated_empty", "cross_Brain_no_membership_or_count", "unknown_topic_coverage"],
            "IdentityDisposition": ["must_match", "must_differ", "unresolved"],
        },
        "source_hash_manifest": {"schema": "string", "documents": "map document ID -> lowercase SHA256 hex of exact UTF-8 text"},
        "topic_coverage_state": {"location": "documents[].metadata.topic_annotation_state", "enum": ["complete", "unknown"]},
        "unknown_pair_semantics": {"disposition": "unknown_topic_coverage", "truth": None},
    }


def document(key, text, scope="BrainA/RepoA/DEV"):
    return {
        "id": key, "title": "PRIVATE_TITLE_SENTINEL", "partition": "held_out",
        "language": "en", "scope": scope, "text": text,
        "mentions": [], "aliases": [], "relations": [], "topics": [],
        "metadata": {"topic_annotation_state": "complete"},
    }


def corpus_fixture():
    a = document("PRIVATE_SOURCE_A", "Café uses Nacre.")
    a["mentions"] = [["cafe", "service", 0, 5, "Café"], ["nacre", "technology", 11, 16, "Nacre"]]
    a["aliases"] = [["cafe", "nacre", "unresolved_equivalence", 0, 17, "Café uses Nacre."]]
    a["relations"] = [["cafe", "nacre", "uses", "supported_navigation", 0, 17, "Café uses Nacre."]]
    a["topics"] = ["PRIVATE_TOPIC_SENTINEL"]
    b = document("B", "Quill config.")
    b["mentions"] = [["quill", "service", 0, 5, "Quill"]]
    empty = document("E", "")
    foreign = document("X", "Nacre remains.", "BrainB/RepoA/DEV")
    foreign["mentions"] = [["nacre", "technology", 0, 5, "Nacre"]]
    return {
        "schema": CORPUS_SCHEMA, "metadata": {"note": "PRIVATE_METADATA_SENTINEL"},
        "documents": [a, b, empty, foreign],
        "topic_pairs": [
            ["PRIVATE_SOURCE_A", "B", "related_workflow"],
            ["PRIVATE_SOURCE_A", "E", "unrelated_empty"],
            ["PRIVATE_SOURCE_A", "X", "cross_Brain_no_membership_or_count"],
            ["B", "E", "unrelated_empty"],
            ["B", "X", "cross_Brain_no_membership_or_count"],
            ["E", "X", "cross_Brain_no_membership_or_count"],
        ],
        "identity_controls": [{
            "id": "PRIVATE_CONTROL_SENTINEL",
            "left": {"document_id": "PRIVATE_SOURCE_A", "mention_key": "nacre"},
            "right": {"document_id": "X", "mention_key": "nacre"},
            "disposition": "unresolved", "reason": "same token alone proves no realm",
            "validation_boundary": "annotations only",
        }],
        "topic_pair_metadata": [],
    }


def align_pair_metadata(corpus):
    documents = {doc["id"]: doc for doc in corpus["documents"]}
    rows = []
    for left, right, disposition in corpus["topic_pairs"]:
        # Invalid references are tested in the core tuple, not repaired here.
        a, b = documents.get(left, {}), documents.get(right, {})
        left_scope = a.get("scope", "?/ ?/ ?").split("/")
        right_scope = b.get("scope", "?/ ?/ ?").split("/")
        unknown = disposition == "unknown_topic_coverage"
        rows.append({
            "left": left, "right": right, "disposition": disposition,
            "reason": "Independent toy declaration",
            "shared_topics": sorted(set(a.get("topics", [])) & set(b.get("topics", []))),
            "cross_brain_boundary": left_scope[0] != right_scope[0],
            "same_repository_scope_token": left_scope[1] == right_scope[1],
            "same_environment_scope_token": left_scope[2] == right_scope[2],
            "label_usefulness": "annotation only", "unsupported_language_in_pair": False,
            "lifecycle_scenario_note": None,
            "topic_truth_status": "unknown" if unknown else "known",
            "scored_topic_positive_or_negative": not unknown,
            "topic_coverage_reason": "Unknown toy source coverage" if unknown else None,
        })
    corpus["topic_pair_metadata"] = rows


class LoaderTests(unittest.TestCase):
    def setUp(self):
        cache = ROOT / ".cache/graph-implementation"
        cache.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="loader-toy-", dir=cache)
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.corpus = corpus_fixture()
        self.interface = interface_fixture()
        self.guide = b"Toy annotation guide: no private examples.\n"
        self.manifest_path = self.directory / "manifest.json"
        self.write_fixture()

    def write_fixture(self):
        align_pair_metadata(self.corpus)
        hashes = {"schema": "toy-source-hashes-1", "documents": {
            d["id"]: sha(d["text"].encode("utf-8")) for d in self.corpus["documents"]
        }}
        files = {"corpus": encode(self.corpus), "guide": self.guide,
                 "source_hashes": encode(hashes), "interface": encode(self.interface)}
        self.manifest = {"schema": "recollect-frozen-source-corpus-manifest-1", "corpus_schema": self.corpus["schema"]}
        for kind, raw in files.items():
            path = self.directory / f"{kind}.json"
            path.write_bytes(raw)
            self.manifest[f"{kind}_path"] = str(path.relative_to(ROOT))
            self.manifest[f"{kind}_sha256"] = sha(raw)
        self.write_manifest()

    def write_manifest(self):
        self.manifest_path.write_bytes(encode(self.manifest))
        self.manifest_hash = sha(self.manifest_path.read_bytes())

    def replace_file(self, kind, raw):
        path = ROOT / self.manifest[f"{kind}_path"]
        path.write_bytes(raw)
        self.manifest[f"{kind}_sha256"] = sha(raw)
        self.write_manifest()

    def load(self):
        return loader.load_frozen_corpus(self.manifest_path, manifest_sha256=self.manifest_hash)

    def reject(self):
        with self.assertRaisesRegex(loader.CorpusValidationError, "^frozen corpus validation failed$"):
            self.load()

    def mutate_reject(self, mutate):
        original = copy.deepcopy(self.corpus)
        mutate(self.corpus)
        self.write_fixture()
        self.reject()
        self.corpus = original

    def test_valid_unicode_and_content_free_summary(self):
        loaded = self.load()
        self.assertEqual(loaded.annotations, self.corpus)
        self.assertEqual([p.truth for p in loaded.topic_pairs], [True, False, False, False, False, False])
        summary = loaded.summary()
        self.assertEqual(summary["counts"]["sources"], 4)
        self.assertEqual(summary["counts"]["topic_pairs"], 6)
        self.assertEqual(summary["counts"]["mentions"], 4)
        self.assertEqual(summary["complete_topic_gold_readiness"], "READY")
        self.assertEqual(summary["producer_or_topic_recipe_qualification"], "NOT_MEASURED")
        self.assertNotIn("complete_topic_qualification", summary)
        self.assertEqual(summary["file_sha256"]["manifest"], self.manifest_hash)
        self.assertNotIn("PRIVATE", json.dumps(summary))

    def test_unresolved_alias_and_identity_remain_explicit(self):
        preconditions = {
            "current_contribution_requires": "Independent current support",
            "withdrawn_source_may_contribute_current_support": False,
            "identity_semantics": "Support eligibility remains independent",
        }
        self.corpus["identity_controls"][0]["eligibility_preconditions"] = preconditions
        self.write_fixture()
        loaded = self.load()
        self.assertEqual(loaded.annotations["documents"][0]["aliases"][0][2], "unresolved_equivalence")
        self.assertEqual(loaded.annotations["identity_controls"][0]["disposition"], "unresolved")
        self.assertEqual(loaded.annotations["identity_controls"][0]["eligibility_preconditions"], preconditions)
        self.assertNotIn("canonical_identity", loaded.summary())

    def test_independent_schema_name_matches_manifest_interface_and_corpus(self):
        independent_schema = "independently-authored-toy-annotation-corpus-9"
        self.corpus["schema"] = independent_schema
        self.interface["corpus_schema_version"] = independent_schema
        self.write_fixture()
        loaded = self.load()
        self.assertEqual(loaded.annotations["schema"], independent_schema)
        self.assertEqual(loaded.summary()["counts"]["sources"], 4)
        self.assertEqual(loaded.summary()["producer_or_topic_recipe_qualification"], "NOT_MEASURED")
        self.interface["corpus_schema_version"] = "different-schema"
        self.write_fixture()
        self.reject()
        self.interface["corpus_schema_version"] = independent_schema
        self.write_fixture()
        corpus = copy.deepcopy(self.corpus)
        corpus["schema"] = "different-schema"
        self.replace_file("corpus", encode(corpus))
        self.reject()

    def test_optional_coverage_and_disposition_ledger_preserved(self):
        metadata = self.corpus["documents"][0]["metadata"]
        metadata.update({
            "coverage": {
                "explicit_alias_clauses": "complete", "explicit_relation_clauses": "complete",
                "unlisted_combinatorial_pairs": "Unlisted endpoint pairs remain unknown",
                "topic_gold": "Complete toy annotation",
            },
            "ambiguities": [{"note": "No canonical realm proof"}],
            "disposition_ledger": [{
                "kind": "alias", "disposition": "unresolved_equivalence",
                "evidence_quote": "Café uses Nacre.", "occurrence": 1,
                "assertion_eligibility": "No accepted assertion", "left": "cafe", "right": "nacre",
                "additional_annotation": {"opaque": "retained"},
            }],
        })
        self.write_fixture()
        self.assertEqual(self.load().annotations["documents"][0]["metadata"], metadata)
        for key, value in (("left", "foreign"), ("occurrence", 2), ("disposition", "supported_equivalence")):
            with self.subTest(key=key):
                raw = copy.deepcopy(self.corpus)
                raw["documents"][0]["metadata"]["disposition_ledger"][0][key] = value
                self.replace_file("corpus", encode(raw))
                self.reject()

    def test_ledger_requires_full_declared_evidence_tuple_for_alias_and_relation(self):
        original = copy.deepcopy(self.corpus)
        for kind in ("alias", "relation"):
            for evidence in ("Café uses Nacre", "Café uses Nacre. Further context."):
                with self.subTest(kind=kind, evidence_length=len(evidence)):
                    self.corpus = copy.deepcopy(original)
                    source = self.corpus["documents"][0]
                    source["text"] = "Café uses Nacre. Further context."
                    row = {
                        "kind": kind,
                        "disposition": "unresolved_equivalence" if kind == "alias" else "supported_navigation",
                        "evidence_quote": evidence,
                        "occurrence": 1,
                        "assertion_eligibility": "Annotation only",
                    }
                    if kind == "alias":
                        row.update({"left": "cafe", "right": "nacre"})
                    else:
                        row.update({"from": "cafe", "to": "nacre", "relation": "uses"})
                    source["metadata"]["disposition_ledger"] = [row]
                    self.write_fixture()
                    # Both quotes are exact and enclose the selected endpoints,
                    # but neither equals the annotated edge's full evidence.
                    self.reject()
                    row["evidence_quote"] = "Café uses Nacre."
                    self.write_fixture()
                    self.assertEqual(self.load().summary()["counts"]["sources"], 4)

    def test_pair_metadata_references_scope_tokens_and_truth_flags_checked(self):
        invalid = [
            ("left", "foreign"), ("right", "PRIVATE_SOURCE_A"),
            ("disposition", "unknown_topic_coverage"),
            ("same_repository_scope_token", False), ("same_environment_scope_token", False),
            ("cross_brain_boundary", True), ("shared_topics", ["foreign topic"]),
            ("scored_topic_positive_or_negative", False), ("topic_truth_status", "unknown"),
            ("unsupported_language_in_pair", 1), ("same_repository", True),
        ]
        for key, value in invalid:
            with self.subTest(key=key):
                raw = copy.deepcopy(self.corpus)
                raw["topic_pair_metadata"][0][key] = value
                self.replace_file("corpus", encode(raw))
                self.reject()
        for mutation in (
            lambda c: c["topic_pair_metadata"].pop(),
            lambda c: c["topic_pair_metadata"].reverse(),
            lambda c: c["topic_pair_metadata"].append(c["topic_pair_metadata"][0]),
        ):
            raw = copy.deepcopy(self.corpus)
            mutation(raw)
            self.replace_file("corpus", encode(raw))
            self.reject()

    def test_unknown_pair_metadata_requires_unknown_status_and_reason(self):
        self.corpus["topic_pairs"][0][2] = "unknown_topic_coverage"
        self.write_fixture()
        for key, value in (("topic_truth_status", "known"), ("scored_topic_positive_or_negative", True), ("topic_coverage_reason", None)):
            with self.subTest(key=key):
                raw = copy.deepcopy(self.corpus)
                raw["topic_pair_metadata"][0][key] = value
                self.replace_file("corpus", encode(raw))
                self.reject()

    def test_conditional_identity_is_metadata_not_fourth_disposition(self):
        self.corpus["identity_controls"][0]["eligibility_preconditions"] = {
            "current_contribution_requires": "Current support",
            "withdrawn_source_may_contribute_current_support": "false",
            "identity_semantics": "Independent eligibility",
        }
        self.write_fixture()
        self.reject()

    def test_unknown_source_preserves_all_controls_and_full_denominators(self):
        self.corpus["documents"][0]["metadata"]["topic_annotation_state"] = "unknown"
        self.corpus["documents"][0]["topics"] = []
        self.corpus["topic_pairs"][0][2] = "unknown_topic_coverage"
        self.write_fixture()
        loaded = self.load()
        self.assertIsNone(loaded.topic_pairs[0].truth)
        counts = loaded.summary()["counts"]
        self.assertEqual((counts["sources"], counts["topic_pairs"]), (4, 6))
        self.assertEqual((counts["topic_pairs_positive"], counts["topic_pairs_negative"], counts["topic_pairs_unknown"]), (0, 5, 1))
        self.assertEqual((counts["topic_sources_known"], counts["topic_sources_unknown"]), (3, 1))
        self.assertEqual(loaded.summary()["complete_topic_gold_readiness"], "NOT_READY")
        self.assertEqual(loaded.summary()["producer_or_topic_recipe_qualification"], "NOT_MEASURED")
        for false_truth in ("unrelated", "related_workflow"):
            self.corpus["topic_pairs"][0][2] = false_truth
            self.write_fixture()
            self.reject()

    def test_pair_unknown_alone_prevents_complete_qualification(self):
        self.corpus["topic_pairs"][0][2] = "unknown_topic_coverage"
        self.write_fixture()
        self.assertEqual(self.load().summary()["complete_topic_gold_readiness"], "NOT_READY")

    def test_empty_topics_never_supply_pair_truth(self):
        for disposition, expected in (("related_explicit", True), ("unrelated", False), ("unknown_topic_coverage", None)):
            self.corpus["documents"][0]["topics"] = []
            self.corpus["topic_pairs"][0][2] = disposition
            self.write_fixture()
            self.assertIs(self.load().topic_pairs[0].truth, expected)

    def test_controls_require_explicit_dispositions_without_token_identity_inference(self):
        # X uses the same repository/environment tokens, but another Brain.
        for index, wrong in ((1, "unrelated"), (2, "related_shared_tokens"), (5, "unrelated_empty")):
            with self.subTest(index=index):
                self.mutate_reject(lambda c: c["topic_pairs"][index].__setitem__(2, wrong))
        self.corpus["documents"][1]["text"] = " "
        self.corpus["documents"][1]["mentions"] = []
        self.corpus["topic_pairs"][0][2] = "unrelated"
        self.write_fixture()
        self.assertIs(self.load().topic_pairs[0].truth, False)

    def test_unknown_empty_source_still_has_unknown_source_coverage(self):
        self.corpus["documents"][2]["metadata"]["topic_annotation_state"] = "unknown"
        self.write_fixture()
        summary = self.load().summary()
        self.assertEqual(summary["counts"]["topic_pairs_unknown"], 0)
        self.assertEqual(summary["complete_topic_gold_readiness"], "NOT_READY")

    def test_every_pin_rejects_raw_drift(self):
        for kind in ("corpus", "guide", "source_hashes", "interface"):
            with self.subTest(kind=kind):
                self.write_fixture()
                path = ROOT / self.manifest[f"{kind}_path"]
                path.write_bytes(path.read_bytes() + b" ")
                self.reject()
        self.write_fixture()
        self.manifest_path.write_bytes(self.manifest_path.read_bytes() + b" ")
        self.reject()

    def test_exact_source_hashes_reject_drift_missing_and_foreign_ids(self):
        for mutate in (
            lambda h: h["documents"].__setitem__("B", "0" * 64),
            lambda h: h["documents"].pop("B"),
            lambda h: h["documents"].__setitem__("foreign", "0" * 64),
        ):
            with self.subTest(mutation=mutate):
                self.write_fixture()
                hashes = json.loads((ROOT / self.manifest["source_hashes_path"]).read_bytes())
                mutate(hashes)
                self.replace_file("source_hashes", encode(hashes))
                self.reject()

    def test_duplicate_missing_self_and_foreign_pairs_reject(self):
        mutations = [
            lambda c: c["topic_pairs"].pop(),
            lambda c: c["topic_pairs"].append(list(reversed(c["topic_pairs"][0][:2])) + ["unrelated"]),
            lambda c: c["topic_pairs"][0].__setitem__(1, "PRIVATE_SOURCE_A"),
            lambda c: c["topic_pairs"][0].__setitem__(1, "foreign"),
        ]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.mutate_reject(mutation)

    def test_unicode_half_open_spans_and_quote_equality(self):
        for start, end, quote in ((0, 4, "Café"), (4, 5, "é"), (0, 5, "Cafe"), (-1, 5, "Café"), (0, 0, ""), (False, 5, "Café"), (0, 18, "Café")):
            with self.subTest(span=(start, end)):
                self.mutate_reject(lambda c: c["documents"][0]["mentions"][0].__setitem__(slice(2, 5), [start, end, quote]))

    def test_endpoint_evidence_must_enclose_selected_occurrences(self):
        for field, offset in (("aliases", 3), ("relations", 4)):
            with self.subTest(field=field):
                # The quote is exact but excludes the first endpoint.
                self.mutate_reject(lambda c: c["documents"][0][field][0].__setitem__(slice(offset, offset + 3), [11, 16, "Nacre"]))

    def test_foreign_local_keys_duplicate_mentions_and_identity_refs_reject(self):
        mutations = [
            lambda c: c["documents"][0]["relations"][0].__setitem__(1, "foreign"),
            lambda c: c["documents"][0]["aliases"][0].__setitem__(0, "foreign"),
            lambda c: c["documents"][0]["mentions"].append(c["documents"][0]["mentions"][0]),
            lambda c: c["identity_controls"][0]["left"].__setitem__("document_id", "foreign"),
            lambda c: c["identity_controls"][0]["left"].__setitem__("mention_key", "foreign"),
            lambda c: c["identity_controls"].append(c["identity_controls"][0]),
            lambda c: c["documents"].append(c["documents"][0]),
        ]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.mutate_reject(mutation)

    def test_duplicate_occurrences_and_unordered_aliases_reject(self):
        duplicate = list(self.corpus["documents"][0]["mentions"][0])
        duplicate[0] = "new_key_same_gold_occurrence"
        self.mutate_reject(lambda c: c["documents"][0]["mentions"].append(duplicate))
        reverse_alias = list(self.corpus["documents"][0]["aliases"][0])
        reverse_alias[0], reverse_alias[1] = reverse_alias[1], reverse_alias[0]
        self.mutate_reject(lambda c: c["documents"][0]["aliases"].append(reverse_alias))

    def test_unsupported_declarations_outcome_fields_and_enums_reject(self):
        mutations = [
            lambda c: c.__setitem__("provider_outcomes", []),
            lambda c: c["documents"][0].__setitem__("score", 1),
            lambda c: c["documents"][0]["mentions"][0].append(0.99),
            lambda c: c["identity_controls"][0].__setitem__("outcome", "success"),
            lambda c: c["documents"][0]["mentions"][0].__setitem__(1, "unknown_kind"),
            lambda c: c["documents"][0]["metadata"].__setitem__("topic_annotation_state", "partial"),
            lambda c: c["topic_pairs"][0].__setitem__(2, "maybe"),
            lambda c: c["identity_controls"][0].__setitem__("disposition", "conditional"),
        ]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.mutate_reject(mutation)
        self.interface["enums"]["Kind"].append("provider_score")
        self.write_fixture()
        self.reject()

    def test_duplicate_json_keys_reject_at_all_levels(self):
        for kind, raw in (
            ("corpus", b'{"schema":"a","schema":"b"}'),
            ("corpus", encode(self.corpus).replace(b'"topic_annotation_state": "complete"', b'"topic_annotation_state":"complete","topic_annotation_state":"unknown"', 1)),
            ("source_hashes", b'{"schema":"toy","documents":{"x":"a","x":"b"}}'),
            ("interface", b'{"schema":"a","schema":"b"}'),
        ):
            with self.subTest(kind=kind):
                self.write_fixture()
                self.replace_file(kind, raw)
                self.reject()
        self.write_fixture()
        raw = self.manifest_path.read_bytes().replace(b'"schema":', b'"schema":"bad","schema":', 1)
        self.manifest_path.write_bytes(raw)
        self.manifest_hash = sha(raw)
        self.reject()

    def test_encoding_surrogates_and_nonfinite_json_reject(self):
        for raw in (b'\xff', b'{"x":"\\ud800"}', b'{"x":NaN}', b'{"x":1e999}'):
            with self.subTest(raw=raw):
                self.replace_file("corpus", raw)
                self.reject()

    def test_explicit_manifest_and_inside_repo_paths_required(self):
        for path in ("../foreign.json", "/etc/passwd", "https://example.invalid/data"):
            with self.subTest(path=path):
                self.write_fixture()
                self.manifest["corpus_path"] = path
                self.write_manifest()
                self.reject()
        self.write_fixture()
        self.manifest["corpus_path"] = self.manifest["guide_path"]
        self.write_manifest()
        self.reject()
        link = self.directory / "foreign-link"
        link.symlink_to(Path("/etc/passwd"))
        self.write_fixture()
        self.manifest["corpus_path"] = str(link.relative_to(ROOT))
        self.write_manifest()
        self.reject()
        with self.assertRaises(loader.CorpusValidationError):
            loader.load_frozen_corpus("../unknown", manifest_sha256="0" * 64)

    def test_finite_document_file_depth_and_collection_bounds(self):
        self.corpus["documents"] = [document(str(i), "") for i in range(251)]
        self.write_fixture()
        self.reject()
        self.corpus = corpus_fixture()
        self.write_fixture()
        self.replace_file("corpus", b" " * (loader.FILE_LIMITS["corpus"] + 1))
        self.reject()
        self.write_fixture()
        node = {}
        self.corpus["metadata"] = node
        for _ in range(40):
            child = {}
            node["child"] = child
            node = child
        self.write_fixture()
        self.reject()
        self.corpus = corpus_fixture()
        self.mutate_reject(lambda c: c["documents"][0].__setitem__("mentions", c["documents"][0]["mentions"] * 65))
        self.mutate_reject(lambda c: c["documents"][0].__setitem__("topics", [str(i) for i in range(257)]))

    def test_full_250_document_cohort_admitted_without_prefix_selection(self):
        self.corpus = {
            "schema": CORPUS_SCHEMA, "metadata": {},
            "documents": [document(str(i), "") for i in range(250)],
            "topic_pairs": [[str(a), str(b), "unrelated_empty"] for a, b in itertools.combinations(range(250), 2)],
            "identity_controls": [], "topic_pair_metadata": [],
        }
        self.write_fixture()
        counts = self.load().summary()["counts"]
        self.assertEqual(counts["sources"], 250)
        self.assertEqual(counts["topic_pairs"], 31_125)
        self.assertEqual(counts["topic_pairs_negative"], 31_125)

    def test_source_guide_interface_and_json_node_overflow_reject(self):
        for kind in ("guide", "interface", "source_hashes"):
            with self.subTest(kind=kind):
                self.write_fixture()
                self.replace_file(kind, b" " * (loader.FILE_LIMITS[kind] + 1))
                self.reject()
        self.corpus["documents"][1]["text"] = "x" * (2 * 1024 * 1024 + 1)
        self.write_fixture()
        self.reject()
        self.corpus = corpus_fixture()
        self.corpus["metadata"]["bounded_opaque_values"] = [None] * 1_000_001
        self.write_fixture()
        self.reject()
        self.write_fixture()
        raw = b" " * (16 * 1024 + 1)
        self.manifest_path.write_bytes(raw)
        self.manifest_hash = sha(raw)
        self.reject()

    def test_resource_preflight_precedes_parser_allocation_and_ignores_quoted_delimiters(self):
        for raw in (b"[" * 34 + b"0" + b"]" * 34,
                    b"[" + b"0," * 1_000_002 + b"0]"):
            with self.subTest(length=len(raw)), \
                 mock.patch.object(loader.json, "loads", side_effect=AssertionError("parser allocation forbidden")):
                with self.assertRaises(loader.CorpusValidationError):
                    loader._parse_json(raw)
        self.corpus["metadata"]["opaque_quoted_punctuation"] = '{[,:"\\' * 100
        self.write_fixture()
        self.assertEqual(self.load().annotations["metadata"], self.corpus["metadata"])

    def test_manifest_and_interface_unsupported_schema_and_fields_reject(self):
        for mutate in (
            lambda m: m.__setitem__("schema", "future-schema"),
            lambda m: m.__setitem__("corpus_schema", "future-corpus"),
            lambda m: m.__setitem__("corpus_schema", ""),
            lambda m: m.__setitem__("corpus_schema", True),
            lambda m: m.__setitem__("provider_outcomes", []),
        ):
            self.write_fixture()
            mutate(self.manifest)
            self.write_manifest()
            self.reject()
        self.write_fixture()
        self.interface["Document"]["text"] = "arbitrary provider outcome"
        self.write_fixture()
        self.reject()
        self.interface = interface_fixture()
        self.interface["outcome_score"] = 1
        self.write_fixture()
        self.reject()

    def test_cli_has_no_annotation_content_on_success_failure_or_bad_arguments(self):
        for argv, expected in (([str(self.manifest_path), "--sha256", self.manifest_hash], 0),
                               (["PRIVATE_PATH_SENTINEL", "--sha256", "0" * 64], 1)):
            out, err = io.StringIO(), io.StringIO()
            with redirect_stdout(out), redirect_stderr(err):
                self.assertEqual(loader.main(argv), expected)
            self.assertNotIn("PRIVATE", out.getvalue() + err.getvalue())
            if expected == 0:
                summary = json.loads(out.getvalue())
                self.assertEqual(summary["counts"]["sources"], 4)
                self.assertEqual(summary["producer_or_topic_recipe_qualification"], "NOT_MEASURED")
                self.assertNotIn("complete_topic_qualification", summary)
            else:
                self.assertEqual(err.getvalue(), "frozen corpus validation failed\n")
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err), self.assertRaises(SystemExit) as raised:
            loader.main(["--PRIVATE_UNSUPPORTED_FLAG"])
        self.assertEqual(raised.exception.code, 2)
        self.assertNotIn("PRIVATE", out.getvalue() + err.getvalue())

    def test_no_provider_or_scorer_execution_is_available(self):
        # Offline admission still works with process and network launch disabled.
        with mock.patch("socket.socket", side_effect=AssertionError("network forbidden")), \
             mock.patch("subprocess.Popen", side_effect=AssertionError("process forbidden")):
            self.assertEqual(self.load().summary()["counts"]["sources"], 4)


if __name__ == "__main__":
    unittest.main()
