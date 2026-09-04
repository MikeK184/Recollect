#!/usr/bin/env python3
"""Exercise an explicitly selected, owned proof installation with real services."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
import http.cookiejar
import importlib.util
import json
import os
from pathlib import Path
import re
import select
import shlex
import ssl
import subprocess
import time
import urllib.error
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("installation", ROOT / "scripts/install.py")
installation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installation)


def environment(path):
    return dict((line.split("=", 1)[0], shlex.split(line.split("=", 1)[1])[0])
                for line in path.read_text().splitlines() if line and not line.startswith("#"))


def process_environment():
    return {key: value for key, value in os.environ.items() if key in (
        "PATH", "HOME", "USER", "TMPDIR", "DOCKER_HOST", "DOCKER_CONTEXT", "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH")}


def eventually(check, label, seconds=60):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if check():
            return
        time.sleep(0.25)
    raise AssertionError("Timed out: " + label)


class ApiFailure(AssertionError):
    def __init__(self, method, path, status, expected, code):
        self.status, self.code = status, code
        super().__init__(f"{method} {path}: status {status}, expected {expected}, code {code}")


class Session:
    def __init__(self, origin, ca=None):
        self.origin = origin
        self.csrf = None
        self.opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({}),
            urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()),
            urllib.request.HTTPSHandler(context=ssl.create_default_context(cafile=ca)))

    def call(self, method, path, body=None, *, expected=200, origin=None, request_key=None):
        headers = {"Content-Type": "application/json", "Origin": origin or self.origin}
        if request_key is not None:
            headers["Idempotency-Key"] = request_key
        if self.csrf:
            headers["X-CSRF-Token"] = self.csrf
        request = urllib.request.Request(self.origin + path, method=method, headers=headers,
            data=json.dumps(body).encode() if body is not None else None)
        try:
            response = self.opener.open(request, timeout=20)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            payload = response.read()
            if response.status != expected:
                try:
                    code = json.loads(payload).get("code", "unavailable")
                except (ValueError, AttributeError):
                    code = "unavailable"
                # Retain only the product's bounded error code, never a body,
                # echoed user input, credential or driver diagnostic.
                if not isinstance(code, str) or not re.fullmatch(r"[a-z_]{1,64}", code):
                    code = "unavailable"
                raise ApiFailure(method, path, response.status, expected, code)
            return json.loads(payload) if payload else None

    def login(self, username, password):
        self.csrf = self.call("POST", "/api/auth/login", {"username": username, "password": password})["csrf_token"]


def verify(name, ca):
    assert name.startswith("proof-"), "Use an explicitly named proof installation"
    saved = installation.load(name)
    installation.configuration(saved)
    private = installation.directory(name)
    runtime = environment(private / "runtime.env")
    assert not runtime.get("OPENAI_API_KEY"), "This fixture must not use a paid model key"
    assert "VAULT_TOKEN" not in runtime
    proof = ROOT / ".cache/installation-runtime" / (name + "-" + uuid.uuid4().hex[:10])
    proof.mkdir(mode=0o700, parents=True)
    counter = 0

    def compose(arguments, *, expected=0, timeout=330):
        nonlocal counter
        counter += 1
        result = subprocess.run(installation.compose_command(saved) + arguments,
            env=process_environment(), cwd=ROOT, capture_output=True, text=True, timeout=timeout)
        (proof / f"compose-{counter}.log").write_text(result.stdout + result.stderr)
        assert result.returncode == expected, f"Owned Compose operation failed; inspect {proof / f'compose-{counter}.log'}"
        return result.stdout

    def sql(statement):
        result = subprocess.run(installation.compose_command(saved) + ["exec", "-T", "postgres",
            "psql", "-X", "-qAt", "-U", "recollect_admin", "-d", "recollect", "-v", "ON_ERROR_STOP=1"],
            input=statement, text=True, env=process_environment(), capture_output=True, timeout=15)
        assert result.returncode == 0, "Owned fixture SQL failed"
        return result.stdout.strip()

    def service_state(role):
        output = compose(["ps", "--all", "--format", "json", role])
        rows = json.loads(output) if output.lstrip().startswith("[") else [json.loads(line) for line in output.splitlines() if line.strip()]
        return next(row for row in rows if row["Service"] == role)

    @contextmanager
    def table_lock(table):
        assert table in ("source_chunks", "sources")
        command = installation.compose_command(saved) + ["exec", "-T", "postgres", "psql", "-X", "-qAt",
            "-U", "recollect_admin", "-d", "recollect", "-v", "ON_ERROR_STOP=1"]
        with (proof / (table + "-lock.log")).open("w") as errors:
            child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=errors, env=process_environment(), text=True, bufsize=1)
            try:
                child.stdin.write(f"BEGIN; LOCK TABLE {table} IN ACCESS EXCLUSIVE MODE; SELECT 'locked';\n")
                child.stdin.flush()
                assert select.select([child.stdout], [], [], 10)[0], "Fixture lock was not acquired"
                assert child.stdout.readline().strip() == "locked"
                yield
            finally:
                if child.poll() is None:
                    child.stdin.write("COMMIT;\n\\q\n")
                    child.stdin.flush()
                    child.stdin.close()
                    child.wait(timeout=15)
                child.stdout.close()

    owner = Session(saved["origin"], ca)
    owner.call("GET", "/health/ready")
    owner.login(runtime["RECOLLECT_OWNER_USERNAME"], runtime["RECOLLECT_OWNER_PASSWORD"])
    initial_instance = owner.call("GET", "/api/operations")["http"]["instance_id"]
    marker = "OwnedInstallationProof " + uuid.uuid4().hex
    brain = owner.call("POST", "/api/brains", {"name": marker})
    base = "/api/brains/" + str(uuid.UUID(brain["id"]))
    member_name, member_password = "proof-" + uuid.uuid4().hex[:10], uuid.uuid4().hex
    invitation = owner.call("POST", "/api/team/invitations", {"username": member_name})
    member = Session(saved["origin"], ca)
    member.call("POST", "/api/auth/enroll", {"token": invitation["token"], "password": member_password})
    member.login(member_name, member_password)
    member.call("GET", base, expected=404)
    private_brain = member.call("POST", "/api/brains", {"name": "Member private " + uuid.uuid4().hex[:8]})
    owner.call("GET", "/api/brains/" + private_brain["id"], expected=404)
    member.call("GET", "/api/operations", expected=403)
    owner.call("PUT", base + "/grants/" + private_brain["owner_id"], {"role": "reader"})
    assert member.call("GET", base)["role"] == "reader"
    owner.call("POST", "/api/brains", {"name": "Invalid origin control"},
               expected=403, origin="https://untrusted.example.invalid")
    if saved["mode"] == "shared":
        assert ca, "Select the owned shared fixture CA"
        try:
            Session(saved["origin"]).call("GET", "/health/ready")
        except urllib.error.URLError as error:
            assert isinstance(error.reason, ssl.SSLCertVerificationError)
        else:
            raise AssertionError("Untrusted TLS client was admitted")

    source_input = {"title": marker, "media_type": "text/plain", "retain_content": True,
                    "content": marker + " records the synthetic service on port 8080.\n"}
    compose(["stop", "--timeout", "90", "worker"])
    source = owner.call("POST", base + "/sources", source_input)
    version = str(uuid.UUID(source["version"]["id"]))
    state_sql = f"SELECT state FROM jobs WHERE target_id='{version}' AND kind='source.process'"
    assert sql(state_sql) == "queued"
    assert owner.call("GET", "/api/operations")["jobs_queued"] > 0
    # Hold actual source materialization while SIGTERM begins worker shutdown.
    with table_lock("source_chunks"):
        compose(["start", "worker"])
        eventually(lambda: sql(state_sql) == "running", "owned worker claimed durable source")
        stopped = subprocess.Popen(installation.compose_command(saved) + ["stop", "--timeout", "90", "worker"],
            env=process_environment(), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(1)
        assert stopped.poll() is None, "Worker did not drain its blocked in-flight job"
    assert stopped.wait(timeout=95) == 0
    assert sql(state_sql) == "succeeded", "SIGTERM lost the in-flight source job"
    compose(["start", "worker"])
    source_path = base + "/sources/" + source["id"] + "/versions/" + version
    assert owner.call("GET", source_path)["content"] == source_input["content"]

    # SIGINT must let an already accepted API write finish before the process exits.
    with ThreadPoolExecutor(max_workers=1) as executor:
        with table_lock("sources"):
            accepted = executor.submit(owner.call, "POST", base + "/sources", source_input)
            eventually(lambda: int(sql("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND usename='recollect_app' AND wait_event_type='Lock' AND query LIKE '%INSERT INTO sources%'")) > 0,
                       "API write entered its database transaction")
            compose(["kill", "--signal", "SIGINT", "api"])
            time.sleep(0.5)
            assert not accepted.done(), "API abandoned an accepted write on SIGINT"
        assert accepted.result(timeout=20)["version"]["retained"] is True
    eventually(lambda: service_state("api")["State"] == "exited", "API drained before restart")
    assert service_state("api")["ExitCode"] == 0
    compose(["up", "-d", "--no-deps", "--wait", "--wait-timeout", "120", "api"])
    assert owner.call("GET", source_path)["content"] == source_input["content"]

    # A selected dependency outage changes actual readiness, without losing data.
    compose(["stop", "--timeout", "90", "neo4j"])
    owner.call("GET", "/health/ready", expected=503)
    owner.call("GET", "/health/live")
    status = owner.call("GET", "/api/status")
    assert not status["ready"] and not status["dependencies"][1]["connected"]
    compose(["up", "-d", "--wait", "--wait-timeout", "180", "neo4j"])
    owner.call("GET", "/health/ready")

    # Startup drains application roles before rerunning migration. A bad operator
    # credential must leave them stopped, not serving an unverified schema.
    operator = private / "operator.env"
    original_operator = operator.read_text()
    try:
        operator.write_text("DATABASE_ADMIN_URL='postgres://recollect_admin:invalid-owned-fixture@postgres:5432/recollect'\n")
        result = subprocess.run(["python3", str(ROOT / "scripts/install.py"), "start", name],
            env=process_environment(), capture_output=True, text=True, timeout=330)
        (proof / "migration-failure.log").write_text(result.stdout + result.stderr)
        assert result.returncode != 0
        for role in ("api", "worker") + (("proxy",) if saved["mode"] == "shared" else ()):
            assert service_state(role)["State"] != "running"
    finally:
        operator.write_text(original_operator)
    restarted = subprocess.run(["python3", str(ROOT / "scripts/install.py"), "start", name],
        env=process_environment(), capture_output=True, text=True, timeout=330)
    (proof / "restart.log").write_text(restarted.stdout + restarted.stderr)
    assert restarted.returncode == 0
    assert owner.call("GET", source_path)["content"] == source_input["content"]
    final = owner.call("GET", "/api/operations")
    assert final["http"]["instance_id"] != initial_instance
    assert marker not in json.dumps(final), "Operational metrics leaked request content"
    assert sql("SELECT count(*) FROM model_requests") == "0"
    # Inspect actual service output privately. Only a fixed assertion leaves this
    # process; credentials and source canaries must never appear in the report.
    logged = compose(["logs", "--no-color", "api", "worker"] +
                     (["proxy"] if saved["mode"] == "shared" else []))
    for canary in [marker, member_password, runtime["RECOLLECT_OWNER_PASSWORD"],
                   runtime["NEO4J_PASSWORD"], runtime["DATABASE_URL"]]:
        assert canary not in logged, "Installation service logs leaked a private input"

    # Private, ignored inputs for the subsequent actual browser/native pairing proof.
    installation.private_write(proof / "browser-credentials.json", json.dumps({
        "instance": name, "origin": saved["origin"], "ca": ca,
        "owner": {"username": runtime["RECOLLECT_OWNER_USERNAME"], "password": runtime["RECOLLECT_OWNER_PASSWORD"]},
        "member": {"username": member_name, "password": member_password},
        "brain": brain, "member_brain": private_brain}))
    report = {"instance": name, "mode": saved["mode"], "origin": saved["origin"],
        "owner_member_isolation": True, "worker_sigterm_drained": True,
        "api_sigint_drained": True, "dependency_outage_detected": True,
        "failed_migration_kept_application_stopped": True, "restart_preserved_evidence": True,
        "private_inputs_absent_from_service_logs": True,
        "model_requests": 0, "proof_directory": str(proof)}
    (proof / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("instance")
    parser.add_argument("--ca")
    arguments = parser.parse_args()
    verify(arguments.instance, arguments.ca)
