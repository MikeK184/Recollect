"""Bounded mixed-workload acceptance; every mutation uses the product API."""
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
import json
import os
import platform
import re
import subprocess
import threading
import time
import uuid

import recovery_installation as runtime
from recovery_transport import private_json

EMPTY = {"repository_ids": [], "area_ids": [], "environment_id": None}


def identifier():
    return str(uuid.uuid4())


def resource_summary(samples):
    maxima = {}
    combined = []
    for sample in samples:
        total = 0
        for row in sample["services"]:
            used = row["MemUsage"].split(" / ", 1)[0]
            match = re.fullmatch(r"([0-9.]+)(B|KiB|MiB|GiB|kB|MB|GB)", used)
            assert match, "Unrecognized Docker memory unit"
            factor = {"B": 1, "KiB": 1024, "MiB": 1024**2, "GiB": 1024**3,
                      "kB": 1000, "MB": 1000**2, "GB": 1000**3}[match[2]]
            memory = round(float(match[1]) * factor)
            cpu = float(row["CPUPerc"].removesuffix("%"))
            prior = maxima.setdefault(row["Name"], {"memory_bytes": 0, "cpu_percent": 0})
            prior["memory_bytes"] = max(prior["memory_bytes"], memory)
            prior["cpu_percent"] = max(prior["cpu_percent"], cpu)
            total += memory
        combined.append(total)
    return {"per_service_sampled_max": maxima, "combined_sampled_memory_max_bytes": max(combined)}


def device(evaluation, token):
    # Independent HTTP state per worker, with no shared cookie jar or mutable scope.
    session = type(evaluation.owner)(evaluation.saved["origin"])
    session.opener.addheaders = [("Authorization", "Bearer " + token)]
    return session


def pair(session, label):
    pending = session.call("POST", "/api/devices/pairings", {"name": label})
    session.call("POST", "/api/devices/pairings/" + pending["user_code"] + "/approve", {"approve": True})
    result = session.call("POST", "/api/devices/pairings/poll", {"device_code": pending["device_code"]})
    session.call("POST", "/api/devices/pairings/finish", {"device_code": pending["device_code"]}, expected=204)
    return result


def publish(e, state, client):
    base, repository = state["base"], state["repositories"][0]
    selected = {**EMPTY, "repository_ids": [repository]}
    task = client.call("POST", base + "/workspace/tasks", {"label": "Integrated structural fixture", "selection": selected})
    operation = client.call("POST", base + "/workspace/tasks/" + task["task"]["id"] + "/operations", {"kind": "capture"})
    e.owner.call("PUT", base + "/repositories/policy", {"allow_file_content": True})
    content = "".join(f"pub fn node_{i}() {{ node_{(i+1)%100}(); node_{(i+2)%100}(); }}\n" for i in range(100))
    facts = [{"id": f"n{i}", "kind": "function", "name": f"node_{i}", "file": "src/lib.rs", "line": i+1,
              "relations": [{"kind": "calls", "target": f"node_{(i+j)%100}", "target_id": f"n{(i+j)%100}"} for j in (1, 2)]} for i in range(100)]
    result = client.call("POST", base + "/repositories/" + repository + "/snapshots", {
        "publication_id": identifier(), "operation_id": operation["id"], "origin": "example.test/integrated/repo00",
        "revision": "a" * 40, "branch": "main", "dirty": False, "captured_at": datetime.now(timezone.utc).isoformat(),
        "adapter": "enola-committed", "adapter_build": "synthetic-evaluation", "extractor_version": "fixture",
        "settings": {"retained_files": ["src/lib.rs"]}, "files": [{"path": "src/lib.rs", "object_id": "b" * 40,
            "mode": "100644", "size": len(content.encode()), "status": "materialized", "extraction": "facts_emitted", "content": content}],
        "facts": facts, "insights": [], "receipt": {"enola_version": "fixture", "extractor_version": "fixture", "fact_count": 100, "quality": {"parse_errors": 0}}})
    snapshot = result["snapshot"]["id"]
    e.wait(lambda: e.owner.call("GET", base + "/repository-snapshots/" + snapshot + "/facts"),
           lambda value: value["total"] == 100, "100 structural facts")
    # Fact visibility can precede completion of the publication job. Let the
    # existing worker discover/project the accepted snapshot before measuring.
    e.wait(lambda: e.owner.call("GET", base + "/graph"),
           lambda value: any(g["snapshot_id"] == snapshot and g["state"] == "ready"
                            and g["node_count"] == 100 and g["edge_count"] == 200
                            for g in value["generations"]), "load structural graph")
    state["graph"] = {"kind": "repository", "snapshot_id": snapshot, "selection": selected, "relations": ["calls"]}


def run(e):
    from importlib import import_module
    # The entrypoint exposes the same declared nearest-rank summary helper.
    summary_latency = import_module("__main__").latency
    path = e.root / "workload-state.json"
    assert not path.exists(), "Existing workload state is retained; use an explicit new run directory"
    owner = e.owner
    state = {"installation": e.saved["name"], "stage": "setup", "run_id": identifier()}
    private_json(path, state)
    baseline = runtime.inventory(e.saved)
    brain = owner.call("POST", "/api/brains", {"name": "Integrated concurrent workspace"})
    base = "/api/brains/" + brain["id"]
    state.update(brain=brain, base=base)
    private_json(path, state)
    account = {"username": "integrated-" + uuid.uuid4().hex[:10], "password": uuid.uuid4().hex}
    invite = owner.call("POST", "/api/team/invitations", {"username": account["username"]})
    member = type(owner)(owner.origin)
    member.call("POST", "/api/auth/enroll", {"token": invite["token"], "password": account["password"]})
    member.login(**account)
    private = member.call("POST", "/api/brains", {"name": "Integrated private canary"})
    member.call("POST", "/api/brains/" + private["id"] + "/sources", {"title": "ForbiddenIntegratedCanary",
        "media_type": "text/plain", "content": "ForbiddenIntegratedCanary belongs only to the member's private Brain.", "retain_content": True})
    owner.call("GET", "/api/brains/" + private["id"], expected=404)
    owner.call("PUT", base + "/grants/" + private["owner_id"], {"role": "writer"})
    accounts = {"member": account, "owner_device": pair(owner, "Integrated owner"), "member_device": pair(member, "Integrated member")}
    private_json(e.root / "workload-credentials.json", accounts)
    state["member_brain"] = private
    private_json(path, state)
    clients = [device(e, accounts[role + "_device"]["token"]) for role in ("owner", "member")]
    for label, client in zip(("owner", "member"), clients):
        root = "/integrated/" + label
        client.call("POST", base + "/workspace/checkouts", {"workspace_root": root, "complete": True, "notes": [],
            "checkouts": [{"local_path": f"{root}/repo{i:02}", "origin": f"https://example.test/integrated/repo{i:02}.git",
                "head": None, "branch": "main", "dirty": False, "status": "available"} for i in range(50)]})
        catalogue = client.call("GET", base + "/workspace")
        assert len(catalogue["repositories"]) == 50 and len(catalogue["checkouts"]) == 50
        assert all(c["observation"]["local_path"].startswith(root + "/") for c in catalogue["checkouts"])
    state["repositories"] = [r["id"] for r in sorted(catalogue["repositories"], key=lambda r: r["canonical_origin"])]
    environments = [owner.call("POST", base + "/evidence/groups", {"kind": "environment", "name": value})["id"] for value in ("Development", "Production")]
    capture_policy = owner.call("GET", base + "/capture/policy")
    owner.call("PUT", base + "/capture/policy", {"base_change": capture_policy["change_id"], "policy": {**capture_policy["policy"], "enabled": True}})
    state["bindings"] = []
    for index in range(4):
        selected = {**EMPTY, "repository_ids": [state["repositories"][index]], "environment_id": environments[index % 2]}
        client = clients[index % 2]
        task = client.call("POST", base + "/workspace/tasks", {"label": f"Concurrent producer {index}", "selection": selected})
        endpoint = base + "/workspace/tasks/" + task["task"]["id"]
        operation = client.call("POST", endpoint + "/operations", {"kind": "capture"})
        reader = client.call("POST", endpoint + "/operations", {"kind": "retrieval"})
        binding = client.call("POST", base + "/capture/bindings", {"id": identifier(), "operation_id": operation["id"], "host": "codex", "host_version": "0.154.0"})
        state["bindings"].append({"task": task, "selection": selected, "binding": binding, "operation": operation, "reader": reader, "account": index % 2})
    state["readers"] = []
    for index in range(4):
        client = clients[index % 2]
        task = client.call("POST", base + "/workspace/tasks", {"label": f"Concurrent reader {index}", "selection": EMPTY})
        operation = client.call("POST", base + "/workspace/tasks/" + task["task"]["id"] + "/operations", {"kind": "retrieval"})
        state["readers"].append(operation)
    private_json(path, state)
    publish(e, state, clients[0])
    state.update(stage="import", documents=[])
    private_json(path, state)
    started = time.perf_counter()
    for index in range(200):
        content = f"Integrated load document {index:03} retains independent engineering evidence.\n" + "The declared deployment requires an observed health probe before success is recorded.\n" * 12
        source = owner.call("POST", base + "/sources", {"title": f"Integrated load document {index:03}", "media_type": "text/plain", "content": content, "retain_content": True})
        state["documents"].append(source)
        private_json(path, state)
    for source in state["documents"]:
        e.wait(lambda: owner.call("GET", base + "/sources/" + source["id"] + "/versions/" + source["version"]["id"]),
               lambda value: value["version"]["processing"] == "ready", "imported workload source")
    import_seconds = time.perf_counter() - started
    assert owner.call("GET", base + "/models/usage")["total"] == 0
    forbidden = owner.call("POST", base + "/recall", {"query": "ForbiddenIntegratedCanary"})
    assert not forbidden["context"]["items"]
    owner.call("POST", base + "/recall", {"query": "load", "limit": 100000}, expected=400)
    assert owner.call("POST", base + "/recall", {"query": "Integrated load document", "limit": 4})["context"]["items"]
    state.update(stage="concurrent", import_seconds=import_seconds,
                 event_ids=[[identifier() for _ in range(30)] for _ in range(4)],
                 analytics_request_ids=[identifier() for _ in range(4)])
    private_json(path, state)
    samples, stop, sample_errors = [], threading.Event(), []

    def sample():
        while not stop.is_set():
            try:
                raw = runtime.command(e.saved, ["stats", "--no-stream", "--format", "json"], timeout=10).decode()
                rows = json.loads(raw) if raw.lstrip().startswith("[") else [json.loads(line) for line in raw.splitlines() if line.strip()]
                samples.append({"elapsed_seconds": time.perf_counter() - started, "services": [
                    {key: row.get(key) for key in ("Name", "CPUPerc", "MemUsage", "MemPerc", "PIDs")} for row in rows]})
            except (ValueError, OSError, json.JSONDecodeError):
                sample_errors.append("resource_sample_unavailable")
            stop.wait(1)

    barrier = threading.Barrier(9)

    def recall(client, body, refusals):
        # The product admits four concurrent recalls. Eight independent
        # callers can receive explicit backpressure; keep every refusal and
        # include this bounded caller wait in the measured operation latency.
        deadline = time.monotonic() + 60
        while True:
            try:
                return client.call("POST", base + "/recall", body)
            except AssertionError as error:
                if (getattr(error, "status", None), getattr(error, "code", None)) != (429, "recall_busy"):
                    raise
                refusals.append("recall_busy")
                if time.monotonic() >= deadline:
                    raise
                time.sleep(.05)

    def producer(index):
        binding = state["bindings"][index]
        role = ("owner", "member")[index % 2]
        client = device(e, accounts[role + "_device"]["token"])
        records, refusals = [], []
        barrier.wait(timeout=20)
        for number in range(30):
            event = {"id": state["event_ids"][index][number], "binding_id": binding["binding"]["id"], "event": {"host_event": "UserPromptSubmit",
                "host_session_id": f"integrated-producer-{index}", "turn_id": str(number), "agent_id": None,
                "tool_use_id": None, "tool_name": None, "kind": "prompt", "outcome": "reported",
                "content": f"IntegratedCapture{index}Event{number} retains its original scope.", "coverage": ["partial_host_coverage"],
                "captured_at": datetime.now(timezone.utc).isoformat()}}
            call_started = time.perf_counter()
            receipt = client.call("POST", base + "/capture/events", event)
            admitted = time.perf_counter()
            assert receipt["state"] == "accepted"
            private_json(e.root / f"workload-producer-{index}-last.json", {"number": number, "receipt": receipt})
            attempts = 0
            def readable():
                nonlocal attempts
                attempts += 1
                result = recall(client, {"operation_id": binding["reader"]["id"], "selection": binding["selection"],
                    "exact": {"kind": "source_version", "id": receipt["source_version_id"]}, "query": f"IntegratedCapture{index}Event{number}", "limit": 4}, refusals)
                return any(item["id"] == receipt["source_version_id"]
                           and event["event"]["content"] in item["text"]
                           for item in result["context"]["items"])
            e.wait(readable, bool, "captured evidence readable under original operation", seconds=60)
            records.append({"receipt": receipt, "admission_ms": (admitted - call_started) * 1000,
                            "readable_ms": (time.perf_counter() - admitted) * 1000, "readability_probes": attempts})
            if number == 5:
                endpoint = base + "/workspace/tasks/" + binding["task"]["task"]["id"] + "/scope"
                changed = client.call("PUT", endpoint, {"base_scope": binding["task"]["task"]["scope"]["id"],
                    "selection": {**EMPTY, "repository_ids": [state["repositories"][index + 4]], "environment_id": environments[(index + 1) % 2]}})
                assert changed["handoff"]["fresh_context_required"]
        return {"kind": "producer", "index": index, "records": records, "capacity_refusals": len(refusals)}

    def reader(index):
        client = device(e, accounts[("owner", "member")[index % 2] + "_device"]["token"])
        rows, refusals = [], []
        barrier.wait(timeout=20)
        for _ in range(30):
            call_started = time.perf_counter()
            result = recall(client, {"operation_id": state["readers"][index]["id"], "query": "Integrated load document", "limit": 4, "context_bytes": 8192}, refusals)
            rows.append((time.perf_counter() - call_started) * 1000)
            assert result["context"]["items"] and result["context_bytes"] <= 8192
            assert "ForbiddenIntegratedCanary" not in json.dumps(result["context"]["items"])
        return {"kind": "reader", "index": index, "latencies": rows, "capacity_refusals": len(refusals)}

    started = time.perf_counter()
    monitor = threading.Thread(target=sample, daemon=True)
    monitor.start()
    results, analytics, failures = [], [], []
    try:
        with ThreadPoolExecutor(max_workers=8) as pool:
            futures = [pool.submit(producer, index) for index in range(4)] + [pool.submit(reader, index) for index in range(4)]
            barrier.wait(timeout=20)
            for request_id in state["analytics_request_ids"]:
                try:
                    analytics.append(owner.call("POST", base + "/graph/analytics", {"scope": state["graph"], "algorithm": "wcc", "direction": "both"}, request_key=request_id))
                    private_json(e.root / "workload-analytics.json", analytics)
                except Exception as error:
                    failures.append({"caller": "analytics", "error": str(error)})
                    break
            for future in as_completed(futures):
                try:
                    results.append(future.result())
                except Exception as error:
                    failures.append({"caller": "capture_or_recall", "error": str(error)})
                private_json(e.root / "workload-results.json", results)
            if failures:
                private_json(e.root / "workload-failures.json", failures)
                raise AssertionError("Concurrent workload failed; inspect its preserved failure report")
        concurrent_seconds = time.perf_counter() - started
        e.wait(lambda: runtime.sql(e.saved, "SELECT count(*) FROM jobs WHERE state IN ('queued','running');"), lambda value: value == "0", "final owned workload drain", seconds=180)
        drain_seconds = time.perf_counter() - started - concurrent_seconds
        observed = [owner.call("POST", base + "/graph/analytics/" + report["id"] + "/view", {}) for report in analytics]
        assert all(not value["rows"] or value["report"]["state"] == "ready" for value in observed)
        fresh = owner.call("POST", base + "/graph/analytics", {"scope": state["graph"], "algorithm": "wcc", "direction": "both"})
        result = e.wait(lambda: owner.call("POST", base + "/graph/analytics/" + fresh["id"] + "/view", {}),
                        lambda value: value["report"]["state"] not in ("queued", "running"), "fresh post-load WCC")
        assert result["report"]["state"] == "ready" and result["report"]["node_count"] == 100
        assert len({row["group"] for row in result["rows"]}) == 1
    finally:
        stop.set()
        monitor.join(timeout=15)
        private_json(e.root / "resource-samples.json", {"samples": samples, "errors": sample_errors})
    for binding in state["bindings"]:
        offset, count = 0, 0
        while True:
            page = owner.call("GET", base + "/capture/events?binding_id=" + binding["binding"]["id"] + "&offset=" + str(offset))
            for entry in page["items"]:
                assert entry["selection"] == binding["selection"] and entry["operation_id"] == binding["operation"]["id"]
                count += 1
            offset += len(page["items"])
            if offset >= page["total"]:
                break
        assert count == 30
    captures = [item for result in results if result["kind"] == "producer" for item in result["records"]]
    reads = [value for result in results if result["kind"] == "reader" for value in result["latencies"]]
    assert len(captures) == 120 and len(reads) == 120
    assert owner.call("GET", base + "/models/usage")["total"] == 0
    assert runtime.sql(e.saved, "SELECT count(*) FROM jobs WHERE brain_id='" + brain["id"]
                       + "' AND state='failed';") == "0", "No failed workload job may hide behind an empty queue"
    final = runtime.inventory(e.saved)
    report = {"installation": e.saved["name"], "host": platform.platform(), "python": platform.python_version(),
        "host_logical_cpus": os.cpu_count(),
        "configured_limits": e.config, "documents": 200, "document_bytes": sum(s["version"]["byte_length"] for s in state["documents"]),
        "repositories": 50, "accounts": 2, "callers": 8, "capture_events": 120, "recall_requests": 120,
        "readability_probe_requests": sum(row["readability_probes"] for row in captures), "paid_provider_requests": 0,
        "recall_capacity_refusals": sum(result["capacity_refusals"] for result in results),
        "import_seconds": import_seconds, "concurrent_seconds": concurrent_seconds, "queue_drain_seconds": drain_seconds,
        "capture_admission": summary_latency([r["admission_ms"] for r in captures]), "recall": summary_latency(reads),
        "write_to_readable": summary_latency([r["readable_ms"] for r in captures]),
        "analytics_during_writes": [r["report"]["state"] for r in observed], "post_load_wcc_nodes": result["report"]["node_count"],
        "database_growth_bytes": final["database_bytes"] - baseline["database_bytes"],
        "sample_count": len(samples), "sample_interval": "one second after each bounded stats call",
        "scope_leaks": 0, "unexpected_request_failures": 0, "invalid_input_refused_and_positive_control": True}
    if samples:
        report["resources"] = resource_summary(samples)
    private_json(e.root / "workload-report.json", report)
    assert report["recall"]["p95_ms"] < 2000 and report["capture_admission"]["p95_ms"] < 2000
    assert report["write_to_readable"]["max_ms"] <= 60000 and report["queue_drain_seconds"] <= 180
    assert samples and not sample_errors
    state["stage"] = "complete"
    private_json(path, state)
    print(json.dumps(report, indent=2), flush=True)
