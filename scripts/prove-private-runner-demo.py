#!/usr/bin/env python3
"""Explicit local SWEG Ubuntu demo proof; leaves the named runner inspectable.

Requires the owned image, private TLS/env files and existing authorized demo
caller credential. Credentials are never printed or copied into evidence.
"""
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent.parent
PRIVATE = ROOT / ".cache/private-runner-ubuntu-demo"
OUTPUT = ROOT / "output/private-runner-ubuntu-2026-10-06"
CONTAINER = "recollect-ubuntu-private-runner-demo"
BRAIN = "5c054930-d266-4c18-a42b-942729f942aa"
BASE = "/api/brains/" + BRAIN
ORIGIN = "http://127.0.0.1:8787"
NAME = "Ubuntu filesystem demo"
KEY = "ubuntu-filesystem-demo-readonly"
spec = importlib.util.spec_from_file_location("runtime_api", ROOT / "scripts/test-installation-runtime.py")
api = importlib.util.module_from_spec(spec)
spec.loader.exec_module(api)


def docker(*args, stdin=None, timeout=45):
    result = subprocess.run([str(ROOT / "scripts/docker.sh"), *args], cwd=ROOT,
                            input=stdin, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise AssertionError("Owned Docker operation failed: " + args[0])
    return result.stdout


def wait(check, label, seconds=50):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(1)
    raise AssertionError("Timed out: " + label)


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    state_path = OUTPUT / "resources.json"
    state = json.loads(state_path.read_text()) if state_path.exists() else {
        "brain_id": BRAIN, "container": CONTAINER, "checks": {}, "calls": {},
        "marker": "Ubuntu private-runner filesystem proof",
        "caller_kind": "MCP protocol client, not an OpenCode LLM session",
    }
    fresh = "--fresh" in sys.argv
    if state.get("task_closed"):
        if not fresh:
            print("Proof already complete; --fresh runs new read/denial checks using the retained resources.")
            return
        state.setdefault("previous_runs", []).append({key: state[key] for key in
            ("task_id", "operation_id", "client_session_id", "calls")})
        for key in ("task_id", "scope_id", "operation_id", "client_session_id", "task_closed"):
            state.pop(key, None)
        state["calls"] = {}

    def save():
        state_path.write_text(json.dumps(state, indent=2) + "\n")

    def checked(name, value=True):
        state["checks"][name] = value
        save()
        print(name + ": passed", flush=True)

    env = api.environment(ROOT / ".env")
    owner = api.Session(ORIGIN)
    owner.login(env["RECOLLECT_OWNER_USERNAME"], env["RECOLLECT_OWNER_PASSWORD"])
    assert owner.call("GET", BASE)["role"] == "admin"
    private_caller = json.loads((ROOT / ".data/runtime/demo-agent.json").read_text())
    assert private_caller["brain_id"] == BRAIN
    caller = api.Session(ORIGIN)
    caller.opener.addheaders = [("Authorization", "Bearer " + private_caller["token"])]
    identity = caller.call("GET", "/api/auth/me")
    assert identity["device_id"] == private_caller["device_id"]
    assert caller.call("GET", BASE)["role"] == "reader"
    state["caller_device_id"] = identity["device_id"]
    state["caller_user"] = identity["user"]["username"]
    save()

    if "container_created" not in state:
        assert not docker("ps", "-a", "--filter", "name=^/" + CONTAINER + "$", "--format", "{{.Names}}")
        assert not docker("volume", "ls", "--filter", "name=^" + CONTAINER + "-state$", "--format", "{{.Name}}")
        docker("run", "-d", "--init", "--name", CONTAINER,
               "--label", "io.recollect.owner=ubuntu-private-runner-demo",
               "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true",
               "--env-file", str(PRIVATE / "runtime.env"),
               "--mount", "type=bind,src=" + str(PRIVATE / "tls") + ",dst=/run/recollect/tls,readonly",
               "--mount", "type=volume,src=" + CONTAINER + "-state,dst=/root/.local/share",
               "recollect-private-runner-demo:local")
        state["container_created"] = True
        save()

    wait(lambda: docker("exec", CONTAINER, "bash", "-c", "[[ -f /run/recollect/bus.env ]] && echo ready || true").strip(),
         "Linux Secret Service session")
    if "runner_device_id" not in state:
        # Native pairing writes the credential straight into Ubuntu Secret Service.
        log = PRIVATE / "pairing.log"
        with os.fdopen(os.open(log, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as out:
            child = subprocess.Popen([str(ROOT / "scripts/docker.sh"), "exec", CONTAINER,
                "bash", "-c", '. /run/recollect/bus.env; recollect-agent pair "Ubuntu filesystem demo runner"'],
                cwd=ROOT, stdout=out, stderr=out)
            try:
                code = wait(lambda: (m.group(1) if (m := re.search(r"Compare code ([A-Z0-9-]+)", log.read_text())) else None),
                            "native pairing code", 20)
                view = owner.call("GET", "/api/devices/pairings/" + code)
                assert view["name"] == "Ubuntu filesystem demo runner"
                owner.call("POST", "/api/devices/pairings/" + code + "/approve", {"approve": True})
                assert child.wait(timeout=30) == 0, "Native pairing failed; private log retained"
            finally:
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=5)
        match = re.search(r"Paired device ([0-9a-f-]{36})", log.read_text())
        assert match, "Native OS-store pairing did not complete"
        state["runner_device_id"] = match.group(1)
        save()
    assert state["runner_device_id"] != state["caller_device_id"]
    checked("separate_caller_and_runner_identities")

    if "runner_id" not in state:
        assert not any(r["name"] == NAME for r in owner.call("GET", BASE + "/mcp/private-runners"))
        runner = owner.call("POST", BASE + "/mcp/private-runners", {
            "name": NAME, "device_id": state["runner_device_id"], "enabled": True,
            "base_revision": None}, request_key=str(uuid.uuid4()))
        state["runner_id"] = runner["id"]
        save()
    reference = "private:" + state["runner_id"]
    docker("exec", "-i", CONTAINER, "bash", "-c", "cat > /run/recollect/runner-id", stdin=state["runner_id"])
    wait(lambda: next((r for r in owner.call("GET", BASE + "/mcp/private-runners")
                       if r["id"] == state["runner_id"] and r["available"]), None), "real private runner lease")
    checked("native_runner_connected")

    tools = [
        {"name": "read_text_file", "description": "Read a text file inside the Ubuntu demo folder.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}},
                         "required": ["path"], "additionalProperties": False}},
        {"name": "list_directory", "description": "List the Ubuntu demo directory.",
         "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}},
                         "required": ["path"], "additionalProperties": False}},
        {"name": "list_allowed_directories", "description": "Show allowed filesystem directories.",
         "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False}},
    ]
    for tool in tools:
        tool["annotations"] = {"readOnlyHint": True, "destructiveHint": False,
                               "idempotentHint": True, "openWorldHint": False}
    if "definition_imported" not in state:
        assert not any(d["key"] == KEY for d in owner.call("GET", "/api/mcp/definitions"))
        owner.call("POST", "/api/mcp/definitions", {
            "key": KEY, "name": "Ubuntu demo filesystem", "description": "Three read-only demo tools.",
            "transport": "stdio", "command": "/usr/local/bin/node",
            "arguments": ["/opt/filesystem/node_modules/@modelcontextprotocol/server-filesystem/dist/index.js", "/srv/recollect-demo"],
            "placements": ["private"], "credential_aliases": [],
            "configuration_schema": {"type": "object", "properties": {}, "additionalProperties": False},
            "tools": tools})
        state["definition_imported"] = True
        save()
    if "connection_id" not in state:
        connection = owner.call("POST", BASE + "/mcp/connections", {
            "name": NAME, "description": "Isolated Ubuntu demo folder", "definition_key": KEY,
            "target": "/srv/recollect-demo", "placement": "private", "runner_reference": reference,
            "credential_alias": None, "environment_id": None, "configuration": {}, "enabled": True,
            "base_revision": None}, request_key=str(uuid.uuid4()))
        state["connection_id"] = connection["summary"]["id"]
        save()
    if "profile_id" not in state:
        profile = owner.call("POST", BASE + "/mcp/profiles", {
            "name": "Ubuntu demo files", "description": "", "environment_id": None, "enabled": True,
            "connection_ids": [state["connection_id"]], "base_revision": None}, request_key=str(uuid.uuid4()))
        state["profile_id"] = profile["profile"]["id"]
        save()

    counter = 0

    def rpc(method, params=None):
        nonlocal counter
        counter += 1
        body = {"jsonrpc": "2.0", "id": counter, "method": method}
        if params is not None:
            body["params"] = params
        request = urllib.request.Request(ORIGIN + BASE + "/mcp/agent", method="POST",
            headers={"Authorization": "Bearer " + private_caller["token"], "Content-Type": "application/json",
                     "Accept": "application/json, text/event-stream", "MCP-Protocol-Version": "2025-11-25", "Origin": ORIGIN},
            data=json.dumps(body).encode())
        with caller.opener.open(request, timeout=20) as response:
            answer = json.load(response)
        assert "error" not in answer, "MCP protocol error"
        return answer["result"]

    def tool(name, arguments, error=False):
        answer = rpc("tools/call", {"name": name, "arguments": arguments})
        assert bool(answer.get("isError", False)) == error, "Unexpected MCP tool outcome: " + name
        return json.loads(answer["content"][0]["text"])

    info = rpc("initialize", {"protocolVersion": "2025-11-25", "capabilities": {},
                              "clientInfo": {"name": "Ubuntu private runner proof caller", "version": "1"}})
    state["protocol_version"] = info["protocolVersion"]
    names = [t["name"] for t in rpc("tools/list")["tools"]]
    assert "mcp.call" in names and "memory.contribute" not in names
    checked("reader_agent_mcp_catalogue")
    if "task_id" not in state:
        task = tool("workspace.start_task", {"input": {"label": "Ubuntu private runner demo proof",
                    "parent_task_id": None, "workspace_id": None, "selection": None},
                    "context_query": "Ubuntu private runner demo"})
        state["task_id"] = task["task"]["id"]
        state["scope_id"] = task["task"]["scope"]["id"]
        save()
    if "operation_id" not in state:
        operation = tool("workspace.begin", {"id": state["task_id"], "input": {
            "kind": "tool", "expected_scope": state["scope_id"]}})
        state["operation_id"] = operation["id"]
        state["client_session_id"] = str(uuid.uuid4())
        save()

    def queue(label, name="read_text_file", arguments=None, error=False):
        if label in state["calls"]:
            return state["calls"][label]
        request_id = str(uuid.uuid4())
        answer = tool("mcp.call", {"operation_id": state["operation_id"], "input": {
            "request_id": request_id, "profile_id": state["profile_id"], "connection_id": state["connection_id"],
            "tool_name": name, "arguments": arguments if arguments is not None else {"path": "/srv/recollect-demo/hello.txt"},
            "client_session_id": state["client_session_id"], "timeout_seconds": 30}}, error=error)
        if error:
            return answer
        state["calls"][label] = answer["id"]
        save()
        return answer["id"]

    def status(call_id):
        return tool("mcp.status", {"id": call_id, "operation_id": state["operation_id"]})

    def terminal(call_id):
        return wait(lambda: (c if (c := status(call_id))["state"] not in
                    ("queued", "starting", "claimed", "running", "dispatched") else None), "terminal call", 50)

    if "use_granted" not in state:
        denied = queue("denied_before_use", error=True)
        (OUTPUT / "no-use-denial.json").write_text(json.dumps(denied, indent=2) + "\n")
        assert denied["code"] == "mcp_permission_required", "Unexpected no-Use denial code: " + denied["code"]
        checked("reader_without_use_denied", denied["code"])
        for username in (state["caller_user"], env["RECOLLECT_OWNER_USERNAME"]):
            owner.call("PUT", BASE + "/mcp/profiles/" + state["profile_id"] + "/grants", {
                "username": username, "group_name": None,
                "rights": {"use_profile": True, "manage": False, "share": False}})
        state["use_granted"] = True
        save()
    discovered = tool("mcp.discover", {"operation_id": state["operation_id"], "input": {"profile_id": state["profile_id"]}})
    assert discovered["total"] == 3 and not discovered["unavailable_connections"]
    assert {entry["tool"]["name"] for entry in discovered["tools"]} == {t["name"] for t in tools}
    (OUTPUT / "discovery.json").write_text(json.dumps(discovered, indent=2) + "\n")
    checked("approved_three_read_tools_discovered")
    current_profile = next(p for p in caller.call("GET", BASE + "/mcp")["profiles"] if p["id"] == state["profile_id"])
    assert current_profile["rights"] == {"use_profile": True, "manage": False, "share": False}
    caller.call("PUT", BASE + "/mcp/profiles/" + state["profile_id"], {
        "name": "Ubuntu demo files", "description": "", "environment_id": None, "enabled": True,
        "connection_ids": [state["connection_id"]], "base_revision": current_profile["revision"]},
        request_key=str(uuid.uuid4()), expected=403)
    caller.call("PUT", BASE + "/mcp/profiles/" + state["profile_id"] + "/grants", {
        "username": state["caller_user"], "group_name": None,
        "rights": {"use_profile": True, "manage": True, "share": True}}, expected=403)
    checked("reader_manage_and_share_denied")
    success = terminal(queue("read_marker"))
    assert success["state"] == "succeeded", "Real filesystem read failed: " + str(success.get("code"))
    assert state["marker"] in json.dumps(success["result"])
    assert success["device_id"] == state["caller_device_id"] and success["runner_reference"] == reference
    assert not success["observations"] or not any(o.get("source_id") for o in success["observations"])
    (OUTPUT / "read-marker.json").write_text(json.dumps(success, indent=2) + "\n")
    checked("private_file_returned_to_reader_agent")
    listing = terminal(queue("list_directory", "list_directory", {"path": "/srv/recollect-demo"}))
    assert listing["state"] == "succeeded" and "hello.txt" in json.dumps(listing["result"])
    checked("private_directory_listing")
    denied = terminal(queue("outside_root", arguments={"path": "/etc/passwd"}))
    assert denied["state"] == "tool_error" and "outside allowed directories" in json.dumps(denied["result"])
    (OUTPUT / "outside-root-denial.json").write_text(json.dumps(denied, indent=2) + "\n")
    checked("outside_directory_denied")
    rejected = queue("write_tool_denied", "write_file", {"path": "/srv/recollect-demo/no.txt", "content": "test"}, error=True)
    assert rejected["code"] in ("not_found", "forbidden", "invalid_input", "invalid_request")
    checked("unapproved_write_tool_denied", rejected["code"])

    if "offline_queue_and_recovery" not in state["checks"]:
        docker("stop", "--timeout", "15", CONTAINER)
        wait(lambda: next((r for r in owner.call("GET", BASE + "/mcp/private-runners")
                           if r["id"] == state["runner_id"] and not r["available"]), None), "expired offline runner lease", 50)
        offline_id = queue("offline_queued")
        time.sleep(2)
        offline = status(offline_id)
        assert offline["state"] == "queued" and offline["runner_reference"] == reference and not offline["dispatched_at"]
        (OUTPUT / "offline-queued.json").write_text(json.dumps(offline, indent=2) + "\n")
        docker("start", CONTAINER)
        recovered = terminal(offline_id)
        assert recovered["state"] == "succeeded" and state["marker"] in json.dumps(recovered["result"])
        (OUTPUT / "offline-recovered.json").write_text(json.dumps(recovered, indent=2) + "\n")
        checked("offline_queue_and_recovery")
    allowed = terminal(queue("allowed_directories", "list_allowed_directories", {}))
    assert allowed["state"] == "succeeded" and "/srv/recollect-demo" in json.dumps(allowed["result"])
    (OUTPUT / "allowed-directories.json").write_text(json.dumps(allowed, indent=2) + "\n")
    checked("allowed_directory_boundary")
    tool("mcp.release", {"input": {"client_session_id": state["client_session_id"]}})
    if not state.get("task_closed"):
        tool("workspace.close", {"id": state["task_id"]})
        state["task_closed"] = True
    state["runner_online_at_closeout"] = next(r for r in owner.call("GET", BASE + "/mcp/private-runners") if r["id"] == state["runner_id"])["available"]
    state["runtime_image_id"] = docker("inspect", "--format", "{{.Image}}", CONTAINER).strip()
    state["node_version"] = docker("exec", CONTAINER, "node", "--version").strip()
    if fresh:
        assert state["runtime_image_id"] == docker("image", "inspect", "--format", "{{.Id}}", "recollect-private-runner-demo:local").strip()
        checked("fresh_container_saved_os_credential")
    state["ready"] = owner.call("GET", "/health/ready")
    save()
    print("Finished; named demo runner retained online", flush=True)


if __name__ == "__main__":
    main()
