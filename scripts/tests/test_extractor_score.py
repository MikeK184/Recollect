import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("extractor_score", Path(__file__).parents[1] / "score-extractor.py")
score = importlib.util.module_from_spec(spec)
spec.loader.exec_module(score)


class ExtractorScoreTests(unittest.TestCase):
    def test_absent_outcomes_retain_every_document_and_english_fn(self):
        report = score.score(score.load_corpora(), [])
        self.assertEqual(report["outcome_documents"], 60)
        self.assertEqual(report["states"], {"not_attempted": 60})
        self.assertEqual(report["corpora"]["original40"]["development"]["outcome_documents"], 30)
        self.assertEqual(report["corpora"]["adversarial20"]["held_out"]["outcome_documents"], 8)
        self.assertGreater(report["corpora"]["adversarial20"]["held_out"]["english_exact_span_type"]["fn"], 0)
        self.assertEqual(report["corpora"]["original40"]["all"]["endpoints"].split(":")[0], "NOT_ANNOTATED")
        self.assertFalse(report["corpora"]["adversarial20"]["held_out"]["unsupported_controls"][0]["passed"])

    def test_fresh_heldout_stays_separate_and_endpoints_are_unannotated(self):
        corpora = score.load_corpora(include_fresh=True)
        report = score.score(corpora, [])
        self.assertEqual(report["outcome_documents"], 72)
        fresh = report["corpora"]["fresh12"]["held_out"]
        self.assertEqual(fresh["outcome_documents"], 12)
        self.assertEqual(fresh["english_exact_span_type"]["fn"], 46)
        self.assertTrue(fresh["endpoints"].startswith("NOT_ANNOTATED"))
        self.assertNotIn("aliases", fresh)
        self.assertNotIn("relations", fresh)
        self.assertEqual(report["corpora"]["fresh12"]["development"]["outcome_documents"], 0)
        self.assertFalse(report["corpora"]["fresh12"]["development"]["entity_micro_target_passed"])

    def test_duplicate_predictions_count_as_fp(self):
        m = ("technology", 0, 4, "Rust")
        self.assertEqual(score.counts([m], [m, m]), {"tp": 1, "fp": 1, "fn": 0})

    def test_foreign_or_duplicate_attempt_is_rejected(self):
        corpora = score.load_corpora()
        with self.assertRaises(ValueError):
            score.score(corpora, [{"corpus": "foreign", "id": "x"}])
        row = {"corpus": "original40", "id": "mapping-01", "state": "gateway_failed"}
        with self.assertRaises(ValueError):
            score.score(corpora, [row, row])

    def test_german_is_observed_without_manufacturing_an_unsupported_pass(self):
        row = {"corpus": "adversarial20", "id": "h07", "state": "mapping_absent", "raw_output": {"mapping": None}}
        report = score.score(score.load_corpora(), [row])
        control = report["corpora"]["adversarial20"]["held_out"]["unsupported_controls"][0]
        self.assertTrue(control["passed"])
        row["state"] = "validated"
        row["raw_output"] = {"mapping": {"language": "en"}}
        row["candidates"] = {"mentions": [], "aliases": [], "relations": []}
        report = score.score(score.load_corpora(), [row])
        self.assertFalse(report["corpora"]["adversarial20"]["held_out"]["unsupported_controls"][0]["passed"])

    def test_unicode_codepoint_gold_converts_to_utf8_bytes(self):
        doc = {"text": "🔐 Zoë", "mentions": [{"type": "person", "start": 2, "end": 5, "text": "Zoë"}]}
        self.assertEqual(score.gold_mentions("original40", doc)["0"], ("person", 5, 9, "Zoë"))


if __name__ == "__main__":
    unittest.main()
