#!/usr/bin/env python3
"""Frozen offline utility experiment; never changes production RRF or calls a model.

Inputs are content-free recorded candidate/feedback bundles. Their eligibility
flags are assertions by the exporter: this script does not prove live canonical
qualification. Native command/privacy proof is a separate required receipt.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path
import time

RECIPE = 'explicit-task-utility-cap-0.05-rrf60-v1'
CAP = .05


def identity(row):
    return (row['brain_id'], json.dumps(row['scope'], sort_keys=True, separators=(',', ':')),
            row['input_kind'], row['revision_id'])


def valid_vote(row):
    return (row.get('vote') in ('helpful', 'unhelpful', 'reused') and row.get('explicit') is True
            and row.get('current') is True and row.get('partition') == 'training'
            and row.get('eligible') is True and row.get('actor_id') and row.get('task_id')
            and row.get('query_context_id') and type(row.get('sequence')) is int)


def rewards(signals, evaluation_tasks):
    if len(signals) > 100_000:
        raise ValueError('Oversized feedback envelope')
    latest = {}
    rejected = Counter()
    for row in signals:
        if not valid_vote(row):
            rejected['not_explicit_current_eligible_training'] += 1
            continue
        if row['task_id'] in evaluation_tasks:
            rejected['evaluation_task'] += 1
            continue
        # Stronger than per-query dedup: repeated deliveries in one task cannot
        # multiply that actor's reward for the same exact input.
        key = (row['actor_id'], row['task_id'], identity(row))
        old = latest.get(key)
        if old is None or old['sequence'] < row['sequence']:
            latest[key] = row
        elif old['sequence'] == row['sequence'] and old['vote'] != row['vote']:
            raise ValueError('Conflicting feedback changes have the same sequence')
    values = defaultdict(list)
    for row in latest.values():
        values[identity(row)].append(-1. if row['vote'] == 'unhelpful' else 1.)
    return {key: sum(v) / len(v) for key, v in values.items()}, rejected, len(latest)


def relevance(order, gold, k):
    dcg = sum((2 ** gold.get(key, 0) - 1) / math.log2(rank + 2) for rank, key in enumerate(order[:k]))
    ideal = sum((2 ** value - 1) / math.log2(rank + 2)
                for rank, value in enumerate(sorted(gold.values(), reverse=True)[:k]))
    return {'ndcg': dcg / ideal if ideal else 0.,
            'mrr': next((1. / (rank + 1) for rank, key in enumerate(order[:k]) if gold.get(key, 0) > 0), 0.),
            'recall': sum(gold.get(key, 0) > 0 for key in order[:k]) / max(1, sum(v > 0 for v in gold.values()))}


def compare(bundle):
    if bundle['recipe'] != RECIPE or bundle['production_rrf_k'] != 60:
        raise ValueError('The frozen utility recipe/RRF baseline must match')
    queries = bundle['queries']
    if len(queries) > 10_000 or len({q['id'] for q in queries}) != len(queries):
        raise ValueError('Duplicate or oversized query envelope')
    utility, rejected, reward_count = rewards(bundle['feedback'], {q['task_id'] for q in queries})
    rows = []
    for query in queries:
        started = time.perf_counter()
        if (query['partition'] not in ('development', 'held_out')
                or type(query['cutoff']) is not int or not 1 <= query['cutoff'] <= 100
                or any(type(v) is not int or not 0 <= v <= 3 for v in query['relevance'].values())):
            raise ValueError('Invalid frozen partition/cutoff/relevance')
        candidates = query['candidates']
        if len(candidates) > 100 or len({identity(c) for c in candidates}) != len(candidates):
            raise ValueError('Duplicate or oversized candidate envelope')
        scores = []
        withheld = 0
        for index, candidate in enumerate(candidates):
            base = candidate['baseline_score']
            if type(base) not in (int, float) or not math.isfinite(base) or base <= 0:
                raise ValueError('Baseline score must be finite and positive')
            if (candidate.get('eligible') is not True or candidate['brain_id'] != query['brain_id']
                    or candidate['scope'] != query['scope']):
                withheld += 1
                continue
            feature = utility.get(identity(candidate), 0.)
            score = base * (1 + CAP * max(-1., min(1., feature)))
            assert abs(score - base) <= base * CAP + 1e-15
            scores.append((candidate['revision_id'], base, score, index))
        baseline = [r[0] for r in sorted(scores, key=lambda r: (-r[1], r[3]))]
        experimental = [r[0] for r in sorted(scores, key=lambda r: (-r[2], r[3]))]
        if query['outcome'] != 'completed':
            baseline, experimental = [], []
        gold = query['relevance']
        base_metrics, experimental_metrics = (relevance(order, gold, query['cutoff']) for order in (baseline, experimental))
        cutoff = query['cutoff']
        bad = sum(gold.get(key, 0) == 0 and key not in baseline[:cutoff] for key in experimental[:cutoff])
        rows.append({'id': query['id'], 'partition': query['partition'], 'outcome': query['outcome'],
                     'baseline_order': baseline, 'experimental_order': experimental,
                     'baseline': base_metrics, 'experimental': experimental_metrics,
                     'bad_promotions': bad, 'withheld_candidates': withheld,
                     'elapsed_ms': (time.perf_counter() - started) * 1000})
    groups = {}
    for partition in ('development', 'held_out', 'all'):
        selected = [row for row in rows if partition == 'all' or row['partition'] == partition]
        groups[partition] = {'queries': len(selected), 'outcomes': dict(Counter(row['outcome'] for row in selected)),
                             'bad_promotions': sum(row['bad_promotions'] for row in selected)}
        for name in ('baseline', 'experimental'):
            groups[partition][name] = {metric: sum(row[name][metric] for row in selected) / max(1, len(selected))
                                       for metric in ('ndcg', 'mrr', 'recall')}
    times = sorted(row['elapsed_ms'] for row in rows)
    return {'recipe': RECIPE, 'partitions': groups, 'independent_reward_votes': reward_count,
            'rejected_feedback': dict(rejected), 'queries': rows,
            'p95_scorer_ms': times[math.ceil(len(times) * .95) - 1] if times else None,
            'production_rrf_k': 60, 'maximum_relative_utility_feature': CAP,
            'production_ranking_changed': False, 'model_calls': 0,
            'native_qualification_proven_by_this_script': False,
            'limits': 'Offline recorded eligibility and utility experiment; no answer, citation, live privacy or general relevance quality claim.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--expected-sha256', required=True)
    parser.add_argument('--recipe-sha256', required=True,
                        help='Freeze the scorer source before held-out comparison')
    args = parser.parse_args()
    raw = args.fixture.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != args.expected_sha256:
        parser.error('Fixture changed after its freeze')
    scorer_digest = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    if scorer_digest != args.recipe_sha256:
        parser.error('Scorer changed after its recipe freeze')
    result = compare(json.loads(raw))
    result['fixture_sha256'] = digest
    result['scorer_sha256'] = scorer_digest
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in ('partitions', 'independent_reward_votes', 'rejected_feedback', 'production_ranking_changed', 'model_calls')}))


if __name__ == '__main__':
    main()
