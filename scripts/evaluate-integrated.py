#!/usr/bin/env python3
"""Opt-in synthetic evaluation through one explicitly owned product installation."""
import argparse
import importlib.util
import json
import math
from pathlib import Path
import statistics
import time
import uuid

import install
import recovery_installation as runtime
from recovery_transport import private_json

spec = importlib.util.spec_from_file_location("installation_proof", Path(__file__).with_name("test-installation-runtime.py"))
proof = importlib.util.module_from_spec(spec)
spec.loader.exec_module(proof)
EMPTY = {"repository_ids": [], "area_ids": [], "environment_id": None}
LANES = {"baseline": ["exact", "lexical"], "graph": ["exact", "lexical", "graph"],
         "semantic": ["exact", "lexical", "semantic"], "full": ["exact", "lexical", "semantic", "graph"]}


def identity():
    return str(uuid.uuid4())


def latency(values):
    values = sorted(values)
    assert values
    return {"count": len(values), "p50_ms": round(statistics.median(values), 3),
            **{name: round(values[max(0, math.ceil(len(values) * fraction) - 1)], 3)
               for name, fraction in (("p95_ms", .95), ("p99_ms", .99), ("max_ms", 1))}}


def claim(version, subject, predicate, value):
    return {"content": {"kind": "claim", "subject": subject, "predicate": predicate,
        "value": value, "rationale": "Declared synthetic evaluation evidence.", "selection": EMPTY,
        "manifest_revision_id": None, "validity": {"kind": "unknown", "from": None, "to": None, "precision": "unknown"},
        "freshness": "current", "operational": "declared", "observed_at": None, "observation": "",
        "supports": [{"kind": "source_version", "id": version, "line_from": None, "line_to": None}]}}


class Evaluation:
    def __init__(self, name, directory):
        assert name.startswith("proof-integrated-"), "Select an owned integrated proof installation"
        self.saved = install.load(name)
        self.config = install.configuration(self.saved)
        self.root = Path(directory).resolve()
        assert self.root.parent == install.ROOT / ".cache" and not self.root.is_symlink()
        self.root.mkdir(mode=0o700, exist_ok=True)
        self.path = self.root / "quality-state.json"
        self.state = json.loads(self.path.read_text()) if self.path.exists() else {
            "run_id": identity(), "installation": name, "stage": "new", "documents": {}, "claims": {}, "attempts": [], "answers": []}
        assert self.state["installation"] == name
        self.inputs = proof.environment(install.directory(name) / "runtime.env")
        assert "VAULT_TOKEN" not in self.inputs
        self.owner = self.session()
        self.corpus = json.loads((install.ROOT / "crates/server/tests/fixtures/semantic-corpus.json").read_text())
        self.questions = json.loads((install.ROOT / "crates/server/tests/fixtures/fusion-corpus.json").read_text())

    def session(self, account=None):
        account = account or {"username": self.inputs["RECOLLECT_OWNER_USERNAME"], "password": self.inputs["RECOLLECT_OWNER_PASSWORD"]}
        session = proof.Session(self.saved["origin"])
        session.login(**account)
        return session

    def save(self):
        private_json(self.path, self.state)

    def wait(self, read, ready, label, seconds=180):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = read()
            if ready(value):
                return value
            time.sleep(.25)
        raise AssertionError("Timed out: " + label)

    def api(self, path, method="GET", data=None):
        return self.owner.call(method, self.state["base"] + path, data)

    def policy(self, **changes):
        current = self.api("/models/policy")["current"]
        return self.api("/models/policy", "PUT", {"base_change": current["change_id"], "policy": {**current["policy"], **changes}})

    def prepare(self):
        assert self.state["stage"] == "new", "An existing quality corpus is preserved"
        assert self.inputs.get("OPENAI_API_KEY"), "This opt-in quality run requires its explicitly configured provider"
        self.state.update(stage="preparing", brain=self.owner.call("POST", "/api/brains", {"name": "Integrated fixed quality corpus"}))
        self.state["base"] = "/api/brains/" + self.state["brain"]["id"]
        self.save()
        for doc in self.corpus["documents"]:
            source = self.api("/sources", "POST", {"title": doc["title"], "content": doc["content"], "media_type": "text/plain", "retain_content": True})
            self.state["documents"][doc["key"]] = {"id": source["id"], "version": source["version"]["id"]}
            self.save()
        for row in self.questions["claims"]:
            result = self.api("/claims", "POST", claim(self.state["documents"][row["source"]]["version"], row["subject"], row["predicate"], row["value"]))
            self.state["claims"][row["key"]] = {"id": result["claim_id"], "revision": result["id"], "source": row["source"]}
            self.save()
        for doc in self.state["documents"].values():
            self.wait(lambda: self.api(f"/sources/{doc['id']}/versions/{doc['version']}"),
                      lambda value: value["version"]["processing"] == "ready", "source materialization")
        graph = self.api("/graph/rebuild", "POST", {"kind": "knowledge"})
        self.wait(lambda: self.api("/graph"), lambda value: any(g["id"] == graph["id"] and g["state"] == "ready" for g in value["generations"]), "quality graph")
        self.state["stage"] = "embedding_requested"
        self.save()
        self.embed()

    def embed(self):
        assert self.state["stage"] in ("embedding_requested", "embedding_admitted")
        current = self.api("/models/policy")["current"]["policy"]
        if not current["enabled"]:
            assert self.api("/models/usage")["total"] == 0
            self.policy(enabled=True, automatic_learning=False, autonomous_memory=False, automatic_embedding=True,
                        purposes=["embedding", "synthesis"], content_classes=["document", "claim", "query"],
                        daily_token_limit=500000, max_input_bytes=16384, max_output_tokens=1024, max_concurrent=1)
        else:
            assert current["automatic_embedding"]
        self.state["stage"] = "embedding_admitted"
        self.save()
        semantic = self.wait(lambda: self.api("/semantic"), lambda value: value["counts"]["ready"] == 17, "17 frozen semantic records")
        assert semantic["profile"]["model"] == "text-embedding-3-large" and semantic["profile"]["dimensions"] == 3072
        self.state.update(stage="prepared", profile=semantic["profile"], indexing_usage=self.api("/models/usage"))
        self.save()
        print("Quality corpus prepared through product APIs.", flush=True)

    def quality(self):
        assert self.state["stage"] in ("prepared", "recall_running", "recall_complete")
        self.state["stage"] = "recall_running"
        self.save()
        for question in self.questions["questions"]:
            for lane, channels in LANES.items():
                previous = next((a for a in self.state["attempts"] if a["question"] == question["key"] and a["lane"] == lane), None)
                if previous and previous["complete"]:
                    continue
                if previous:
                    assert "result" in previous, "An unresolved paid-capable attempt is preserved; inspect canonical request state"
                    attempt = previous
                    result, elapsed = attempt["result"], attempt["elapsed_ms"]
                else:
                    request_id = identity() if "semantic" in channels else None
                    attempt = {"question": question["key"], "lane": lane, "request_id": request_id, "complete": False}
                    self.state["attempts"].append(attempt)
                    self.save()
                    started = time.perf_counter()
                    result = self.api("/recall", "POST", {"query": question["query"], "channels": channels,
                        "semantic_request_id": request_id, "limit": 4, "context_bytes": 8192, "source_diversity": True})
                    elapsed = (time.perf_counter() - started) * 1000
                    attempt.update(result=result, elapsed_ms=elapsed)
                    self.save()
                assert result["context_bytes"] <= 8192 and len(result["context"]["items"]) <= 4
                relevant = {self.state["documents"][key]["version"] for key in question["relevant_sources"]}
                relevant.update(c["id"] for c in self.state["claims"].values() if c["source"] in question["relevant_sources"])
                target = self.state["claims"][question["target"]]["id"] if question["target"] else None
                items = result["context"]["items"]
                for item in items:
                    if item["kind"] == "claim":
                        expected = next(c for c in self.state["claims"].values() if c["id"] == item["id"])
                        assert item["revision_id"] == expected["revision"]
                        expected_version = self.state["documents"][expected["source"]]["version"]
                        assert any(p["id"] == expected_version for p in item["provenance"])
                    else:
                        assert item["id"] in {d["version"] for d in self.state["documents"].values()}
                attempt.update(complete=True, elapsed_ms=elapsed, result=result, hit=target is not None and any(i["id"] == target for i in items),
                    irrelevant=sum(i["id"] not in relevant for i in items), supported=target is not None)
                self.save()
        self.policy(automatic_embedding=False)
        self.state.update(stage="recall_complete", retrieval_usage=self.api("/models/usage"))
        self.save()
        summary = {}
        for lane in LANES:
            rows = [a for a in self.state["attempts"] if a["lane"] == lane]
            summary[lane] = {"target_hits": sum(a["hit"] for a in rows), "supported_questions": 6,
                "unsupported_with_context": sum(bool(a["result"]["context"]["items"]) for a in rows if not a["supported"]),
                "irrelevant_records": sum(a["irrelevant"] for a in rows), "latency": latency([a["elapsed_ms"] for a in rows]),
                "context_bytes": [a["result"]["context_bytes"] for a in rows]}
        private_json(self.root / "retrieval-report.json", summary)
        assert summary["full"]["target_hits"] == 6
        print(json.dumps({"retrieval": summary}), flush=True)

    def handover(self, run):
        for offset in range(0, 100, 20):
            page = self.api("/handovers?offset=" + str(offset))["items"]
            match = next((value for value in page if value["id"] == run), None)
            if match:
                return match
            if len(page) < 20:
                break
        raise AssertionError("Persisted handover run is not visible")

    def synthesize(self):
        assert self.state["stage"] in ("recall_complete", "synthesis_running", "synthesis_complete")
        self.state["stage"] = "synthesis_running"
        self.save()
        for recall in self.state["attempts"]:
            previous = next((a for a in self.state["answers"] if (a["question"], a["lane"]) == (recall["question"], recall["lane"])), None)
            if previous and previous["complete"]:
                continue
            question = next(q for q in self.questions["questions"] if q["key"] == recall["question"])
            contributions = [i["revision_id"] for i in recall["result"]["context"]["items"] if i["kind"] == "claim"]
            if previous:
                assert previous.get("run"), "Unresolved synthesis admission is preserved; never resubmit blindly"
                answer = previous
            else:
                answer = {"question": question["key"], "lane": recall["lane"], "contributions": contributions, "complete": False}
                self.state["answers"].append(answer)
                self.save()
                if not contributions:
                    answer.update(complete=True, outcome="no_claim_context", answer=None)
                    self.save()
                    continue
                answer["started_at"] = time.time()
                self.save()
                answer["run"] = self.api("/handovers", "POST", {"title": question["query"], "contributions": contributions})["id"]
                self.save()
            run = self.wait(lambda: self.handover(answer["run"]), lambda value: value["state"] not in ("queued", "running"), "governed synthesis")
            answer.update(outcome=run["state"], error_code=run["error_code"], elapsed_ms=(time.time() - answer["started_at"]) * 1000, model_request_id=run["request_id"])
            if run["state"] == "succeeded":
                result = self.api("/claims/" + run["claim_id"])["selected"]["revision"]
                assert sorted(result["content"]["handover"]["contributions"]) == sorted(contributions)
                answer["answer"] = result["content"]
                answer["revision_id"] = result["id"]
            answer["complete"] = True
            self.save()
            assert run["state"] == "succeeded", "A model attempt failed; preserve it without automatic retry"
            print("Synthesis completed:", answer["question"], answer["lane"], flush=True)
        self.state.update(stage="synthesis_complete", final_usage=self.api("/models/usage"))
        self.save()
        private_json(self.root / "synthetic-answers.json", {"questions": self.questions["questions"], "facts": self.questions["claims"], "answers": self.state["answers"]})
        print("Generated synthetic outputs ready for factual evaluation.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("name")
    parser.add_argument("directory")
    parser.add_argument("stage", choices=("prepare", "embed", "quality", "synthesize", "workload"))
    args = parser.parse_args()
    evaluation = Evaluation(args.name, args.directory)
    if args.stage == "workload":
        from evaluation_workload import run
        run(evaluation)
    else:
        getattr(evaluation, args.stage)()


if __name__ == "__main__":
    main()
