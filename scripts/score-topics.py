#!/usr/bin/env python3
"""Frozen idealized-input topic comparison; never qualifies a production recipe.

This scorer consumes recorded memberships. Native canonical eligibility and GDS
execution require their independent receipts. No provider or vector is invoked.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
from itertools import combinations
import json
from pathlib import Path

BASELINES = {'native_weighted', 'metadata', 'entity_overlap', 'eligible_vector'}
OUTCOMES = {'assigned', 'unassigned', 'unsupported', 'failed', 'ineligible'}


def object_once(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise ValueError('Duplicate JSON key')
        out[key] = value
    return out


def read(path):
    return json.loads(path.read_bytes(), object_pairs_hook=object_once)


def ratio(n, d):
    return n / d if d else None


def pair_truth(a, b, annotations):
    if a['scope'].split('/')[0] != b['scope'].split('/')[0]:
        return None, 'brain_boundary'
    explicit = annotations.get(tuple(sorted((a['id'], b['id']))))
    if explicit is not None:
        return explicit, 'explicit_annotation'
    left, right = set(a['topics']), set(b['topics'])
    if left and right:
        return bool(left & right), 'topic_categories'
    return None, 'unknown'


def pair_metrics(rows):
    measured = [r for r in rows if r['expected_related'] is not None]
    tp = sum(r['predicted_related'] and r['expected_related'] for r in measured)
    fp = sum(r['predicted_related'] and not r['expected_related'] for r in measured)
    fn = sum(not r['predicted_related'] and r['expected_related'] for r in measured)
    return {'pairs': len(rows), 'scored_pairs': len(measured),
            'truth_sources': dict(Counter(r['truth_source'] for r in rows)),
            'tp': tp, 'fp': fp, 'fn': fn, 'precision': ratio(tp, tp + fp),
            'recall': ratio(tp, tp + fn), 'f1': ratio(2 * tp, 2 * tp + fp + fn),
            'cross_brain_shared_groups': sum(r['boundary_violation'] for r in rows),
            'unscored_shared_pairs': sum(r['predicted_related'] and r['truth_source'] == 'unknown' for r in rows)}


def labels_for(documents, members):
    groups, repeated_errors = {}, 0
    for row in members.values():
        for group in row['groups']:
            old = groups.get(group['id'])
            if old is not None and old != group:
                repeated_errors += 1
            groups[group['id']] = group
    membership = defaultdict(set)
    for key, row in members.items():
        for group in row['groups']:
            membership[group['id']].add(key)
    checks = []
    for key, group in sorted(groups.items()):
        kind = group.get('provenance_kind')
        supported = False
        reason = 'missing_or_invalid_span_provenance'
        if kind == 'canonical_scope':
            # Metadata labels have declared field origins, never fake quotes.
            supported = None if group.get('label_evidence') is None else False
            reason = 'canonical_metadata_requires_native_field_receipt'
        elif kind == 'exact_entity_mention':
            span = group.get('label_evidence') or {}
            doc = documents.get(span.get('document_id'))
            start, end = span.get('byte_start'), span.get('byte_end')
            if (doc is not None and doc['id'] in membership[key]
                    and type(start) is int and type(end) is int
                    and 0 <= start < end <= len(doc['text'].encode())):
                try:
                    exact = doc['text'].encode()[start:end].decode()
                    supported = (exact == span.get('quote') == group.get('label')
                                 and any(m[1] in ('technology', 'concept')
                                         and m[2:5] == [start, end, exact] for m in doc['mentions']))
                except UnicodeDecodeError:
                    supported = False
            if supported:
                reason = 'exact_eligible_member_gold_span'
        checks.append({'id': key, 'label': group['label'], 'provenance_kind': kind,
                       'supported': supported, 'reason': reason})
    return {'groups': len(checks), 'supported': sum(r['supported'] is True for r in checks),
            'unverified_canonical_metadata_labels': sum(r['supported'] is None for r in checks),
            'inconsistent_group_records': repeated_errors, 'checks': checks,
            'semantic_usefulness': 'manual_review_required',
            'limits': 'Exact source provenance and declared canonical field origin are separate from semantic usefulness and native authorization.'}


def compare(corpus, receipt):
    docs = {d['id']: d for d in corpus['documents']}
    if len(docs) != len(corpus['documents']) or len(docs) > 100:
        raise ValueError('Duplicate or oversized corpus')
    if receipt.get('input_mode') != 'gold_upper_bound' or receipt.get('model_calls') != 0:
        raise ValueError('This experiment requires explicit gold upper-bound inputs and zero model calls')
    baselines = receipt['baselines']
    if {b['name'] for b in baselines} != BASELINES or len(baselines) != len(BASELINES):
        raise ValueError('Every baseline must have exactly one outcome')
    annotations = {}
    for left, right, value in corpus['topic_pairs']:
        key = tuple(sorted((left, right)))
        if left not in docs or right not in docs or left == right or key in annotations:
            raise ValueError('Invalid or duplicate explicit pair')
        if value in ('unrelated', 'unrelated_empty', 'cross_Brain_no_membership_or_count'):
            annotations[key] = False
        elif value.startswith('related_'):
            annotations[key] = True
        else:
            raise ValueError('Unknown explicit pair interpretation')
    output = []
    for baseline in baselines:
        state = baseline['state']
        if state not in ('completed', 'failed', 'not_measured'):
            raise ValueError('Invalid baseline state')
        if state == 'not_measured':
            if not baseline.get('reason'):
                raise ValueError('An unmeasured baseline needs its reason')
            output.append({'name': baseline['name'], 'state': state, 'reason': baseline['reason'], 'quality_qualified': False})
            continue
        members = {}
        for row in baseline.get('documents', []):
            key = row['id']
            if key not in docs or key in members or row['outcome'] not in OUTCOMES:
                raise ValueError('Foreign/duplicate document or invalid outcome')
            groups = row['groups']
            if len(groups) > 128 or len({g['id'] for g in groups}) != len(groups):
                raise ValueError('Duplicate or oversized group memberships')
            if any(not isinstance(g['id'], str) or not g['id'] or not isinstance(g['label'], str)
                   or not g['label'] or len(g['label'].encode()) > 1024 for g in groups):
                raise ValueError('Invalid group identity or label')
            if bool(groups) != (row['outcome'] == 'assigned'):
                raise ValueError('Failed/unassigned outcomes cannot hide published groups')
            members[key] = row
        missing = sorted(set(docs) - set(members))
        for key in missing:
            members[key] = {'id': key, 'outcome': 'failed', 'groups': []}
        rows = []
        for left, right in combinations(sorted(docs), 2):
            a, b = docs[left], docs[right]
            expected, source = pair_truth(a, b, annotations)
            shared = bool({g['id'] for g in members[left]['groups']} & {g['id'] for g in members[right]['groups']})
            partition = a['partition'] if a['partition'] == b['partition'] else 'cross_partition'
            rows.append({'left': left, 'right': right, 'partition': partition,
                         'expected_related': expected, 'truth_source': source,
                         'predicted_related': shared, 'boundary_violation': shared and source == 'brain_boundary'})
        assignment = {}
        for partition in ('development', 'held_out', 'all'):
            selected = [d for d in docs.values() if partition == 'all' or d['partition'] == partition]
            annotated = [d for d in selected if d['topics']]
            english = [d for d in annotated if d['language'] == 'en']
            assigned = lambda ds: sum(members[d['id']]['outcome'] == 'assigned' for d in ds)
            assignment[partition] = {'documents': len(selected), 'annotated_documents': len(annotated),
                                     'assigned_annotated': assigned(annotated), 'coverage': ratio(assigned(annotated), len(annotated)),
                                     'english_annotated_coverage': ratio(assigned(english), len(english)),
                                     'unannotated_documents': len(selected) - len(annotated),
                                     'assigned_unannotated': assigned([d for d in selected if not d['topics']]),
                                     'outcomes': dict(Counter(members[d['id']]['outcome'] for d in selected))}
        output.append({'name': baseline['name'], 'state': state, 'missing_document_outcomes': missing,
                       'assignment': assignment, 'pairs': rows,
                       'pair_metrics': {part: pair_metrics([r for r in rows if part == 'all' or r['partition'] == part])
                                        for part in ('development', 'held_out', 'cross_partition', 'all')},
                       'unsupported_language_assignments': [d['id'] for d in docs.values()
                                                            if d['language'] != 'en' and members[d['id']]['outcome'] == 'assigned'],
                       'label_rubric': labels_for(docs, members), 'quality_qualified': False})
    return {'input_mode': 'gold_upper_bound', 'documents': len(docs), 'total_pairs': len(docs) * (len(docs) - 1) // 2,
            'baselines': output, 'model_calls': 0, 'production_recipe_qualified': False,
            'native_execution_proven_by_this_script': False,
            'limits': 'Idealized extractor inputs; no extraction, vector, general usefulness, live authorization or production quality claim.'}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--corpus', type=Path, required=True)
    p.add_argument('--receipt', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--expected-corpus-sha256', required=True)
    p.add_argument('--expected-recipe-sha256', required=True)
    p.add_argument('--scorer-sha256', required=True)
    p.add_argument('--compatibility-note', help='Required when correcting frozen corpus schema compatibility after native execution; never permits recipe/threshold tuning')
    args = p.parse_args()
    corpus_hash = hashlib.sha256(args.corpus.read_bytes()).hexdigest()
    scorer_hash = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    if corpus_hash != args.expected_corpus_sha256 or scorer_hash != args.scorer_sha256:
        p.error('Corpus or scorer changed after freeze')
    receipt = read(args.receipt)
    if receipt.get('corpus_sha256') != corpus_hash or receipt.get('recipe_sha256') != args.expected_recipe_sha256:
        p.error('Native receipt does not match frozen corpus/recipe')
    original_scorer = receipt.get('scorer_sha256')
    if original_scorer != scorer_hash and not args.compatibility_note:
        p.error('Native recipe used another frozen scorer; preserve that hash and explicitly record a compatibility correction')
    result = compare(read(args.corpus), receipt)
    result.update(corpus_sha256=corpus_hash, recipe_sha256=args.expected_recipe_sha256, scorer_sha256=scorer_hash,
                  native_recipe_scorer_sha256=original_scorer, compatibility_note=args.compatibility_note,
                  receipt_sha256=hashlib.sha256(args.receipt.read_bytes()).hexdigest())
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'documents': result['documents'], 'total_pairs': result['total_pairs'],
                      'production_recipe_qualified': False,
                      'baselines': [{'name': b['name'], 'state': b['state'], 'pair_metrics': b.get('pair_metrics'),
                                     'assignment': b.get('assignment')} for b in result['baselines']]}))


if __name__ == '__main__':
    main()
