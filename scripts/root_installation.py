"""Bridge the owned repository root app to the existing encrypted recovery format."""
import json
from pathlib import Path
import re
import subprocess

import install
from recovery_transport import private_json

NAME = "root-local"


def validate(saved):
    if (saved.get("name") != NAME or saved.get("project") != "recollect"
            or saved.get("workspace") != str(install.ROOT) or saved.get("root") is not True
            or saved.get("mode") != "personal" or saved.get("origin") != "http://127.0.0.1:8787"
            or not re.fullmatch(r"sha256:[a-f0-9]{64}", saved.get("checkpoint_image", ""))):
        raise ValueError("Root recovery ownership differs from this repository app")
    for path in (install.ROOT / ".env", install.ROOT / "compose.yaml"):
        if path.is_symlink() or not path.is_file():
            raise ValueError("Root app configuration is unavailable")


def compose(saved):
    validate(saved)
    override = install.directory(NAME) / "checkpoint-image.json"
    if override.is_symlink() or not override.is_file():
        raise ValueError("Pinned checkpoint image is unavailable")
    expected = {"services": {role: {"image": saved["checkpoint_image"]}
                for role in ("api", "worker", "migrate")}}
    if json.loads(override.read_text()) != expected:
        raise ValueError("Pinned checkpoint image differs from recorded ownership")
    return base() + ["-f", str(override)]


def base():
    return [str(install.ROOT / "scripts/docker.sh"), "compose", "--project-directory", str(install.ROOT),
            "--project-name", "recollect", "--env-file", str(install.ROOT / ".env"),
            "-f", str(install.ROOT / "compose.yaml")]


def configuration(saved):
    validate(saved)
    resolved = json.loads(install.run(saved, ["config", "--format", "json"], capture=True, timeout=15))
    services = resolved["services"]
    if resolved["name"] != "recollect" or set(services) != {"postgres", "neo4j", "migrate", "api", "worker"}:
        raise ValueError("Root app project or service ownership differs")
    volume = resolved.get("volumes", {}).get("postgres_data", {})
    if volume.get("external") or volume.get("name") != "recollect_postgres_data":
        raise ValueError("Root database is not the owned persistent volume")
    for role in ("api", "worker", "migrate"):
        service = services[role]
        if service.get("image") != saved["checkpoint_image"]:
            raise ValueError("Checkpoint must use the currently installed image")
        mounted = {m["target"]: m for m in service.get("volumes", [])}
        for target in ("/var/lib/recollect", "/var/lib/recollect/artifacts", "/var/lib/recollect/erasure-journal"):
            mount = mounted.get(target, {})
            source = Path(mount.get("source", "/unavailable"))
            if mount.get("type") != "bind" or source.is_symlink() or not source.is_dir() or not source.resolve().is_relative_to(install.ROOT / ".data"):
                raise ValueError("Root recovery requires owned regular repository data paths")
        env = service.get("environment", {})
        if role != "migrate" and any(k in env for k in ("DATABASE_ADMIN_URL", "POSTGRES_PASSWORD", "VAULT_TOKEN")):
            raise ValueError("Application roles contain an administrative credential name")
        if env.get("RECOLLECT_CREDENTIAL_FILE") != "/var/lib/recollect/credentials.json":
            raise ValueError("Root account file no longer uses its declared mounted path")
    return {"name": NAME, "mode": "personal", "origin": saved["origin"], "project": "recollect",
            "configured_only": True, "journal_mirror_configured": False,
            "services": [{"name": k, "image": v["image"]} for k, v in services.items()]}


def register():
    # Inspect immutable image IDs, never environments or container arguments.
    env = {k: v for k, v in __import__("os").environ.items() if k in
           ("PATH", "HOME", "USER", "TMPDIR", "DOCKER_HOST", "DOCKER_CONTEXT", "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH")}
    result = subprocess.run(base() + ["ps", "--all", "--quiet", "api", "worker"],
                            cwd=install.ROOT, env=env, capture_output=True, text=True, timeout=15)
    ids = result.stdout.split()
    if result.returncode or len(ids) != 2 or any(not re.fullmatch(r"[a-f0-9]{12,64}", i) for i in ids):
        raise ValueError("Update requires the existing owned API and worker containers")
    result = subprocess.run([str(install.ROOT / "scripts/docker.sh"), "inspect", "--format", "{{.Image}}"] + ids,
                            cwd=install.ROOT, env=env, capture_output=True, text=True, timeout=15)
    images = set(result.stdout.split())
    if result.returncode or len(images) != 1:
        raise ValueError("API and worker must use the same installed checkpoint image")
    saved = {"name": NAME, "project": "recollect", "workspace": str(install.ROOT), "root": True,
             "mode": "personal", "origin": "http://127.0.0.1:8787", "checkpoint_image": images.pop()}
    validate(saved)
    directory = install.directory(NAME)
    if directory.exists():
        old = install.load(NAME)
        validate(old)
    else:
        directory.mkdir(mode=0o700, parents=True)
    private_json(directory / "checkpoint-image.json", {"services": {role: {"image": saved["checkpoint_image"]}
                 for role in ("api", "worker", "migrate")}})
    private_json(directory / "installation.json", saved)
    configuration(saved)
    return saved
