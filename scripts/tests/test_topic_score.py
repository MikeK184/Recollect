import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('topic_score', Path(__file__).resolve().parents[1] / 'score-topics.py')
score = importlib.util.module_from_spec(spec)
spec.loader.exec_module(score)


class TopicScoreTests(unittest.TestCase):
    def fixture(self):
        docs = [{'id': key, 'partition': part, 'scope': scope, 'language': lang,
                 'text': 'Rust', 'mentions': [['rust', 'technology', 0, 4, 'Rust']], 'topics': topics}
                for key, part, scope, lang, topics in (
                    ('a', 'development', 'alpha/repo/DEV', 'en', ['code']),
                    ('b', 'held_out', 'alpha/repo/PROD', 'de', ['code']),
                    ('c', 'held_out', 'beta/repo/PROD', 'en', ['code']),
                    ('d', 'held_out', 'alpha/unknown/unknown', 'en', []))]
        corpus = {'documents': docs, 'topic_pairs': []}
        group = {'id': 'shared', 'label': 'Rust', 'provenance_kind': 'exact_entity_mention',
                 'label_evidence': {'document_id': 'a', 'byte_start': 0, 'byte_end': 4, 'quote': 'Rust'}}
        receipt = {'input_mode': 'gold_upper_bound', 'model_calls': 0,
                   'baselines': [{'name': name, 'state': 'not_measured', 'reason': 'fixture'}
                                 for name in sorted(score.BASELINES - {'native_weighted'})] +
                                [{'name': 'native_weighted', 'state': 'completed', 'documents':
                                  [{'id': 'a', 'outcome': 'assigned', 'groups': [group]},
                                   {'id': 'b', 'outcome': 'unsupported', 'groups': []},
                                   {'id': 'c', 'outcome': 'assigned', 'groups': [group]},
                                   {'id': 'd', 'outcome': 'unassigned', 'groups': []}]}]}
        return corpus, receipt

    def native(self, corpus, receipt):
        return next(b for b in score.compare(corpus, receipt)['baselines'] if b['name'] == 'native_weighted')

    def test_unsupported_and_missing_stay_in_denominator_cross_brain_is_separate(self):
        corpus, receipt = self.fixture()
        native = self.native(corpus, receipt)
        self.assertEqual(native['assignment']['all']['coverage'], 2 / 3)
        self.assertEqual(native['pair_metrics']['all']['fn'], 1)
        self.assertEqual(native['pair_metrics']['all']['cross_brain_shared_groups'], 1)
        self.assertEqual(native['pair_metrics']['all']['truth_sources']['unknown'], 2)
        receipt['baselines'][-1]['documents'].pop(1)
        native = self.native(corpus, receipt)
        self.assertEqual(native['missing_document_outcomes'], ['b'])
        self.assertEqual(native['assignment']['all']['coverage'], 2 / 3)
        self.assertFalse(score.compare(corpus, receipt)['production_recipe_qualified'])

    def test_explicit_unrelated_overrides_shared_category_without_inventing_unknown_negatives(self):
        corpus, receipt = self.fixture()
        corpus['topic_pairs'] = [['a', 'b', 'unrelated']]
        native = self.native(corpus, receipt)
        self.assertEqual(native['pair_metrics']['all']['fn'], 0)
        self.assertEqual(native['pair_metrics']['all']['scored_pairs'], 1)
        self.assertEqual(len(native['pairs']), 6)

    def test_existing_corpus_pair_labels_are_supported_before_any_metrics(self):
        corpus, receipt = self.fixture()
        corpus['topic_pairs'] = [['a', 'd', 'unrelated_empty'], ['a', 'c', 'cross_Brain_no_membership_or_count']]
        native = self.native(corpus, receipt)
        self.assertEqual(native['pair_metrics']['all']['scored_pairs'], 2)
        self.assertEqual(native['pair_metrics']['all']['cross_brain_shared_groups'], 1)
        actual = score.read(Path(__file__).resolve().parents[2] / 'docs/research/graph-organization-adversarial-fixtures-2026-10-18.json')
        receipt['baselines'] = [{'name': name, 'state': 'not_measured', 'reason': 'parser control only'} for name in score.BASELINES]
        parsed = score.compare(actual, receipt)
        self.assertEqual((parsed['documents'], parsed['total_pairs']), (20, 190))

    def test_label_must_be_exact_member_span_and_does_not_prove_usefulness(self):
        corpus, receipt = self.fixture()
        native = self.native(corpus, receipt)
        self.assertEqual(native['label_rubric']['supported'], 1)
        self.assertEqual(native['label_rubric']['semantic_usefulness'], 'manual_review_required')
        for row in receipt['baselines'][-1]['documents']:
            for group in row['groups']:
                group['label_evidence']['document_id'] = 'd'
        self.assertEqual(self.native(corpus, receipt)['label_rubric']['supported'], 0)

    def test_failed_outcome_cannot_hide_groups_or_claim_paid_gold_experiment(self):
        corpus, receipt = self.fixture()
        receipt['baselines'][-1]['documents'][0]['outcome'] = 'failed'
        with self.assertRaises(ValueError):
            self.native(corpus, receipt)

    def test_declared_metadata_label_requires_independent_native_evidence(self):
        corpus, receipt = self.fixture()
        for row in receipt['baselines'][-1]['documents']:
            for group in row['groups']:
                group['provenance_kind'] = 'canonical_scope'
                group.pop('label_evidence', None)
        rubric = self.native(corpus, receipt)['label_rubric']
        self.assertEqual(rubric['supported'], 0)
        self.assertEqual(rubric['unverified_canonical_metadata_labels'], 1)
        receipt['model_calls'] = 1
        with self.assertRaises(ValueError):
            self.native(corpus, receipt)


if __name__ == '__main__':
    unittest.main()
