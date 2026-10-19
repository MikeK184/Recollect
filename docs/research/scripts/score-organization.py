#!/usr/bin/env python3
"""Score recorded extractor output; never call a provider or fabricate predictions.

Input: {metadata: {...}, documents: [{id, outcome, mentions: [SourceCandidate
mention objects]}]}. A completed record contains the actual adapted candidates.
Missing/failed/unsupported documents stay in the outcome and recall denominator.
This scoring receipt alone never qualifies a production producer profile.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
DEFAULT_GOLD = [
    ROOT / 'docs/research/automatic-knowledge-mapping-fixtures-2026-10-09.json',
    ROOT / 'docs/research/graph-organization-adversarial-fixtures-2026-10-18.json',
]


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'Duplicate JSON field: {key}')
        result[key] = value
    return result


def load(path):
    return json.loads(path.read_text(), object_pairs_hook=unique_object)


def gold_mentions(document):
    text = document['text']
    result = []
    for mention in document['mentions']:
        if isinstance(mention, list):
            _, kind, start, end, quote = mention
        else:
            kind, quote = mention['type'], mention['text']
            # The October 9 GLiNER corpus uses Python character coordinates;
            # convert its declared character spans, never search for a quote.
            char_start, char_end = mention['start'], mention['end']
            if text[char_start:char_end] != quote:
                raise ValueError(f'Invalid original gold span: {document["id"]}')
            start = len(text[:char_start].encode())
            end = len(text[:char_end].encode())
        if text.encode()[start:end].decode() != quote:
            raise ValueError(f'Invalid UTF-8 gold span: {document["id"]}')
        result.append((kind, start, end, quote))
    return Counter(result)


def prediction_mentions(document, prediction):
    result = []
    raw = document['text'].encode()
    for index, mention in enumerate(prediction.get('mentions', [])):
        try:
            span = mention['span']
            kind, start, end, quote = mention['kind'], span['byte_start'], span['byte_end'], span['quote']
            if type(start) is not int or type(end) is not int or not 0 <= start < end <= len(raw):
                raise ValueError('Invalid bounds')
            if raw[start:end].decode() != quote:
                raise ValueError('Invalid quote')
            result.append((kind, start, end, quote))
        except (KeyError, TypeError, ValueError, UnicodeError):
            # Invalid returned coordinates count as false positives as well as
            # failing to recover their expected gold mention.
            result.append(('invalid_prediction', index, index, ''))
    return Counter(result)


def counts(expected, actual):
    tp = sum((expected & actual).values())
    return tp, sum((actual - expected).values()), sum((expected - actual).values())


def metrics(rows):
    tp, fp, fn = (sum(row[k] for row in rows) for k in ('tp', 'fp', 'fn'))
    return {
        'documents': len(rows), 'tp': tp, 'fp': fp, 'fn': fn,
        'precision': tp / (tp + fp) if tp + fp else 0.0,
        'recall': tp / (tp + fn) if tp + fn else 0.0,
        'macro_precision': sum(r['precision'] for r in rows) / len(rows) if rows else 0.0,
        'macro_recall': sum(r['recall'] for r in rows) / len(rows) if rows else 0.0,
    }


def score(documents, predictions, partition):
    by_id = defaultdict(list)
    for prediction in predictions:
        by_id[prediction['id']].append(prediction)
    results = []
    known = {d['id'] for d in documents}
    for document in documents:
        if partition != 'all' and document['partition'] != partition:
            continue
        expected = gold_mentions(document)
        offered = by_id.get(document['id'], [])
        prediction = offered[0] if len(offered) == 1 else {}
        outcome = prediction.get('outcome', 'missing' if not offered else 'duplicate_document')
        actual = prediction_mentions(document, prediction) if outcome == 'completed' else Counter()
        tp, fp, fn = counts(expected, actual)
        results.append({
            'id': document['id'], 'partition': document['partition'], 'language': document['language'],
            'outcome': outcome, 'tp': tp, 'fp': fp, 'fn': fn,
            'precision': tp / (tp + fp) if tp + fp else 0.0,
            'recall': tp / (tp + fn) if tp + fn else 0.0,
        })
    groups = {}
    for name in ('development', 'held_out'):
        rows = [r for r in results if r['partition'] == name]
        english = [r for r in rows if r['language'] == 'en']
        groups[name] = {'all_outcomes': metrics(rows), 'english': metrics(english)}
    return {
        'partitions': groups, 'all_outcomes': metrics(results),
        'outcomes': dict(sorted(Counter(r['outcome'] for r in results).items())),
        'unknown_prediction_ids': sorted(set(by_id) - known), 'documents': results,
        'production_profile_qualified': False,
        'unverified': ['provider provenance', 'alias and relation dispositions', 'identity merges',
                       'native topic pairs and labels', 'eligible-vector comparison', 'normal runtime'],
    }


def self_test():
    documents = [dict(id='a', partition='development', language='en', text='🔐 Vault',
                      mentions=[['v', 'technology', 5, 10, 'Vault']]),
                 dict(id='b', partition='development', language='en', text='Vault',
                      mentions=[['v', 'technology', 0, 5, 'Vault']])]
    mention = dict(kind='technology', span=dict(byte_start=5, byte_end=10, quote='Vault'))
    report = score(documents, [dict(id='a', outcome='completed', mentions=[mention, mention])], 'all')
    assert (report['all_outcomes']['tp'], report['all_outcomes']['fp'], report['all_outcomes']['fn']) == (1, 1, 1)
    assert report['outcomes'] == {'completed': 1, 'missing': 1}
    wrong = dict(kind='technology', span=dict(byte_start=2, byte_end=7, quote='Vault'))
    report = score(documents, [dict(id='a', outcome='completed', mentions=[wrong])], 'all')
    assert (report['all_outcomes']['tp'], report['all_outcomes']['fp'], report['all_outcomes']['fn']) == (0, 1, 2)
    print('PASS: missing cases, duplicate mentions, and invalid UTF-8 coordinates retain errors')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--predictions', type=Path)
    parser.add_argument('--gold', type=Path, action='append')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--partition', choices=['development', 'held_out', 'all'], default='development')
    parser.add_argument('--recipe-sha256')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not args.predictions or not args.output:
        parser.error('--predictions and --output are required')
    if args.partition in ('held_out', 'all') and not re.fullmatch('[0-9a-f]{64}', args.recipe_sha256 or ''):
        parser.error('Freeze the recipe and supply its --recipe-sha256 before held-out scoring')
    files = args.gold or DEFAULT_GOLD
    documents = [d for path in files for d in load(path)['documents']]
    if len({d['id'] for d in documents}) != len(documents):
        raise ValueError('Duplicate gold document ID')
    recorded = load(args.predictions)
    result = score(documents, recorded['documents'], args.partition)
    result.update({
        'recipe_sha256': args.recipe_sha256,
        'gold_sha256': {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in files},
        'predictions_sha256': hashlib.sha256(args.predictions.read_bytes()).hexdigest(),
        'recorded_metadata': recorded.get('metadata', {}),
        'provider_calls_by_scorer': 0,
    })
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps({k: result[k] for k in ('partitions', 'outcomes', 'unknown_prediction_ids', 'production_profile_qualified')}))


if __name__ == '__main__':
    main()
