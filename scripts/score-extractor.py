#!/usr/bin/env python3
"""Score frozen production-extractor outcomes locally, without a model judge.

The original40 has entity-only gold. New20 endpoint coverage is distinct from
canonical identity, alias disposition and topic acceptance, which this runner
does not exercise. Missing/failed/unattempted documents stay in denominators.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORPORA = {
    "original40": ("docs/research/automatic-knowledge-mapping-fixtures-2026-10-09.json", "80e2040b63efa5738f888f99cae3234adfae6b0ad484ccd256da189d6bdd0dbc"),
    "adversarial20": ("docs/research/graph-organization-adversarial-fixtures-2026-10-18.json", "a0ecfe7dd692c2517f00035c0be7205841e27e7b17551f7a21580edc6d1c916c"),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


FRESH = ("docs/research/graph-organization-fresh-heldout-v1-2026-10-18.json", "a01e61e95454d02aec7d4446530a13d16900dbe50550495c7be052f89bea3d67")


def load_corpora(include_fresh=False):
    result = {}
    entries = dict(CORPORA)
    if include_fresh:
        entries["fresh12"] = FRESH
    for name, (path, expected) in entries.items():
        raw = (ROOT / path).read_bytes()
        if sha(raw) != expected:
            raise ValueError(f"Frozen {name} corpus changed")
        result[name] = json.loads(raw)
    return result


def gold_mentions(name, doc):
    text = doc["text"]
    raw = text.encode()
    result = {}
    for index, mention in enumerate(doc["mentions"]):
        if name == "original40":
            a, b = mention["start"], mention["end"]
            if text[a:b] != mention["text"]:
                raise ValueError("Invalid frozen codepoint span")
            key, kind = str(index), mention["type"]
            start, end, quote = len(text[:a].encode()), len(text[:b].encode()), mention["text"]
        else:
            key, kind, start, end, quote = mention
        if raw[start:end].decode() != quote:
            raise ValueError("Invalid frozen UTF-8 span")
        result[key] = (kind, start, end, quote)
    return result


def predicted_mentions(doc, outcome):
    """Validate adapted output again before scoring a locally edited receipt."""
    if outcome.get("state") != "validated":
        return {}, [], []
    value = outcome.get("candidates")
    if not isinstance(value, dict):
        raise ValueError("Validated outcome lacks candidates")
    raw = doc["text"].encode()
    mentions = {}
    for m in value["mentions"]:
        span = m["span"]
        start, end, quote = span["byte_start"], span["byte_end"], span["quote"]
        if m["key"] in mentions or not 0 <= start < end <= len(raw) or raw[start:end].decode() != quote:
            raise ValueError("Invalid or duplicate predicted mention")
        mentions[m["key"]] = (m["kind"], start, end, quote)
    for edge in value.get("aliases", []) + value.get("relations", []):
        a, b = (edge["left"], edge["right"]) if "left" in edge else (edge["from"], edge["to"])
        if a not in mentions or b not in mentions or a == b:
            raise ValueError("Invalid predicted endpoints")
        span = edge["span"]
        if raw[span["byte_start"]:span["byte_end"]].decode() != span["quote"]:
            raise ValueError("Invalid predicted endpoint evidence")
    return mentions, value.get("aliases", []), value.get("relations", [])


def counts(expected, predicted):
    # Multisets ensure duplicate predictions cannot inflate recall or vanish.
    expected, predicted = Counter(expected), Counter(predicted)
    tp = sum((expected & predicted).values())
    return {"tp": tp, "fp": sum(predicted.values()) - tp, "fn": sum(expected.values()) - tp}


def metrics(rows):
    total = {k: sum(r[k] for r in rows) for k in ("tp", "fp", "fn")}
    tp, fp, fn = (total[k] for k in ("tp", "fp", "fn"))
    # No predicted positives is perfect precision only if gold is also empty.
    precision = tp / (tp + fp) if tp + fp else (1.0 if not fn else 0.0)
    recall = tp / (tp + fn) if tp + fn else 1.0
    per_doc = []
    for row in rows:
        a, b, c = (row[k] for k in ("tp", "fp", "fn"))
        per_doc.append((a / (a + b) if a + b else (1.0 if not c else 0.0), a / (a + c) if a + c else 1.0))
    return {**total, "precision": precision, "recall": recall, "documents": len(rows),
            "macro_precision": sum(r[0] for r in per_doc) / len(rows) if rows else None,
            "macro_recall": sum(r[1] for r in per_doc) / len(rows) if rows else None}


def endpoint(mentions, a, b, span, relation=None, alias=False):
    pair = (mentions[a], mentions[b])
    if alias:
        pair = tuple(sorted(pair))
    return pair + (relation, span[0], span[1], span[2])


def score(corpora, outcomes):
    expected_ids = {(name, d["id"]) for name, corpus in corpora.items() for d in corpus["documents"]}
    by_id = {}
    for row in outcomes:
        key = row.get("corpus"), row.get("id")
        if key not in expected_ids or key in by_id:
            raise ValueError("Foreign or duplicate outcome; no attempt selection or best-of scoring")
        by_id[key] = row
    rows = []
    for name, corpus in corpora.items():
        for doc in corpus["documents"]:
            outcome = by_id.get((name, doc["id"]), {"state": "not_attempted"})
            if outcome.get("text_sha256", sha(doc["text"].encode())) != sha(doc["text"].encode()):
                raise ValueError("Outcome is bound to different source text")
            if outcome.get("partition", doc["partition"]) != doc["partition"]:
                raise ValueError("Outcome changed the frozen partition")
            gold = gold_mentions(name, doc)
            pred, aliases, relations = predicted_mentions(doc, outcome)
            row = {"corpus": name, "id": doc["id"], "partition": doc["partition"], "language": doc["language"],
                   "state": outcome["state"], "mentions": counts(gold.values(), pred.values()),
                   "expected_mentions": len(gold), "predicted_mentions": len(pred),
                   "adapter_error": outcome.get("adapter_error"), "gateway_error": outcome.get("error_code"),
                   "returned_mapping_language": (outcome.get("raw_output") or {}).get("mapping", {}).get("language") if isinstance((outcome.get("raw_output") or {}).get("mapping"), dict) else None}
            if name == "adversarial20":
                for kind, offered in (("aliases", aliases), ("relations", relations)):
                    alias = kind == "aliases"
                    expected = []
                    statuses = Counter()
                    gold_positive = []
                    negative_keys = {}
                    for e in doc[kind]:
                        a, b = e[:2]
                        relation, status, span = (None, e[2], e[3:]) if alias else (e[2], e[3], e[4:])
                        key = endpoint(gold, a, b, span, relation, alias)
                        expected.append(key); statuses[status] += 1
                        if status in ("supported_equivalence", "supported_navigation"):
                            gold_positive.append(key)
                        else:
                            negative_keys[key] = status
                    predicted = [endpoint(pred, e["left"] if alias else e["from"], e["right"] if alias else e["to"],
                                          [e["span"]["byte_start"], e["span"]["byte_end"], e["span"]["quote"]],
                                          None if alias else e["relation"], alias) for e in offered]
                    row[kind] = {"endpoint_coverage_all_gold_dispositions": counts(expected, predicted),
                                 "positive_only_candidate_screen": counts(gold_positive, predicted),
                                 "gold_dispositions": dict(statuses),
                                 "predicted_on_nonpositive_gold": dict(Counter(negative_keys[p] for p in predicted if p in negative_keys)),
                                 "disposition_accuracy": "NOT_MEASURED: extractor schema has no disposition; candidate presence is not accepted equivalence or truth"}
            if doc["language"] != "en":
                row["unsupported_span_overlap_diagnostic"] = row["mentions"]
                # V1 does not qualify unsupported-language predictions, even if
                # they happen to overlap manual spans. They remain FN/FP in the
                # all-corpus quality view rather than creating a perfect skip.
                row["mentions"] = {"tp": 0, "fp": len(pred), "fn": len(gold)}
                row["unsupported_control"] = {"expected": "unsupported_language or mapping=null; no fabricated English completion",
                    "actual_state": outcome["state"], "actual_mapping_null": "mapping" in (outcome.get("raw_output") or {}) and outcome["raw_output"]["mapping"] is None,
                    "actual_adapter_error": outcome.get("adapter_error"),
                    "passed": outcome.get("adapter_error") == "UnsupportedLanguage" or (outcome.get("raw_output") or {}).get("mapping", "missing") is None}
                # Report separately; never manufacture a skipped/unsupported outcome.
                row["all_corpus_mentions"] = row["mentions"]
            rows.append(row)
    report = {"kind": "production-extractor-only", "outcome_documents": len(rows), "recorded_documents": len(by_id),
              "states": dict(Counter(r["state"] for r in rows)), "corpora": {}, "documents": rows,
              "not_measured": ["canonical identity/realm/false-merge behavior", "alias acceptance dispositions", "canonical/native topic membership and names"],
              "profile_qualification": "NOT_QUALIFIED: extractor report alone does not satisfy canonical identity/topic/disposition gates"}
    for name in corpora:
        report["corpora"][name] = {}
        for partition in ("development", "held_out", "all"):
            group = [r for r in rows if r["corpus"] == name and (partition == "all" or r["partition"] == partition)]
            english = [r for r in group if r["language"] == "en"]
            cell = {"outcome_documents": len(group), "english_documents": len(english),
                    "states": dict(Counter(r["state"] for r in group)),
                    "english_exact_span_type": metrics([r["mentions"] for r in english]),
                    "all_corpus_exact_span_type": metrics([r["mentions"] for r in group]),
                    "unsupported_controls": [r["unsupported_control"] for r in group if "unsupported_control" in r]}
            entity = cell["english_exact_span_type"]
            cell["entity_micro_target_passed"] = bool(english) and entity["precision"] >= .95 and entity["recall"] >= .80
            if name != "adversarial20":
                cell["endpoints"] = "NOT_ANNOTATED: never interpreted as empty gold"
            else:
                for kind in ("aliases", "relations"):
                    cell[kind] = {"endpoint_coverage_all_gold_dispositions": metrics([r[kind]["endpoint_coverage_all_gold_dispositions"] for r in english]),
                                  "positive_only_candidate_screen": metrics([r[kind]["positive_only_candidate_screen"] for r in english]),
                                  "disposition_accuracy": "NOT_MEASURED"}
            report["corpora"][name][partition] = cell
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("campaign", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--include-fresh-heldout", action="store_true", help="Reconciliation arm only: include separate12entity-only held-out annotations; endpoints remain unannotated")
    args = parser.parse_args()
    path = args.campaign / "outcomes.jsonl"
    outcomes = [json.loads(line) for line in path.read_text().splitlines() if line.strip()] if path.exists() else []
    report = score(load_corpora(args.include_fresh_heldout), outcomes)
    report["campaign"] = str(args.campaign)
    report["outcomes_sha256"] = sha(path.read_bytes()) if path.exists() else None
    output = args.output or args.campaign / "score.json"
    if output.exists():
        raise ValueError("Never overwrite an existing score; use a new output path")
    with output.open("x") as file:
        json.dump(report, file, indent=2, ensure_ascii=False); file.write("\n")
    print(json.dumps({"output": str(output), "outcome_documents": report["outcome_documents"],
                      "recorded_documents": report["recorded_documents"], "states": report["states"],
                      "profile_qualification": report["profile_qualification"]}))


if __name__ == "__main__":
    main()
