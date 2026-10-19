import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('feedback_score', Path(__file__).resolve().parents[1] / 'score-feedback.py')
score = importlib.util.module_from_spec(spec)
spec.loader.exec_module(score)


class UtilityBoundaryTests(unittest.TestCase):
    def fixture(self):
        base = {'brain_id': 'brain', 'scope': {'repository_ids': ['repo'], 'environment_id': 'dev'}, 'input_kind': 'source_version'}
        a = dict(base, revision_id='a', baseline_score=1/61, eligible=True)
        b = dict(base, revision_id='b', baseline_score=1/62, eligible=True)
        vote = dict(base, revision_id='b', actor_id='actor', task_id='past', query_context_id='query',
                    explicit=True, eligible=True, current=True, partition='training', vote='helpful', sequence=1)
        query = dict(id='query', brain_id='brain', scope=base['scope'], task_id='current', partition='held_out',
                     candidates=[a, b], relevance={'a': 0, 'b': 1}, cutoff=1, outcome='completed')
        return {'recipe': score.RECIPE, 'production_rrf_k': 60, 'feedback': [vote], 'queries': [query]}

    def test_repeated_queries_and_changes_do_not_multiply_task_reward(self):
        fixture = self.fixture()
        fixture['feedback'] += [dict(fixture['feedback'][0], query_context_id=f'repeat-{n}') for n in range(100)]
        report = score.compare(fixture)
        self.assertEqual(report['independent_reward_votes'], 1)
        self.assertEqual(report['queries'][0]['experimental_order'], ['b', 'a'])
        fixture['feedback'].append(dict(fixture['feedback'][0], sequence=2, vote='unhelpful'))
        report = score.compare(fixture)
        self.assertEqual(report['independent_reward_votes'], 1)
        self.assertEqual(report['queries'][0]['experimental_order'], ['a', 'b'])

    def test_current_task_heldout_implicit_foreign_and_erased_feedback_cannot_reward(self):
        for patch in [{'task_id': 'current'}, {'partition': 'held_out'}, {'explicit': False},
                      {'brain_id': 'foreign'}, {'current': False}, {'eligible': False},
                      {'scope': {'repository_ids': ['repo'], 'environment_id': 'prod'}}]:
            fixture = self.fixture()
            fixture['feedback'][0].update(patch)
            report = score.compare(fixture)
            self.assertEqual(report['queries'][0]['experimental_order'], ['a', 'b'], patch)

    def test_ineligible_candidate_is_not_created_by_positive_feedback(self):
        fixture = self.fixture()
        fixture['queries'][0]['candidates'][1]['eligible'] = False
        report = score.compare(fixture)
        self.assertEqual(report['queries'][0]['experimental_order'], ['a'])
        self.assertEqual(report['queries'][0]['withheld_candidates'], 1)

    def test_failed_query_retains_the_relevance_denominator(self):
        fixture = self.fixture()
        failed = copy.deepcopy(fixture['queries'][0])
        failed.update(id='failed', task_id='another', outcome='unavailable')
        fixture['queries'].append(failed)
        report = score.compare(fixture)
        self.assertEqual(report['partitions']['held_out']['queries'], 2)
        self.assertEqual(report['partitions']['held_out']['experimental']['mrr'], .5)
        self.assertFalse(report['production_ranking_changed'])

    def test_mistaken_explicit_reward_is_reported_as_a_bad_promotion(self):
        fixture = self.fixture()
        fixture['queries'][0]['relevance'] = {'a': 1, 'b': 0}
        report = score.compare(fixture)
        self.assertEqual(report['queries'][0]['bad_promotions'], 1)
        self.assertLess(report['partitions']['all']['experimental']['mrr'], report['partitions']['all']['baseline']['mrr'])


if __name__ == '__main__':
    unittest.main()
