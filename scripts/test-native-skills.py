#!/usr/bin/env python3
"""Installed native skill fixtures; isolated homes and synthetic local providers.

No personal harness configuration or credentials are loaded. This is runtime
proof of the particular process started here, never the user's running host.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import queue
import re
import base64
import shlex
import sys
import subprocess
import threading
import time
import urllib.error
import urllib.request
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

REPO = Path(__file__).resolve().parent.parent
PROOF_ROOT = REPO / ".cache/native-skills-proof"
CODEX = os.environ.get("RECOLLECT_CODEX_BIN", "/opt/homebrew/bin/codex")
OPENCODE = os.environ.get("RECOLLECT_OPENCODE_BIN", "/opt/homebrew/bin/opencode")
CLAUDE = os.environ.get("RECOLLECT_CLAUDE_BIN", str(REPO / ".cache/native-skill-hosts/claude/node_modules/@anthropic-ai/claude-code-darwin-arm64/claude"))


def isolated_env(root: Path) -> dict[str, str]:
    env = {k: v for k, v in os.environ.items() if not any(x in k.upper() for x in
        ("TOKEN", "API_KEY", "SECRET", "ANTHROPIC", "CLAUDE", "CODEX", "OPENAI", "OPENCODE"))}
    for key, path in {"HOME": root / "home", "CODEX_HOME": root / "codex-home",
            "CLAUDE_CONFIG_DIR": root / "claude-config", "XDG_CONFIG_HOME": root / "xdg-config",
            "XDG_DATA_HOME": root / "xdg-data", "XDG_STATE_HOME": root / "xdg-state",
            "XDG_CACHE_HOME": root / "xdg-cache"}.items():
        path.mkdir(parents=True, exist_ok=True)
        env[key] = str(path)
    env.update({"NO_PROXY": "127.0.0.1,localhost", "DISABLE_TELEMETRY": "1",
        "DISABLE_ERROR_REPORTING": "1", "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
        "CLAUDE_CODE_DISABLE_AUTO_MEMORY": "1", "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY": "1",
        "OPENCODE_DISABLE_AUTOUPDATE": "true", "OPENCODE_DISABLE_DEFAULT_PLUGINS": "true",
        "OPENCODE_DISABLE_MODELS_FETCH": "true"})
    return env


class JsonProcess:
    def __init__(self, command: list[str], workspace: Path, env: dict, log: Path):
        self.log = log.open("w")
        self.process = subprocess.Popen(command, cwd=workspace, env=env, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=self.log, text=True, bufsize=1)
        self.lines: queue.Queue = queue.Queue()
        self.events = []
        def reader():
            for line in self.process.stdout:
                try:
                    item = json.loads(line)
                except ValueError:
                    item = {"unparsed": line}
                self.lines.put(item)
        threading.Thread(target=reader, daemon=True).start()
        self.counter = 0

    def send(self, item):
        self.process.stdin.write(json.dumps(item) + "\n")
        self.process.stdin.flush()

    def until(self, predicate, timeout=30):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            try:
                item = self.lines.get(timeout=min(1, end-time.monotonic()))
            except queue.Empty:
                if self.process.poll() is not None:
                    raise RuntimeError(f"native host exited: {self.process.returncode}")
                continue
            self.events.append(item)
            if predicate(item):
                return item
        raise TimeoutError(f"Native host response timeout; last events {self.events[-3:]}")

    def rpc(self, method, params):
        self.counter += 1
        request_id = self.counter
        self.send({"id": request_id, "method": method, "params": params})
        result = self.until(lambda x: x.get("id") == request_id)
        if "error" in result:
            raise RuntimeError(f"{method}: {result['error']}")
        return result.get("result")

    def close(self):
        self.process.terminate()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.log.close()


class Provider:
    """Responds only to local fixture API calls; retains requests as proof."""
    def __init__(self, root: Path):
        self.requests = []
        self.plan = []
        self.lock = threading.Lock()
        outer = self
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass
            def do_GET(self):
                self.respond({"data": [{"id": "fixture", "object": "model", "created": 0, "owned_by": "fixture"}]})
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers.get("content-length", "0"))) or b"{}")
                with outer.lock:
                    outer.requests.append({"path": self.path, "body": body})
                if self.path.split("?")[0].endswith("count_tokens"):
                    return self.respond({"input_tokens": 42})
                with outer.lock:
                    action = outer.plan.pop(0) if outer.plan else {"text": "NATIVE_SKILL_FIXTURE_COMPLETE"}
                if self.path.split("?")[0].endswith("responses"):
                    return self.codex_response(body, action)
                if self.path.split("?")[0].endswith("messages"):
                    return self.messages(body, action)
                self.respond({})
            def respond(self, value):
                data = json.dumps(value).encode()
                self.send_response(200)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            def sse(self, events):
                data = "".join(f"event: {name}\ndata: {json.dumps(value)}\n\n" for name, value in events).encode()
                self.send_response(200)
                self.send_header("content-type", "text/event-stream")
                self.send_header("content-length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            def codex_response(self, body, action):
                if "command" in action:
                    names = [x.get("name") for x in body.get("tools", [])]
                    name = next((x for x in ["exec_command", "shell_command", "shell"] if x in names), "exec_command")
                    args = {"cmd": action["command"], "max_output_tokens": 1000} if name == "exec_command" else {"command": action["command"]}
                    if name == "shell":
                        args = {"command": ["/bin/sh", "-c", action["command"]]}
                    item = {"type": "function_call", "id": "fc_fixture", "call_id": f"call_{len(outer.requests)}",
                        "name": name, "arguments": json.dumps(args), "status": "completed"}
                else:
                    item = {"type": "message", "id": "msg_fixture", "role": "assistant", "status": "completed",
                        "content": [{"type": "output_text", "text": action["text"], "annotations": []}]}
                response = {"id": f"resp_{len(outer.requests)}", "status": "completed", "output": [item],
                    "usage": {"input_tokens": 42, "output_tokens": 20, "total_tokens": 62}}
                self.sse([( "response.created", {"type": "response.created", "response": {**response, "status": "in_progress", "output": []}}),
                    ("response.output_item.added", {"type": "response.output_item.added", "output_index": 0, "item": item}),
                    ("response.output_item.done", {"type": "response.output_item.done", "output_index": 0, "item": item}),
                    ("response.completed", {"type": "response.completed", "response": response})])
            def messages(self, body, action):
                tool = "read" in action or "skill" in action
                names = [x.get("name") for x in body.get("tools", [])]
                read_name = "Read" if "Read" in names else "read"
                read_tool = next((x for x in body.get("tools", []) if x.get("name") == read_name), {})
                properties = read_tool.get("input_schema", {}).get("properties", {})
                path_key = next((x for x in ("file_path", "filePath", "path") if x in properties), "file_path")
                block = ({"type": "tool_use", "id": f"tool_fixture_{len(outer.requests)}", "name": "Skill", "input": {"skill": action["skill"]}}
                    if "skill" in action else {"type": "tool_use", "id": f"tool_fixture_{len(outer.requests)}", "name": read_name, "input": {path_key: action["read"]}}) if tool else {"type": "text", "text": action["text"]}
                stop = "tool_use" if tool else "end_turn"
                message = {"id": "msg_fixture", "type": "message", "role": "assistant", "model": "claude-sonnet-4-6", "content": [block],
                    "stop_reason": stop, "stop_sequence": None, "usage": {"input_tokens": 42, "output_tokens": 20}}
                if not body.get("stream"):
                    return self.respond(message)
                start = {**block, "input": {}} if tool else {"type": "text", "text": ""}
                delta = {"type": "input_json_delta", "partial_json": json.dumps(block["input"])} if tool else {"type": "text_delta", "text": block["text"]}
                self.sse([("message_start", {"type": "message_start", "message": {**message, "content": [], "stop_reason": None}}),
                    ("content_block_start", {"type": "content_block_start", "index": 0, "content_block": start}),
                    ("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": delta}),
                    ("content_block_stop", {"type": "content_block_stop", "index": 0}),
                    ("message_delta", {"type": "message_delta", "delta": {"stop_reason": stop, "stop_sequence": None}, "usage": {"output_tokens": 20}}),
                    ("message_stop", {"type": "message_stop"})])
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.url = f"http://127.0.0.1:{self.server.server_port}"
        self.root = root
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        (self.root / "provider-requests.json").write_text(json.dumps(self.requests, indent=2))


class NativeFixture:
    """Drive the production installer; model responses remain synthetic."""
    def __init__(self, root: Path, workspace: Path, host: str, endpoint: str):
        self.root, self.workspace, self.host = root, workspace, host
        self.task = str(uuid.uuid4())
        self.config = {"endpoint": endpoint, "brain": "33333333-3333-4333-8333-333333333333",
            "device": "44444444-4444-4444-8444-444444444444", "profile": "native-proof"}
        self.skill_id = "11111111-1111-4111-8111-111111111111"
        self.instructions_id = "22222222-2222-4222-8222-222222222222"
        self.calls = []

    def call(self, action, bundles=(), persistent=False, task=None, skill_id=None):
        request = {"action": action, "root": str(self.root / "plugin-data"), "workspace": str(self.workspace),
            "host": self.host, "config": self.config, "environment": None, "task": task or self.task,
            "persistent": persistent, "bundles": list(bundles), "id": skill_id}
        result = subprocess.run([str(REPO / "target/debug/examples/native_skill_fixture")],
            input=json.dumps(request), text=True, capture_output=True)
        if result.returncode:
            raise RuntimeError(f"native fixture {action}: {result.stderr}")
        report = json.loads(result.stdout)
        assert not report["errors"], report
        assert report["native_loaded"] is False, "placement must not claim native activation"
        self.calls.append({"request": request, "report": report})
        (self.root / "installer-calls.json").write_text(json.dumps(self.calls, indent=2))
        return report

    def bundle(self, version, instructions=False):
        name = "native-instructions-fixture" if instructions else "native-skill-fixture"
        entry = "AGENTS.md" if instructions else "SKILL.md"
        prefix = "NATIVE_INSTRUCTIONS" if instructions else "NATIVE_BODY"
        body = f"{prefix}_V{version}_8282\nRead references/proof.md using the read tool.\n"
        if not instructions:
            body = f"---\nname: {name}\ndescription: Native fixture version {version}\n---\n" + body
        files = [{"path": entry, "content": body}, {"path": "references/proof.md", "content": f"NATIVE_REFERENCE_V{version}_8282\n"}]
        return {"skill": {"id": self.instructions_id if instructions else self.skill_id,
            "brain_id": self.config["brain"], "name": name, "description": f"Native fixture version {version}",
            "kind": "instructions" if instructions else "skill", "version": version, "environment_id": None,
            "profile_ids": [], "tags": [], "enabled": True, "file_paths": [f["path"] for f in files],
            "updated_at": "2026-10-18T00:00:00Z"}, "files": files}

    def install(self, version, instructions=False, persistent=False):
        report = self.call("install", [self.bundle(version, instructions)], persistent)
        entry = next(x for x in report["entries"] if x["id"] == (self.instructions_id if instructions else self.skill_id))
        return Path(entry["path"]), entry["native_name"]


def preserved_local_files(workspace: Path):
    files = {workspace / "AGENTS.md": "LOCAL_AGENTS_PRESERVED_8282\n",
        workspace / "CLAUDE.md": "LOCAL_CLAUDE_PRESERVED_8282\n",
        workspace / ".agents/skills/existing-local/SKILL.md": "---\nname: existing-local\ndescription: Local preserved skill\n---\nLOCAL_SKILL_PRESERVED_8282\n"}
    for path, content in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    return files


def assert_preserved(files):
    for path, content in files.items():
        assert path.read_text() == content, f"local file changed: {path}"


def claude_startup_hook_proof(root: Path):
    """A root created by SessionStart becomes usable without a manual reload."""
    root.mkdir()
    workspace = root / "workspace"
    workspace.mkdir()
    subprocess.run(["git", "init", "-q", str(workspace)], check=True)
    env = isolated_env(root)
    provider = Provider(root)
    fixture = NativeFixture(root, workspace, "claude_code", provider.url)
    request = {"action": "install", "root": str(root / "plugin-data"), "workspace": str(workspace),
        "host": "claude_code", "config": fixture.config, "environment": None, "task": fixture.task,
        "persistent": False, "bundles": [fixture.bundle(1)], "id": None}
    request_path = root / "startup-install.json"
    request_path.write_text(json.dumps(request))
    hook = root / "startup-hook.py"
    hook.write_text("import json,pathlib,subprocess\n"
        f"p=subprocess.run([{str(REPO / 'target/debug/examples/native_skill_fixture')!r}],input=pathlib.Path({str(request_path)!r}).read_text(),text=True,capture_output=True,check=True)\n"
        f"pathlib.Path({str(root / 'startup-installer-report.json')!r}).write_text(p.stdout)\n"
        "print(json.dumps({'hookSpecificOutput':{'hookEventName':'SessionStart','reloadSkills':True}}))\n")
    claude_dir = workspace / ".claude"
    claude_dir.mkdir()
    (claude_dir / "settings.json").write_text(json.dumps({"hooks": {"SessionStart": [{"hooks": [{
        "type": "command", "command": shlex.quote(sys.executable) + " " + shlex.quote(str(hook)), "timeout": 10}]}]}}))
    env.update(ANTHROPIC_API_KEY="synthetic-fixture", ANTHROPIC_BASE_URL=provider.url)
    host = JsonProcess([CLAUDE, "--print", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
        "--no-session-persistence", "--strict-mcp-config", "--setting-sources", "project", "--allowedTools", "Read,Skill",
        "--model", "claude-sonnet-4-6"], workspace, env, root / "stderr.log")
    alias = "recollect-claude-" + fixture.skill_id.replace("-", "")
    target = workspace / ".claude/skills" / alias
    assert not target.exists()
    provider.plan = [{"text": "STARTUP_COMPLETE"}]
    try:
        host.send({"type": "control_request", "request_id": "initialize", "request": {"subtype": "initialize"}})
        host.until(lambda x: x.get("type") == "control_response")
        host.send({"type": "user", "message": {"role": "user", "content": "Initialize this isolated native-skill fixture."}, "parent_tool_use_id": None, "session_id": ""})
        done = host.until(lambda x: x.get("type") == "result", 45)
        assert not done.get("is_error"), done
        init = next(x for x in host.events if x.get("type") == "system" and x.get("subtype") == "init")
        assert alias in init["skills"], init
        before = len(provider.requests)
        provider.plan = [{"skill": alias}, {"read": str(target / "references/proof.md")}, {"text": "NATIVE_SKILL_FIXTURE_COMPLETE"}]
        host.send({"type": "user", "message": {"role": "user", "content": "Activate the installed native skill using the Skill tool and read its reference."}, "parent_tool_use_id": None, "session_id": ""})
        done = host.until(lambda x: x.get("type") == "result", 45)
        assert not done.get("is_error"), done
        bodies = json.dumps(provider.requests[before:])
        assert "NATIVE_BODY_V1_8282" in bodies and "NATIVE_REFERENCE_V1_8282" in bodies
        report = json.loads((root / "startup-installer-report.json").read_text())
        assert report["state"] == "placed" and report["native_loaded"] is False
        return {"new_skill_root_created_by_hook": True, "native_catalog": True, "native_activation": True,
            "reference_read": True, "refresh": "SessionStart reloadSkills:true", "process": host.process.pid}
    finally:
        (root / "host-events.json").write_text(json.dumps(host.events, indent=2))
        host.close()
        provider.close()
        fixture.call("release")


def claude_proof(root: Path):
    workspace = root / "workspace"
    workspace.mkdir(parents=True)
    subprocess.run(["git", "init", "-q", str(workspace)], check=True)
    originals = preserved_local_files(workspace)
    env = isolated_env(root)
    provider = Provider(root)
    fixture = NativeFixture(root, workspace, "claude_code", provider.url)
    env.update(ANTHROPIC_API_KEY="synthetic-fixture", ANTHROPIC_BASE_URL=provider.url)
    host = JsonProcess([CLAUDE, "--print", "--input-format", "stream-json", "--output-format", "stream-json",
        "--verbose", "--no-session-persistence", "--strict-mcp-config", "--setting-sources", "project",
        "--allowedTools", "Read,Skill", "--model", "claude-sonnet-4-6"], workspace, env, root / "stderr.log")
    proof = {"host": "claude_code", "version": subprocess.check_output([CLAUDE, "--version"], text=True).strip()}
    def control(subtype):
        request_id = str(uuid.uuid4())
        host.send({"type": "control_request", "request_id": request_id, "request": {"subtype": subtype}})
        result = host.until(lambda x: x.get("type") == "control_response" and x.get("response", {}).get("request_id") == request_id)
        assert result["response"]["subtype"] == "success", result
        return result["response"]["response"]
    def activate(target, alias, version, instructions=False):
        provider.plan = [{"read": str(target.parent / "AGENTS.md")} if instructions else {"read": str(target.parent / "references/proof.md")},
            {"read": str(target.parent / "references/proof.md")}, {"text": "NATIVE_SKILL_FIXTURE_COMPLETE"}] if instructions else [
                {"read": str(target.parent / "references/proof.md")}, {"text": "NATIVE_SKILL_FIXTURE_COMPLETE"}]
        before = len(provider.requests)
        host.send({"type": "user", "message": {"role": "user", "content": "/" + alias}, "parent_tool_use_id": None, "session_id": ""})
        done = host.until(lambda x: x.get("type") == "result", 45)
        assert not done.get("is_error"), done
        bodies = json.dumps(provider.requests[before:])
        marker = "NATIVE_INSTRUCTIONS" if instructions else "NATIVE_BODY"
        assert f"{marker}_V{version}_8282" in bodies, bodies[-3000:]
        assert f"NATIVE_REFERENCE_V{version}_8282" in bodies, bodies[-3000:]
        assert "skills.fetch" in bodies, "native wrapper must require current canonical check"
        if not instructions:
            activation = json.dumps(provider.requests[before]["body"]["messages"]).rsplit(f"Base directory for this skill: {target.parent}", 1)[-1]
            assert f"NATIVE_BODY_V{version}_8282" in activation
            assert f"NATIVE_BODY_V{3-version}_8282" not in activation
        return {"catalog": True, "body_activated": True, "reference_read": True, "same_process": host.process.pid,
            "session": done["session_id"], "refresh": "reload_skills control request"}
    try:
        control("initialize")
        first = control("reload_skills")
        assert fixture.skill_id.replace("-", "") not in json.dumps(first)
        for version in (1, 2):
            target, alias = fixture.install(version)
            catalog = control("reload_skills")
            assert alias in json.dumps(catalog), catalog
            proof[f"v{version}"] = activate(target, alias, version)
        instruction, instruction_alias = fixture.install(1, instructions=True, persistent=True)
        assert (instruction.parent / "AGENTS.md").read_text() == fixture.bundle(1, True)["files"][0]["content"]
        assert instruction_alias in json.dumps(control("reload_skills"))
        proof["instructions"] = activate(instruction, instruction_alias, 1, True)
        second_task = str(uuid.uuid4())
        fixture.call("install", [fixture.bundle(2)], task=second_task)
        fixture.call("release")
        catalog = control("reload_skills")
        assert alias in json.dumps(catalog), "closing first task must preserve second task's native copy"
        fixture.call("release", task=second_task)
        catalog = control("reload_skills")
        assert alias not in json.dumps(catalog), catalog
        assert instruction_alias in json.dumps(catalog), catalog
        proof["temporary_removed_persistent_retained"] = True
        proof["two_live_tasks_reference_cleanup"] = True
        fixture.call("uninstall", persistent=True, skill_id=fixture.instructions_id)
        assert instruction_alias not in json.dumps(control("reload_skills"))
        proof["persistent_uninstalled"] = True
        assert_preserved(originals)
        proof["existing_local_files_preserved"] = True
        (root / "host-events.json").write_text(json.dumps(host.events, indent=2))
        proof["startup_hook"] = claude_startup_hook_proof(root / "startup-hook-proof")
        return proof
    finally:
        host.close()
        provider.close()


def codex_proof(root: Path):
    workspace = root / "workspace"
    workspace.mkdir(parents=True)
    subprocess.run(["git", "init", "-q", str(workspace)], check=True)
    originals = preserved_local_files(workspace)
    env = isolated_env(root)
    provider = Provider(root)
    fixture = NativeFixture(root, workspace, "codex", provider.url)
    (Path(env["CODEX_HOME"]) / "config.toml").write_text(f'''model = "fixture"
model_provider = "fixture"
approval_policy = "never"
sandbox_mode = "danger-full-access"
[model_providers.fixture]
name = "Synthetic native skills fixture"
base_url = "{provider.url}/v1"
wire_api = "responses"
requires_openai_auth = false
[analytics]
enabled = false
''')
    host = JsonProcess([CODEX, "app-server", "--stdio"], workspace, env, root / "stderr.log")
    alias = "recollect-codex-11111111111141118111111111111111"
    proof = {"host": "codex", "version": subprocess.check_output([CODEX, "--version"], text=True).strip()}
    try:
        host.rpc("initialize", {"clientInfo": {"name": "recollect-native-skills-fixture", "version": "1.0"}, "capabilities": {"experimentalApi": True}})
        host.send({"method": "initialized", "params": {}})
        initial = host.rpc("skills/list", {"cwds": [str(workspace)], "forceReload": True})
        assert alias not in json.dumps(initial)
        thread = host.rpc("thread/start", {"cwd": str(workspace), "model": "fixture", "modelProvider": "fixture", "approvalPolicy": "never", "sandbox": "danger-full-access", "ephemeral": True})
        thread_id = thread["thread"]["id"]
        for version in (1, 2):
            target, alias = fixture.install(version)
            catalog = host.rpc("skills/list", {"cwds": [str(workspace)], "forceReload": True})
            assert alias in json.dumps(catalog), catalog
            provider.plan = [{"command": f"cat '{target.parent / 'references/proof.md'}'"}, {"text": "NATIVE_SKILL_FIXTURE_COMPLETE"}]
            before = len(provider.requests)
            host.rpc("turn/start", {"threadId": thread_id, "input": [{"type": "skill", "name": alias, "path": str(target)},
                {"type": "text", "text": f"Activate the selected native skill version {version} and read its reference."}]})
            done = host.until(lambda x: x.get("method") == "turn/completed", 45)
            assert done["params"]["turn"]["status"] == "completed", done
            bodies = json.dumps(provider.requests[before:])
            assert f"NATIVE_BODY_V{version}_8282" in bodies, bodies[-2000:]
            assert f"NATIVE_REFERENCE_V{version}_8282" in bodies, bodies[-2000:]
            assert "skills.fetch" in bodies
            activated = [b["text"] for item in provider.requests[before]["body"]["input"]
                for b in item.get("content", []) if isinstance(b, dict) and b.get("text", "").startswith("<skill>")
                and f"<name>{alias}</name>" in b["text"]][-1]
            assert f"NATIVE_BODY_V{version}_8282" in activated
            assert f"NATIVE_BODY_V{3-version}_8282" not in activated
            proof[f"v{version}"] = {"catalog": True, "body_activated": True, "reference_read": True, "same_process": host.process.pid, "thread": thread_id}
        instruction, instruction_alias = fixture.install(1, instructions=True, persistent=True)
        assert (instruction.parent / "AGENTS.md").read_text() == fixture.bundle(1, True)["files"][0]["content"]
        assert instruction_alias in json.dumps(host.rpc("skills/list", {"cwds": [str(workspace)], "forceReload": True}))
        provider.plan = [{"command": f"cat '{instruction.parent / 'AGENTS.md'}' '{instruction.parent / 'references/proof.md'}'"}, {"text": "NATIVE_SKILL_FIXTURE_COMPLETE"}]
        before = len(provider.requests)
        host.rpc("turn/start", {"threadId": thread_id, "input": [{"type": "skill", "name": instruction_alias, "path": str(instruction)},
            {"type": "text", "text": "Activate the instructions skill and read AGENTS.md and its reference."}]})
        done = host.until(lambda x: x.get("method") == "turn/completed", 45)
        assert done["params"]["turn"]["status"] == "completed", done
        bodies = json.dumps(provider.requests[before:])
        assert "NATIVE_INSTRUCTIONS_V1_8282" in bodies and "NATIVE_REFERENCE_V1_8282" in bodies
        proof["instructions"] = {"catalog": True, "body_activated": True, "source_resource_preserved": True, "reference_read": True}
        second_task = str(uuid.uuid4())
        fixture.call("install", [fixture.bundle(2)], task=second_task)
        fixture.call("release")
        catalog = host.rpc("skills/list", {"cwds": [str(workspace)], "forceReload": True})
        assert alias in json.dumps(catalog), "closing first task must preserve second task's native copy"
        fixture.call("release", task=second_task)
        removed = host.rpc("skills/list", {"cwds": [str(workspace)], "forceReload": True})
        assert alias not in json.dumps(removed), removed
        assert instruction_alias in json.dumps(removed), removed
        proof["temporary_removed_persistent_retained"] = True
        proof["two_live_tasks_reference_cleanup"] = True
        fixture.call("uninstall", persistent=True, skill_id=fixture.instructions_id)
        assert instruction_alias not in json.dumps(host.rpc("skills/list", {"cwds": [str(workspace)], "forceReload": True}))
        proof["persistent_uninstalled"] = True
        assert_preserved(originals)
        proof["existing_local_files_preserved"] = True
        (root / "host-events.json").write_text(json.dumps(host.events, indent=2))
        return proof
    finally:
        host.close()
        provider.close()


def opencode_proof(root: Path):
    workspace = root / "workspace"
    workspace.mkdir(parents=True)
    subprocess.run(["git", "init", "-q", str(workspace)], check=True)
    originals = preserved_local_files(workspace)
    env = isolated_env(root)
    env.update(OPENCODE_TEST_HOME=env["HOME"], OPENCODE_CONFIG_PROJECT_DISABLE="true", ANTHROPIC_API_KEY="synthetic-fixture")
    env["RECOLLECT_PLUGIN_DATA"] = str(root / "plugin-data")
    provider = Provider(root)
    fixture = NativeFixture(root, workspace, "opencode", provider.url)
    compatibility = NativeFixture(root, workspace, "codex", provider.url)
    compatibility.task = fixture.task
    plugin = root / "host-plugin"
    plugin.mkdir()
    (plugin / "package.json").write_text(json.dumps({"name": "recollect-native-skills-proof", "version": "0.0.0", "type": "module", "main": "index.mjs"}))
    (plugin / "index.mjs").write_text(f'''
import {{createNativeSkillReload}} from {json.dumps((REPO / "plugins/recollect/plugins/recollect-memory/opencode/index.mjs").as_uri())};
import {{readFile, writeFile, appendFile}} from "node:fs/promises";
export default {{id:"recollect-native-skills-proof",async setup(ctx){{
  let reloadCalls=0;
  const actualNativeContext={{skill:{{
    list:(...args)=>ctx.skill.list(...args),
    transform:(callback)=>ctx.skill.transform((editor)=>{{
      const before=editor.list().filter(x=>x.id.startsWith("recollect-")).map(x=>({{id:x.id,path:x.path,content:x.content}}));
      callback(editor);
      const after=editor.list().filter(x=>x.id.startsWith("recollect-")).map(x=>x.id);
      void appendFile({json.dumps(str(root / "transform-trace.jsonl"))},JSON.stringify({{before,after}})+"\\n");
    }}),
    reload:async()=>{{await ctx.skill.reload();reloadCalls++;}},
  }}}};
  const reload = await createNativeSkillReload(actualNativeContext);
  let last="",running=false;
  const timer=setInterval(async()=>{{
    if(running)return;running=true;
    try{{
      const trigger=JSON.parse(await readFile({json.dumps(str(root / "reload-trigger.json"))},"utf8"));
      if(trigger.nonce!==last){{
        last=trigger.nonce;
        const result=await reload({{recollectNativeSkillsSnapshot:trigger.pointer}});
        await writeFile({json.dumps(str(root / "reload-ack.json"))},JSON.stringify({{nonce:last,result,reloadCalls}}));
      }}
    }}catch(error){{
      if(last)await writeFile({json.dumps(str(root / "reload-ack.json"))},JSON.stringify({{nonce:last,error:String(error)}}));
    }}finally{{running=false;}}
  }},50);
  return async()=>{{clearInterval(timer);await reload.dispose?.();}};
}}}};
''')
    config_dir = Path(env["XDG_CONFIG_HOME"]) / "opencode"
    config_dir.mkdir(exist_ok=True)
    (config_dir / "opencode.json").write_text(json.dumps({
        "skills": [str(workspace / ".opencode/skills"), str(workspace / ".agents/skills")],
        "model": "anthropic/claude-sonnet-4-6", "snapshots": False,
        "providers": {"anthropic": {"settings": {"baseURL": provider.url + "/v1"},
            "models": {"claude-sonnet-4-6": {"limit": {"context": 200000, "output": 1000}}}}},
        "permissions": [{"action": "read", "resource": "*", "effect": "allow"}],
        "mcp": {"servers": {}}, "plugins": [str(plugin)]}))
    log = (root / "stderr.log").open("w")
    process = subprocess.Popen([OPENCODE, "serve", "--hostname", "127.0.0.1", "--port", "0"],
        cwd=workspace, env=env, stdout=log, stderr=log)
    events = []
    proof = {"host": "opencode", "version": subprocess.check_output([OPENCODE, "--version"], text=True).strip()}
    try:
        url, password = None, None
        for _ in range(100):
            output = (root / "stderr.log").read_text()
            url, password = re.search(r"http://127.0.0.1:\d+", output), re.search(r"server password (\S+)", output)
            if url and password:
                break
            time.sleep(0.05)
        assert url and password, "OpenCode fixture did not start"
        auth = "Basic " + base64.b64encode(("opencode:" + password[1]).encode()).decode()
        def api(path, data=None):
            request = urllib.request.Request(url[0] + path, headers={"Authorization": auth, "Content-Type": "application/json"},
                data=json.dumps(data).encode() if data is not None else None)
            try:
                with urllib.request.urlopen(request, timeout=20) as response:
                    body = response.read()
                    value = json.loads(body) if body else None
                    events.append({"path": path, "request": data, "response": value})
                    return value
            except urllib.error.HTTPError as error:
                events.append({"path": path, "request": data, "status": error.code})
                raise
        def catalog(expect=None, absent=False):
            end = time.monotonic() + 12
            while time.monotonic() < end:
                value = api("/api/skill")
                found = [x for x in value["data"] if x["name"] == expect]
                if expect is None or bool(found) != absent:
                    return value
                time.sleep(0.1)
            raise AssertionError(f"OpenCode registry did not reconcile {expect}; absent={absent}")
        def native_reload():
            report = fixture.call("status")
            foreign = compatibility.call("status")
            entries = [{k: v for k, v in entry.items() if k != "tasks"} for entry in report["entries"]]
            snapshot_dir = root / "plugin-data/native-skills/catalogues"
            snapshot_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
            snapshot = snapshot_dir / f"{fixture.config['device']}-{fixture.task}.json"
            owned = [{k: entry[k] for k in ("id", "path", "host", "brain_id")} for entry in report["entries"] + foreign["entries"]]
            body = {"scope_id": "55555555-5555-4555-8555-555555555555", "task_id": fixture.task,
                "brain_id": fixture.config["brain"], "workspace": str(workspace), "entries": entries,
                "owned_exports": owned, "native_loaded": False}
            snapshot.write_text(json.dumps(body))
            snapshot.chmod(0o600)
            nonce = str(uuid.uuid4())
            pointer = {"snapshot": str(snapshot), "workspace": str(workspace), "state": report["state"],
                "changed": True, "native_loaded": False}
            (root / "reload-trigger.json").write_text(json.dumps({"nonce": nonce, "pointer": pointer}))
            end = time.monotonic() + 10
            while time.monotonic() < end:
                try:
                    ack = json.loads((root / "reload-ack.json").read_text())
                    if ack["nonce"] == nonce:
                        assert "error" not in ack, ack
                        assert ack["result"].get("state") not in ("unavailable", "partial", "conflict"), ack
                        assert ack["reloadCalls"] > 0, "the real native ctx.skill.reload was not called"
                        events.append({"native_transform_reload": ack})
                        return ack
                except (FileNotFoundError, ValueError):
                    pass
                time.sleep(0.05)
            raise AssertionError("production native skill reload helper did not acknowledge the snapshot")
        sources = api("/api/config")
        assert all(str(root) in x["path"] for x in sources), "fixture imported a nonfixture configuration source"
        assert fixture.skill_id.replace("-", "") not in json.dumps(catalog("existing-local"))
        session = api("/api/session", {"location": {"directory": str(workspace)}, "title": "Native managed-skill fixture",
            "model": {"id": "claude-sonnet-4-6", "providerID": "anthropic"}})
        session_id = session["data"]["id"]
        def activate(target, alias, version, instructions=False):
            selected = next(x for x in catalog(alias)["data"] if x["name"] == alias)
            before = len(provider.requests)
            provider.plan = ([{"read": str(target.parent / "AGENTS.md")}] if instructions else []) + [
                {"read": str(target.parent / "references/proof.md")}, {"text": "NATIVE_SKILL_FIXTURE_COMPLETE"}]
            api(f"/api/experimental/session/{session_id}/skill", {"id": selected["id"], "resume": True})
            end = time.monotonic() + 20
            final = None
            while time.monotonic() < end:
                context = api(f"/api/session/{session_id}/context")
                assistants = [x for x in context["data"] if x["type"] == "assistant"]
                if assistants and assistants[-1].get("finish") == "stop" and assistants[-1].get("time", {}).get("created", 0) >= activation_time:
                    final = context
                    break
                time.sleep(0.1)
            assert final, "native OpenCode activation did not finish a synthetic model turn"
            # The last native skill message is the newly activated version;
            # older versions intentionally remain in this conversation history.
            message = [x for x in final["data"] if x["type"] == "skill" and x["skill"] == selected["id"]][-1]
            if not instructions:
                assert f"NATIVE_BODY_V{version}_8282" in message["text"]
                assert f"NATIVE_BODY_V{3-version}_8282" not in message["text"]
            marker = "NATIVE_INSTRUCTIONS" if instructions else "NATIVE_BODY"
            bodies = json.dumps(provider.requests[before:])
            assert f"{marker}_V{version}_8282" in bodies
            assert f"NATIVE_REFERENCE_V{version}_8282" in bodies
            assert "skills.fetch" in message["text"]
            return {"catalog": True, "body_activated": True, "reference_read": True, "same_process": process.pid,
                "session": session_id, "refresh": "production ctx.skill.transform + ctx.skill.reload"}
        for version in (1, 2):
            target, alias = fixture.install(version)
            if version == 1:
                # Native ownership can grow while our own source/version stays
                # unchanged; the late compatibility export must be deduplicated.
                native_reload()
            compatibility.install(version)
            native_reload()
            # The registry's content must change, even when the alias is stable.
            end = time.monotonic() + 12
            while time.monotonic() < end:
                value = catalog(alias)
                selected = next(x for x in value["data"] if x["name"] == alias)
                if f"NATIVE_BODY_V{version}_8282" in selected["content"]:
                    break
                time.sleep(0.1)
            assert f"NATIVE_BODY_V{version}_8282" in selected["content"]
            assert "recollect-codex-" not in json.dumps(value), "cross-root owned duplicate was not removed"
            activation_time = time.time() * 1000
            proof[f"v{version}"] = activate(target, alias, version)
        instruction, instruction_alias = fixture.install(1, instructions=True, persistent=True)
        native_reload()
        assert (instruction.parent / "AGENTS.md").read_text() == fixture.bundle(1, True)["files"][0]["content"]
        activation_time = time.time() * 1000
        proof["instructions"] = activate(instruction, instruction_alias, 1, True)
        second_task = str(uuid.uuid4())
        fixture.call("install", [fixture.bundle(2)], task=second_task)
        fixture.call("release")
        native_reload()
        catalog(alias)
        fixture.call("release", task=second_task)
        compatibility.call("release")
        native_reload()
        catalog(alias, absent=True)
        catalog(instruction_alias)
        proof["temporary_removed_persistent_retained"] = True
        proof["two_live_tasks_reference_cleanup"] = True
        try:
            api(f"/api/experimental/session/{session_id}/skill", {"id": selected["id"], "resume": False})
            raise AssertionError("removed native skill remained activatable")
        except urllib.error.HTTPError as error:
            assert error.code == 404
        fixture.call("uninstall", persistent=True, skill_id=fixture.instructions_id)
        native_reload()
        catalog(instruction_alias, absent=True)
        proof["persistent_uninstalled"] = True
        assert_preserved(originals)
        proof["existing_local_files_preserved"] = True
        proof["production_transform_reload_and_cross_root_dedup"] = True
        proof["late_cross_root_duplicate_reconciled"] = True
        return proof
    finally:
        (root / "host-events.json").write_text(json.dumps(events, indent=2))
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        log.close()
        provider.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", choices=["codex", "claude_code", "opencode", "all"], default="all")
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--offline", "-p", "recollect-agent", "--example", "native_skill_fixture"], cwd=REPO, check=True)
    run = PROOF_ROOT / str(uuid.uuid4())
    run.mkdir(parents=True)
    results = {"platform": platform.platform(), "run": str(run), "synthetic_provider": True,
        "paid_provider_calls": 0, "canonical_service_connectivity_tested": False,
        "proof_scope": "The installed processes started by this fixture; native transport/activation and local ownership lifecycle. Current canonical authorization and model obedience are separate checks. Previously delivered context is not retracted.",
        "hosts": {}}
    for host in (["codex", "claude_code", "opencode"] if args.host == "all" else [args.host]):
        root = run / host
        root.mkdir()
        try:
            result = globals()[host.replace("_code", "") + "_proof"](root)
            results["hosts"][host] = {"status": "pass", **result}
        except Exception as error:
            results["hosts"][host] = {"status": "fail", "error": str(error)}
        print(json.dumps({"host": host, **results["hosts"][host]}), flush=True)
    (run / "results.json").write_text(json.dumps(results, indent=2))
    print(f"Proof receipt: {run / 'results.json'}", flush=True)
    return int(any(v["status"] != "pass" for v in results["hosts"].values()))


if __name__ == "__main__":
    raise SystemExit(main())
