#!/usr/bin/env python3
"""Offline synthetic support corpus checks/scoring. Never calls a provider.

Scores declared observations; a result file cannot prove runtime/provider execution.
Gold labels must remain outside future gateway requests.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "crates/server/tests/fixtures/memory-support/corpus.json"
VERDICTS = {"supported", "contradicted", "insufficient"}
PARTITIONS = {"calibration", "held_out"}
ROLES = {"user_assertion", "assistant_statement", "reported_tool_observation", "source_document"}
CATEGORIES = {
    "paraphrase", "quote_meaning", "wrong_citation", "negation", "history",
    "assistant_proposal", "failed_tool", "conditional", "environment", "entity",
    "injection", "correction", "retirement", "direct_contribution", "handover",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def keys(value: object, required: set[str], label: str) -> None:
    require(isinstance(value, dict) and set(value) == required, f"{label}: unexpected or missing fields")


def number(value: object, label: str, integral: bool = False) -> None:
    require(type(value) in ({int} if integral else {int, float})
            and 0 <= value <= 1_000_000_000 and math.isfinite(value),
            f"{label}: invalid nonnegative metric")


def text(value: object, label: str, maximum: int = 8192) -> None:
    require(isinstance(value, str) and 0 < len(value.encode("utf-8")) <= maximum,
            f"{label}: empty or oversized text")


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, "Duplicate JSON object field")
        result[key] = value
    return result


def load(path: Path) -> dict:
    require(path.stat().st_size <= 5 * 1024 * 1024, "JSON input exceeds 5 MiB")
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    require(isinstance(value, dict), "JSON input must be an object")
    return value


def validate_corpus(corpus: dict) -> dict:
    keys(corpus, {"version", "name", "thresholds", "cases"}, "corpus")
    require(type(corpus["version"]) is int and corpus["version"] == 1, "Unsupported corpus version")
    require(corpus["name"] == "synthetic-memory-support-v1", "Unknown corpus identity")
    keys(corpus["thresholds"], {"critical_false_usable", "minimum_positive_usability", "minimum_completed_fraction"}, "thresholds")
    require(type(corpus["thresholds"]["critical_false_usable"]) is int
            and type(corpus["thresholds"]["minimum_completed_fraction"]) is int
            and type(corpus["thresholds"]["minimum_positive_usability"]) is float,
            "Threshold types are fixed")
    require(corpus["thresholds"] == {
        "critical_false_usable": 0, "minimum_positive_usability": 0.8,
        "minimum_completed_fraction": 1,
    }, "Thresholds are fixed before held-out evaluation")
    cases = corpus["cases"]
    require(isinstance(cases, list) and 1 <= len(cases) <= 200, "Use 1–200 corpus cases")
    seen = set()
    for case in cases:
        keys(case, {"id", "partition", "category", "producer", "critical", "evidence", "candidate", "expected"}, "case")
        text(case["id"], "case ID", 100)
        require(case["id"] not in seen, "Duplicate corpus case ID")
        seen.add(case["id"])
        require(case["partition"] in PARTITIONS, "Unknown partition")
        require(case["category"] in CATEGORIES | {"partial_tool"}, "Unknown category")
        require(case["producer"] in {"server_extraction", "direct_contribution", "handover", "retirement"}, "Unknown producer")
        require(type(case["critical"]) is bool, "Critical flag must be boolean")
        evidence, candidate, expected = case["evidence"], case["candidate"], case["expected"]
        keys(evidence, {"role", "text", "selection", "fact_time"}, "evidence")
        require(evidence["role"] in ROLES, "Unknown canonical source role")
        require(evidence["fact_time"] in {"current", "historical"}, "Unknown fact-time fixture")
        text(evidence["text"], "evidence")
        keys(candidate, {"kind", "assertion", "selection", "line_from", "line_to", "quote"}, "candidate")
        require(candidate["kind"] in {"claim", "decision", "procedure", "replacement", "retirement", "handover"}, "Unknown candidate action")
        text(candidate["assertion"], "assertion")
        for selection in (evidence["selection"], candidate["selection"]):
            keys(selection, {"environment"}, "selection")
            if selection["environment"] is not None:
                text(selection["environment"], "environment", 100)
        start, end = candidate["line_from"], candidate["line_to"]
        lines = evidence["text"].splitlines()
        require(type(start) is int and type(end) is int and 1 <= start <= end <= len(lines), "Citation bounds invalid")
        require(candidate["quote"] == "\n".join(lines[start - 1:end]), "Citation quotation mismatch")
        keys(expected, {"verdict", "usable", "reason"}, "gold disposition")
        require(expected["verdict"] in VERDICTS and type(expected["usable"]) is bool, "Invalid gold disposition")
        require(not expected["usable"] or expected["verdict"] == "supported", "Usable gold requires supported evidence")
        require(case["critical"] == (not expected["usable"]), "Only negative fixtures are critical")
        text(expected["reason"], "gold rationale")
    counts = {}
    for partition in sorted(PARTITIONS):
        selected = [case for case in cases if case["partition"] == partition]
        require(CATEGORIES <= {case["category"] for case in selected}, f"{partition}: missing required category")
        positives = sum(case["expected"]["usable"] for case in selected)
        negatives = len(selected) - positives
        require(positives >= 10 and negatives >= 10, f"{partition}: needs positive and negative controls")
        counts[partition] = {"cases": len(selected), "positive": positives, "critical_negative": negatives}
    return {"corpus": corpus["name"], "version": corpus["version"], "partitions": counts,
            "provider_calls": 0, "semantic_quality_measured": False}


def score(corpus: dict, results: dict, partition: str) -> dict:
    validate_corpus(corpus)
    require(partition in PARTITIONS, "Unknown scoring partition")
    keys(results, {"version", "corpus", "mode", "versions", "observations"}, "results")
    require(type(results["version"]) is int and results["version"] == corpus["version"]
            and results["corpus"] == corpus["name"], "Result/corpus version mismatch")
    require(results["mode"] in {"fixture", "real-model"}, "Unknown result mode")
    keys(results["versions"], {"requested_model", "returned_model", "prompt", "schema", "verifier"}, "versions")
    for label, value in results["versions"].items():
        text(value, label, 200)
    observations = results["observations"]
    require(isinstance(observations, list) and 1 <= len(observations) <= 200, "Use 1–200 observations")
    all_cases = {case["id"]: case for case in corpus["cases"]}
    selected = {key: case for key, case in all_cases.items() if case["partition"] == partition}
    seen = {}
    for observation in observations:
        keys(observation, {"case_id", "state", "verdict", "usable", "request_count", "charged_tokens", "latency_ms"}, "observation")
        ident = observation["case_id"]
        require(isinstance(ident, str) and ident in all_cases, "Unknown observed case ID")
        require(ident not in seen, "Duplicate observation; do not pick a favorable attempt")
        require(observation["state"] in {"completed", "failed", "uncertain"}, "Unknown assessment state")
        require(observation["verdict"] in VERDICTS if observation["state"] == "completed"
                else observation["verdict"] is None, "Verdict inconsistent with assessment state")
        require(type(observation["usable"]) is bool, "Usability must be boolean")
        for field in ("request_count", "charged_tokens"):
            number(observation[field], field, integral=True)
        number(observation["latency_ms"], "latency_ms")
        if results["mode"] == "fixture":
            require(observation["request_count"] == observation["charged_tokens"] == 0,
                    "A fixture cannot report paid provider usage")
        seen[ident] = observation
    missing = sorted(set(selected) - set(seen))
    observed = [seen[key] for key in selected if key in seen]
    positive_ids = [key for key, case in selected.items() if case["expected"]["usable"]]
    critical_false_usable = [key for key, case in selected.items() if case["critical"]
                             and key in seen and seen[key]["usable"]]
    unverified_usable = [key for key in selected if key in seen and seen[key]["usable"]
                        and (seen[key]["state"] != "completed" or seen[key]["verdict"] != "supported")]
    correct_verdicts = sum(key in seen and seen[key]["state"] == "completed"
                          and seen[key]["verdict"] == case["expected"]["verdict"]
                          for key, case in selected.items())
    useful = sum(key in seen and seen[key]["state"] == "completed" and seen[key]["usable"]
                 for key in positive_ids)
    completed = sum(obs["state"] == "completed" for obs in observed)
    complete = completed == len(selected) and not missing
    thresholds_met = (complete and not critical_false_usable and not unverified_usable
                      and useful / len(positive_ids) >= corpus["thresholds"]["minimum_positive_usability"])
    latency = sorted(obs["latency_ms"] for obs in observed)
    return {
        "corpus": corpus["name"], "partition": partition, "mode": results["mode"],
        "versions": results["versions"], "expected_cases": len(selected),
        "completed": completed, "failed": sum(obs["state"] == "failed" for obs in observed),
        "uncertain": sum(obs["state"] == "uncertain" for obs in observed), "missing": missing,
        "critical_false_usable": critical_false_usable, "unverified_usable": unverified_usable,
        "positive_usability": useful / len(positive_ids),
        "verdict_accuracy": correct_verdicts / len(selected),
        "request_count": sum(obs["request_count"] for obs in observed),
        "charged_tokens": sum(obs["charged_tokens"] for obs in observed),
        "latency_ms_p50": latency[len(latency) // 2] if latency else None,
        "latency_ms_p95": latency[min(len(latency) - 1, math.ceil(len(latency) * .95) - 1)] if latency else None,
        "fixture_enforcement_met": thresholds_met if results["mode"] == "fixture" else None,
        "declared_quality_thresholds_met": thresholds_met if results["mode"] == "real-model" else False,
        "runtime_evidence_verified": False,
        "note": "Offline scoring of declared observations; independently verify gateway/runtime receipts before cutover.",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("check")
    scoring = sub.add_parser("score")
    scoring.add_argument("results", type=Path)
    scoring.add_argument("--partition", choices=sorted(PARTITIONS), default="held_out")
    args = parser.parse_args()
    try:
        corpus = load(CORPUS)
        result = validate_corpus(corpus) if args.command == "check" else score(corpus, load(args.results), args.partition)
        print(json.dumps(result, indent=2, allow_nan=False))
        if args.command == "score":
            return 0 if (result["fixture_enforcement_met"] or result["declared_quality_thresholds_met"]) else 1
        return 0
    except (ValueError, OSError, TypeError, KeyError) as error:
        print(f"Baseline input rejected: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
