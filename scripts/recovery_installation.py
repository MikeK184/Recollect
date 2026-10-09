"""Recovery I/O restricted to one validated saved installation."""
import json
import os
from pathlib import Path
import re
import subprocess
import uuid

import install


def environment():
    return {k: v for k, v in os.environ.items() if k in (
        "PATH", "HOME", "USER", "TMPDIR", "DOCKER_HOST", "DOCKER_CONTEXT", "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH")}


def command(saved, arguments, *, source=None, destination=None, timeout=300):
    try:
        result = subprocess.run(install.compose_command(saved) + arguments,
                                stdin=source if source is not None else subprocess.DEVNULL,
                                stdout=destination if destination is not None else subprocess.PIPE,
                                stderr=subprocess.PIPE, env=environment(), cwd=install.ROOT, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ValueError("Installation recovery command unavailable or deadline exceeded") from error
    if result.returncode:
        raise ValueError("Installation recovery command failed; the recorded hold/state is preserved")
    return result.stdout


def sql(saved, statement):
    # SQL statements here are fixed application metadata queries. Database
    # credentials remain in the selected container's environment, never argv.
    import tempfile
    with tempfile.TemporaryFile() as input_file:
        input_file.write(statement.encode())
        input_file.seek(0)
        raw = command(saved, ["exec", "-T", "postgres", "sh", "-c",
            'PGPASSWORD="$POSTGRES_PASSWORD" exec psql -X -h 127.0.0.1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At --set=ON_ERROR_STOP=1'], source=input_file)
    return raw.decode().strip()


def inventory(saved):
    return json.loads(sql(saved, """
SELECT jsonb_build_object(
 'installation_id',(SELECT id FROM privacy_installation),
 'checkpoint_at',clock_timestamp(),
 'postgres_major',current_setting('server_version_num')::integer / 10000,
 'database_bytes',pg_database_size(current_database()),
 'migrations',(SELECT jsonb_agg(name ORDER BY name) FROM recollect_migrations),
 'backup_days',(SELECT coalesce(min(coalesce((p.policy->>'backup_days')::integer,7)),7)
    FROM brains b LEFT JOIN retention_policies p ON p.brain_id=b.id),
 'brain_ids',(SELECT coalesce(jsonb_agg(id ORDER BY id),'[]') FROM brains),
 'brain_retention',(SELECT coalesce(jsonb_object_agg(b.id,coalesce((p.policy->>'backup_days')::integer,7)),'{}')
    FROM brains b LEFT JOIN retention_policies p ON p.brain_id=b.id),
 'credential_count',(SELECT count(*) FROM accounts WHERE credential_id IS NOT NULL),
 'journal_entries',(SELECT coalesce(jsonb_agg(jsonb_build_object('id',id,'sequence',sequence) ORDER BY sequence),'[]') FROM recollect_privacy_positions()),
 'artifacts',(SELECT coalesce(jsonb_agg(jsonb_build_object('brain_id',brain_id,'id',id,'byte_length',byte_length) ORDER BY brain_id,id),'[]') FROM (
   SELECT brain_id,artifact_id AS id,byte_length FROM source_versions WHERE artifact_id IS NOT NULL AND privacy_state='active'
   UNION SELECT brain_id,id,byte_length FROM repository_artifacts
   UNION SELECT brain_id,artifact_id,byte_length FROM repository_files WHERE artifact_id IS NOT NULL) a),
 'counts',jsonb_build_object('brains',(SELECT count(*) FROM brains),'sources',(SELECT count(*) FROM source_versions),
    'claims',(SELECT count(*) FROM claims),'model_requests',(SELECT count(*) FROM model_requests),'mcp_calls',(SELECT count(*) FROM mcp_calls))
);
"""))


def services(saved):
    raw = command(saved, ["ps", "--all", "--format", "json"]).decode()
    return json.loads(raw) if raw.lstrip().startswith("[") else [json.loads(line) for line in raw.splitlines() if line.strip()]


def restore_label(restore_id):
    if str(uuid.UUID(restore_id)) != restore_id:
        raise ValueError("Restore ownership is invalid")
    return ["--label", "io.recollect.recovery=" + restore_id]


def application(saved, action, restore_id=None):
    label = restore_label(restore_id) if restore_id else []
    return command(saved, ["run", "--rm", "--no-deps", "-T"] + label + ["migrate", action])


def retire_restore_helpers(saved, restore_id):
    label = restore_label(restore_id)[1]
    docker = str(install.ROOT / "scripts/docker.sh")
    result = subprocess.run([docker, "ps", "--no-trunc", "-q", "--filter",
        "label=com.docker.compose.project=" + saved["project"], "--filter",
        "label=com.docker.compose.service=migrate", "--filter", "label=" + label],
        capture_output=True, text=True, env=environment(), timeout=15)
    if result.returncode:
        raise ValueError("Cannot inspect interrupted restore helpers")
    containers = result.stdout.split()
    if not all(re.fullmatch(r"[0-9a-f]{64}", value) for value in containers):
        raise ValueError("Interrupted restore helper ownership is invalid")
    if containers:
        result = subprocess.run([docker, "stop", "--timeout", "5"] + containers,
            capture_output=True, env=environment(), timeout=30)
        if result.returncode:
            raise ValueError("Cannot retire interrupted restore helpers")


def retire_restore_import(saved, restore_id):
    restore_label(restore_id)
    # pg_restore may survive loss of its attached CLI. Terminate only this
    # attempt's import, waiting for rollback before checking the checkpoint.
    result = sql(saved, f"""
SELECT pg_terminate_backend(pid,5000) FROM pg_stat_activity
WHERE application_name='recollect-restore-{restore_id}'
  AND datname=current_database() AND usename=current_user AND pid<>pg_backend_pid();
""")
    if any(line != "t" for line in result.splitlines()):
        raise ValueError("Interrupted database import did not stop")


def schema(saved):
    # Compose run would create app_data before the fresh-target check. Probe the
    # selected image without mounts, credentials, network or persistent state.
    resolved = install.configuration(saved)
    image = next(s["image"] for s in resolved["services"] if s["name"] == "api")
    result = subprocess.run([str(install.ROOT / "scripts/docker.sh"), "run", "--rm",
        "--network", "none", "--read-only", "--memory", "128m", "--cpus", "1", "--pids-limit", "32",
        "--entrypoint", "recollect-server", image, "recovery-schema"],
        capture_output=True, env=environment(), timeout=30)
    if result.returncode:
        raise ValueError("Selected image does not expose compatible recovery schema information")
    return json.loads(result.stdout)


def snapshot(saved, destination):
    with Path(destination).open("xb") as output:
        command(saved, ["run", "--rm", "--no-deps", "-T", "--entrypoint", "tar", "migrate",
                        "-cf", "-", "-C", "/var/lib/recollect"] +
                        (["artifacts", "credentials.json", "erasure-journal"] if saved.get("root") else ["."]), destination=output)


def upload(saved, source, restore_id=None):
    label = restore_label(restore_id) if restore_id else []
    with Path(source).open("rb") as input_file:
        command(saved, ["run", "--rm", "--no-deps", "-T"] + label + ["--entrypoint", "tar", "migrate",
                        "-xf", "-", "--no-same-owner", "-C", "/var/lib/recollect"], source=input_file)


def dump(saved, destination):
    with Path(destination).open("xb") as output:
        command(saved, ["exec", "-T", "postgres", "sh", "-c",
            'PGPASSWORD="$POSTGRES_PASSWORD" exec pg_dump -h 127.0.0.1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -Fc'], destination=output)


def restore_dump(saved, source, restore_id):
    restore_label(restore_id)
    with Path(source).open("rb") as input_file:
        command(saved, ["exec", "-T", "--env", "PGAPPNAME=recollect-restore-" + restore_id, "postgres", "sh", "-c",
            'PGPASSWORD="$POSTGRES_PASSWORD" exec pg_restore -h 127.0.0.1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" --exit-on-error --single-transaction'], source=input_file)


def fresh(saved):
    if saved.get("root"):
        raise ValueError("Restore requires a fresh named target, never the existing root app")
    # Refuse an existing volume even if containers are stopped. An explicit
    # matching interrupted restore is handled separately by the recovery state.
    result = subprocess.run([str(install.ROOT / "scripts/docker.sh"), "volume", "ls", "--format", "{{.Name}}"],
                            capture_output=True, text=True, env=environment(), timeout=15)
    if result.returncode:
        raise ValueError("Cannot verify fresh destination volumes")
    existing = set(result.stdout.splitlines())
    names = {saved["project"] + "_" + n for n in ("postgres_data", "graph_data", "graph_logs", "graph_plugins", "app_data")}
    if names & existing:
        raise ValueError("Restore requires fresh named destination volumes; existing data was preserved")


def source_stopped(manifest, target=None):
    source = manifest.get("source_installation", {})
    name = source.get("name", "")
    project = source.get("project", "")
    if source.get("root"):
        import root_installation
        root_installation.validate(source)
    elif not re.fullmatch(r"[a-z][a-z0-9-]{0,39}", name) or project != "recollect-install-" + name:
        raise ValueError("Backup source installation identity is invalid")
    projects = {project}
    # A previous restore can be the current writer even while the original
    # backup source stays stopped. Inspect only this tool's owned descriptors;
    # never infer ownership from arbitrary containers or customer installations.
    for directory in install.INSTALLS.iterdir():
        status = directory / "recovery-identity.json"
        if not status.exists():
            status = directory / "recovery-status.json"
        if directory.is_symlink() or status.is_symlink() or not status.is_file():
            continue
        if status.stat().st_size > 32768:
            raise ValueError("Recorded recovery ownership exceeds its metadata envelope")
        recorded = json.loads(status.read_text())
        if (recorded.get("installation_id") == manifest["installation_id"]
                and (status.name == "recovery-identity.json" or recorded.get("operation") == "restore")):
            saved = install.load(directory.name)
            if target is None or saved["project"] != target["project"]:
                projects.add(saved["project"])
    for project in projects:
        if target is not None and project == target["project"]:
            continue
        result = subprocess.run([str(install.ROOT / "scripts/docker.sh"), "ps", "--filter",
            "label=com.docker.compose.project=" + project, "--format", '{{.Label "com.docker.compose.service"}}'],
            capture_output=True, text=True, env=environment(), timeout=15)
        if result.returncode:
            raise ValueError("Cannot inspect source installation ownership")
        if set(result.stdout.splitlines()) & {"api", "worker"}:
            raise ValueError("Stop the current source or previously recovered application before restoring its identity")
