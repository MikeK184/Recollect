#!/usr/bin/env python3
"""Encrypted recovery for an explicit named Recollect installation."""
import argparse
from contextlib import contextmanager
from datetime import datetime, timedelta, timezone
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import sys
import tempfile
import time
import uuid

import install
import recovery_archive as archive
import recovery_installation as runtime
from recovery_transport import Sftp, decrypt, encrypt, private_json, recipients, regular


def now():
    return datetime.now(timezone.utc)


def instant(value):
    result = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if result.tzinfo is None:
        raise ValueError("Recovery timestamps require a timezone")
    return result


def config(saved):
    value = archive.json_file(install.directory(saved["name"]) / "recovery.json", 16_384)
    if set(value) - {"installation_id", "backup_dir", "age_binary", "sftp_binary", "recipients", "remote", "mirror_enabled", "runtime_host"}:
        raise ValueError("Recovery configuration contains unsupported fields")
    archive.identifier(value["installation_id"])
    regular(value["age_binary"])
    recipients(value["recipients"])
    root = Path(value["backup_dir"])
    if not root.is_absolute() or root.is_symlink() or not root.is_dir():
        raise ValueError("Recovery backup directory is unavailable")
    owner = archive.json_file(root / "owner.json", 1024)
    if owner != {"installation_id": value["installation_id"], "kind": "recollect-recovery"}:
        raise ValueError("Local recovery directory has a different owner")
    if value.get("remote"):
        Sftp(value["remote"], value.get("sftp_binary", "/usr/bin/sftp"))
    return value


def remote(cfg):
    if not cfg.get("remote"):
        raise ValueError("This installation has no configured SFTP destination")
    return Sftp(cfg["remote"], cfg.get("sftp_binary", "/usr/bin/sftp"))


@contextmanager
def locked(saved):
    path = install.directory(saved["name"]) / "recovery.lock"
    with os.fdopen(os.open(path, os.O_CREAT | os.O_RDWR, 0o600), "w") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError("Another recovery operation owns this installation") from error
        yield


@contextmanager
def attempted(saved, action):
    if action in ("list", "status"):
        yield
        return
    path = install.directory(saved["name"]) / "recovery-attempt.json"
    outcome = {"operation": action, "state": "running", "started_at": now().isoformat()}
    private_json(path, outcome)
    try:
        yield
        outcome["state"] = "complete"
    except BaseException:
        outcome.update(state="failed", error=action.replace("-", "_") + "_failed")
        raise
    finally:
        outcome["finished_at"] = now().isoformat()
        private_json(path, outcome)


def configure(saved, path, installation_id=None):
    value = archive.json_file(regular(path), 16_384)
    if set(value) - {"backup_dir", "age_binary", "sftp_binary", "recipients", "remote"}:
        raise ValueError("Supply recovery path/key references and public recipients only")
    regular(value["age_binary"])
    recipients(value["recipients"])
    if installation_id is None:
        value["installation_id"] = runtime.inventory(saved)["installation_id"]
    else:
        # Recovery bootstrap must work after the source machine/database is
        # unavailable. The known canonical ID selects an existing remote owner.
        runtime.fresh(saved)
        value["installation_id"] = archive.identifier(installation_id)
    root = Path(value["backup_dir"])
    if not root.is_absolute() or root.is_symlink():
        raise ValueError("Choose an explicit absolute backup directory")
    if root.exists() and any(root.iterdir()):
        if archive.json_file(root / "owner.json", 1024) != {"installation_id": value["installation_id"], "kind": "recollect-recovery"}:
            raise ValueError("Existing backup directory belongs to another installation")
    else:
        root.mkdir(mode=0o700, parents=True, exist_ok=True)
        private_json(root / "owner.json", {"installation_id": value["installation_id"], "kind": "recollect-recovery"})
    if value.get("remote"):
        if installation_id is None:
            remote(value).initialize(value["installation_id"])
        else:
            remote(value).check(value["installation_id"])
    private_json(install.directory(saved["name"]) / "recovery.json", value)
    config(saved)
    return {"configured": True, "remote_connected": bool(value.get("remote")),
            "runtime_mirror": "not_enabled", "schedule": "not_installed"}


@contextmanager
def stopped(saved, status):
    roles = {"api", "worker", "proxy"}
    prior = [row["Service"] for row in runtime.services(saved)
             if row["Service"] in roles and row["State"] == "running"]
    status["resume_roles"] = prior
    status["resume"] = "pending" if prior else "not_needed"
    private_json(install.directory(saved["name"]) / "recovery-status.json", status)
    try:
        if prior:
            runtime.command(saved, ["stop", "--timeout", "90"] + prior)
        remaining = [row for row in runtime.services(saved)
                     if row["Service"] in roles and row["State"] == "running"]
        if remaining:
            raise ValueError("Application did not drain; backup was not taken")
        yield
    except BaseException:
        if status.get("state") != "complete":
            status.update(state="failed", error="operation_interrupted_or_failed")
        raise
    finally:
        if prior:
            try:
                runtime.command(saved, ["up", "--no-deps", "-d", "--wait", "--wait-timeout", "120"] + prior, timeout=150)
                status["resume"] = "complete"
            except ValueError:
                status["resume"] = "failed"
        private_json(install.directory(saved["name"]) / "recovery-status.json", status)


def snapshot(saved, temporary):
    destination = temporary / "application"
    destination.mkdir(mode=0o700)
    tar = temporary / "application.tar"
    runtime.snapshot(saved, tar)
    archive.extract(tar, destination, allowed={"artifacts", "account-credentials.json", "credentials.json", "mcp-receipts", "erasure-journal", "recovery"})
    if saved.get("root"):
        if (destination / "credentials.json").is_file():
            (destination / "credentials.json").rename(destination / "account-credentials.json")
        receipts = destination / "artifacts/.mcp-receipts"
        if receipts.is_dir():
            receipts.rename(destination / "mcp-receipts")
    tar.unlink()
    return destination


def journal_package(application, inventory, target):
    target.mkdir(mode=0o700)
    root = target / "erasure-journal"
    root.mkdir(mode=0o700)
    identity = inventory["installation_id"]
    for name in (identity, "analytics-" + identity):
        source = application / "erasure-journal" / name
        destination = root / name
        destination.mkdir(mode=0o700)
        for file in source.iterdir():
            if file.name.endswith(".tmp"):
                continue
            if not file.is_file() or file.is_symlink():
                raise ValueError("Independent journal storage contains an unexpected member")
            shutil.copyfile(file, destination / file.name)
    analytics = sorted(p.name for p in (root / ("analytics-" + identity)).iterdir() if p.name != "installation.json")
    private_json(target / "journal-inventory.json", {"installation_id": identity,
                 "exported_at": now().isoformat(), "entries": inventory["journal_entries"], "analytics": analytics})
    archive.validate_journals(target, inventory)


def export_journals(saved, cfg):
    status = {"operation": "export_journals", "state": "running", "started_at": now().isoformat()}
    root = Path(cfg["backup_dir"])
    with tempfile.TemporaryDirectory(prefix=".journal-", dir=root) as directory:
        temporary = Path(directory)
        with stopped(saved, status):
            runtime.application(saved, "backup-prepare")
            inventory = runtime.inventory(saved)
            if inventory["installation_id"] != cfg["installation_id"]:
                raise ValueError("Selected installation identity changed")
            application = snapshot(saved, temporary)
            journal_package(application, inventory, temporary / "journal")
            archive.pack(temporary / "journal", temporary / "journal.tar")
            name = f"journal-{uuid.uuid4()}.tar.age"
            encrypt(cfg["age_binary"], cfg["recipients"], temporary / "journal.tar", root / name)
            status.update(state="complete", journal=name)
    if status.get("resume") == "failed":
        raise ValueError("Journal exported but application resume remains pending")
    return status


def fetch_journals(cfg, identity):
    connection = remote(cfg)
    installation = cfg["installation_id"]
    root = Path(cfg["backup_dir"])
    with tempfile.TemporaryDirectory(prefix=".journal-fetch-", dir=root) as directory:
        temporary = Path(directory)
        journal = temporary / "journal"
        for name in (installation, "analytics-" + installation):
            target = journal / "erasure-journal" / name
            target.mkdir(mode=0o700, parents=True)
            private_json(target / "installation.json", installation)
        complete = lambda names: [name for name in names if ".pending-" not in name]
        selected = {kind: complete(connection.list(installation, kind)) for kind in ("journal", "analytics")}
        records, analytical = [], []
        for kind, names in selected.items():
            for index, name in enumerate(names):
                pattern = r"\d{20}-[a-f0-9-]{36}\.json\.age" if kind == "journal" else r"[a-f0-9-]{36}\.json\.age"
                if not re.fullmatch(pattern, name):
                    raise ValueError("Remote journal contains an unexpected complete filename")
                encrypted = temporary / f"{kind}-{index}.age"
                plain = temporary / f"{kind}-{index}.json"
                connection.get(installation, kind + "/" + name, encrypted)
                decrypt(cfg["age_binary"], identity, encrypted, plain)
                value = archive.json_file(plain, 32 * 1024 * 1024 if kind == "journal" else 8192)
                if kind == "journal":
                    entry = value["entry"]
                    expected = f"{entry['sequence']:020}-{archive.identifier(entry['id'])}.json.age"
                    if name != expected:
                        raise ValueError("Remote journal identity differs from its filename")
                    records.append(value)
                else:
                    if value.get("installation_id") != installation or name != archive.identifier(value["id"]) + ".json.age":
                        raise ValueError("Remote analytical identity differs")
                    target = value["id"] + ".json"
                    analytical.append(target)
                    private_json(journal / "erasure-journal" / ("analytics-" + installation) / target, value)
                plain.unlink()
                encrypted.unlink()
        # A changed retained set requires retry, not a declaration that a mixed
        # observation was the current independent journal.
        for kind, names in selected.items():
            if complete(connection.list(installation, kind)) != names:
                raise ValueError("Remote journal changed during fetch; retry against its current set")
        entries = archive.remote_chain(records, installation)
        for entry in entries:
            private_json(journal / "erasure-journal" / installation / (entry["id"] + ".json"), entry)
        inventory = {"installation_id": installation, "exported_at": now().isoformat(),
                     "entries": [{"id": e["id"], "sequence": e["sequence"]} for e in entries], "analytics": sorted(analytical)}
        private_json(journal / "journal-inventory.json", inventory)
        archive.validate_journals(journal, {"installation_id": installation, "journal_entries": []})
        archive.pack(journal, temporary / "journal.tar")
        name = f"journal-{uuid.uuid4()}.tar.age"
        encrypt(cfg["age_binary"], cfg["recipients"], temporary / "journal.tar", root / name)
    return {"operation": "fetch_journals", "state": "complete", "journal": name,
            "erasure_entries": len(entries), "analytical_entries": len(analytical)}


def mirror_files(temporary, cfg, runtime_host=None):
    root = temporary / "files" / "recovery"
    root.mkdir(mode=0o700, parents=True)
    for field, filename in (("identity_file", "mirror-identity"), ("known_hosts_file", "mirror-known-hosts")):
        shutil.copyfile(regular(cfg["remote"][field]), root / filename)
        (root / filename).chmod(0o600)
    value = {**cfg["remote"], "recipients": cfg["recipients"],
             "age_binary": "/usr/local/bin/age", "sftp_binary": "/usr/bin/sftp",
             "identity_file": "/var/lib/recollect/recovery/mirror-identity",
             "known_hosts_file": "/var/lib/recollect/recovery/mirror-known-hosts",
             "host": runtime_host or cfg.get("runtime_host") or cfg["remote"]["host"]}
    private_json(root / "mirror.json", value)
    archive.pack(temporary / "files", temporary / "mirror.tar")
    return temporary / "mirror.tar"


def mirror_override(saved):
    override = {"services": {role: {"environment": {
        "RECOLLECT_ERASURE_MIRROR_CONFIG": "/var/lib/recollect/recovery/mirror.json"}}
        for role in ("api", "worker", "migrate")}}
    private_json(install.directory(saved["name"]) / "recovery.yaml", override)
    install.configuration(saved)


def enable_mirror(saved, cfg, runtime_host=None):
    connection = remote(cfg)
    connection.check(cfg["installation_id"])
    observed = runtime.inventory(saved)
    if observed["installation_id"] != cfg["installation_id"]:
        raise ValueError("Mirror source ownership differs")
    status = {"operation": "enable_mirror", "state": "running", "started_at": now().isoformat()}
    with tempfile.TemporaryDirectory(prefix=".mirror-", dir=install.directory(saved["name"])) as directory:
        temporary = Path(directory)
        staged = mirror_files(temporary, cfg, runtime_host)
        with stopped(saved, status):
            runtime.upload(saved, staged)
            mirror_override(saved)
            # This is a real call from the selected runtime image, including its
            # own hostname/known-hosts resolution and every existing journal entry.
            runtime.application(saved, "privacy-reconcile")
            cfg.update(mirror_enabled=True, runtime_host=runtime_host or cfg.get("runtime_host") or cfg["remote"]["host"])
            private_json(install.directory(saved["name"]) / "recovery.json", cfg)
            status.update(state="complete", mirror="synchronized")
    if status.get("resume") == "failed":
        raise ValueError("Mirror synchronized but application resume remains pending")
    return status


def create(saved, cfg):
    started = time.monotonic()
    root = Path(cfg["backup_dir"])
    before = runtime.inventory(saved)
    required = (before["database_bytes"] + sum(a["byte_length"] for a in before["artifacts"])) * 3 + 256 * 1024**2
    if shutil.disk_usage(root).free < required:
        raise ValueError("Insufficient local staging space; no service was stopped")
    if cfg.get("remote"):
        remote(cfg).check(cfg["installation_id"])
    backup = str(uuid.uuid4())
    status = {"operation": "backup", "backup_id": backup, "state": "running", "started_at": now().isoformat()}
    record = root / (backup + ".json")
    private_json(record, status)
    try:
        with tempfile.TemporaryDirectory(prefix=".backup-" + backup + "-", dir=root) as directory:
            temporary = Path(directory)
            bundle = temporary / "bundle"
            bundle.mkdir(mode=0o700)
            with stopped(saved, status):
                runtime.application(saved, "backup-prepare")
                inventory = runtime.inventory(saved)
                if inventory["installation_id"] != cfg["installation_id"]:
                    raise ValueError("Selected installation identity changed")
                runtime.dump(saved, bundle / "database.dump")
                application = snapshot(saved, temporary)
                app = bundle / "app"
                app.mkdir(mode=0o700)
                for item in inventory["artifacts"]:
                    path = Path("artifacts") / item["brain_id"] / (item["id"] + ".txt")
                    destination = app / path
                    destination.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                    shutil.copyfile(application / path, destination)
                application_files = {}
                for name in ("account-credentials.json", "mcp-receipts"):
                    source = application / name
                    if source.is_file():
                        shutil.copyfile(source, app / name)
                    elif source.is_dir():
                        shutil.copytree(source, app / name)
                for file in app.rglob("*"):
                    name = file.relative_to(app).as_posix()
                    if file.is_file() and not name.startswith("artifacts/"):
                        application_files[name] = file.stat().st_size
                journal_package(application, inventory, temporary / "journal")
                archive.pack(temporary / "journal", temporary / "journal.tar")
                encrypt(cfg["age_binary"], cfg["recipients"], temporary / "journal.tar", root / ("journal-" + backup + ".tar.age"))
                manifest = {**inventory, "backup_id": backup, "source_installation": saved,
                    "journal_durability": "encrypted_sftp" if install.configuration(saved)["journal_mirror_configured"] else "local",
                    "database_dump_bytes": (bundle / "database.dump").stat().st_size,
                    "application_files": application_files,
                    "runtime_inputs": ["owner", "database", "graph", "model_provider", "mcp_bindings", "oidc_if_configured"]}
                private_json(bundle / "manifest.json", manifest)
                schema = runtime.schema(saved)
                archive.validate_bundle(bundle, schema)
                archive.pack(bundle, temporary / "backup.tar")
                encrypt(cfg["age_binary"], cfg["recipients"], temporary / "backup.tar", temporary / "backup.age")
                final = root / (backup + ".tar.age")
                if final.exists():
                    raise ValueError("Backup identity already exists")
                os.rename(temporary / "backup.age", final)
                status.update(state="complete", checkpoint_at=inventory["checkpoint_at"],
                    expires_at=(instant(inventory["checkpoint_at"]) + timedelta(days=inventory["backup_days"])).isoformat(),
                    backup_days=inventory["backup_days"], brain_ids=inventory["brain_ids"],
                    archive_bytes=final.stat().st_size, artifact_bytes=sum(a["byte_length"] for a in inventory["artifacts"]),
                    counts=inventory["counts"], local="complete", remote="pending" if cfg.get("remote") else "not_configured")
            status["duration_seconds"] = round(time.monotonic() - started, 3)
            private_json(record, status)
        if cfg.get("remote"):
            try:
                transfer(cfg, backup)
            finally:
                status = archive.json_file(record)
    except (ValueError, OSError):
        if status["state"] != "complete":
            status.update(state="failed", error="backup_failed")
        private_json(record, status)
        raise
    finally:
        private_json(install.directory(saved["name"]) / "recovery-status.json", status)
    if status.get("resume") == "failed":
        raise ValueError("Backup completed but application resume failed; inspect recovery status")
    return status


def transfer(cfg, backup):
    backup = archive.identifier(backup)
    root = Path(cfg["backup_dir"])
    record = root / (backup + ".json")
    state = archive.json_file(record)
    if state.get("state") != "complete" or instant(state["expires_at"]) <= now():
        raise ValueError("Only completed unexpired archives can be transferred")
    try:
        remote(cfg).put(cfg["installation_id"], root / (backup + ".tar.age"), "backups/" + backup + ".tar.age")
        state["remote"] = "complete"
        state.pop("error", None)
    except ValueError:
        state.update(remote="failed", error="transfer_failed")
        raise
    finally:
        private_json(record, state)
    return state


def fetch(saved, cfg, backup, identity):
    backup = archive.identifier(backup)
    root = Path(cfg["backup_dir"])
    final = root / (backup + ".tar.age")
    record = root / (backup + ".json")
    if final.exists() or record.exists():
        raise ValueError("Fetch preserves existing local archive identities")
    with tempfile.TemporaryDirectory(prefix=".fetch-", dir=root) as directory:
        temporary = Path(directory)
        remote(cfg).get(cfg["installation_id"], "backups/" + backup + ".tar.age", temporary / "received.age")
        decrypt(cfg["age_binary"], identity, temporary / "received.age", temporary / "received.tar")
        bundle = temporary / "bundle"
        bundle.mkdir(mode=0o700)
        archive.extract(temporary / "received.tar", bundle, allowed={"manifest.json", "database.dump", "app"})
        manifest = archive.validate_bundle(bundle, runtime.schema(saved))
        expires = instant(manifest["checkpoint_at"]) + timedelta(days=manifest["backup_days"])
        if manifest["backup_id"] != backup or manifest["installation_id"] != cfg["installation_id"] or expires <= now():
            raise ValueError("Fetched backup ownership or retention differs")
        os.rename(temporary / "received.age", final)
        state = {"operation": "fetch", "backup_id": backup, "state": "complete", "started_at": now().isoformat(),
                 "checkpoint_at": manifest["checkpoint_at"], "expires_at": expires.isoformat(),
                 "backup_days": manifest["backup_days"], "brain_ids": manifest["brain_ids"],
                 "archive_bytes": final.stat().st_size, "local": "complete", "remote": "complete"}
        private_json(record, state)
    return state


def prune(saved, cfg):
    root = Path(cfg["backup_dir"])
    inventory = runtime.inventory(saved)
    if inventory["installation_id"] != cfg["installation_id"]:
        raise ValueError("Current retention belongs to another installation")
    removed, failed = [], []
    for state in listing(saved, cfg)["backups"]:
        backup = archive.identifier(state["backup_id"])
        if state.get("state") == "pruned":
            continue
        complete = state.get("state") == "complete"
        if complete:
            current = [inventory["brain_retention"][b] for b in state.get("brain_ids", []) if b in inventory["brain_retention"]]
            deadline = min(instant(state["expires_at"]),
                instant(state["checkpoint_at"]) + timedelta(days=min(current, default=state["backup_days"])))
            expired = deadline <= now()
        else:
            expired = instant(state["started_at"]) + timedelta(days=1) <= now()
        if not expired:
            continue
        try:
            if cfg.get("remote"):
                connection = remote(cfg)
                for name in connection.list(cfg["installation_id"], "backups"):
                    if name == backup + ".tar.age" or name.startswith(backup + ".tar.age.pending-"):
                        connection.remove(cfg["installation_id"], "backups/" + name)
                state["remote"] = "pruned"
            (root / (backup + ".tar.age")).unlink(missing_ok=True)
            for path in root.glob(".backup-" + backup + "-*"):
                if path.is_dir() and not path.is_symlink():
                    shutil.rmtree(path)
            # Journals are independently retained and deliberately untouched.
            state.update(state="pruned", local="pruned", pruned_at=now().isoformat())
            state.pop("error", None)
            removed.append(backup)
        except (ValueError, OSError):
            state["error"] = "prune_failed"
            failed.append(backup)
        private_json(root / (backup + ".json"), state)
    result = {"operation": "prune", "state": "failed" if failed else "complete", "pruned": removed, "failed": failed}
    private_json(install.directory(saved["name"]) / "recovery-prune.json", result)
    if failed:
        raise ValueError("One or more archive prunes remain pending")
    return result


def resume_source(saved):
    path = install.directory(saved["name"]) / "recovery-status.json"
    status = archive.json_file(path)
    if status.get("operation") not in ("backup", "export_journals", "enable_mirror"):
        raise ValueError("No source backup operation is available to resume")
    roles = status.get("resume_roles", [])
    if not set(roles) <= {"api", "worker", "proxy"}:
        raise ValueError("Source resume ownership is invalid")
    observed = runtime.inventory(saved)
    runtime.source_stopped({"source_installation": saved, "installation_id": observed["installation_id"]}, saved)
    if roles:
        runtime.command(saved, ["up", "--no-deps", "-d", "--wait", "--wait-timeout", "120"] + roles, timeout=150)
    status["resume"] = "complete"
    if status["state"] == "running":
        status.update(state="interrupted", error="backup_interrupted")
    private_json(path, status)
    return status


def listing(saved, cfg=None):
    private = install.directory(saved["name"])
    details = {name: archive.json_file(private / filename) if (private / filename).exists() else None
               for name, filename in (("last_operation", "recovery-status.json"),
                   ("last_attempt", "recovery-attempt.json"), ("last_prune", "recovery-prune.json"))}
    if cfg is None:
        return {"installation": saved["name"], "configured": False,
                "latest_checkpoint_age_seconds": None, "checkpoint_within_24_hours": False,
                "schedule": "not_configured", "backups": [], **details}
    records = []
    for path in Path(cfg["backup_dir"]).glob("*.json"):
        try:
            archive.identifier(path.stem)
        except ValueError:
            continue
        records.append(archive.json_file(path))
    complete = [r for r in records if r.get("state") == "complete"]
    latest = max((instant(r["checkpoint_at"]) for r in complete), default=None)
    return {"installation": saved["name"], "configured": True,
            "latest_checkpoint_age_seconds": round((now() - latest).total_seconds()) if latest else None,
            "checkpoint_within_24_hours": bool(latest and now() - latest <= timedelta(hours=24)),
            "schedule": "operator_managed", "backups": sorted(records, key=lambda r: r["started_at"]), **details}


def scheduled(saved, cfg):
    outcome = {"operation": "scheduled", "started_at": now().isoformat()}
    for name, operation in (("backup", create), ("prune", prune)):
        try:
            outcome[name] = {"state": "complete", "result": operation(saved, cfg)}
        except (ValueError, OSError):
            outcome[name] = {"state": "failed", "error": name + "_failed"}
    outcome["state"] = "complete" if all(outcome[k]["state"] == "complete" for k in ("backup", "prune")) else "failed"
    private_json(install.directory(saved["name"]) / "recovery-schedule.json", outcome)
    if outcome["state"] != "complete":
        raise ValueError("Scheduled backup or pruning failed; inspect their separate outcomes")
    return outcome


@contextmanager
def restore_staging(saved):
    root = install.directory(saved["name"])
    owner = {"kind": "recollect-restore-staging", "installation": saved["name"]}
    # The CLI holds this installation's lock. Only our exact marked staging
    # directories can be retired; unrelated or unmarked paths are preserved.
    for directory in root.glob(".restore-*"):
        marker = directory / "staging-owner.json"
        if directory.is_symlink() or not directory.is_dir() or marker.is_symlink() or not marker.is_file():
            continue
        recorded = archive.json_file(marker, 1024)
        identifier = recorded.get("id", "")
        if (re.fullmatch(r"[0-9a-f-]{36}", identifier)
                and directory.name.startswith(".restore-" + identifier + "-")
                and recorded == {**owner, "id": identifier}):
            shutil.rmtree(directory)
    identifier = str(uuid.uuid4())
    with tempfile.TemporaryDirectory(prefix=".restore-" + identifier + "-", dir=root) as directory:
        private_json(Path(directory) / "staging-owner.json", {**owner, "id": identifier})
        yield Path(directory)


def restore(saved, cfg, source, journals, identity, resume=False, runtime_host=None):
    started = time.monotonic()
    state_path = install.directory(saved["name"]) / "recovery-status.json"
    with restore_staging(saved) as temporary:
        decrypt(cfg["age_binary"], identity, source, temporary / "backup.tar")
        bundle = temporary / "bundle"
        bundle.mkdir(mode=0o700)
        archive.extract(temporary / "backup.tar", bundle, allowed={"manifest.json", "database.dump", "app"})
        # Schema discovery does not connect to a DB or read runtime credentials.
        schema = runtime.schema(saved)
        manifest = archive.validate_bundle(bundle, schema)
        if cfg.get("installation_id") != manifest["installation_id"]:
            raise ValueError("Backup belongs to another configured source installation")
        if instant(manifest["checkpoint_at"]) + timedelta(days=manifest["backup_days"]) <= now():
            raise ValueError("Backup has passed its admitted retention deadline")
        decrypt(cfg["age_binary"], identity, journals, temporary / "journal.tar")
        journal = temporary / "journal"
        journal.mkdir(mode=0o700)
        archive.extract(temporary / "journal.tar", journal, allowed={"journal-inventory.json", "erasure-journal"})
        archive.validate_journals(journal, manifest)
        mirror = manifest.get("journal_durability") == "encrypted_sftp" or cfg.get("mirror_enabled", False) or runtime_host is not None
        if mirror:
            remote(cfg).check(manifest["installation_id"])
        runtime.source_stopped(manifest, saved)
        if resume:
            status = archive.json_file(state_path)
            if status.get("operation") != "restore" or status.get("backup_id") != manifest["backup_id"] or status.get("state") == "complete":
                raise ValueError("There is no matching interrupted restore to resume")
        else:
            runtime.fresh(saved)
            status = {"operation": "restore", "restore_id": str(uuid.uuid4()), "backup_id": manifest["backup_id"],
                      "installation_id": manifest["installation_id"], "source_origin": manifest["source_installation"]["origin"],
                      "state": "held", "phase": "fresh", "started_at": now().isoformat()}
            private_json(state_path, status)
            private_json(install.directory(saved["name"]) / "recovery-identity.json",
                         {key: status[key] for key in ("installation_id", "restore_id")})
        try:
            if resume:
                runtime.retire_restore_helpers(saved, status["restore_id"])
            if status["phase"] == "fresh":
                hold = temporary / "hold"
                (hold / "erasure-journal").mkdir(mode=0o700, parents=True)
                private_json(hold / "erasure-journal/recovery-hold.json", {k: status[k] for k in ("restore_id", "backup_id", "installation_id", "source_origin")})
                archive.pack(hold, temporary / "hold.tar")
                runtime.upload(saved, temporary / "hold.tar", status["restore_id"])
                status["phase"] = "held"
                private_json(state_path, status)
            runtime.command(saved, ["up", "-d", "--wait", "--wait-timeout", "300", "postgres", "neo4j"], timeout=330)
            if resume:
                runtime.retire_restore_import(saved, status["restore_id"])
            if status["phase"] == "held":
                exists = runtime.sql(saved, "SELECT to_regclass('public.recollect_migrations') IS NOT NULL;")
                if exists == "f":
                    runtime.restore_dump(saved, bundle / "database.dump", status["restore_id"])
                else:
                    # pg_restore commits atomically. A crash between commit and
                    # state persistence may resume only that same checkpoint.
                    restored_id = runtime.sql(saved, "SELECT id FROM privacy_installation;")
                    applied = json.loads(runtime.sql(saved, "SELECT json_agg(name ORDER BY name) FROM recollect_migrations;"))
                    if restored_id != manifest["installation_id"] or applied != manifest["migrations"]:
                        raise ValueError("Interrupted database import does not match the held checkpoint")
                status["phase"] = "database_imported"
                private_json(state_path, status)
            if status["phase"] == "database_imported":
                files = temporary / "files"
                shutil.copytree(bundle / "app", files)
                shutil.copytree(journal / "erasure-journal", files / "erasure-journal")
                archive.pack(files, temporary / "files.tar")
                runtime.upload(saved, temporary / "files.tar", status["restore_id"])
                status["phase"] = "files_installed"
                private_json(state_path, status)
            if status["phase"] == "files_installed":
                if mirror:
                    mirror_stage = temporary / "mirror"
                    mirror_stage.mkdir(mode=0o700)
                    runtime.upload(saved, mirror_files(mirror_stage, cfg, runtime_host), status["restore_id"])
                    mirror_override(saved)
                runtime.application(saved, "recovery-prepare", status["restore_id"])
                status["phase"] = "prepared"
                private_json(state_path, status)
            if status["phase"] == "prepared":
                runtime.application(saved, "recovery-release", status["restore_id"])
                status["phase"] = "released"
                private_json(state_path, status)
            roles = ["api", "worker"] + (["proxy"] if saved["mode"] == "shared" else [])
            runtime.command(saved, ["up", "--no-deps", "-d", "--wait", "--wait-timeout", "180"] + roles, timeout=210)
            status.update(state="complete", phase="serving", duration_seconds=round(time.monotonic() - started, 3),
                          checkpoint_at=manifest["checkpoint_at"], recovered_counts=manifest["counts"], graph="rebuild_queued",
                          journal_durability="encrypted_sftp" if install.configuration(saved)["journal_mirror_configured"] else "local")
            destination_config = install.directory(saved["name"]) / "recovery.json"
            if mirror and destination_config.is_file():
                configured = config(saved)
                if configured["installation_id"] == manifest["installation_id"] and configured.get("remote") == cfg.get("remote"):
                    configured.update(mirror_enabled=True, runtime_host=runtime_host or cfg.get("runtime_host") or cfg["remote"]["host"])
                    private_json(destination_config, configured)
            private_json(state_path, status)
            return status
        except (ValueError, OSError):
            status.update(state="failed", error="restore_failed")
            private_json(state_path, status)
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest="action", required=True)
    for name in ("configure", "create", "list", "status", "transfer", "fetch", "prune", "scheduled", "resume-source", "enable-mirror", "export-journals", "fetch-journals", "restore"):
        command = actions.add_parser(name)
        command.add_argument("name")
        if name == "configure":
            command.add_argument("--file", required=True)
            command.add_argument("--installation-id", help="Known source UUID for offline recovery into a fresh destination")
        if name == "enable-mirror":
            command.add_argument("--runtime-host", help="Explicit container-reachable hostname, covered by the same known-hosts file")
        if name in ("transfer", "fetch"):
            command.add_argument("backup_id")
        if name in ("fetch", "fetch-journals"):
            command.add_argument("--identity", required=True)
        if name == "restore":
            command.add_argument("--config", required=True, help="Existing recovery configuration with tool references")
            command.add_argument("--archive", required=True)
            command.add_argument("--journals", required=True, help="Latest separately exported encrypted journal, never the checkpoint copy")
            command.add_argument("--identity", required=True)
            command.add_argument("--resume", action="store_true")
            command.add_argument("--runtime-host", help="Explicit container-reachable SFTP hostname covered by the known-hosts file")
    args = parser.parse_args()
    try:
        saved = install.load(args.name)
        with locked(saved), attempted(saved, args.action):
            if args.action not in ("list", "status", "fetch-journals"):
                install.configuration(saved)
            if args.action == "configure":
                result = configure(saved, args.file, args.installation_id)
            elif args.action == "resume-source":
                result = resume_source(saved)
            elif args.action == "restore":
                cfg = archive.json_file(regular(args.config), 16_384)
                result = restore(saved, cfg, regular(args.archive), regular(args.journals), regular(args.identity), args.resume, args.runtime_host)
            elif args.action in ("list", "status") and not (install.directory(saved["name"]) / "recovery.json").exists():
                result = listing(saved)
            else:
                cfg = config(saved)
                if args.action == "create":
                    result = create(saved, cfg)
                elif args.action == "enable-mirror":
                    result = enable_mirror(saved, cfg, args.runtime_host)
                elif args.action == "export-journals":
                    result = export_journals(saved, cfg)
                elif args.action == "transfer":
                    result = transfer(cfg, args.backup_id)
                elif args.action == "fetch":
                    result = fetch(saved, cfg, args.backup_id, regular(args.identity))
                elif args.action == "fetch-journals":
                    result = fetch_journals(cfg, regular(args.identity))
                elif args.action == "prune":
                    result = prune(saved, cfg)
                elif args.action == "scheduled":
                    result = scheduled(saved, cfg)
                else:
                    result = listing(saved, cfg)
        print(json.dumps(result, indent=2))
    except (ValueError, OSError, KeyError, TypeError):
        # Exception details may contain archive content or operator paths. Keep
        # the durable status specific and the CLI failure content-free.
        parser.exit(1, "Recovery operation did not complete; inspect the named installation's recovery status.\n")


if __name__ == "__main__":
    main()
