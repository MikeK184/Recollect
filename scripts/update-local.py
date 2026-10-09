#!/usr/bin/env python3
"""Checkpoint the existing local app, then build, migrate and verify its update."""
import json
import argparse
import contextlib
import datetime
import fcntl
import os
from pathlib import Path
import subprocess
import sys
import urllib.request

import install
import recovery
import root_installation
from recovery_transport import private_json


@contextlib.contextmanager
def update_lock():
    directory = install.ROOT / ".data/install"
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    if directory.is_symlink():
        raise ValueError("Installation directory must be regular")
    path = directory / "root-update.lock"
    with os.fdopen(os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600), "w") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError("Another local update owns this installation") from error
        yield


def expected_build():
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=install.ROOT,
                              capture_output=True, text=True, check=True).stdout.strip()
    dirty = subprocess.run(["git", "status", "--porcelain", "--untracked-files=normal"],
                           cwd=install.ROOT, capture_output=True, text=True, check=True).stdout
    return {"revision": revision + ("-dirty" if dirty else ""),
            "built_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")}


def verify_runtime(expected):
    with urllib.request.urlopen("http://127.0.0.1:8787/health/ready", timeout=15) as response:
        ready = json.load(response)
    migration = sorted((install.ROOT / "crates/server/migrations").glob("*.sql"))[-1].stem
    if (not ready.get("ready") or not ready.get("schema_current")
            or ready.get("build") != expected or ready.get("migration") != migration):
        raise ValueError("Updated runtime has not proved the expected build/schema")
    result = subprocess.run(root_installation.base() + ["ps", "--all", "--quiet", "api", "worker"],
                            cwd=install.ROOT, capture_output=True, text=True, timeout=15)
    ids = result.stdout.split()
    if result.returncode or len(ids) != 2:
        raise ValueError("Updated API and worker containers are unavailable")
    result = subprocess.run([str(install.ROOT / "scripts/docker.sh"), "inspect", "--format", "{{.Image}}"] + ids,
                            cwd=install.ROOT, capture_output=True, text=True, timeout=15)
    if result.returncode or len(set(result.stdout.split())) != 1:
        raise ValueError("Updated API and worker images differ")
    return ready


def checkpoint(saved):
    directory = install.directory(saved["name"])
    if not (directory / "recovery.json").exists():
        tools = install.ROOT / ".cache/recovery-tools"
        candidates = sorted(tools.glob("age-v1.3.2-*/age"))
        if len(candidates) != 1 or candidates[0].is_symlink():
            raise ValueError("Install the pinned recovery tools with scripts/setup-recovery-tools.py first")
        age = candidates[0].resolve()
        keygen = age.with_name("age-keygen")
        identity = directory / "local-recovery-identity.txt"
        if not identity.exists():
            result = subprocess.run([str(keygen), "-o", str(identity)], capture_output=True, timeout=30)
            if result.returncode:
                raise ValueError("Local recovery identity generation failed")
            identity.chmod(0o600)
        if identity.is_symlink() or identity.stat().st_mode & 0o077:
            raise ValueError("Local recovery identity must remain private")
        result = subprocess.run([str(keygen), "-y", str(identity)], capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise ValueError("Local recovery recipient is unavailable")
        config = {"backup_dir": str(install.ROOT / ".data/backups/root-local"),
                  "age_binary": str(age), "recipients": [result.stdout.strip()]}
        bootstrap = directory / "recovery-bootstrap.json"
        private_json(bootstrap, config)
        recovery.configure(saved, bootstrap)
    with recovery.locked(saved), recovery.attempted(saved, "create"):
        status = recovery.create(saved, recovery.config(saved))
    if status.get("state") != "complete" or status.get("resume") == "failed":
        raise ValueError("Checkpoint or old-runtime resume did not complete")
    return status


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    try:
        with update_lock():
            saved = root_installation.register()
            status = checkpoint(saved)
            print(json.dumps({"checkpoint": "complete", "backup_id": status["backup_id"],
                              "local_encrypted": True, "off_machine": False}), flush=True)
            expected = expected_build()
            env = {**os.environ, "RECOLLECT_UPDATE_BUILD_REVISION": expected["revision"],
                   "RECOLLECT_UPDATE_BUILD_TIME": expected["built_at"]}
            result = subprocess.run([str(install.ROOT / "scripts/stack.sh"), "up", "--build"], cwd=install.ROOT, env=env)
            if result.returncode:
                raise ValueError("Update failed; retained checkpoint is available and no rollback was attempted")
            ready = verify_runtime(expected)
            print(json.dumps({"update": "complete", "runtime": ready, "checkpoint": status["backup_id"]}, indent=2))
    except (ValueError, OSError, KeyError, subprocess.SubprocessError):
        print("Local update did not complete. Existing data/checkpoint were preserved; inspect update and recovery status.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
