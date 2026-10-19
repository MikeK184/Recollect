#!/usr/bin/env python3
"""Offline six-control audit. No model judge; unknown paraphrases need human review.

Exact span/type and required candidate actions are reported separately from
canonical mutation/support quality, which extraction-only calls do not measure.
Hard-safety checks below cover deterministic shape, target identity and spans;
zero such failures never proves negation, historical meaning or the complete
frozen semantic rubric. Independent semantic review remains required.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import uuid

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / 'docs/research/automatic-reconciliation-native-controls-v3-2026-10-18.json'
HASH = 'd15e0171a691e1656dcd4e3955eca39859c6b73a19b8257402c692e69101cf81'


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate JSON field; no hidden favorable value')
        result[key] = value
    return result


def load():
    raw = CORPUS.read_bytes()
    if hashlib.sha256(raw).hexdigest() != HASH:
        raise ValueError('Independent native controls changed after review')
    return json.loads(raw, object_pairs_hook=unique_object)


def runtime_binding(case, row):
    """Validate a reported symbol map; this cannot replace native runtime proof."""
    memory = case['canonical_memories'][0]
    mapping = row.get('canonical_id_map')
    prior = mapping.get('prior_revision') if isinstance(mapping, dict) else None
    offered = row.get('actual_offered_revision_ids')
    if not isinstance(prior, dict) or not isinstance(offered, list):
        return None, [], False
    actual = prior.get('actual')
    try:
        uuid.UUID(actual)
        for target in offered:
            uuid.UUID(target)
    except (ValueError, TypeError, AttributeError):
        return None, [], False
    expected = [actual] if case['offered_memory_symbols'] else []
    valid = (prior.get('frozen') == memory['revision_id']
             and prior.get('symbol') == memory['symbol'] and offered == expected)
    return actual, offered, valid


def score(corpus, outcomes):
    cases = {c['id']: c for c in corpus['cases']}
    indexed = {}
    for outcome in outcomes:
        if not isinstance(outcome, dict):
            raise ValueError('Control outcome must be an object')
        ident = outcome.get('id')
        if ident not in cases or ident in indexed:
            raise ValueError('Foreign or duplicate control; no best-of selection')
        indexed[ident] = outcome
    rows = []
    for ident, case in cases.items():
        row = indexed.get(ident, {'state': 'not_attempted'})
        state = row.get('state', 'missing_state')
        if row.get('partition', case['partition']) != case['partition']:
            raise ValueError('Frozen control partition changed')
        value = row.get('raw_output')
        malformed = not isinstance(value, dict)
        value = value if isinstance(value, dict) else {}
        claims, retirements = value.get('claims'), value.get('retirements')
        malformed |= not isinstance(claims, list) or not isinstance(retirements, list)
        malformed |= (isinstance(claims, list) and any(not isinstance(c, dict) for c in claims))
        malformed |= (isinstance(retirements, list) and any(not isinstance(r, dict) for r in retirements))
        claims = claims if isinstance(claims, list) and all(isinstance(c, dict) for c in claims) else []
        retirements = retirements if isinstance(retirements, list) and all(isinstance(r, dict) for r in retirements) else []
        claim_fields = {'subject', 'predicate', 'value', 'rationale', 'context_role',
                        'line_from', 'line_to', 'kind', 'procedure', 'replaces_revision'}
        malformed |= any(not claim_fields.issubset(c)
                         or any(not isinstance(c.get(k), str) for k in ('subject', 'predicate', 'value', 'rationale', 'kind'))
                         or any(type(c.get(k)) is not int or c[k] < 1 for k in ('line_from', 'line_to'))
                         for c in claims)
        malformed |= any(not {'revision_id', 'reason', 'line_from', 'line_to'}.issubset(r)
                         or not isinstance(r.get('reason'), str)
                         or any(type(r.get(k)) is not int or r[k] < 1 for k in ('line_from', 'line_to'))
                         for r in retirements)
        malformed |= any(c.get('replaces_revision') is not None and not isinstance(c.get('replaces_revision'), str) for c in claims)
        malformed |= any(not isinstance(r.get('revision_id'), str) for r in retirements)
        if malformed:
            claims, retirements = [], []
        actual, actual_offered, binding_valid = runtime_binding(case, row)
        frozen = case['canonical_memories'][0]['revision_id']
        offered = {frozen} if case['offered_memory_symbols'] else set()
        translate = lambda target: frozen if binding_valid and target == actual else target
        replacements = [translate(c.get('replaces_revision')) for c in claims if c.get('replaces_revision')]
        retired = [translate(r.get('revision_id')) for r in retirements]
        hard = ['malformed_control_output'] if malformed else []
        if state not in ('validated', 'mapping_absent', 'adapter_rejected', 'gateway_failed', 'not_attempted'):
            hard.append('invalid_transport_state')
        if state in ('not_attempted', 'gateway_failed'):
            hard.append('no_completed_control_output')
        if not binding_valid:
            hard.append('missing_or_invalid_runtime_binding')
        raw_targets = ([c.get('replaces_revision') for c in claims if c.get('replaces_revision')]
                       + [r.get('revision_id') for r in retirements])
        if any(target not in actual_offered for target in raw_targets):
            hard.append('target_absent_from_actual_offered_enum')
        if any(target not in offered for target in replacements + retired):
            hard.append('unoffered_or_manual_target')
        if set(replacements) & set(retired):
            hard.append('same_target_replaced_and_retired')
        if len(replacements + retired) != len(set(replacements + retired)):
            hard.append('target_used_more_than_once')
        if set(retired) & set(case['gold']['retirements_forbidden']):
            hard.append('forbidden_retirement')
        if ident in ('rc02', 'rc04', 'rc05', 'rc06') and replacements:
            hard.append('protected_or_unchanged_target_replaced')
        matched = 0
        for required in case['gold']['claims_required']:
            matched += any((candidate.get('replaces_revision') is None or candidate.get('replaces_revision') in actual_offered)
                           and all(candidate.get(k) == required.get(k) for k in
                               ('subject', 'predicate', 'value', 'kind', 'procedure', 'context_role', 'line_from', 'line_to'))
                           and translate(candidate.get('replaces_revision')) == required.get('replaces_revision')
                           for candidate in claims)
        needed_retired = {r['revision_id'] for r in case['gold']['retirements_required']}
        valid_retirements = {translate(candidate.get('revision_id')) for candidate in retirements
                             if candidate.get('revision_id') in actual_offered
                             and any(translate(candidate.get('revision_id')) == expected['revision_id']
                                    and candidate.get('line_from') == expected['line_from']
                                    and candidate.get('line_to') == expected['line_to']
                                    and isinstance(candidate.get('reason'), str)
                                    and candidate['reason'].strip()
                                    for expected in case['gold']['retirements_required'])}
        missing_retirements = len(needed_retired - valid_retirements)
        if not binding_valid:
            matched = 0
            missing_retirements = len(needed_retired)
        if ident == 'rc05' and len(claims) > 1:
            hard.append('duplicate_unchanged_candidates')
        gold = Counter((m['kind'], m['span']['byte_start'], m['span']['byte_end'], m['span']['quote'])
                       for m in case['gold']['mapping']['mentions'])
        predicted = Counter()
        candidates = row.get('candidates') or {}
        if not isinstance(candidates, dict):
            candidates = {}
            hard.append('malformed_adapted_mapping')
        raw = case['source']['text'].encode()
        try:
            if row.get('state') == 'validated':
                for field in ('mentions', 'relations', 'aliases'):
                    if not isinstance(candidates.get(field), list):
                        raise ValueError('Adapted mapping list missing')
                for m in candidates['mentions']:
                    if not isinstance(m.get('key'), str) or not isinstance(m.get('kind'), str):
                        raise ValueError('Adapted mention key/kind missing')
                    span = m['span']
                    start, end = span['byte_start'], span['byte_end']
                    if (type(start) is not int or type(end) is not int
                            or not 0 <= start < end <= len(raw)
                            or raw[start:end].decode() != span['quote']):
                        raise ValueError('Adapted mention is not exact UTF8 source0')
                    predicted[(m['kind'], start, end, span['quote'])] += 1
                if len({m['key'] for m in candidates['mentions']}) != len(candidates['mentions']):
                    raise ValueError('Duplicate adapted mention key')
        except (KeyError, TypeError, ValueError, UnicodeError, AttributeError):
            candidates, predicted = {}, Counter()
            hard.append('malformed_adapted_mapping')
        gold_keys = {m['key']: (m['kind'], m['span']['byte_start'], m['span']['byte_end'], m['span']['quote'])
                     for m in case['gold']['mapping']['mentions']}
        pred_keys = {m['key']: (m['kind'], m['span']['byte_start'], m['span']['byte_end'], m['span']['quote'])
                     for m in candidates.get('mentions', [])} if row.get('state') == 'validated' else {}
        def edge_key(edge, keys, alias=False, gold=False):
            a, b = (edge['left'], edge['right']) if alias else (edge['from'], edge['to'])
            if a not in keys or b not in keys:
                raise ValueError('Foreign control relation endpoint')
            if alias and (a == b or keys[a] == keys[b]):
                raise ValueError('Alias endpoints must be distinct')
            ends = (keys[a], keys[b])
            if alias:
                ends = tuple(sorted(ends))
            span = edge['span']
            return ends + (None if alias else edge['semantic_relation' if gold else 'relation'],
                           span['byte_start'], span['byte_end'], span['quote'])
        expected_edges = Counter(edge_key(e, gold_keys, gold=True) for e in case['gold']['mapping']['relations'])
        try:
            actual_edges = Counter(edge_key(e, pred_keys) for e in candidates.get('relations', [])) if row.get('state') == 'validated' else Counter()
            if row.get('state') == 'validated':
                for edge in candidates.get('relations', []) + candidates.get('aliases', []):
                    span = edge['span']
                    start, end = span['byte_start'], span['byte_end']
                    if (type(start) is not int or type(end) is not int
                            or not 0 <= start < end <= len(raw)
                            or raw[start:end].decode() != span['quote']):
                        raise ValueError('Adapted edge is not exact UTF8 source0')
                for alias in candidates.get('aliases', []):
                    edge_key(alias, pred_keys, alias=True)
        except (KeyError, TypeError, ValueError, UnicodeError):
            candidates, predicted, actual_edges = {}, Counter(), Counter()
            hard.append('malformed_adapted_mapping')
        edge_tp = sum((expected_edges & actual_edges).values())
        tp = sum((gold & predicted).values())
        # Paraphrases/historical meaning and supported canonical mutation cannot
        # become an automatic pass from local string matching.
        rows.append({'id': ident, 'partition': case['partition'], 'state': state,
                     'hard_safety_failures': hard, 'required_claims': len(case['gold']['claims_required']),
                     'runtime_binding_checked': binding_valid,
                     'exact_required_claims_matched': matched,
                     'required_retirements': len(needed_retired), 'missing_required_retirements': missing_retirements,
                     'mentions': {'tp': tp, 'fp': sum(predicted.values()) - tp, 'fn': sum(gold.values()) - tp},
                     'typed_relation_endpoints': {'tp': edge_tp, 'fp': sum(actual_edges.values()) - edge_tp, 'fn': sum(expected_edges.values()) - edge_tp},
                     'alias_candidates': {'tp': 0, 'fp': len(candidates.get('aliases', [])) if row.get('state') == 'validated' else 0, 'fn': 0},
                     'raw_claim_candidates': value.get('claims'), 'raw_retirement_candidates': value.get('retirements'),
                     'independent_semantic_review': 'REQUIRED: use frozen rubric, no post-output changes',
                     'canonical_mutation_support_quality': 'NOT_MEASURED: no current learning/support model execution'})
    return {'controls_sha256': HASH, 'control_denominator': 6, 'recorded_controls': len(indexed),
            'partitions': {p: sum(r['partition'] == p for r in rows) for p in ('development', 'held_out')},
            'hard_safety_failures': sum(bool(r['hard_safety_failures']) for r in rows),
            'hard_safety_check_scope': 'Deterministic shape/target/span checks only; zero is not a semantic safety pass. Negation/history and the full frozen rubric require independent review.',
            'controls': rows, 'profile_qualification': 'NOT_QUALIFIED: independent semantic and canonical gates remain separate'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('campaign', type=Path)
    args = parser.parse_args()
    path = args.campaign / 'control-outcomes.jsonl'
    rows = [json.loads(line, object_pairs_hook=unique_object) for line in path.read_text().splitlines() if line.strip()] if path.exists() else []
    report = score(load(), rows)
    out = args.campaign / 'control-score.json'
    with out.open('x') as file:
        json.dump(report, file, indent=2, ensure_ascii=False)
        file.write('\n')
    print(json.dumps({'controls': 6, 'recorded': len(rows), 'output': str(out),
                      'profile_qualification': report['profile_qualification']}))


if __name__ == '__main__':
    main()
