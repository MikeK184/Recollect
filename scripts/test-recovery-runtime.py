#!/usr/bin/env python3
"""Staged recovery acceptance against explicitly named, owned proof installations.

Private state permits inspection between stages; credentials never enter reports.
Run only with generated proof credentials and a repository-owned SFTP fixture.
"""
import argparse
import copy
from datetime import timedelta
import importlib.util
import json
import os
from pathlib import Path
import platform
import signal
import shutil
import subprocess
import tempfile
import time
import uuid

import install
import recovery
import recovery_archive as archive
import recovery_installation as runtime
from recovery_transport import decrypt, encrypt, execute, private_json

spec = importlib.util.spec_from_file_location("installation_proof", Path(__file__).with_name("test-installation-runtime.py"))
proof = importlib.util.module_from_spec(spec)
spec.loader.exec_module(proof)
EMPTY = {"repository_ids": [], "area_ids": [], "environment_id": None}


def identifier():
    return str(uuid.uuid4())


class Device(proof.Session):
    def __init__(self, origin, token):
        super().__init__(origin)
        self.opener.addheaders = [("Authorization", "Bearer " + token)]


class Drill:
    def __init__(self, directory):
        self.root = Path(directory).resolve()
        if self.root.parent != (install.ROOT / ".cache").resolve() or self.root.is_symlink():
            raise ValueError("Choose a direct private recovery fixture directory under .cache")
        self.root.mkdir(mode=0o700, exist_ok=True)
        self.state_path = self.root / "state.json"
        self.state = archive.json_file(self.state_path) if self.state_path.exists() else {}

    def save(self):
        private_json(self.state_path, self.state)

    def selected(self, role):
        saved = install.load(self.state[role])
        assert saved["name"].startswith("proof-recovery-"), "Select an owned recovery proof installation"
        env = proof.environment(install.directory(saved["name"]) / "runtime.env")
        assert not env.get("OPENAI_API_KEY") and "VAULT_TOKEN" not in env
        install.configuration(saved)
        return saved

    def owner(self, role="source"):
        saved = self.selected(role)
        env = proof.environment(install.directory(saved["name"]) / "runtime.env")
        session = proof.Session(saved["origin"])
        session.login(env["RECOLLECT_OWNER_USERNAME"], env["RECOLLECT_OWNER_PASSWORD"])
        return session

    def record(self, field, value):
        self.state[field] = value
        self.save()
        return value

    def initialize(self, source, target, fixture):
        assert not self.state, "Existing drill state is preserved; select its next stage"
        self.state.update(source=source, target=target, fixture=str(Path(fixture).resolve()))
        self.save()
        self.selected("source")
        runtime.fresh(self.selected("target"))
        configured = archive.json_file(self.state["fixture"])
        assert configured["container"].startswith("recollect-recovery-")
        # Trust comes from the locally generated server public key, never an
        # unauthenticated network scan. Add the explicit container-facing alias.
        hosts = Path(configured["remote"]["known_hosts_file"])
        public = (Path(configured["root"]) / "server-key.pub").read_text().split()
        alias = f"[host.docker.internal]:{configured['remote']['port']} {public[0]} {public[1]}\n"
        if alias not in hosts.read_text():
            with hosts.open("a") as output:
                output.write(alias)
        arch = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "amd64"}[platform.machine()]
        age = install.ROOT / f".cache/recovery-tools/age-v1.3.2-{platform.system().lower()}-{arch}/age"
        identity = self.root / "identity"
        execute([age.with_name("age-keygen"), "-o", identity])
        recipient = execute([age.with_name("age-keygen"), "-y", identity]).decode().strip()
        cfg = {"backup_dir": str(self.root / "backups"), "age_binary": str(age),
               "recipients": [recipient], "remote": configured["remote"]}
        private_json(self.root / "input.json", cfg)
        recovery.configure(self.selected("source"), self.root / "input.json")
        self.record("identity", str(identity))
        recovery.enable_mirror(self.selected("source"), recovery.config(self.selected("source")), "host.docker.internal")
        self.record("mirror_connected", True)

    def fixture(self):
        binary = install.ROOT / ".cache/mcp-hosts/target/debug/examples/mcp-fixture"
        assert binary.is_file(), "Build the Linux mcp-fixture example with the owned host image first"
        external = self.root / "connector"
        external.mkdir(mode=0o777, exist_ok=True)
        external.chmod(0o777)  # Synthetic effects only; host parent is private.
        for role in ("source", "target"):
            selected = self.selected(role)
            path = install.directory(selected["name"]) / "overrides.yaml"
            override = archive.json_file(path)
            for service in ("api", "worker"):
                override["services"][service]["volumes"] = [str(binary) + ":/run/recovery-mcp-fixture:ro",
                                                            str(external) + ":/recovery-proof"]
            private_json(path, override)
            install.configuration(selected)

    def effects(self):
        return (self.root / "connector/effects.txt").read_text().splitlines()

    def proposal(self, version, subject, value):
        return {"content": {"kind": "claim", "subject": subject, "predicate": "configuration",
            "value": value, "rationale": "Controlled recovery evidence.", "selection": EMPTY,
            "manifest_revision_id": None, "validity": {"kind": "interval", "from": "2026-01-01T00:00:00Z",
            "to": None, "precision": "second"}, "freshness": "current", "operational": "declared",
            "observed_at": None, "observation": "", "supports": [{"kind": "source_version", "id": version,
            "line_from": 1, "line_to": 1}]}}

    def source(self, owner, title):
        return owner.call("POST", self.state["base"] + "/sources", {
            "title": title, "media_type": "text/plain", "content": title + " uses port 8080.\n",
            "retain_content": True})

    def review(self, owner, claim, action, content=None):
        result = owner.call("POST", self.state["base"] + "/claims/" + claim["claim_id"] + "/review", {
            "base_revision": claim["id"], "action": action, "reason": "Controlled recovery review.", "content": content})
        return result["claims"][0]["revision"]

    def corpus(self):
        assert "base" not in self.state, "Corpus is already partially or fully created; inspect its private state"
        owner = self.owner()
        brain = self.record("brain", owner.call("POST", "/api/brains", {"name": "Recovery acceptance"}))
        base = self.record("base", "/api/brains/" + brain["id"])
        member_name, password = "recovery-" + uuid.uuid4().hex[:10], uuid.uuid4().hex
        invitation = owner.call("POST", "/api/team/invitations", {"username": member_name})
        member = proof.Session(owner.origin)
        member.call("POST", "/api/auth/enroll", {"token": invitation["token"], "password": password})
        member.login(member_name, password)
        private_brain = member.call("POST", "/api/brains", {"name": "Independent private recovery scope"})
        owner.call("GET", "/api/brains/" + private_brain["id"], expected=404)
        owner.call("PUT", base + "/grants/" + private_brain["owner_id"], {"role": "reader"})
        self.state.update(member={"username": member_name, "password": password}, member_brain=private_brain)
        self.save()
        retained = self.record("retained", self.source(owner, "RetainedIndependentCobalt"))
        proposed = owner.call("POST", base + "/claims", self.proposal(retained["version"]["id"], "Recovery service", "8080"))
        accepted = self.review(owner, proposed, "accept")
        changed = copy.deepcopy(accepted["content"])
        changed["value"] = "8081"
        corrected = self.record("corrected", self.review(owner, accepted, "correct", changed))
        self.record("historical", accepted)
        rejected = owner.call("POST", base + "/claims", self.proposal(retained["version"]["id"], "Rejected recovery service", "invalid"))
        self.record("rejected", self.review(owner, rejected, "reject"))
        settings = owner.call("GET", base + "/capture/policy")
        settings["policy"].update(enabled=True, managed_tools=True)
        owner.call("PUT", base + "/capture/policy", {"base_change": settings["change_id"], "policy": settings["policy"]})
        pairing = owner.call("POST", "/api/devices/pairings", {"name": "Recovery proof companion"})
        owner.call("POST", "/api/devices/pairings/" + pairing["user_code"] + "/approve", {"approve": True})
        paired = owner.call("POST", "/api/devices/pairings/poll", {"device_code": pairing["device_code"]})
        owner.call("POST", "/api/devices/pairings/finish", {"device_code": pairing["device_code"]}, expected=204)
        self.record("device", paired)
        device = Device(owner.origin, paired["token"])
        task = device.call("POST", base + "/workspace/tasks", {"label": "Recovery capture", "selection": EMPTY})
        operation = device.call("POST", base + "/workspace/tasks/" + task["task"]["id"] + "/operations", {"kind": "capture"})
        bound = device.call("POST", base + "/capture/bindings", {"id": identifier(), "operation_id": operation["id"], "host": "codex", "host_version": "0.154.0"})
        self.state["capture"] = []
        for label in ["EraseCapturedZircon", "RetainedCapturedAmber"]:
            event = {"id": identifier(), "binding_id": bound["id"], "event": {
                "host_event": "UserPromptSubmit", "host_session_id": "recovery-proof", "turn_id": label,
                "agent_id": None, "tool_use_id": None, "tool_name": None, "kind": "prompt", "outcome": "reported",
                "content": label + " records a synthetic service.", "coverage": ["partial_host_coverage"],
                "captured_at": recovery.now().isoformat()}}
            self.state["capture"].append({"input": event, "receipt": device.call("POST", base + "/capture/events", event)})
            self.save()
        claim = owner.call("POST", base + "/claims", self.proposal(self.state["capture"][0]["receipt"]["source_version_id"], "EraseDerivedZircon", "captured"))
        self.record("erase_claim", self.review(owner, claim, "accept"))
        self.repository(device, owner)
        self.managed(owner)
        self.graphs(owner)
        self.record("corpus_complete", True)

    def graphs(self, owner):
        base = self.state["base"]
        for kind, snapshot in [("knowledge", None), ("repository", self.state["snapshot"]["snapshot"]["id"])]:
            generation = owner.call("POST", base + "/graph/rebuild", {"kind": kind, "snapshot_id": snapshot})
            proof.eventually(lambda: any(g["id"] == generation["id"] and g["state"] == "ready"
                for g in owner.call("GET", base + "/graph")["generations"]), "source graph generation", seconds=120)

    def repository(self, device, owner):
        base = self.state["base"]
        device.call("POST", base + "/workspace/checkouts", {"workspace_root": "/recovery-fixture", "complete": False, "notes": [],
            "checkouts": [{"local_path": "/recovery-fixture/repo", "origin": "https://example.test/team/recovery.git", "head": None,
                           "branch": "main", "dirty": False, "status": "available"}]})
        repo = device.call("GET", base + "/workspace")["repositories"][0]["id"]
        self.record("repository_id", repo)
        selection = {**EMPTY, "repository_ids": [repo]}
        task = device.call("POST", base + "/workspace/tasks", {"label": "Recovery repository", "selection": selection})
        operation = device.call("POST", base + "/workspace/tasks/" + task["task"]["id"] + "/operations", {"kind": "capture"})
        owner.call("PUT", base + "/repositories/policy", {"allow_file_content": True})
        content = "pub fn recovery_entry() { recovery_end(); }\npub fn recovery_end() {}\n"
        facts = [{"id": "entry", "kind": "function", "name": "recovery_entry", "file": "src/lib.rs", "line": 1,
                  "relations": [{"kind": "calls", "target": "end", "target_id": "end"}]},
                 {"id": "end", "kind": "function", "name": "recovery_end", "file": "src/lib.rs", "line": 2}]
        publication = {"publication_id": identifier(), "operation_id": operation["id"], "origin": "example.test/team/recovery",
            "revision": "a" * 40, "branch": "main", "dirty": False, "captured_at": recovery.now().isoformat(),
            "adapter": "enola-committed", "adapter_build": "fixture", "extractor_version": "fixture",
            "settings": {"retained_files": ["src/lib.rs"]}, "files": [{"path": "src/lib.rs", "object_id": "b" * 40,
            "mode": "100644", "size": len(content.encode()), "status": "materialized", "extraction": "facts_emitted", "content": content}],
            "facts": facts, "insights": [], "receipt": {"enola_version": "fixture", "extractor_version": "fixture", "fact_count": 2, "quality": {"parse_errors": 0}}}
        self.record("snapshot", device.call("POST", base + "/repositories/" + repo + "/snapshots", publication))
        snapshot = self.state["snapshot"]["snapshot"]["id"]
        proof.eventually(lambda: len(owner.call("GET", base + "/repository-snapshots/" + snapshot + "/facts")["items"]) == 2,
                         "repository materialization", seconds=90)

    def managed(self, owner):
        self.fixture()
        manifest = json.loads((install.ROOT / "crates/server/tests/fixtures/mcp-runtime.json").read_text())
        manifest.update(key="recovery-fixture", name="Recovery MCP fixture", command="/run/recovery-mcp-fixture")
        approval = self.root / "mcp-manifest.json"
        private_json(approval, manifest)
        approval.chmod(0o644)
        runtime.command(self.selected("source"), ["run", "--rm", "--no-deps", "-T", "--volume", str(approval) + ":/run/recovery-fixture.json:ro",
            "migrate", "mcp-definition-import", "/run/recovery-fixture.json"])
        base = self.state["base"] + "/mcp"
        connection = owner.call("POST", base + "/connections", {"name": "Recovery fixture", "description": "Owned proof endpoint",
            "definition_key": "recovery-fixture", "target": "owned-recovery-fixture",
            "placement": "central", "runner_reference": None, "credential_alias": None, "environment_id": None,
            "configuration": {"marker": "/recovery-proof/effects.txt"}, "enabled": True, "base_revision": None})
        profile = owner.call("POST", base + "/profiles", {"name": "Recovery proof profile", "description": "Owned proof",
            "environment_id": None, "connection_ids": [connection["summary"]["id"]], "enabled": True, "base_revision": None})
        owner.call("PUT", base + "/profiles/" + profile["profile"]["id"] + "/grants", {
            "username": "owner", "group_name": None, "rights": {"use_profile": True, "manage": True, "share": True}})
        self.state.update(connection=connection, profile=profile)
        self.save()
        runtime.command(self.selected("source"), ["up", "--no-deps", "-d", "--wait", "api", "worker"])
        result = self.call(owner, "inspect", {"text": "ManagedRetainedSapphire"})
        self.record("inspect", result)
        proof.eventually(lambda: owner.call("GET", base + "/calls/" + result["id"] + "/observations")["items"][0]["state"] == "published",
                         "managed observation publication", seconds=60)
        self.record("managed_observation", owner.call("GET", base + "/calls/" + result["id"] + "/observations")["items"][0])

    def call(self, owner, tool, arguments, wait=True):
        body = {"request_id": identifier(), "profile_id": self.state["profile"]["profile"]["id"],
            "connection_id": self.state["connection"]["summary"]["id"], "tool_name": tool, "arguments": arguments,
            "environment_id": None, "operation_id": None, "client_session_id": identifier(), "timeout_seconds": 30}
        call = owner.call("POST", self.state["base"] + "/mcp/calls", body)
        if tool == "effect":
            self.record("effect_input", body)
        return self.finished(owner, call["id"]) if wait else call

    def finished(self, owner, call):
        endpoint = self.state["base"] + "/mcp/calls/" + call
        proof.eventually(lambda: owner.call("GET", endpoint)["state"] not in ("queued", "starting", "running"), "MCP completion", seconds=90)
        result = owner.call("GET", endpoint)
        assert result["state"] in ("succeeded", "unknown"), "Managed fixture failed; inspect its private state"
        return result

    def backup(self):
        assert self.state.get("corpus_complete") and "backup" not in self.state
        source, owner = self.selected("source"), self.owner()
        effect = self.record("effect", self.call(owner, "effect", {}))
        assert effect["state"] == "unknown"
        assert self.effects().count("effect " + effect["id"]) == 1
        self.record("effect_observed", True)
        runtime.command(source, ["stop", "--timeout", "90", "worker"])
        # Only the crash-state model row is seeded: no paid provider is called.
        # All evidence, review, graph and managed operations use product APIs.
        model = self.record("model_request_id", identifier())
        policy = identifier()
        brain, actor = self.state["brain"]["id"], self.state["brain"]["owner_id"]
        runtime.sql(source, f"INSERT INTO model_policies(id,brain_id,policy,created_by) VALUES('{policy}','{brain}','{{}}','{actor}'); "
            f"INSERT INTO model_requests(id,brain_id,actor_id,operation_id,policy_id,purpose,model,prompt_label,schema_label,state,call_token,reserved_tokens,charged_tokens) "
            f"VALUES('{model}','{brain}','{actor}','{identifier()}','{policy}','extraction','fixture','fixture','fixture','running','{identifier()}',100,100);")
        cfg = recovery.config(source)
        self.record("backup", recovery.create(source, cfg))
        assert self.state["backup"]["local"] == self.state["backup"]["remote"] == "complete"
        assert next(s for s in runtime.services(source) if s["Service"] == "worker")["State"] != "running"
        runtime.command(source, ["start", "worker"])

    def erase(self):
        assert self.state.get("effect_observed") and "journals" not in self.state
        owner, source, base = self.owner(), self.selected("source"), self.state["base"]
        target = {"kind": "source", "id": self.state["capture"][0]["receipt"]["source_id"]}
        preview = owner.call("POST", base + "/erasures/preview", target)
        erasure = self.record("erasure", owner.call("POST", base + "/erasures", {"target": target, "eligibility_epoch": preview["eligibility_epoch"]}))
        erasure_id = archive.identifier(erasure["id"])
        proof.eventually(lambda: runtime.sql(source, f"SELECT state FROM privacy_requests WHERE id='{erasure_id}'") == "complete",
                         "post-checkpoint mirrored erasure", seconds=120)
        cfg = recovery.config(source)
        runtime.command(source, ["stop", "--timeout", "90", "api", "worker"])
        self.record("journals", recovery.fetch_journals(cfg, self.state["identity"]))
        assert self.state["journals"]["erasure_entries"] >= 1
        self.record("source_stopped", True)

    def restore(self, resume=False):
        assert self.state.get("source_stopped")
        source, target = self.selected("source"), self.selected("target")
        cfg = recovery.config(source)
        root = Path(cfg["backup_dir"])
        self.record("restore_started_at", recovery.now().isoformat())
        self.record("restore", recovery.restore(target, cfg, Path(self.state.get("downloaded_archive",
            root / (self.state["backup"]["backup_id"] + ".tar.age"))),
            root / self.state["journals"]["journal"], self.state["identity"], resume=resume))

    def failures(self):
        """Use the real encrypted corpus, CLI preflight and independent target volumes."""
        assert self.state.get("source_stopped")
        source, target = self.selected("source"), self.selected("target")
        cfg = recovery.config(source)
        root = Path(cfg["backup_dir"])
        encrypted = root / (self.state["backup"]["backup_id"] + ".tar.age")
        journals = root / self.state["journals"]["journal"]
        identity = Path(self.state["identity"])
        failures = []
        with tempfile.TemporaryDirectory(prefix="failure-inputs-", dir=self.root) as temporary:
            temporary = Path(temporary)

            def refused(label, bundle=encrypted, journal=journals, key=identity, destination=target):
                args = ["python3", str(install.ROOT / "scripts/recovery.py"), "restore", destination["name"],
                    "--config", str(install.directory(source["name"]) / "recovery.json"), "--archive", str(bundle),
                    "--journals", str(journal), "--identity", str(key)]
                result = subprocess.run(args, capture_output=True, env=runtime.environment(), timeout=120)
                assert result.returncode != 0, "Invalid restore was admitted: " + label
                attempt = archive.json_file(install.directory(destination["name"]) / "recovery-attempt.json")
                assert attempt["state"] == "failed"
                runtime.fresh(target)
                failures.append(label)

            wrong = temporary / "wrong-identity"
            execute([Path(cfg["age_binary"]).with_name("age-keygen"), "-o", wrong])
            refused("wrong_identity", key=wrong)
            damaged = bytearray(encrypted.read_bytes())
            damaged[-1] ^= 1
            corrupt = temporary / "corrupt.age"
            corrupt.write_bytes(damaged)
            refused("corrupt_ciphertext", bundle=corrupt)
            refused("missing_journal", journal=temporary / "missing.age")
            for kind, selected in (("bundle", encrypted), ("journal", journals)):
                decrypt(cfg["age_binary"], identity, selected, temporary / (kind + ".tar"))
                (temporary / kind).mkdir(mode=0o700)
                archive.extract(temporary / (kind + ".tar"), temporary / kind,
                    allowed={"manifest.json", "database.dump", "app"} if kind == "bundle" else {"journal-inventory.json", "erasure-journal"})

            def variant(label, kind, change):
                directory = temporary / label
                shutil.copytree(temporary / kind, directory)
                change(directory)
                archive.pack(directory, temporary / (label + ".tar"))
                target_file = temporary / (label + ".age")
                encrypt(cfg["age_binary"], cfg["recipients"], temporary / (label + ".tar"), target_file)
                refused(label, **({"bundle": target_file} if kind == "bundle" else {"journal": target_file}))

            def metadata(directory, file, change):
                value = archive.json_file(directory / file)
                change(value)
                private_json(directory / file, value)

            variant("missing_artifact", "bundle", lambda p: next((p / "app/artifacts").rglob("*.txt")).unlink())
            variant("unknown_migration", "bundle", lambda p: metadata(p, "manifest.json", lambda v: v["migrations"].append("999_future")))
            variant("wrong_journal_owner", "journal", lambda p: metadata(p, "journal-inventory.json", lambda v: v.update(installation_id=identifier())))
            variant("older_journal", "journal", lambda p: metadata(p, "journal-inventory.json", lambda v: v.update(exported_at="2020-01-01T00:00:00Z")))
            installation = cfg["installation_id"]
            entry = archive.json_file(temporary / "journal/journal-inventory.json")["entries"][0]["id"]
            variant("missing_journal_entry", "journal", lambda p: (p / "erasure-journal" / installation / (entry + ".json")).unlink())
            before = runtime.inventory(source)
            refused("nonempty_target_preserved", destination=source)
            after = runtime.inventory(source)
            assert before["counts"] == after["counts"] and before["journal_entries"] == after["journal_entries"]
        downloaded = self.root / "downloaded"
        downloaded.mkdir(mode=0o700)
        private_json(downloaded / "owner.json", {"installation_id": cfg["installation_id"], "kind": "recollect-recovery"})
        received = recovery.fetch(target, {**cfg, "backup_dir": str(downloaded)}, self.state["backup"]["backup_id"], identity)
        assert received["state"] == "complete"
        archive_file = downloaded / (received["backup_id"] + ".tar.age")
        assert archive_file.read_bytes() == encrypted.read_bytes()
        self.record("downloaded_archive", str(archive_file))
        self.record("failure_controls", failures)
        private_json(self.root / "failure-report.json", {"refused_before_volume_creation": failures, "actual_download_authenticated": True})

    def publication(self):
        source = self.selected("source")
        assert not any(row["Service"] in ("api", "worker") and row["State"] == "running" for row in runtime.services(source))
        cfg = recovery.config(source)
        connection = recovery.remote(cfg)
        installation = cfg["installation_id"]
        journals_before = connection.list(installation, "journal")
        extra = recovery.create(source, cfg)
        extra_id = extra["backup_id"]
        root = Path(cfg["backup_dir"])
        connection.remove(installation, "backups/" + extra_id + ".tar.age")
        with tempfile.TemporaryDirectory(prefix="transport-fault-", dir=self.root) as temporary:
            temporary = Path(temporary)
            hosts = temporary / "untrusted-hosts"
            hosts.write_text("")
            untrusted = {**cfg, "remote": {**cfg["remote"], "known_hosts_file": str(hosts)}}
            try:
                recovery.transfer(untrusted, extra_id)
            except ValueError:
                pass
            else:
                raise AssertionError("Untrusted transport was admitted")
            assert archive.json_file(root / (extra_id + ".json"))["remote"] == "failed"
            # Execute the actual SFTP flush, then kill this owned helper before
            # the rename. The real transport leaves only the partial identity.
            wrapper = temporary / "interrupt-sftp"
            wrapper.write_text("#!/usr/bin/env python3\n" + '''import os, signal, subprocess, sys, tempfile
from pathlib import Path
args=sys.argv[1:]; index=args.index('-b')+1
batch=Path(args[index]).read_text()
if '\\nrename ' not in batch:
    raise SystemExit(subprocess.run(['/usr/bin/sftp']+args).returncode)
with tempfile.TemporaryDirectory() as directory:
    path=Path(directory)/'commands'; path.write_text(batch.split('\\nrename ')[0]+'\\n')
    args[index]=str(path)
    result=subprocess.run(['/usr/bin/sftp']+args)
if result.returncode:
    raise SystemExit(result.returncode)
os.kill(os.getpid(), signal.SIGKILL)
''')
            wrapper.chmod(0o700)
            # The wrapper uses the installed Python through its absolute path;
            # production SFTP's cleared PATH need not contain a developer Python.
            import sys
            wrapper.write_text(wrapper.read_text().replace("#!/usr/bin/env python3", "#!" + sys.executable, 1))
            try:
                recovery.transfer({**cfg, "sftp_binary": str(wrapper)}, extra_id)
            except ValueError:
                pass
            else:
                raise AssertionError("Interrupted upload was marked complete")
        names = connection.list(installation, "backups")
        assert extra_id + ".tar.age" not in names
        assert any(name.startswith(extra_id + ".tar.age.pending-") for name in names)
        recovery.transfer(cfg, extra_id)
        assert extra_id + ".tar.age" in connection.list(installation, "backups")
        # Age only an owned backup record to exercise expiry without changing
        # the machine clock. The retained original and independent journal stay.
        old = archive.json_file(root / (extra_id + ".json"))
        old["expires_at"] = (recovery.now() - timedelta(seconds=1)).isoformat()
        private_json(root / (extra_id + ".json"), old)
        pruned = recovery.prune(source, cfg)
        assert pruned["pruned"] == [extra_id]
        assert not (root / (extra_id + ".tar.age")).exists()
        assert not any(name.startswith(extra_id) for name in connection.list(installation, "backups"))
        original = self.state["backup"]["backup_id"]
        assert (root / (original + ".tar.age")).is_file()
        assert original + ".tar.age" in connection.list(installation, "backups")
        assert connection.list(installation, "journal") == journals_before
        assert all((root / name).is_file() for name in (self.state["journals"]["journal"], "journal-" + extra_id + ".tar.age"))
        scheduled = recovery.scheduled(source, cfg)
        assert scheduled["state"] == "complete" and recovery.listing(source, cfg)["checkpoint_within_24_hours"]
        report = {"untrusted_host_failed": True, "real_upload_interrupted_before_rename": True,
            "partial_not_complete": True, "retry_published_complete": True, "expired_backup_and_partial_pruned": True,
            "independent_journals_and_retained_backup_preserved": True, "scheduled_entrypoint_passed": True,
            "global_schedule_installed": False, "expiry_fixture": "owned_record_deadline_aged"}
        private_json(self.root / "publication-report.json", report)
        self.record("publication_controls", report)

    def rollback(self, name):
        assert name and name.startswith("proof-recovery-")
        destination = install.load(name)
        runtime.fresh(destination)
        current, source = self.selected("target"), self.selected("source")
        override = install.directory(name) / "overrides.yaml"
        selected_override = archive.json_file(install.directory(current["name"]) / "overrides.yaml")
        private_json(override, selected_override)
        cfg = recovery.config(source)
        root = Path(cfg["backup_dir"])
        bundle = Path(self.state["downloaded_archive"])
        journal = root / self.state["journals"]["journal"]
        try:
            recovery.restore(destination, cfg, bundle, journal, self.state["identity"])
        except ValueError as error:
            assert "current source or previously recovered" in str(error)
        else:
            raise AssertionError("A second active restored writer was admitted")
        runtime.fresh(destination)
        before = runtime.inventory(current)
        runtime.sql(current, "INSERT INTO recollect_migrations(name) VALUES('999_recovery_fixture_unrecognized');")
        result = subprocess.run(["python3", str(install.ROOT / "scripts/install.py"), "start", current["name"]],
            capture_output=True, env=runtime.environment(), timeout=360)
        (self.root / "failed-upgrade.log").write_bytes(result.stdout + result.stderr)
        assert result.returncode != 0
        assert not any(row["Service"] in ("api", "worker") and row["State"] == "running" for row in runtime.services(current))
        assert runtime.inventory(current)["counts"] == before["counts"]
        # Retain all failed-installation volumes. Stop its graph/DB to leave room
        # for the explicitly fresh compatible checkpoint on this local machine.
        runtime.command(current, ["stop", "--timeout", "90", "postgres", "neo4j"])
        gate = self.root / "restore-gate.sh"
        gate.write_text('''#!/bin/sh
if [ "$1" = recovery-prepare ]; then
    mkdir -p /var/lib/recollect/recovery
    touch /var/lib/recollect/recovery/drill-gate-entered
    while :; do sleep 1; done
fi
exec /usr/local/bin/recollect-server "$@"
''')
        gate.chmod(0o755)
        gated = copy.deepcopy(selected_override)
        gated["services"]["migrate"].update(entrypoint=["/run/recovery-drill-gate"],
            volumes=[str(gate) + ":/run/recovery-drill-gate:ro"], labels={"io.recollect.fixture": "recovery-gate"})
        private_json(override, gated)
        arguments = ["python3", str(install.ROOT / "scripts/recovery.py"), "restore", name,
            "--config", str(install.directory(source["name"]) / "recovery.json"), "--archive", str(bundle),
            "--journals", str(journal), "--identity", self.state["identity"]]
        started = time.monotonic()
        docker = str(install.ROOT / "scripts/docker.sh")

        def owned_helpers():
            result = subprocess.run([docker, "ps", "-q", "--filter", "label=com.docker.compose.project=" + destination["project"],
                "--filter", "label=io.recollect.fixture=recovery-gate"], capture_output=True, text=True, env=runtime.environment(), timeout=15)
            assert result.returncode == 0
            return result.stdout.split()

        with (self.root / "interrupted-restore.log").open("wb") as output:
            process = subprocess.Popen(arguments, stdout=output, stderr=output, env=runtime.environment(), start_new_session=True)
            status_path = install.directory(name) / "recovery-status.json"
            try:
                proof.eventually(lambda: status_path.is_file() and archive.json_file(status_path).get("phase") == "files_installed",
                                 "durable restore hold before preparation", seconds=180)
                proof.eventually(lambda: runtime.sql(destination, "SELECT count(*) FROM recollect_migrations") == "25",
                                 "checkpoint imported in its transaction")
                proof.eventually(lambda: len(owned_helpers()) == 1, "restore helper running before operator interruption")
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=15)
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=15)
        assert len(owned_helpers()) == 1, "The helper must outlive the interrupted operator for this proof"
        abandoned = list(install.directory(name).glob(".restore-*/staging-owner.json"))
        assert len(abandoned) == 1
        restore_id = archive.identifier(archive.json_file(status_path)["restore_id"])
        # Seed a tagged import transaction to verify exact-session termination
        # and rollback. This control is not a second full pg_restore invocation.
        runtime.command(destination, ["exec", "-T", "-d", "--env", "PGAPPNAME=recollect-restore-" + restore_id,
            "postgres", "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" exec psql -X -h 127.0.0.1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" '
            '-c "BEGIN; CREATE TABLE recovery_aborted_import_probe(id integer); SELECT pg_sleep(180); COMMIT;"'])
        proof.eventually(lambda: runtime.sql(destination, f"SELECT count(*) FROM pg_stat_activity WHERE application_name='recollect-restore-{restore_id}';") == "1",
                         "owned import transaction waiting across resume")
        private_json(override, selected_override)
        result = subprocess.run(["python3", str(install.ROOT / "scripts/install.py"), "start", name],
            capture_output=True, env=runtime.environment(), timeout=360)
        (self.root / "held-start-refused.log").write_bytes(result.stdout + result.stderr)
        assert result.returncode != 0
        assert not any(row["Service"] in ("api", "worker") and row["State"] == "running" for row in runtime.services(destination))
        result = subprocess.run(arguments + ["--resume"], capture_output=True, env=runtime.environment(), timeout=360)
        (self.root / "resumed-restore.log").write_bytes(result.stdout + result.stderr)
        assert result.returncode == 0, "Held restore did not resume; inspect its private log"
        resumed = json.loads(result.stdout)
        assert resumed["state"] == "complete"
        assert not owned_helpers()
        assert not list(install.directory(name).glob(".restore-*/staging-owner.json"))
        assert runtime.sql(destination, "SELECT to_regclass('public.recovery_aborted_import_probe') IS NULL;") == "t"
        # Remove only the exact marker created by this fault fixture.
        runtime.command(destination, ["exec", "-T", "api", "rm", "-f", "/var/lib/recollect/recovery/drill-gate-entered"])
        shutil.copyfile(self.root / "report.json", self.root / "first-restore-report.json")
        self.state.update(previous_target=current["name"], target=name, restore=resumed)
        self.state.pop("blocked_reentry", None)
        self.save()
        self.verify()
        duration = round(time.monotonic() - started, 3)
        assert duration <= 900
        report = {"active_recovered_writer_refused": True, "failed_upgrade_left_application_stopped": True,
            "failed_upgrade_volumes_preserved": True, "operator_killed_after_transaction_and_file_install": True,
            "durable_hold_prevented_ordinary_startup": True, "explicit_resume_completed": True,
            "abandoned_helper_and_staging_retired_by_resume": True, "tagged_import_transaction_rolled_back": True,
            "compatible_checkpoint_reverified": True, "restore_resume_and_verification_seconds": duration}
        private_json(self.root / "rollback-report.json", report)
        self.record("rollback_controls", report)

    def verify(self):
        assert self.state.get("restore", {}).get("state") == "complete"
        started = time.monotonic()
        target, owner, base = self.selected("target"), self.owner("target"), self.state["base"]
        member = proof.Session(owner.origin)
        member.login(**self.state["member"])
        assert member.call("GET", base)["role"] == "reader"
        owner.call("GET", "/api/brains/" + self.state["member_brain"]["id"], expected=404)
        member.call("GET", "/api/operations", expected=403)
        retained = self.state["retained"]
        assert "RetainedIndependentCobalt" in owner.call("GET", base + "/sources/" + retained["id"] + "/versions/" + retained["version"]["id"])["content"]
        corrected = self.state["corrected"]
        assert owner.call("GET", base + "/claims/" + corrected["claim_id"])["selected"]["revision"]["id"] == corrected["id"]
        history = owner.call("GET", base + "/claims/" + corrected["claim_id"] + "/review")
        assert self.state["historical"]["id"] in json.dumps(history)
        erased = self.state["capture"][0]["receipt"]
        # Privacy details remain content-free and denied at read and recall.
        erased_detail = owner.call("GET", base + "/sources/" + erased["source_id"] + "/versions/" + erased["source_version_id"])
        assert erased_detail.get("content") is None
        replay = Device(owner.origin, self.state["device"]["token"]).call("POST", base + "/capture/events", self.state["capture"][0]["input"])
        assert replay["state"] == "removed"
        recalled = owner.call("POST", base + "/recall", {"query": "EraseCapturedZircon EraseDerivedZircon", "limit": 20})
        assert not any(term in json.dumps(recalled) for term in ("EraseCapturedZircon", "EraseDerivedZircon"))
        # The manually reviewed source also supports a rejected assertion. Its
        # raw fragment must remain suppressed; the two other origins are the
        # genuinely independent recall controls.
        assert not owner.call("POST", base + "/recall", {"query": "RetainedIndependentCobalt", "limit": 20})["context"]["items"]
        for word in ("RetainedCapturedAmber", "ManagedRetainedSapphire"):
            result = owner.call("POST", base + "/recall", {"query": word, "limit": 20})
            assert result["context"]["items"], "Independent evidence is missing after restore"
        if "blocked_reentry" not in self.state:
            fresh = self.source(owner, "ReentryIndependentControl")
            blocked = owner.call("POST", base + "/claims", self.proposal(fresh["version"]["id"], "Rejected recovery service", "invalid"))
            assert blocked["admission"] == "blocked_by_rule"
            self.record("blocked_reentry", blocked["id"])
        proof.eventually(lambda: any(g["kind"] == "knowledge" and g["state"] == "ready"
            for g in owner.call("GET", base + "/graph")["generations"]), "restored knowledge graph", seconds=120)
        view = owner.call("POST", base + "/graph/view", {"scope": {"kind": "knowledge"}})
        assert view["total_nodes"] > 0 and corrected["id"] in json.dumps(view["nodes"])
        assert "raw_evidence_blocked_by_review_rule" in view["coverage"]["reasons"]
        assert self.state["erase_claim"]["id"] not in json.dumps(view) and self.state["blocked_reentry"] not in json.dumps(view)
        snapshot = self.state["snapshot"]["snapshot"]["id"]
        structural = owner.call("POST", base + "/graph/view", {"scope": {"kind": "repository", "snapshot_id": snapshot,
            "selection": {**EMPTY, "repository_ids": [self.state["repository_id"]]}}})
        assert structural["total_nodes"] == 2 and structural["total_edges"] == 1
        self.fixture()
        effect = self.state["effect"]
        path = base + "/mcp/calls/" + effect["id"]
        assert owner.call("GET", path)["state"] == "unknown"
        assert owner.call("POST", base + "/mcp/calls", self.state["effect_input"])["id"] == effect["id"]
        lookup = {"request_id": identifier(), "client_session_id": identifier()}
        receipt = owner.call("POST", path + "/reconcile", lookup)
        result = self.finished(owner, receipt["id"])
        assert result["state"] == "succeeded"
        assert result["result"]["structuredContent"]["outcome"] == "succeeded"
        assert owner.call("POST", path + "/reconcile", lookup)["id"] == receipt["id"]
        assert owner.call("GET", path)["resolutions"][0]["kind"] == "connector_receipt"
        assert self.effects().count("effect " + effect["id"]) == 1
        model = archive.identifier(self.state["model_request_id"])
        assert runtime.sql(target, f"SELECT state||':'||charged_tokens FROM model_requests WHERE id='{model}'") == "uncertain:100"
        assert runtime.sql(target, "SELECT count(*) FROM model_requests") == "1"
        report = {"source": self.state["source"], "target": self.state["target"], "origin": owner.origin,
            "encrypted_sftp_backup_restore": True, "current_erasure_prevents_resurrection": True,
            "erased_capture_replay_stays_removed": True,
            "independent_evidence_and_scope_retained": True, "rejection_blocks_reentry": True,
            "rejected_support_fragment_stays_out_of_recall": True,
            "empty_neo4j_rebuilt_and_queried": True, "effect_count": 1, "receipt_only_reconciled": True,
            "model_accounting_retained": True, "paid_provider_calls": 0,
            "backup_seconds": self.state["backup"]["duration_seconds"], "restore_seconds": self.state["restore"]["duration_seconds"],
            "verification_seconds": round(time.monotonic() - started, 3),
            "counts": self.state["backup"]["counts"], "archive_bytes": self.state["backup"]["archive_bytes"],
            "artifact_bytes": self.state["backup"]["artifact_bytes"]}
        assert report["restore_seconds"] + report["verification_seconds"] <= 900
        private_json(self.root / "report.json", report)
        print(json.dumps(report, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory")
    parser.add_argument("stage", choices=("initialize", "corpus", "backup", "erase", "failures", "restore", "resume", "verify", "publication", "rollback"))
    parser.add_argument("--source")
    parser.add_argument("--target")
    parser.add_argument("--fixture")
    args = parser.parse_args()
    drill = Drill(args.directory)
    if args.stage == "initialize":
        assert args.source and args.target and args.fixture
        drill.initialize(args.source, args.target, args.fixture)
    elif args.stage in ("restore", "resume"):
        drill.restore(resume=args.stage == "resume")
    elif args.stage == "rollback":
        drill.rollback(args.target)
    else:
        getattr(drill, args.stage)()
    print("Recovery drill stage completed: " + args.stage)


if __name__ == "__main__":
    main()
