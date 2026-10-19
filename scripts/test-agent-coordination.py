#!/usr/bin/env python3
"""Zero-model-call macOS/Ubuntu coordination proof against the real Rust server.

Every database, process and container is a UUID-owned disposable fixture. Never
loads .env or targets an existing Brain. Private credentials remain in .cache.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import http.cookiejar
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import time
import urllib.error
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / ".cache/agent-coordination"
OWNER = "agent-coordination-proof"


def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def process_env():
    return {key: value for key, value in os.environ.items() if key in (
        "PATH", "HOME", "USER", "TMPDIR", "DOCKER_HOST", "DOCKER_CONTEXT",
        "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH", "RUSTUP_HOME", "CARGO_HOME")}


def docker(*args, stdin=None, timeout=60):
    result = subprocess.run([str(ROOT / "scripts/docker.sh"), *args], cwd=ROOT,
                            env=process_env(), input=stdin, text=True,
                            capture_output=True, timeout=timeout)
    assert result.returncode == 0, "Owned Docker operation failed: " + args[0]
    return result.stdout.strip()


def eventually(check, label, seconds=30):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        try:
            result = check()
            if result:
                return result
        except (OSError, urllib.error.URLError, AssertionError):
            pass
        time.sleep(0.2)
    raise AssertionError("Timed out: " + label)


class Api:
    def __init__(self, origin, token=None):
        self.origin, self.token, self.csrf = origin, token, None
        self.opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}),
            urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))

    def call(self, method, path, body=None, expected=200, headers=None):
        hdr = {"Content-Type": "application/json"}
        if self.token:
            hdr["Authorization"] = "Bearer " + self.token
        if self.csrf:
            hdr["X-CSRF-Token"] = self.csrf
        hdr.update(headers or {})
        request = urllib.request.Request(self.origin + path, method=method,
            headers=hdr, data=json.dumps(body).encode() if body is not None else None)
        try:
            response = self.opener.open(request, timeout=15)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            payload = json.loads(response.read() or "null")
            assert response.status == expected, f"{method} {path}: {response.status}, expected {expected}"
            return payload

    def login(self, username, password):
        self.csrf = self.call("POST", "/api/auth/login",
                             {"username": username, "password": password})["csrf_token"]


class Fixture:
    def __init__(self, path=None):
        if path:
            self.path = Path(path).resolve()
            assert self.path.parent == CACHE and self.path.name.startswith("proof-")
            self.state = json.loads((self.path / "resources.json").read_text())
            self.env = dict(line.split("=", 1) for line in
                            (self.path / "runtime.env").read_text().splitlines())
        else:
            self.path = CACHE / ("proof-" + uuid.uuid4().hex[:12])
            self.path.mkdir(mode=0o700, parents=True)
            self.state = {"database": self.path.name + "-db", "peer": self.path.name + "-ubuntu",
                          "graph": self.path.name + "-graph",
                          "owner": OWNER, "checks": {}, "passed": False, "model_calls": None,
                          "fixture_sha256": sha256(Path(__file__))}
            self.env = {}
        self.server = None

    def save(self):
        (self.path / "resources.json").write_text(json.dumps(self.state, indent=2) + "\n")

    def checked(self, name):
        self.state["checks"][name] = True
        self.save()
        print(name + ": PASS", flush=True)

    def prepare(self):
        private = {key: uuid.uuid4().hex for key in
                   ("POSTGRES_PASSWORD", "RECOLLECT_DB_PASSWORD", "RECOLLECT_OWNER_PASSWORD",
                    "NEO4J_PASSWORD", "RECOLLECT_FIXTURE_KEYRING_PASSWORD")}
        private.update(POSTGRES_USER="recollect_admin", POSTGRES_DB="recollect")
        envfile = self.path / "postgres.env"
        with os.fdopen(os.open(envfile, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as out:
            out.write("".join(key + "=" + value + "\n" for key, value in private.items()))
        self.save()
        docker("run", "-d", "--name", self.state["database"],
               "--label", "io.recollect.owner=" + OWNER, "--env-file", str(envfile),
               "--shm-size=256m", "-p", "127.0.0.1::5432", "-v",
               str(ROOT / "infra/postgres-init.sh") + ":/docker-entrypoint-initdb.d/recollect.sh:ro",
               "pgvector/pgvector:pg17@sha256:ac08538c6f8b9904c33c8224c5e5706dbe760aca29db1d096972b4052c22a75d")
        port = docker("port", self.state["database"], "5432/tcp").split(":")[-1]
        eventually(lambda: "accepting connections" in docker("exec", self.state["database"],
                    "pg_isready", "-U", "recollect_admin", "-d", "recollect"), "fixture database")
        graph_env = self.path / "graph.env"
        with os.fdopen(os.open(graph_env, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as out:
            out.write("NEO4J_AUTH=neo4j/" + private["NEO4J_PASSWORD"] + "\n")
        docker("run", "-d", "--name", self.state["graph"], "--label", "io.recollect.owner=" + OWNER,
               "--env-file", str(graph_env), "--memory", "2g", "--cpus", "2", "-p", "127.0.0.1::7474",
               "-e", 'NEO4J_PLUGINS=["graph-data-science"]',
               "-e", "NEO4J_dbms_security_procedures_unrestricted=gds.*",
               "-e", "NEO4J_server_memory_heap_initial__size=256m",
               "-e", "NEO4J_server_memory_heap_max__size=512m",
               "-e", "NEO4J_server_memory_pagecache_size=256m", "neo4j:2026.08@sha256:d8f4c156caa3af76499134947deb11d13042e471d9733060449f2a01eb7a248e")
        graph_port = docker("port", self.state["graph"], "7474/tcp").split(":")[-1]
        graph_origin = "http://127.0.0.1:" + graph_port
        eventually(lambda: Api(graph_origin).call("GET", "/"), "owned graph readiness", seconds=180)
        self.env = {**private, "RECOLLECT_OWNER_USERNAME": "mike-fixture",
            "DATABASE_URL": "postgres://recollect_app:" + private["RECOLLECT_DB_PASSWORD"] + "@127.0.0.1:" + port + "/recollect",
            "DATABASE_ADMIN_URL": "postgres://recollect_admin:" + private["POSTGRES_PASSWORD"] + "@127.0.0.1:" + port + "/recollect",
            "RECOLLECT_ARTIFACT_DIR": str(self.path / "artifacts"),
            "RECOLLECT_ERASURE_JOURNAL": str(self.path / "erasure"),
            "RECOLLECT_CREDENTIAL_FILE": str(self.path / "credentials.json"),
            "RECOLLECT_TEST_STATE_DIR": str(self.path),
            "NEO4J_URL": graph_origin, "RUST_LOG": "error"}
        with os.fdopen(os.open(self.path / "runtime.env", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as out:
            out.write("".join(key + "=" + value + "\n" for key, value in self.env.items()))
        with os.fdopen(os.open(self.path / "peer.env", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as out:
            out.write("RECOLLECT_FIXTURE_KEYRING_PASSWORD=" + private["RECOLLECT_FIXTURE_KEYRING_PASSWORD"] + "\n")
        self.save()
        return self.path

    def start_server(self):
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = self.state.get("port", listener.getsockname()[1])
        self.state["port"] = port
        self.origin = "http://127.0.0.1:" + str(port)
        env = {**process_env(), **self.env, "RECOLLECT_BIND": "0.0.0.0:" + str(port),
               "RECOLLECT_PUBLIC_ORIGIN": self.origin}
        binary = ROOT / "target/debug/recollect-server"
        self.state.setdefault("binary_sha256", {})["macos_server"] = sha256(binary)
        with (self.path / "migration.log").open("w") as log:
            result = subprocess.run([str(binary), "migrate"], cwd=ROOT, env=env,
                                    stdout=log, stderr=log, timeout=120)
        assert result.returncode == 0, "Fixture migration failed; private migration log retained"
        log = (self.path / "server.log").open("a")
        self.server = subprocess.Popen([str(binary), "serve"], cwd=ROOT, env=env, stdout=log, stderr=log)
        log.close()
        eventually(lambda: Api(self.origin).call("GET", "/health/live"), "fixture Rust server")
        self.save()

    def start_peer(self):
        docker("build", "-t", "recollect-agent-coordination-peer:local",
               str(ROOT / "infra/agent-coordination"), timeout=300)
        docker("run", "-d", "--init", "--name", self.state["peer"],
               "--label", "io.recollect.owner=" + OWNER,
               "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true",
               "--env-file", str(self.path / "peer.env"),
               "-e", "RECOLLECT_FIXTURE_PORT=" + str(self.state["port"]),
               "recollect-agent-coordination-peer:local")
        eventually(lambda: docker("exec", self.state["peer"], "test", "-f", "/run/recollect-fixture-bus") == "",
                   "Ubuntu secret service")
        assert "Ubuntu 24.04" in docker("exec", self.state["peer"], "cat", "/etc/os-release")

    def peer(self, token, method, path, body=None, expected=200, headers=None):
        data = {"token": token, "method": method, "path": path, "body": body, "headers": headers or {}}
        result = json.loads(docker("exec", "-i", self.state["peer"], "python3",
                                  "/opt/recollect-fixture/peer.py", stdin=json.dumps(data)))
        assert result["status"] == expected, f"Ubuntu {method} {path}: {result['status']}, expected {expected}"
        return result["body"]

    def stream_peer(self, token, path, after=None):
        command = [str(ROOT / "scripts/docker.sh"), "exec", "-i", self.state["peer"],
                   "python3", "/opt/recollect-fixture/peer.py"]
        child = subprocess.Popen(command, cwd=ROOT, env=process_env(), text=True,
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        data = {"token": token, "path": path, "stream": True}
        if after is not None:
            data["headers"] = {"Last-Event-ID": str(after)}
        child.stdin.write(json.dumps(data))
        child.stdin.close()
        child.stdin = None
        return child

    def sql(self, statement):
        # This database/container was created by prepare() for this run only.
        assert self.state["owner"] == OWNER
        return docker("exec", "-i", self.state["database"], "psql", "-X", "-qAt",
                      "-U", "recollect_admin", "-d", "recollect", "-v", "ON_ERROR_STOP=1",
                      stdin=statement)

    def stop_server(self):
        if self.server and self.server.poll() is None:
            self.server.terminate()
            try:
                self.server.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.server.kill()
                self.server.wait(timeout=5)
        self.server = None

    def cleanup(self):
        self.stop_server()
        for key in ("peer", "graph", "database"):
            name = self.state.get(key)
            if not name:
                continue
            names = docker("ps", "-a", "--filter", "name=^/" + name + "$", "--format", "{{.Names}}")
            if names:
                owner = docker("inspect", "--format", '{{index .Config.Labels "io.recollect.owner"}}', name)
                assert owner == OWNER, "Fixture ownership mismatch"
                docker("rm", "-fv", name)
        self.state["cleaned_up"] = True
        self.save()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--fixture", type=Path)
    parser.add_argument("--cleanup", action="store_true")
    args = parser.parse_args()
    fixture = Fixture(args.fixture)
    if args.cleanup:
        assert args.fixture
        fixture.cleanup()
        return
    if args.prepare:
        assert not args.fixture
        print(fixture.prepare())
        return
    try:
        if not args.fixture:
            fixture.prepare()
        fixture.start_server()
        fixture.start_peer()
        run_proof(fixture)
    finally:
        fixture.cleanup()


def run_proof(fixture):
    owner = Api(fixture.origin)
    owner.login(fixture.env["RECOLLECT_OWNER_USERNAME"], fixture.env["RECOLLECT_OWNER_PASSWORD"])
    brain = owner.call("POST", "/api/brains", {"name": "team-fixture"})
    foreign = owner.call("POST", "/api/brains", {"name": "private-fixture"})
    base = "/api/brains/" + brain["id"]
    coord = base + "/coordination"
    invitation = owner.call("POST", "/api/team/invitations", {"username": "paul-fixture"})
    password = uuid.uuid4().hex
    paul_account = Api(fixture.origin)
    paul_account.call("POST", "/api/auth/enroll", {"token": invitation["token"], "password": password})
    paul_account.login("paul-fixture", password)
    paul_id = paul_account.call("GET", "/api/auth/me")["user"]["id"]
    owner.call("PUT", base + "/grants/" + paul_id, {"role": "writer"})

    def pair(account, label, host):
        anonymous = Api(fixture.origin)
        started = anonymous.call("POST", "/api/devices/pairings", {
            "name": label, "host_kind": host, "integration": "plugin"})
        account.call("POST", "/api/devices/pairings/" + started["user_code"] + "/approve", {"approve": True})
        paired = anonymous.call("POST", "/api/devices/pairings/poll", {"device_code": started["device_code"]})
        assert paired["state"] == "claimed" or paired["state"] == "approved"
        anonymous.call("POST", "/api/devices/pairings/finish", {"device_code": started["device_code"]}, expected=204)
        return paired["device"]["id"], paired["token"]

    mike_device, mike_token = pair(owner, "mike-mac-fixture", "codex")
    paul_device, paul_token = pair(paul_account, "paul-ubuntu-fixture", "claude_code")
    mike = Api(fixture.origin, mike_token)

    def paul(method, path, body=None, expected=200, headers=None):
        return fixture.peer(paul_token, method, path, body, expected, headers)

    def register(call, directory, origin):
        call("POST", base + "/workspace/checkouts", {"workspace_root": directory,
            "complete": True, "notes": [], "checkouts": [{"local_path": directory + "/infra",
            "origin": origin, "branch": "main", "head": "a" * 40, "dirty": False,
            "status": "available"}]})

    register(mike.call, "/fixture/mike/team", "https://git.example.invalid/team/infra.git")
    register(paul, "/fixture/paul/team", "git@git.example.invalid:team/infra.git")
    repos = mike.call("GET", base + "/workspace/repositories")["items"]
    assert len(repos) == 1, "Equivalent origins must select one repository"
    repository = repos[0]["id"]
    environment = owner.call("POST", base + "/evidence/groups", {
        "kind": "environment", "name": "production-fixture", "description": "Synthetic only"})["id"]

    def task(call, label, repositories=None, env=environment):
        value = call("POST", base + "/workspace/tasks", {"label": label,
            "parent_task_id": None, "workspace_id": None,
            "selection": {"repository_ids": repositories or [repository], "area_ids": [], "environment_id": env}})
        return value["task"]

    mike_task = task(mike.call, "Mike plans synthetic Vault change")
    paul_task = task(paul, "Paul plans synthetic Vault change")

    def announcement(work, host, label, goal, topics, paths):
        return {"id": str(uuid.uuid4()), "task_id": work["id"], "expected_scope": work["scope"]["id"],
            "expected_revision": None, "host_kind": host, "label": label, "goal": goal,
            "topics": topics, "intents": [{"repository_id": repository, "path": path,
            "mode": "write", "phase": "planned"} for path in paths]}

    mine = announcement(mike_task, "codex", "Mike fixture", "Configure Vault certificate renewal",
                        ["vault-pki"], ["clusters/production/vault.yaml"])
    theirs = announcement(paul_task, "claude_code", "Paul fixture", "Check unrelated ingress routing",
                          ["ingress-routing"], [])
    first = mike.call("POST", coord + "/sessions", mine)
    peer = paul("POST", coord + "/sessions", theirs)
    assert first["session"]["account_name"] == "mike-fixture"
    assert peer["session"]["account_name"] == "paul-fixture"
    assert not peer["overlaps"]
    mike_path = coord + "/sessions/" + mine["id"]
    paul_path = coord + "/sessions/" + theirs["id"]
    assert mike.call("POST", coord + "/sessions", mine)["session"]["revision"] == first["session"]["revision"]
    fixture.checked("two_accounts_mac_and_ubuntu_canonical_repository")

    # Establish the Ubuntu stream before creating the matching work declaration.
    stream = fixture.stream_peer(paul_token, paul_path + "/events")
    theirs["expected_revision"] = peer["session"]["revision"]
    theirs["goal"] = "Configure Vault certificate renewal"
    theirs["topics"] = ["vault-pki"]
    theirs["intents"] = mine["intents"]
    peer = paul("POST", coord + "/sessions", theirs)
    try:
        output, _ = stream.communicate(timeout=25)
        pushed = json.loads(output)["event"]
    finally:
        if stream.poll() is None:
            stream.kill()
            stream.wait(timeout=5)
    inbox = paul("GET", paul_path + "/inbox")
    assert pushed["data"]["id"] in [event["id"] for event in inbox["events"]]
    assert any(overlap["reason"] == "file_write_overlap" for overlap in peer["overlaps"])
    mike_inbox = mike.call("GET", mike_path + "/inbox")
    assert any(event["kind"] == "overlap" for event in mike_inbox["events"])
    fixture.checked("planned_file_overlap_push_and_pull_both_peers")

    seen_ids = [event["id"] for event in inbox["events"]]
    for _ in range(2):
        paul("POST", paul_path + "/heartbeat", {"expected_revision": peer["session"]["revision"]})
    assert [event["id"] for event in paul("GET", paul_path + "/inbox")["events"]] == seen_ids
    event = inbox["events"][0]
    for _ in range(2):
        paul("POST", paul_path + "/inbox/" + event["id"] + "/ack", {})
    assert [event["id"] for event in paul("GET", paul_path + "/inbox")["events"]] == seen_ids
    public = paul("GET", coord + "/sessions")["sessions"]
    mike_peer = next(value for value in public if value["id"] == mine["id"])
    assert "task_id" not in mike_peer and "scope_id" not in mike_peer
    assert "/fixture/mike" not in json.dumps(public)
    paul("GET", base + "/workspace/tasks/" + mike_task["id"], expected=404)
    paul("GET", mike_path + "/inbox", expected=404)
    paul("POST", mike_path + "/heartbeat", {"expected_revision": 1}, expected=404)
    paul("GET", "/api/brains/" + foreign["id"] + "/coordination/sessions", expected=404)
    fixture.checked("heartbeat_dedup_ack_privacy_and_cross_brain_denial")

    def message(sender, recipient, kind, text, reply_to=None):
        return {"id": str(uuid.uuid4()), "from_session_id": sender, "to_session_id": recipient,
                "kind": kind, "text": text, "reply_to": reply_to}

    proposal = message(mine["id"], theirs["id"], "proposal",
                       "I will update renewal settings; please validate the certificate policy.")
    receipt = mike.call("POST", coord + "/messages", proposal)
    assert receipt["sender"]["account_name"] == "mike-fixture"
    assert mike.call("POST", coord + "/messages", proposal)["id"] == receipt["id"]
    changed = {**proposal, "text": "Conflicting replay"}
    mike.call("POST", coord + "/messages", changed, expected=409)
    acceptance = message(theirs["id"], mine["id"], "accept",
                         "Agreed. I will validate policy and leave renewal settings to Mike.", proposal["id"])
    accepted = paul("POST", coord + "/messages", acceptance)
    assert accepted["sender"]["account_name"] == "paul-fixture"
    assert any(event["payload"]["data"].get("id") == accepted["id"]
               for event in mike.call("GET", mike_path + "/inbox")["events"] if event["kind"] == "message")
    forged = message(mine["id"], theirs["id"], "note", "Impersonation attempt")
    paul("POST", coord + "/messages", forged, expected=404)
    fixture.checked("attributed_proposal_acceptance_and_message_idempotency")

    card = mike.call("GET", coord + "/a2a/.well-known/agent-card.json")
    interface = card["supportedInterfaces"][0]
    assert interface["protocolVersion"] == "1.0" and interface["protocolBinding"] == "HTTP+JSON"
    assert card["capabilities"]["streaming"] is False
    a2a_message = message(theirs["id"], mine["id"], "note", "A2A confirms the synthetic work split.")
    envelope = {"message": {"messageId": a2a_message["id"], "role": "ROLE_USER",
        "parts": [{"data": {"schema": "recollect.coordination.v1", "action": "send", "message": a2a_message},
                   "mediaType": "application/json"}]}}
    a2a = paul("POST", coord + "/a2a/message:send", envelope,
               headers={"A2A-Version": "1.0", "Content-Type": "application/a2a+json"})
    assert a2a["message"]["role"] == "ROLE_AGENT"
    assert a2a["message"]["parts"][0]["data"]["receipt"]["id"] == a2a_message["id"]
    unsupported = paul("POST", coord + "/a2a/message:send",
        {**envelope, "configuration": {"acceptedOutputModes": ["text/plain"]}}, expected=400,
        headers={"A2A-Version": "1.0", "Content-Type": "application/a2a+json"})
    assert any(detail.get("reason") == "CONTENT_TYPE_NOT_SUPPORTED"
               for detail in unsupported["error"]["details"])
    fixture.checked("official_a2a_1_0_message_wire_between_peers")

    mcp_headers = {"Accept": "application/json, text/event-stream"}
    initialized = mike.call("POST", base + "/mcp/agent", {"jsonrpc": "2.0", "id": 1,
        "method": "initialize", "params": {"protocolVersion": "2025-11-25", "capabilities": {},
        "clientInfo": {"name": "recollect-coordination-fixture", "version": "1"}}}, headers=mcp_headers)
    mcp_headers["MCP-Protocol-Version"] = initialized["result"]["protocolVersion"]
    listed = mike.call("POST", base + "/mcp/agent", {"jsonrpc": "2.0", "id": 2,
        "method": "tools/list", "params": {}}, headers=mcp_headers)
    assert {"coordination.announce", "coordination.peers", "coordination.inbox", "coordination.send"}.issubset(
        {tool["name"] for tool in listed["result"]["tools"]})
    called = mike.call("POST", base + "/mcp/agent", {"jsonrpc": "2.0", "id": 3,
        "method": "tools/call", "params": {"name": "coordination.peers", "arguments": {}}}, headers=mcp_headers)
    assert not called["result"].get("isError")
    assert "paul-fixture" in json.dumps(called["result"])
    mcp_note = message(mine["id"], theirs["id"], "note", "MCP typed peer message fixture")
    called = mike.call("POST", base + "/mcp/agent", {"jsonrpc": "2.0", "id": 4,
        "method": "tools/call", "params": {"name": "coordination.send", "arguments": {"input": mcp_note}}}, headers=mcp_headers)
    assert not called["result"].get("isError") and mcp_note["id"] in json.dumps(called["result"])
    fixture.checked("real_mcp_discovery_peer_read_and_typed_message")

    conflicting = {**mine, "expected_revision": 9, "goal": "Stale update must not replace current work"}
    mike.call("POST", coord + "/sessions", conflicting, expected=409)
    # Concurrent transport retries share one stable message identity and one event.
    concurrent = message(mine["id"], theirs["id"], "note", "Concurrent retry fixture")
    with ThreadPoolExecutor(max_workers=2) as pool:
        results = list(pool.map(lambda _: Api(fixture.origin, mike_token).call(
            "POST", coord + "/messages", concurrent), range(2)))
    assert results[0]["id"] == results[1]["id"]
    assert sum(event["kind"] == "message" and event["payload"]["data"]["id"] == concurrent["id"]
               for event in paul("GET", paul_path + "/inbox")["events"]) == 1
    fixture.checked("optimistic_revision_and_concurrent_message_replay")

    lease_task = task(paul, "Lease expiry fixture")
    lease = announcement(lease_task, "other", "Lease fixture", "Unrelated metrics", [], [])
    lease_receipt = paul("POST", coord + "/sessions", lease)
    lease_id = str(uuid.UUID(lease["id"]))
    lease_path = coord + "/sessions/" + lease_id
    fixture.sql("UPDATE coordination_sessions SET expires_at=now()-interval '1 second' WHERE id='" + lease_id + "';")
    assert lease_id not in [value["id"] for value in mike.call("GET", coord + "/sessions")["sessions"]]
    paul("POST", lease_path + "/heartbeat", {"expected_revision": lease_receipt["session"]["revision"]}, expected=409)
    assert paul("GET", lease_path + "/inbox")["events"] == []
    lease["expected_revision"] = lease_receipt["session"]["revision"]
    paul("POST", coord + "/sessions", lease)
    expiring = message(mine["id"], lease_id, "note", "Retention expiry fixture")
    mike.call("POST", coord + "/messages", expiring)
    fixture.sql("UPDATE coordination_events SET expires_at=now()-interval '1 second' WHERE session_id='" + lease_id + "';")
    gap = paul("GET", lease_path + "/inbox")
    assert gap["gap"] and gap["next_cursor"] > 0 and gap["peers"]
    paul("POST", lease_path + "/close", {})
    assert paul("GET", lease_path + "/inbox")["gap"]
    fixture.checked("server_timed_expiry_reannounce_retained_pull_and_cursor_gap")

    before = paul("GET", paul_path + "/inbox")
    fixture.stop_server()
    fixture.start_server()
    recovered = paul("GET", paul_path + "/inbox")
    assert [event["id"] for event in recovered["events"]] == [event["id"] for event in before["events"]]
    after = before["next_cursor"]
    note = message(mine["id"], theirs["id"], "note", "Persistent inbox survived the server restart.")
    mike.call("POST", coord + "/messages", note)
    replay = fixture.stream_peer(paul_token, paul_path + "/events", after)
    try:
        output, _ = replay.communicate(timeout=25)
        result = json.loads(output)["event"]["data"]
        assert result["payload"]["data"]["id"] == note["id"]
    finally:
        if replay.poll() is None:
            replay.kill()
            replay.wait(timeout=5)
    fixture.checked("server_restart_durable_inbox_and_sse_cursor_reconnect")

    # Native Ubuntu plugin uses its normal OS-store connection and the same token.
    # The binary is supplied by the separately built Linux fixture.
    linux_binary = ROOT / ".cache/agent-coordination-linux/recollect-plugin"
    if linux_binary.exists():
        native_proof(fixture, linux_binary, brain["id"], paul_token, mine, mike, coord)
    else:
        raise AssertionError("Build the Ubuntu-compatible recollect-plugin fixture binary before proof")

    paul("POST", paul_path + "/close", {})
    active = mike.call("GET", coord + "/sessions")["sessions"]
    assert theirs["id"] not in [value["id"] for value in active]
    paul("POST", paul_path + "/heartbeat", {"expected_revision": peer["session"]["revision"]}, expected=409)
    owner.call("DELETE", base + "/grants/" + paul_id)
    paul("GET", coord + "/sessions", expected=404)
    paul_account.call("DELETE", "/api/devices/" + paul_device, expected=204)
    paul("GET", "/api/auth/me", expected=401)
    fixture.checked("close_membership_revocation_and_device_revocation")
    fixture.state["brain_name"] = "team-fixture"
    fixture.state["peer_os"] = "Ubuntu 24.04"
    fixture.state["server_os"] = "macOS"
    fixture.state["model_calls"] = int(fixture.sql("SELECT count(*) FROM model_requests;"))
    assert fixture.state["model_calls"] == 0
    fixture.checked("zero_model_requests_in_canonical_database")
    fixture.state["passed"] = True
    fixture.state["completed_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    fixture.save()
    print("Proof: " + str(fixture.path / "resources.json"), flush=True)


def native_proof(fixture, binary, brain, token, mine, mike, coord):
    fixture.state.setdefault("binary_sha256", {})["ubuntu_plugin"] = sha256(binary)
    fixture.save()
    docker("cp", str(binary), fixture.state["peer"] + ":/usr/local/bin/recollect-plugin")
    prefix = [str(ROOT / "scripts/docker.sh"), "exec", "-i", fixture.state["peer"],
              "bash", "-c", ". /run/recollect-fixture-bus; export RECOLLECT_PLUGIN_DATA=/root/.local/share/recollect-proof; exec \"$@\"", "fixture"]

    def plugin(*args, stdin=None):
        result = subprocess.run(prefix + ["/usr/local/bin/recollect-plugin", *args], cwd=ROOT,
            env=process_env(), input=stdin, text=True, capture_output=True, timeout=20)
        assert result.returncode == 0, "Ubuntu native plugin failed: " + args[0]
        return json.loads(result.stdout)

    connected = plugin("connect", "--url", "http://127.0.0.1:18787", "--brain", brain,
                       "--token-stdin", stdin=token)
    assert connected["state"] == "connected"
    setup = """import json,pathlib,subprocess,sys
brain=json.load(sys.stdin)['brain']
root=pathlib.Path('/fixture/paul/team'); (root/'.recollect').mkdir(parents=True,exist_ok=True)
(root/'.recollect/workspace.toml').write_text('brain = "'+brain+'"\\n')
repo=root/'infra'; repo.mkdir(exist_ok=True)
subprocess.run(['git','init','-q',str(repo)],check=True)
subprocess.run(['git','-C',str(repo),'remote','add','origin','https://git.example.invalid/team/infra.git'],check=True)
(repo/'clusters/production').mkdir(parents=True,exist_ok=True)
(repo/'clusters/production/vault.yaml').write_text('# synthetic fixture; no real configuration\\n')
"""
    docker("exec", "-i", fixture.state["peer"], "python3", "-c", setup,
           stdin=json.dumps({"brain": brain}))
    plugin("workspace", "refresh", "/fixture/paul/team")
    host_session = "native-fixture-session"
    common = {"session_id": host_session, "cwd": "/fixture/paul/team/infra", "host_version": "fixture-v1"}

    def hook(name, **fields):
        return plugin("hook", "claude_code", stdin=json.dumps({**common, "hook_event_name": name, **fields}))

    started = hook("SessionStart", source="startup")
    context = started["hookSpecificOutput"]["additionalContext"]
    native_id = re.search(r"your session ([0-9a-f-]{36})", context).group(1)
    hook("UserPromptSubmit", prompt="Configure Vault certificate renewal", prompt_id="fixture-prompt")

    def automatic_topic():
        mike.call("POST", coord + "/sessions/" + mine["id"] + "/heartbeat", {"expected_revision": 1})
        peers = mike.call("GET", coord + "/sessions?session_id=" + mine["id"])
        return next((overlap for overlap in peers["overlaps"] if
            overlap["peer"]["id"] == native_id and overlap["reason"] == "topic_overlap"), None)

    topic = eventually(automatic_topic, "automatic current-checkout topic overlap before editing", seconds=30)
    assert topic["tokens"] and not topic["peer"]["intents"]
    fixture.checked("native_checkout_context_and_topic_overlap_before_any_edit")
    hook("PreToolUse", tool_name="Write", tool_use_id="fixture-write",
         tool_input={"file_path": "/fixture/paul/team/infra/clusters/production/vault.yaml", "content": "synthetic"})

    def automatic_intent():
        mike.call("POST", coord + "/sessions/" + mine["id"] + "/heartbeat", {"expected_revision": 1})
        peers = mike.call("GET", coord + "/sessions")["sessions"]
        return next((peer for peer in peers if peer["id"] == native_id and any(
            intent["path"] == "clusters/production/vault.yaml" for intent in peer["intents"])), None)

    automatic = eventually(automatic_intent, "native planned file announcement", seconds=30)
    assert automatic["account_name"] == "paul-fixture" and automatic["goal"] == "Configure Vault certificate renewal"
    fixture.checked("ubuntu_native_hook_automatic_goal_and_planned_file_intent")
    marker = "NATIVE_PEER_FIXTURE_" + uuid.uuid4().hex[:8]
    proposal = {"id": str(uuid.uuid4()), "from_session_id": mine["id"], "to_session_id": native_id,
                "kind": "proposal", "text": marker + ": I own renewal; please validate policy.", "reply_to": None}
    mike.call("POST", coord + "/messages", proposal)

    def delivered_hook():
        result = hook("PreToolUse", tool_name="Bash", tool_use_id=str(uuid.uuid4()), tool_input={"command": "true"})
        return result if marker in json.dumps(result) else None

    delivery = eventually(delivered_hook, "native worker-to-hook delivery", seconds=30)
    text = delivery["hookSpecificOutput"]["additionalContext"]
    assert "mike-fixture" in text and "mike-mac-fixture" in text and "untrusted" in text
    assert delivery.get("systemMessage"), "Peer notification must also be human-visible"
    assert len(json.dumps(delivery).encode()) <= 8192
    repeat = hook("PreToolUse", tool_name="Bash", tool_use_id="repeat", tool_input={"command": "true"})
    assert marker not in json.dumps(repeat), "Local hooks must not redeliver an event"
    reply = {"id": str(uuid.uuid4()), "from_session_id": native_id, "to_session_id": mine["id"],
             "kind": "accept", "text": "Paul accepts through the native CLI.", "reply_to": proposal["id"]}
    result = plugin("coordination", "send", stdin=json.dumps(reply))
    assert result["sender"]["account_name"] == "paul-fixture"
    pulled = plugin("coordination", "inbox", "--session", native_id)
    assert any(event["kind"] == "message" and event["payload"]["data"]["id"] == proposal["id"] for event in pulled["events"])
    fixture.checked("native_hook_visible_peer_identity_dedup_and_cli_negotiation")

    watched = subprocess.Popen(prefix + ["timeout", "--signal=INT", "10s", "/usr/local/bin/recollect-plugin",
                        "coordination", "watch", "--session", native_id], cwd=ROOT, env=process_env(),
                        text=True, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        time.sleep(0.5)
        notice = {**proposal, "id": str(uuid.uuid4()), "kind": "note", "text": "WATCH_PEER_FIXTURE: policy validation can begin."}
        mike.call("POST", coord + "/messages", notice)
        stdout, stderr = watched.communicate(timeout=15)
        (fixture.path / "native-watch.txt").write_text(stderr)
        assert "mike-fixture" in stderr and "WATCH_PEER_FIXTURE" in stderr
        assert not stdout.strip() or json.loads(stdout)["state"] == "stopped"
        # Only fabricated peer content is retained in this readable evidence.
        (fixture.path / "native-hook.json").write_text(json.dumps(delivery, indent=2) + "\n")
    finally:
        if watched.poll() is None:
            watched.kill()
            watched.wait(timeout=5)
    fixture.checked("native_live_watch_readable_identity_and_clean_stdout")
    hook("SessionEnd")


if __name__ == "__main__":
    main()
