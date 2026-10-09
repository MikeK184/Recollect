#!/usr/bin/env python3
"""Reconcile the frozen October 8 campaign receipts offline; never call a provider."""
import argparse
import json
import math
from pathlib import Path
from uuid import UUID

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / ".cache"
B = "longmemeval-glm47-no-reasoning"
A = "longmemeval-1fdefad9-748f-4c5a-8f4f-7d9b1406445f"
C = "longmemeval-glm53-throughput"


def read(relative):
    return json.loads((CACHE / relative).read_text())


def amount(value):
    assert isinstance(value, (int, float)) and not isinstance(value, bool)
    assert math.isfinite(value) and value >= 0
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--interim", action="store_true")
    parser.add_argument("--account", type=Path, default=CACHE / "openrouter-account-snapshot.json")
    parser.add_argument("--recovered-run", type=Path, default=CACHE / "longmemeval-glm53-recovery")
    args = parser.parse_args()
    assert args.account.resolve().is_relative_to(CACHE)
    account = json.loads(args.account.read_text())
    run_b = read(B + "/report.json")
    run_c = read(C + "/report.json") if (CACHE / C / "report.json").exists() else None
    if not args.interim:
        assert run_c and run_c["complete"] and len(run_c["rows"]) == 50
        assert all(row["complete"] for row in run_c["rows"])
        for name, count in [("primary", 50), ("audit-first-25", 25)]:
            assert len((CACHE / C / f"{name}.jsonl.eval-results-gpt-4o").read_text().splitlines()) == count
        recovered = args.recovered_run.resolve()
        assert recovered.is_relative_to(CACHE)
        final_recovery = json.loads((recovered / "report.json").read_text())
        assert final_recovery["complete"] and len(final_recovery["rows"]) == 50
        assert all(row["complete"] and row["pipeline_state"] == "completed" for row in final_recovery["rows"])
        for name, count in [("primary", 50), ("audit-first-25", 25)]:
            assert len((recovered / f"{name}.jsonl.eval-results-gpt-4o").read_text().splitlines()) == count
    components, identities = [], set()

    def component(name, receipts, unknown_reserve=.1, declared=None, notes=None,
                  allow_unidentified=False):
        known = 0.0
        unknown, reservations = [], 0.0
        tokens, unknown_tokens = 0, 0
        for index, row in enumerate(receipts):
            identity = row.get("id")
            if identity:
                assert isinstance(identity, str) and str(UUID(identity)) == identity
                assert identity not in identities, f"Duplicate paid identity: {identity}"
                identities.add(identity)
            else:
                assert allow_unidentified, "Only declared identifier-free preflights may omit identities"
            assert row["state"] in ["succeeded", "failed", "uncertain"]
            cost = row["cost_usd"]
            if cost is None:
                unknown.append(identity or f"{name}:unidentified:{index}")
                reservations += amount(row.get("reservation_usd", unknown_reserve))
            else:
                known += amount(cost)
            if row.get("total_tokens") is None:
                unknown_tokens += 1
            else:
                assert isinstance(row["total_tokens"], int) and not isinstance(row["total_tokens"], bool)
                tokens += amount(row["total_tokens"])
        if declared is not None:
            assert math.isclose(known, amount(declared), abs_tol=1e-9), name
        components.append({"name": name, "requests": len(receipts),
                           "known_billed_usd": known, "unknown_cost_requests": len(unknown),
                           "unknown_cost_request_ids": unknown,
                           "unresolved_reservation_usd": reservations,
                           "known_received_total_tokens": tokens,
                           "unknown_token_requests": unknown_tokens, "notes": notes or []})

    hotpot = read("public-benchmark-6e5def27-89fe-4548-8868-514a100bb529/report.json")
    assert hotpot["complete"] and len(hotpot["model_ledger"]) == 97
    component("HotpotQA Qwen", hotpot["model_ledger"], declared=hotpot["actual_cost_usd"])
    for name in ["openrouter-support", "openrouter-support-glm53"]:
        support = read(name + "/receipts.json")
        component(name, [r for case in support["cases"] for r in case["gateway_receipts"]],
                  declared=support["actual_cost_usd"])
    luna_support = CACHE / "openrouter-support-gpt6-luna/results.receipts.json"
    if luna_support.exists():
        support = json.loads(luna_support.read_text())
        component("openrouter-support-gpt6-luna", [r for case in support["cases"] for r in case.get("gateway_receipts", [])],
                  declared=support.get("actual_cost_usd"))
    elif not args.interim:
        raise AssertionError("Missing Luna native support diagnostic")
    runs = [(A, read(A + "/report.json")), (B, run_b)]
    if run_c:
        runs.append((C, run_c))
    for path in sorted(CACHE.glob("longmemeval-glm53-recovery*/report.json")):
        runs.append((path.parent.name, json.loads(path.read_text())))
    for name, report in runs:
        requests = [r for row in report["rows"] for r in row["model_ledger"]]
        assert all(r["provider"] == "openrouter" for r in requests)
        component(name, requests, .18 if name == A else .1, report["actual_cost_usd"],
                  ["Compatible prior index receipts are excluded from this experiment and retained under their original run."] if name in [B, C] else [])
    component("Discarded Qwen alias attempts", read("openrouter-discarded-qwen-receipts.json"),
              declared=.00003746)

    judge_successes = 0
    proxies = ["cognee-bench/proxy", "cognee-bench/vector-proxy"] + [path.parent.relative_to(CACHE).as_posix() for path in sorted(CACHE.glob("longmemeval-judge-proxy*/requests.jsonl"))]
    for name in proxies:
        events = [json.loads(line) for line in (CACHE / name / "requests.jsonl").read_text().splitlines()]
        admitted, blocked = {}, 0
        for event in events:
            identity, kind = event["id"], event["event"]
            if kind == "reserved":
                assert identity not in admitted
                admitted[identity] = {"id": identity, "state": "running", "cost_usd": None,
                                      "reservation_usd": amount(event["reservation_usd"])}
            elif identity in admitted and kind in ["completed", "invalid_response", "failed", "uncertain"]:
                row = admitted[identity]
                assert row["state"] == "running", "Repeated terminal receipt"
                row["state"] = {"completed": "succeeded", "uncertain": "uncertain"}.get(kind, "failed")
                row["cost_usd"] = event.get("actual_cost_usd")
                row["total_tokens"] = event.get("usage", {}).get("total_tokens")
            elif kind == "rejected" and identity not in admitted:
                blocked += 1
        if name.startswith("longmemeval-judge-proxy"):
            judge_successes += sum(row["state"] == "succeeded" for row in admitted.values())
        component(name, list(admitted.values()), notes=[
            f"{blocked} locally rejected client calls never reserved or dispatched upstream.",
            "Judge receipts include the one initial preflight as well as primary/audit judgments."
            if name.startswith("longmemeval-judge-proxy") else "Only reserved identities count as provider attempts."])
    if not args.interim:
        assert judge_successes == 151, "Original and final recovery each require 50 primary/25 audit plus the initial preflight; completed judgments are not repeated"

    for path in ["openrouter-preflight/glm47-no-reasoning.json", "openrouter-preflight/low-effort.json"]:
        receipt = read(path)
        component(path, [{"state": "succeeded", "cost_usd": receipt["usage"]["cost"],
                          "total_tokens": receipt["usage"]["total_tokens"]}],
                  notes=["Original preflight retained usage without a gateway/proxy request identity."],
                  allow_unidentified=True)
    component("openrouter-preflight/report.json", [
        {"state": "succeeded" if r["valid"] else "failed", "cost_usd": r.get("usage", {}).get("cost"),
         "total_tokens": r.get("usage", {}).get("total_tokens")}
        for r in read("openrouter-preflight/report.json")["requests"]],
        notes=["Muse 403 is a known outcome with unavailable cost; reserve USD 0.10."],
        allow_unidentified=True)
    luna = read("openrouter-preflight/gpt6-luna-none.json")
    component("openrouter-preflight/gpt6-luna-none.json", [
        {"id": luna["id"], "state": luna["state"], "cost_usd": luna.get("usage", {}).get("cost"),
         "total_tokens": luna.get("usage", {}).get("total_tokens"), "reservation_usd": luna["reservation_usd"]}])

    known = sum(row["known_billed_usd"] for row in components)
    reservation = sum(row["unresolved_reservation_usd"] for row in components)
    usage, limit, remaining = (amount(account[key]) for key in ["usage", "limit", "remaining"])
    assert usage <= min(10, limit) and max(usage, known) + reservation <= 8
    result = {"interim": args.interim, "components": components, "known_billed_usd": known,
              "recovered_run": str(args.recovered_run.resolve()) if not args.interim else None,
              "unknown_cost_requests": sum(row["unknown_cost_requests"] for row in components),
              "unresolved_reservation_usd": reservation, "unique_identified_requests": len(identities),
              "key_observation": account, "key_usage_plus_unresolved_reservations_usd": usage + reservation,
              "known_receipts_plus_unresolved_reservations_usd": known + reservation,
              "key_reported_minus_known_receipts_usd": usage - known,
              "notes": ["Preserved/restored database copies do not create new paid attempts.",
                        "Key usage is cumulative and may include unresolved or unattributed charges.",
                        "Unknown costs are not zero. Native/preflight reservations are USD 0.10 each; A retains USD 0.18; proxy reservations use recorded values.",
                        "Active indexing is absent from an interim case ledger until that case is finalized.",
                        "This script makes no network or provider calls."]}
    output = CACHE / ("openrouter-campaign-accounting-interim.json" if args.interim else "openrouter-campaign-accounting-final.json")
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"output": str(output), "interim": args.interim,
                      "known_billed_usd": known, "unresolved_reservation_usd": reservation,
                      "key_usage_usd": usage, "key_remaining_usd": remaining}))


if __name__ == "__main__":
    main()
