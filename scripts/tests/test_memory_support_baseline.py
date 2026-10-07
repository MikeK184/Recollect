import copy
import importlib.util
import json
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "memory_support_baseline.py"
spec = importlib.util.spec_from_file_location("memory_support_baseline", SCRIPT)
baseline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(baseline)


class SupportBaselineTests(unittest.TestCase):
    def setUp(self):
        self.corpus = baseline.load(baseline.CORPUS)
        self.results = {
            "version": 1, "corpus": self.corpus["name"], "mode": "fixture",
            "versions": {key: "fixture-v1" for key in ("requested_model", "returned_model", "prompt", "schema", "verifier")},
            "observations": [{"case_id": case["id"], "state": "completed",
                              "verdict": case["expected"]["verdict"], "usable": case["expected"]["usable"],
                              "request_count": 0, "charged_tokens": 0, "latency_ms": 0}
                             for case in self.corpus["cases"] if case["partition"] == "held_out"],
        }

    def test_corpus_has_valid_but_unsupported_quotes_and_positive_controls(self):
        summary = baseline.validate_corpus(self.corpus)
        self.assertEqual(summary["partitions"]["held_out"]["cases"], 26)
        self.assertGreaterEqual(summary["partitions"]["held_out"]["positive"], 10)
        self.assertFalse(summary["semantic_quality_measured"])
        case = next(case for case in self.corpus["cases"] if case["id"] == "held-valid-unrelated")
        self.assertIn(case["candidate"]["quote"], case["evidence"]["text"])
        self.assertFalse(case["expected"]["usable"])

    def test_oracle_fixture_does_not_prove_semantic_quality(self):
        result = baseline.score(self.corpus, self.results, "held_out")
        self.assertTrue(result["fixture_enforcement_met"])
        self.assertFalse(result["declared_quality_thresholds_met"])
        self.assertFalse(result["runtime_evidence_verified"])
        self.assertEqual(result["request_count"], 0)

    def test_critical_false_publication_fails_even_with_correct_verdict(self):
        bad = next(obs for obs in self.results["observations"] if obs["verdict"] != "supported")
        bad["usable"] = True
        result = baseline.score(self.corpus, self.results, "held_out")
        self.assertFalse(result["fixture_enforcement_met"])
        self.assertIn(bad["case_id"], result["critical_false_usable"])

    def test_withholding_everything_cannot_pass(self):
        for observation in self.results["observations"]:
            observation["usable"] = False
        result = baseline.score(self.corpus, self.results, "held_out")
        self.assertEqual(result["positive_usability"], 0)
        self.assertFalse(result["fixture_enforcement_met"])

    def test_positive_published_despite_failed_assessment_cannot_pass(self):
        positive = next(obs for obs in self.results["observations"] if obs["usable"])
        positive["verdict"] = "contradicted"
        result = baseline.score(self.corpus, self.results, "held_out")
        self.assertFalse(result["fixture_enforcement_met"])
        self.assertIn(positive["case_id"], result["unverified_usable"])

    def test_missing_and_uncertain_are_not_dropped_from_denominator(self):
        missing = self.results["observations"].pop()
        self.results["observations"][0].update(state="uncertain", verdict=None, usable=False)
        result = baseline.score(self.corpus, self.results, "held_out")
        self.assertEqual(result["expected_cases"], 26)
        self.assertIn(missing["case_id"], result["missing"])
        self.assertEqual(result["uncertain"], 1)
        self.assertFalse(result["fixture_enforcement_met"])

    def test_duplicate_unknown_and_invalid_metrics_are_rejected(self):
        for mutation in ("duplicate", "unknown", "negative", "boolean", "nonfinite", "overflow", "fixture-charge"):
            with self.subTest(mutation=mutation):
                results = copy.deepcopy(self.results)
                if mutation == "duplicate":
                    results["observations"].append(results["observations"][0])
                elif mutation == "unknown":
                    results["observations"][0]["case_id"] = "unknown"
                elif mutation == "negative":
                    results["observations"][0]["charged_tokens"] = -1
                elif mutation == "boolean":
                    results["observations"][0]["request_count"] = True
                elif mutation == "nonfinite":
                    results["observations"][0]["latency_ms"] = float("nan")
                elif mutation == "overflow":
                    results["observations"][0]["charged_tokens"] = 10 ** 400
                else:
                    results["observations"][0]["charged_tokens"] = 1
                with self.assertRaises(ValueError):
                    baseline.score(self.corpus, results, "held_out")

    def test_duplicate_json_fields_cannot_hide_a_favorable_value(self):
        with self.assertRaises(ValueError):
            json.loads('{"charged_tokens": 40, "charged_tokens": 0}', object_pairs_hook=baseline.unique_object)

    def test_invalid_span_quote_and_threshold_changes_are_rejected(self):
        for mutation in ("range", "quote", "threshold"):
            with self.subTest(mutation=mutation):
                corpus = copy.deepcopy(self.corpus)
                if mutation == "range":
                    corpus["cases"][0]["candidate"]["line_to"] = 100
                elif mutation == "quote":
                    corpus["cases"][0]["candidate"]["quote"] = "Unrelated text"
                else:
                    corpus["thresholds"]["minimum_positive_usability"] = .5
                with self.assertRaises(ValueError):
                    baseline.validate_corpus(corpus)

    def test_declared_model_results_still_need_independent_runtime_proof(self):
        self.results["mode"] = "real-model"
        result = baseline.score(self.corpus, self.results, "held_out")
        self.assertTrue(result["declared_quality_thresholds_met"])
        self.assertFalse(result["runtime_evidence_verified"])


if __name__ == "__main__":
    unittest.main()
