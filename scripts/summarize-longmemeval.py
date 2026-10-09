#!/usr/bin/env python3
"""Summarize a completed, officially judged run without making provider calls."""
import argparse
import hashlib
import json
import math
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TYPES = ["single-session-user", "single-session-preference", "single-session-assistant",
         "multi-session", "temporal-reasoning", "knowledge-update"]
JUDGE = "gpt-4o-2024-08-06"


def pipeline_state(row):
    return row.get("pipeline_state", row.get("answer_response", {}).get("state"))


def percentile(values, fraction):
    values = sorted(values)
    if not values:
        return None
    position = (len(values) - 1) * fraction
    lower = math.floor(position)
    upper = math.ceil(position)
    return values[lower] + (values[upper] - values[lower]) * (position - lower)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("--invalid-latency", action="append", default=[],
                        help="Disclosed case whose original elapsed measurement is unavailable")
    args = parser.parse_args()
    run = args.run.resolve()
    assert run.is_relative_to(ROOT / ".cache"), "Only owned benchmark reports"
    report = json.loads((run / "report.json").read_text())
    assert report["complete"] and len(report["rows"]) == 50
    assert all(row["complete"] for row in report["rows"])
    inputs = ROOT / ".cache/longmemeval-input"
    manifest = json.loads((inputs / "manifest.json").read_text())
    assert report["manifest"]["sha256"] == manifest["sha256"]
    for name in ["subset-50.json", "evaluate_qa.py", "print_qa_metrics.py"]:
        assert hashlib.sha256((inputs / name).read_bytes()).hexdigest() == manifest["sha256"][name]
    references = json.loads((inputs / "subset-50.json").read_text())
    expected_ids = [row["question_id"] for row in references]
    assert len({row["id"] for row in report["rows"]}) == 50
    assert {row["id"] for row in report["rows"]} == set(expected_ids)
    # Warm histories can be journaled before a later cold history. Canonicalize
    # analysis order by the frozen manifest without altering any raw receipt.
    by_id = {row["id"]: row for row in report["rows"]}
    report["rows"] = [by_id[identity] for identity in expected_ids]
    assert all(row["type"] == reference["question_type"]
               for row, reference in zip(report["rows"], references))
    assert len(set(expected_ids)) == 50
    invalid_latency = set(args.invalid_latency)
    assert invalid_latency <= set(expected_ids)
    primary = [json.loads(line) for line in (run / "primary.jsonl.eval-results-gpt-4o").read_text().splitlines()]
    audit = [json.loads(line) for line in (run / "audit-first-25.jsonl.eval-results-gpt-4o").read_text().splitlines()]
    assert [row["question_id"] for row in primary] == expected_ids
    assert [row["question_id"] for row in audit] == expected_ids[:25]
    for row in primary + audit:
        assert row["autoeval_label"]["model"] == JUDGE
        assert type(row["autoeval_label"]["label"]) is bool
    labels = {row["question_id"]: row["autoeval_label"]["label"] for row in primary}
    assert all(result["hypothesis"] == row["hypothesis"] for result, row in zip(primary, report["rows"]))
    requests = [request for row in report["rows"] for request in row["model_ledger"]]
    assert len({request["id"] for request in requests}) == len(requests)
    assert all(request["state"] in ["succeeded", "failed"] for request in requests)
    assert all(request["provider"] == report["provider"] for request in requests)
    assert all(request["model"] in [report["text_model"], report["embedding_model"]] for request in requests)
    assert all(request["cost_usd"] is None or
               (math.isfinite(request["cost_usd"]) and request["cost_usd"] >= 0)
               for request in requests)
    assert all(row["hypothesis"] == "" for row in report["rows"]
               if pipeline_state(row) == "failed")
    known_cost = sum(request["cost_usd"] for request in requests if request["cost_usd"] is not None)
    assert math.isclose(known_cost, report["actual_cost_usd"], abs_tol=1e-9)
    by_type = {}
    for kind in TYPES:
        rows = [row for row in report["rows"] if row["type"] == kind]
        assert rows, "Every official type must remain in the denominator"
        durations = [row["elapsed_ms"] for row in rows
                     if row["id"] not in invalid_latency and row.get("elapsed_ms") is not None]
        delivered = [request["input_tokens"] for row in rows for request in row["model_ledger"]
                     if request["purpose"] == "answering" and request["input_tokens"] is not None]
        by_type[kind] = {
            "questions": len(rows), "correct": sum(labels[row["id"]] for row in rows),
            "accuracy": sum(labels[row["id"]] for row in rows) / len(rows),
            "pipeline_failures": [row["id"] for row in rows if pipeline_state(row) == "failed"],
            "misses": [row["id"] for row in rows if not labels[row["id"]]],
            "latency_samples": len(durations), "latency_ms_p50": percentile(durations, .5),
            "latency_ms_p95": percentile(durations, .95),
            "delivered_model_input_token_samples": len(delivered),
            "delivered_model_input_tokens_p50": percentile(delivered, .5),
        }
    abstention = [value for qid, value in labels.items() if "_abs" in qid]
    assert len(abstention) == 8
    metrics = {
        "task_averaged_accuracy": sum(row["accuracy"] for row in by_type.values()) / 6,
        "overall_accuracy": sum(labels.values()) / 50,
        "abstention_accuracy": sum(abstention) / len(abstention),
    }
    official = (run / "primary-metrics.txt").read_text()
    for kind, row in by_type.items():
        assert f"{kind}: {round(row['accuracy'], 4)} ({row['questions']})" in official
    for label, key in [("Task-averaged Accuracy", "task_averaged_accuracy"),
                       ("Overall Accuracy", "overall_accuracy"), ("Abstention Accuracy", "abstention_accuracy")]:
        assert f"{label}: {round(metrics[key], 4)}" in official
    disagreements = [a["question_id"] for a, b in zip(primary[:25], audit)
                     if a["autoeval_label"]["label"] != b["autoeval_label"]["label"]]
    saved_audit = json.loads((run / "judge-audit.json").read_text())
    assert saved_audit["disagreements"] == len(disagreements)
    all_durations = [row["elapsed_ms"] for row in report["rows"]
                     if row["id"] not in invalid_latency and row.get("elapsed_ms") is not None]
    completed_durations = [row["elapsed_ms"] for row in report["rows"]
                           if pipeline_state(row) == "completed" and row["id"] not in invalid_latency
                           and row.get("elapsed_ms") is not None]
    available_recall = [row["session_recall"] for row in report["rows"]
                        if row.get("answer_response", {}).get("recall") is not None
                        and row.get("session_recall") is not None]
    non_abstention_recall = [row["session_recall"] for row in report["rows"]
                            if "_abs" not in row["id"]
                            and row.get("answer_response", {}).get("recall") is not None
                            and row.get("session_recall") is not None]
    completed_rows = [row for row in report["rows"] if pipeline_state(row) == "completed"]
    correct_completed = sum(labels[row["id"]] for row in completed_rows)
    cold_preparation = []
    for row in report["rows"]:
        if row.get("warm_profile_id") or not row.get("started_at"):
            continue
        query_times = [request["created_at"] for request in row["model_ledger"]
                       if request.get("prompt_label") == "semantic-query-1"]
        if not query_times:
            continue
        start = datetime.fromisoformat(row["started_at"].replace("Z", "+00:00"))
        end = datetime.fromisoformat(min(query_times).replace("Z", "+00:00"))
        elapsed = (end - start).total_seconds() * 1000
        assert elapsed >= 0
        cold_preparation.append({"id": row["id"], "elapsed_ms": elapsed})
    summary = {
        "manifest": manifest, "provider": report["provider"], "text_model": report["text_model"],
        "embedding_model": report["embedding_model"], "embedding_dimensions": report["embedding_dimensions"],
        "recall": report["recall"], "prompt": report["prompt"], "learning": report["learning"],
        "extraction": report["extraction"], "official_metrics": metrics, "by_type": by_type,
        "pipeline_states": report["pipeline_states"], "judge": JUDGE,
        "ask_latency_ms": {"all_samples": len(all_durations),
                           "all_p50": percentile(all_durations, .5),
                           "all_p95": percentile(all_durations, .95),
                           "completed_samples": len(completed_durations),
                           "completed_p50": percentile(completed_durations, .5),
                           "completed_p95": percentile(completed_durations, .95)},
        "available_response_session_recall": {"samples": len(available_recall),
                                              "mean": sum(available_recall) / len(available_recall) if available_recall else None,
                                              "non_abstention_samples": len(non_abstention_recall),
                                              "non_abstention_mean": sum(non_abstention_recall) / len(non_abstention_recall) if non_abstention_recall else None},
        "answer_availability": {"completed": len(completed_rows), "questions": 50,
                                "fraction": len(completed_rows) / 50,
                                "correct_completed_answers": correct_completed,
                                "completed_answer_accuracy": correct_completed / len(completed_rows) if completed_rows else None},
        "cold_preparation": {"cases": cold_preparation,
                             "samples": len(cold_preparation),
                             "p50_ms": percentile([row["elapsed_ms"] for row in cold_preparation], .5),
                             "p95_ms": percentile([row["elapsed_ms"] for row in cold_preparation], .95)},
        "judge_audit_questions": 25, "judge_disagreements": disagreements,
        "failed_pipeline_cases_credited_by_official_judge": [row["id"] for row in report["rows"]
            if pipeline_state(row) == "failed" and labels[row["id"]]],
        "indexing_failures": [row["id"] for row in report["rows"]
            if row.get("pipeline_failure", {}).get("stage") == "indexing"],
        "ingestion_failures": [row["id"] for row in report["rows"]
            if row.get("pipeline_failure", {}).get("stage") == "ingestion"],
        "unavailable_ask_latency_case_ids": [row["id"] for row in report["rows"]
            if row.get("elapsed_ms") is None],
        "invalid_latency_case_ids": sorted(invalid_latency), "model_request_count": len(requests),
        "known_billed_usd": known_cost,
        "unknown_cost_request_ids": [request["id"] for request in requests if request["cost_usd"] is None],
        "known_received_total_tokens": sum(request["total_tokens"] for request in requests if request["total_tokens"] is not None),
        "unknown_token_request_ids": [request["id"] for request in requests if request["total_tokens"] is None],
        "notes": ["Official accuracy includes all 50 cases, including failures and abstention.",
                  "Input tokens are provider-reported complete answering prompts, including context and instructions.",
                  "Latency covers the Ask request, excludes ingestion, and uses linear-interpolated percentiles.",
                  "Invalid elapsed measurements are disclosed and excluded only from latency, not accuracy.",
                  "Indexing failures have an empty hypothesis, no Ask call or fabricated API response, and unavailable Ask latency/recall.",
                  "Rejected history imports remain failed cases; their frozen inputs are not sanitized or bypassed, and no incomplete Ask is dispatched.",
                  "Fast failed calls are separated from completed-answer latency.",
                  "Session recall is unavailable when a failed Ask response did not retain recall; an empty legacy list is not a proved retrieval miss.",
                  "Non-abstention session recall separates required supporting evidence from deliberately unavailable facts. Both recall fields include only retained API recall.",
                  "Completed-answer accuracy is a diagnostic conditional on availability, not an official full-denominator metric.",
                  "Warm journal rows are analyzed in frozen manifest order without changing raw receipts.",
                  "Cold preparation spans case creation/import/processing/index readiness until query-embedding admission; warm histories and failed indexing are excluded. It is not pure embedding throughput.",
                  "Known billed USD excludes unresolved amounts, prior warm-index experiments and judge costs."],
    }
    (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({"summary": str(run / "summary.json"), **metrics, "known_billed_usd": known_cost}))


if __name__ == "__main__":
    main()
