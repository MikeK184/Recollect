import importlib.util
from pathlib import Path
import unittest
import json

SPEC = importlib.util.spec_from_file_location('reconciliation_score', Path(__file__).resolve().parents[1] / 'score-reconciliation-controls.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

class ReconciliationScoreTests(unittest.TestCase):
    def outcome(self, ident, raw=None, **extra):
        case = next(c for c in MODULE.load()['cases'] if c['id'] == ident)
        memory = case['canonical_memories'][0]
        actual = '71000000-0000-4000-8000-000000000001'
        row = {'id': ident, 'state': 'mapping_absent',
               'canonical_id_map': {'prior_revision': {'actual': actual, 'frozen': memory['revision_id'], 'symbol': memory['symbol']}},
               'actual_offered_revision_ids': [actual] if case['offered_memory_symbols'] else [],
               'raw_output': raw if raw is not None else {'claims': [], 'retirements': []}}
        return row | extra

    def test_all_six_missing_controls_remain_in_denominator(self):
        report = MODULE.score(MODULE.load(), [])
        self.assertEqual(report['control_denominator'], 6)
        self.assertEqual(report['partitions'], {'development': 3, 'held_out': 3})
        self.assertEqual(sum(r['mentions']['fn'] for r in report['controls']), 21)
        self.assertEqual(sum(r['typed_relation_endpoints']['fn'] for r in report['controls']), 4)

    def test_dynamic_ids_cannot_hide_manual_target_or_forbidden_retirement(self):
        corpus = MODULE.load()
        report = MODULE.score(corpus, [{'id': 'rc06', 'state': 'mapping_absent',
            'canonical_id_map': {'prior_revision': {'actual': 'runtime'}},
            'raw_output': {'claims': [{'replaces_revision': 'runtime'}], 'retirements': []}},
            {'id': 'rc02', 'state': 'mapping_absent', 'raw_output': {'claims': [],
                'retirements': [{'revision_id': corpus['cases'][1]['canonical_memories'][0]['revision_id']}]}}])
        self.assertTrue(report['controls'][5]['hard_safety_failures'])
        self.assertTrue(report['controls'][1]['hard_safety_failures'])

    def test_duplicate_attempt_cannot_select_a_better_result(self):
        with self.assertRaises(ValueError):
            MODULE.score(MODULE.load(), [{'id': 'rc01', 'state': 'not_attempted'}] * 2)

    def test_malformed_response_is_counted_as_failed_control(self):
        report = MODULE.score(MODULE.load(), [{'id': 'rc01', 'state': 'adapter_rejected', 'raw_output': {'claims': 'wrong', 'retirements': []}}])
        first = report['controls'][0]
        self.assertIn('malformed_control_output', first['hard_safety_failures'])
        self.assertEqual(first['exact_required_claims_matched'], 0)
        self.assertEqual(report['control_denominator'], 6)

    def test_non_object_array_members_are_failures_not_benign_empty_responses(self):
        for ident in ('rc02', 'rc05'):
            for raw in ({'claims': [1], 'retirements': []}, {'claims': [], 'retirements': [None]}):
                report = MODULE.score(MODULE.load(), [self.outcome(ident, raw)])
                row = next(r for r in report['controls'] if r['id'] == ident)
                self.assertIn('malformed_control_output', row['hard_safety_failures'])
                self.assertEqual(row['raw_claim_candidates'], raw['claims'])

    def test_frozen_uuid_is_not_an_actually_offered_runtime_target(self):
        case = MODULE.load()['cases'][2]
        row = self.outcome('rc03', {'claims': [], 'retirements': [{
            'revision_id': case['canonical_memories'][0]['revision_id'],
            'reason': 'Explicitly withdrawn in line one.', 'line_from': 1, 'line_to': 1}]})
        result = MODULE.score(MODULE.load(), [row])['controls'][2]
        self.assertIn('target_absent_from_actual_offered_enum', result['hard_safety_failures'])
        # A frozen symbol must never count as a successfully emitted runtime action.
        self.assertEqual(result['missing_required_retirements'], 1)

    def test_wrong_or_missing_runtime_bijection_cannot_count_required_actions(self):
        for changes in ({'actual_offered_revision_ids': []}, {'canonical_id_map': {}},
                        {'actual_offered_revision_ids': ['not-a-uuid']}):
            row = self.outcome('rc03', **changes)
            result = MODULE.score(MODULE.load(), [row])['controls'][2]
            self.assertFalse(result['runtime_binding_checked'])
            self.assertIn('missing_or_invalid_runtime_binding', result['hard_safety_failures'])

    def test_retirement_requires_exact_runtime_target_and_cited_span(self):
        for line, missing in ((1, 0), (2, 1)):
            row = self.outcome('rc03')
            row['raw_output']['retirements'] = [{'revision_id': row['actual_offered_revision_ids'][0],
                'reason': 'The earlier assertion is explicitly withdrawn.', 'line_from': line, 'line_to': line}]
            result = MODULE.score(MODULE.load(), [row])['controls'][2]
            self.assertEqual(result['missing_required_retirements'], missing)
            self.assertIn('NOT_MEASURED', result['canonical_mutation_support_quality'])

    def test_malformed_adapted_mapping_preserves_all_denominators(self):
        for invalid in ({'span': None}, 1):
            row = self.outcome('rc01', state='validated', candidates={'mentions': [invalid], 'relations': [], 'aliases': []})
            report = MODULE.score(MODULE.load(), [row])
            self.assertEqual(report['control_denominator'], 6)
            self.assertEqual(len(report['controls']), 6)
            self.assertIn('malformed_adapted_mapping', report['controls'][0]['hard_safety_failures'])
            self.assertEqual(sum(r['mentions']['fn'] for r in report['controls']), 21)

    def test_missing_transport_state_is_retained_as_a_failed_row(self):
        row = self.outcome('rc02')
        del row['state']
        result = MODULE.score(MODULE.load(), [row])['controls'][1]
        self.assertEqual(result['state'], 'missing_state')
        self.assertIn('invalid_transport_state', result['hard_safety_failures'])

    def test_malformed_relation_invalidates_the_entire_adapted_batch(self):
        case = MODULE.load()['cases'][0]
        mapping = {'mentions': case['gold']['mapping']['mentions'], 'relations': [{
            'from': 'm3', 'to': 'foreign', 'relation': 'uses',
            'span': case['gold']['mapping']['relations'][0]['span']}], 'aliases': []}
        row = self.outcome('rc01', state='validated', candidates=mapping)
        result = MODULE.score(MODULE.load(), [row])['controls'][0]
        self.assertIn('malformed_adapted_mapping', result['hard_safety_failures'])
        self.assertEqual(result['mentions'], {'tp': 0, 'fp': 0, 'fn': 4})
        self.assertEqual(result['typed_relation_endpoints'], {'tp': 0, 'fp': 0, 'fn': 1})

    def test_foreign_and_identical_alias_endpoints_invalidate_whole_batch(self):
        case = MODULE.load()['cases'][4]
        for right in ('foreign', 'm1'):
            mapping = {'mentions': case['gold']['mapping']['mentions'], 'relations': [],
                       'aliases': [{'left': 'm1', 'right': right,
                                    'span': case['gold']['mapping']['mentions'][0]['span']}]}
            result = MODULE.score(MODULE.load(), [self.outcome('rc05', state='validated', candidates=mapping)])['controls'][4]
            self.assertIn('malformed_adapted_mapping', result['hard_safety_failures'])
            self.assertEqual(result['mentions'], {'tp': 0, 'fp': 0, 'fn': 4})

    def test_zero_deterministic_failures_is_never_semantic_qualification(self):
        rows = [self.outcome(c['id']) for c in MODULE.load()['cases']]
        report = MODULE.score(MODULE.load(), rows)
        self.assertEqual(report['hard_safety_failures'], 0)
        self.assertIn('not a semantic safety pass', report['hard_safety_check_scope'])
        self.assertIn('NOT_QUALIFIED', report['profile_qualification'])
        self.assertTrue(all('REQUIRED' in r['independent_semantic_review'] for r in report['controls']))

    def test_boolean_line_number_cannot_equal_an_integer_gold_span(self):
        claim = MODULE.load()['cases'][0]['gold']['claims_required'][0] | {'rationale': 'source', 'line_from': True}
        row = self.outcome('rc01', {'claims': [claim], 'retirements': []})
        result = MODULE.score(MODULE.load(), [row])['controls'][0]
        self.assertIn('malformed_control_output', result['hard_safety_failures'])
        self.assertEqual(result['exact_required_claims_matched'], 0)

    def test_duplicate_json_fields_cannot_hide_an_unsafe_output(self):
        with self.assertRaises(ValueError):
            json.loads('{"claims":[1],"claims":[]}', object_pairs_hook=MODULE.unique_object)
