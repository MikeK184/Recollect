#!/usr/bin/env python3
"""Operate one explicitly named Recollect installation; never reset its data."""
import argparse
import ipaddress
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import ssl
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
INSTALLS = ROOT / ".data" / "install"


def private_write(path, text):
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as out:
        out.write(text)


def environment(values):
    lines = []
    for key, value in values.items():
        value = str(value)
        if any(c in value for c in "\n\r\0"):
            raise ValueError("Environment values must be single-line text")
        lines.append(key + "='" + value.replace("'", "\\'") + "'\n")
    return "".join(lines)


def directory(name):
    if not re.fullmatch(r"[a-z][a-z0-9-]{0,39}", name):
        raise ValueError("Use an installation name of 1–40 lowercase letters, digits or hyphens")
    path = INSTALLS / name
    if path.is_symlink() or path.resolve().parent != INSTALLS.resolve():
        raise ValueError("Installation must be a direct owned directory")
    return path


def settings(args):
    if args.mode == "personal":
        if args.origin or args.cert or args.key or args.bind not in (None, "127.0.0.1"):
            raise ValueError("Personal mode uses loopback HTTP; select shared for HTTPS")
        port = args.port if args.port is not None else 8787
        origin, bind, cert, key = f"http://127.0.0.1:{port}", "127.0.0.1", None, None
    else:
        if not args.origin or not args.cert or not args.key:
            raise ValueError("Shared mode requires --origin, --cert and --key")
        parsed = urllib.parse.urlsplit(args.origin)
        if (parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password
                or parsed.path not in ("", "/") or parsed.query or parsed.fragment
                or not re.fullmatch(r"[a-zA-Z0-9.-]+", parsed.hostname)):
            raise ValueError("Use a fixed HTTPS hostname origin without a path or credentials")
        port = parsed.port or 443
        if args.port is not None and args.port != port:
            raise ValueError("The shared public port must match the HTTPS origin")
        origin = "https://" + parsed.hostname + (f":{port}" if port != 443 else "")
        bind = args.bind or "0.0.0.0"
        ipaddress.ip_address(bind)
        cert, key = (str(Path(p).resolve(strict=True)) for p in (args.cert, args.key))
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        try:
            context.load_cert_chain(cert, key)
        except ssl.SSLError as error:
            raise ValueError("TLS certificate/key could not be loaded as a matching pair") from error
    if not 1 <= port <= 65535:
        raise ValueError("Public port must be between 1 and 65535")
    return {"name": args.name, "mode": args.mode, "origin": origin, "port": port,
            "bind": bind, "cert": cert, "key": key, "project": f"recollect-install-{args.name}"}


def initialize(args):
    path, desired = directory(args.name), settings(args)
    if path.exists():
        saved = load(args.name)
        if saved != desired:
            raise ValueError("This installation already has different settings; existing files were preserved")
        print(f"Preserved existing installation {args.name}. Configuration: {path}")
        return
    path.mkdir(mode=0o700, parents=True, exist_ok=False)
    db, app, owner, graph = (secrets.token_urlsafe(32) for _ in range(4))
    runtime = {
        "DATABASE_URL": f"postgres://recollect_app:{app}@postgres:5432/recollect",
        "RECOLLECT_OWNER_USERNAME": "owner", "RECOLLECT_OWNER_PASSWORD": owner,
        "RECOLLECT_PUBLIC_ORIGIN": desired["origin"],
        "NEO4J_URL": "http://neo4j:7474", "NEO4J_USERNAME": "neo4j", "NEO4J_PASSWORD": graph,
        "RECOLLECT_TEXT_MODEL": "gpt-5.6-luna",
        "RECOLLECT_EMBEDDING_MODEL": "text-embedding-3-large",
        "RECOLLECT_EMBEDDING_DIMENSIONS": "3072",
        "OPENAI_API_KEY": "",
    }
    private_write(path / "runtime.env", environment(runtime))
    private_write(path / "operator.env", environment({
        "DATABASE_ADMIN_URL": f"postgres://recollect_admin:{db}@postgres:5432/recollect"}))
    private_write(path / "database.env", environment({"POSTGRES_USER": "recollect_admin",
        "POSTGRES_DB": "recollect", "POSTGRES_PASSWORD": db, "RECOLLECT_DB_PASSWORD": app}))
    private_write(path / "graph.env", environment({"NEO4J_AUTH": f"neo4j/{graph}"}))
    private_write(path / "mcp-bindings.json", '{"bindings":[],"vault_sources":{}}\n')
    # This file contains references only, never secret values. Its parent remains
    # private; container UID 10001 needs read access to the single read-only mount.
    (path / "mcp-bindings.json").chmod(0o644)
    private_write(path / "overrides.yaml", "# Operator-owned provider/image additions for this installation only.\nservices: {}\n")
    compose = {"RECOLLECT_INSTALL_DIR": str(path), "RECOLLECT_PUBLIC_PORT": desired["port"],
               "RECOLLECT_PUBLIC_BIND": desired["bind"]}
    if desired["mode"] == "shared":
        compose.update(RECOLLECT_SITE_HOST=urllib.parse.urlsplit(desired["origin"]).hostname,
                       RECOLLECT_TLS_CERT=desired["cert"], RECOLLECT_TLS_KEY=desired["key"])
    private_write(path / "compose.env", environment(compose))
    # The descriptor is written last; partial initialization is explicit and is
    # never silently regenerated over credentials that may already have been used.
    private_write(path / "installation.json", json.dumps(desired, indent=2) + "\n")
    print(f"Created {args.mode} installation {args.name}. Owner credentials: {path / 'runtime.env'}")
    print(f"Configured origin: {desired['origin']}. No service has been started.")


def load(name):
    path = directory(name)
    try:
        saved = json.loads((path / "installation.json").read_text())
        if (saved["name"] != name or saved["project"] != f"recollect-install-{name}"
                or saved["mode"] not in ("personal", "shared")):
            raise ValueError("Saved installation ownership is invalid")
        for file in ("runtime.env", "operator.env", "database.env", "graph.env", "compose.env", "mcp-bindings.json"):
            if not (path / file).is_file():
                raise ValueError("Installation configuration is incomplete; existing files were preserved")
        return saved
    except (OSError, KeyError, json.JSONDecodeError) as error:
        raise ValueError("Installation configuration is missing or incomplete; initialize a new explicit name") from error


def compose_command(saved):
    command = [str(ROOT / "scripts/docker.sh"), "compose", "--project-directory", str(ROOT / "infra/product"),
            "--project-name", saved["project"], "--env-file", str(directory(saved["name"]) / "compose.env"),
            "-f", str(ROOT / "infra/product/compose.yaml"),
            "-f", str(ROOT / "infra/product" / (saved["mode"] + ".yaml"))]
    override = directory(saved["name"]) / "overrides.yaml"
    if override.is_file():
        command.extend(["-f", str(override)])
    recovery = directory(saved["name"]) / "recovery.yaml"
    if recovery.is_file():
        command.extend(["-f", str(recovery)])
    return command


def run(saved, arguments, *, capture=False, timeout=300):
    # Keep CLI/context discovery, never ambient application or root credentials.
    env = {k: v for k, v in os.environ.items() if k in (
        "PATH", "HOME", "USER", "TMPDIR", "DOCKER_HOST", "DOCKER_CONTEXT", "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH")}
    try:
        result = subprocess.run(compose_command(saved) + arguments, cwd=ROOT, env=env,
                                capture_output=capture, text=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ValueError("Docker command is unavailable or exceeded its deadline; inspect saved service state") from error
    if result.returncode != 0:
        # Captured config output may contain credentials; never echo it on error.
        raise ValueError("Compose command failed; no successful runtime state is claimed")
    return result.stdout if capture else None


def configuration(saved):
    # Parse privately; output only a fixed, secret-free summary of the resolved model.
    resolved = json.loads(run(saved, ["config", "--format", "json"], capture=True, timeout=15))
    services = resolved["services"]
    if resolved["name"] != saved["project"]:
        raise ValueError("Resolved Compose project differs from saved installation ownership")
    for name in ("postgres_data", "graph_data", "graph_logs", "graph_plugins", "app_data"):
        volume = resolved.get("volumes", {}).get(name, {})
        if volume.get("external") or volume.get("name") != saved["project"] + "_" + name:
            raise ValueError("Required data volumes must belong to the saved installation")
    for role, source, target in (("postgres", "postgres_data", "/var/lib/postgresql/data"),
                                 ("neo4j", "graph_data", "/data"),
                                 ("api", "app_data", "/var/lib/recollect"),
                                 ("worker", "app_data", "/var/lib/recollect"),
                                 ("migrate", "app_data", "/var/lib/recollect")):
        if not any(mount.get("type") == "volume" and mount.get("source") == source and mount.get("target") == target
                   for mount in services[role].get("volumes", [])):
            raise ValueError("A required runtime data path no longer uses its owned volume")
    for name in ("api", "worker"):
        env = services[name].get("environment", {})
        if any(key in env for key in ("DATABASE_ADMIN_URL", "VAULT_TOKEN", "POSTGRES_PASSWORD")):
            raise ValueError("Application environment includes an administrative credential name")
    if "DATABASE_ADMIN_URL" not in services["migrate"].get("environment", {}):
        raise ValueError("Migration service is missing its explicit administrative URL")
    mirrors = {services[role].get("environment", {}).get("RECOLLECT_ERASURE_MIRROR_CONFIG")
               for role in ("api", "worker", "migrate")}
    if len(mirrors) != 1:
        raise ValueError("API, worker and operator must share the optional deletion mirror configuration")
    for name in ("postgres", "neo4j") + (("api",) if saved["mode"] == "shared" else ()):
        if services[name].get("ports"):
            raise ValueError("An internal service unexpectedly publishes a host port")
    for service in services.values():
        if (int(service.get("mem_limit", 0)) <= 0 or float(service.get("cpus", 0)) <= 0
                or int(service.get("pids_limit", 0)) <= 0):
            raise ValueError("Every installation service requires positive memory, CPU and PID limits")
    return {"name": saved["name"], "mode": saved["mode"], "origin": saved["origin"],
            "project": resolved["name"], "configured_only": True,
            "journal_mirror_configured": bool(next(iter(mirrors))),
            "services": [{"name": key, "image": value.get("image"),
                          "memory_limit": value.get("mem_limit"), "cpus": value.get("cpus"),
                          "ports": value.get("ports", [])} for key, value in services.items()]}


def build(saved):
    if shutil.disk_usage(ROOT).free < 12 * 1024**3:
        raise ValueError("Building requires at least 12 GiB free; no caches or data were deleted")
    run(saved, ["build", "api"], timeout=3600)


def start(saved):
    configuration(saved)
    names = ["api", "worker"] + (["proxy"] if saved["mode"] == "shared" else [])
    run(saved, ["stop", "--timeout", "90"] + names, timeout=300)
    run(saved, ["up", "-d", "--wait", "--wait-timeout", "300", "postgres", "neo4j"], timeout=330)
    # Always rerun the explicit migration role, including after a stopped install.
    # No application starts after a failed migration or privacy reconciliation.
    run(saved, ["up", "--no-deps", "--force-recreate", "--abort-on-container-exit",
                "--exit-code-from", "migrate", "migrate"], timeout=300)
    run(saved, ["up", "-d", "--wait", "--wait-timeout", "300"] + names, timeout=330)
    print(f"Started saved installation {saved['name']}. Open {saved['origin']}.")


def diagnostics(saved, ca=None):
    report = {"name": saved["name"], "mode": saved["mode"], "origin": saved["origin"],
              "services": [], "resources": [], "health": {}}
    try:
        raw = run(saved, ["ps", "--all", "--format", "json"], capture=True, timeout=10)
        rows = json.loads(raw) if raw.lstrip().startswith("[") else [json.loads(line) for line in raw.splitlines() if line.strip()]
        report["services"] = [{k: row.get(k) for k in ("Service", "State", "Health", "ExitCode")} for row in rows]
        report["docker"] = "connected"
    except (ValueError, json.JSONDecodeError):
        report["docker"] = "unavailable"
    if report["docker"] == "connected":
        try:
            raw = run(saved, ["stats", "--no-stream", "--format", "json"], capture=True, timeout=10)
            rows = json.loads(raw) if raw.lstrip().startswith("[") else [json.loads(line) for line in raw.splitlines() if line.strip()]
            report["resources"] = [{k: row.get(k) for k in ("Name", "CPUPerc", "MemUsage", "MemPerc", "PIDs")} for row in rows]
            report["resource_observation"] = "available"
        except (ValueError, json.JSONDecodeError):
            report["resource_observation"] = "unavailable"
    context = ssl.create_default_context(cafile=ca)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), urllib.request.HTTPSHandler(context=context))
    for kind in ("live", "ready"):
        try:
            with opener.open(saved["origin"] + "/health/" + kind, timeout=10) as response:
                report["health"][kind] = {"status": response.status}
        except urllib.error.HTTPError as error:
            report["health"][kind] = {"status": error.code}
        except (OSError, urllib.error.URLError):
            report["health"][kind] = {"status": "unavailable"}
    states = {row["Service"]: row for row in report["services"]}
    required = ["postgres", "neo4j", "api", "worker"] + (["proxy"] if saved["mode"] == "shared" else [])
    running = all(states.get(name, {}).get("State") == "running" for name in required)
    healthy = all(states.get(name, {}).get("Health") == "healthy" for name in ("postgres", "neo4j", "api"))
    report["ready"] = (report.get("docker") == "connected" and running and healthy
                       and report["health"].get("ready", {}).get("status") == 200)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    init = commands.add_parser("init")
    init.add_argument("name")
    init.add_argument("mode", choices=("personal", "shared"))
    init.add_argument("--origin")
    init.add_argument("--port", type=int)
    init.add_argument("--bind")
    init.add_argument("--cert")
    init.add_argument("--key")
    for action in ("config", "build", "start", "stop", "diagnose"):
        command = commands.add_parser(action)
        command.add_argument("name")
        if action == "diagnose":
            command.add_argument("--ca", help="Explicit CA file for this diagnostic client only")
    args = parser.parse_args()
    try:
        if args.command == "init":
            initialize(args)
            return
        saved = load(args.name)
        if args.command == "config":
            print(json.dumps(configuration(saved), indent=2))
        elif args.command == "build":
            configuration(saved)
            build(saved)
        elif args.command == "start":
            start(saved)
        elif args.command == "stop":
            run(saved, ["stop", "--timeout", "90"], timeout=300)
            print("Stopped only this installation's services; persistent data was preserved.")
        else:
            report = diagnostics(saved, args.ca)
            print(json.dumps(report, indent=2))
            if not report["ready"]:
                sys.exit(1)
    except (ValueError, OSError) as error:
        parser.exit(1, str(error) + "\n")


if __name__ == "__main__":
    main()
