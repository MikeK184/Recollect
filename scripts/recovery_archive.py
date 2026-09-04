"""Authenticated recovery packages. Call extract only AFTER age succeeds."""
import json
import os
from datetime import datetime
from pathlib import Path, PurePosixPath
import re
import shutil
import tarfile
import uuid

# Explicit local operator envelope, not a production capacity promise.
MAX_BYTES = 64 * 1024**3
MAX_FILES = 200_000


def identifier(value):
    if not isinstance(value, str) or str(uuid.UUID(value)) != value:
        raise ValueError("Recovery identity is invalid")
    return value


def extract(archive, destination, *, allowed):
    destination = Path(destination)
    if not destination.is_dir() or any(destination.iterdir()) or destination.is_symlink():
        raise ValueError("Archive extraction requires a private empty directory")
    total, seen = 0, set()
    with tarfile.open(archive, "r:") as source:
        for member in source:
            name = member.name.removeprefix("./").rstrip("/")
            if name in ("", ".") and member.isdir():
                continue
            path = PurePosixPath(name)
            if (path.is_absolute() or ".." in path.parts or str(path) != name
                    or not path.parts or path.parts[0] not in allowed
                    or any(ord(c) < 32 or ord(c) == 127 for c in name)
                    or name in seen or not (member.isfile() or member.isdir())
                    or member.size < 0):
                raise ValueError("Archive contains an unexpected path, duplicate or non-regular member")
            seen.add(name)
            total += member.size
            if total > MAX_BYTES or len(seen) > MAX_FILES:
                raise ValueError("Recovery archive exceeds the supported local envelope")
            target = destination.joinpath(*path.parts)
            target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            if member.isdir():
                target.mkdir(mode=0o700, exist_ok=True)
            else:
                with os.fdopen(os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb") as out:
                    data = source.extractfile(member)
                    if data is None:
                        raise ValueError("Archive member cannot be read")
                    shutil.copyfileobj(data, out, length=1024 * 1024)
                if target.stat().st_size != member.size:
                    raise ValueError("Archive member has an unexpected length")
    return seen


def pack(directory, destination):
    directory = Path(directory)
    files = sorted(directory.rglob("*"))
    if len(files) > MAX_FILES or sum(p.stat().st_size for p in files if p.is_file()) > MAX_BYTES:
        raise ValueError("Recovery package exceeds the supported local envelope")
    with tarfile.open(destination, "x:") as target:
        for path in files:
            if path.is_symlink() or not (path.is_dir() or path.is_file()):
                raise ValueError("Recovery packages require regular files and directories")
            target.add(path, arcname=path.relative_to(directory).as_posix(), recursive=False)
    Path(destination).chmod(0o600)


def json_file(path, maximum=32 * 1024 * 1024):
    path = Path(path)
    if path.is_symlink() or not path.is_file() or path.stat().st_size > maximum:
        raise ValueError("Recovery metadata is missing, oversized or not a regular file")
    try:
        return json.loads(path.read_text())
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError("Recovery metadata is invalid") from error


def validate_bundle(root, schema):
    root = Path(root)
    manifest = json_file(root / "manifest.json")
    for key in ("backup_id", "installation_id"):
        identifier(manifest.get(key))
    applied = manifest.get("migrations")
    if (not isinstance(applied, list) or not applied
            or applied != schema["migrations"][:len(applied)]
            or manifest.get("postgres_major") != schema["postgres_major"]):
        raise ValueError("Backup database history is unknown, gapped or incompatible")
    if not (root / "database.dump").is_file() or (root / "database.dump").stat().st_size != manifest.get("database_dump_bytes"):
        raise ValueError("Required database dump is missing or has a different length")
    if not 1 <= manifest.get("backup_days", 0) <= 365:
        raise ValueError("Backup retention admission is invalid")
    expected = {"manifest.json", "database.dump"}
    for artifact in manifest.get("artifacts", []):
        brain, item = identifier(artifact["brain_id"]), identifier(artifact["id"])
        name = f"app/artifacts/{brain}/{item}.txt"
        path = root / name
        if not path.is_file() or path.stat().st_size != artifact["byte_length"]:
            raise ValueError("Required retained artifact is missing or has a different length")
        expected.add(name)
    for name, length in manifest.get("application_files", {}).items():
        if (name != "account-credentials.json" and not re.fullmatch(r"mcp-receipts/[a-zA-Z0-9/._-]+", name)):
            raise ValueError("Unexpected application file in recovery manifest")
        path = root / "app" / name
        if ".." in PurePosixPath(name).parts or not path.is_file() or path.stat().st_size != length:
            raise ValueError("Required application file is missing or has a different length")
        expected.add("app/" + name)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file()}
    if actual != expected:
        raise ValueError("Backup contains files outside its admitted manifest")
    if manifest.get("credential_count", 0) and "app/account-credentials.json" not in actual:
        raise ValueError("Restored accounts require the retained credential file")
    return manifest


def validate_journals(root, manifest):
    """Sequence values may have legitimate PostgreSQL rollback gaps. Compare
    exact retained identities to the independent inventory, never max(sequence)
    alone or a made-up requirement that identity sequences are contiguous.
    """
    root = Path(root)
    inventory = json_file(root / "journal-inventory.json")
    installation = identifier(manifest["installation_id"])
    if inventory.get("installation_id") != installation:
        raise ValueError("Independent journal belongs to another installation")
    if "checkpoint_at" in manifest:
        try:
            checkpoint = datetime.fromisoformat(manifest["checkpoint_at"].replace("Z", "+00:00"))
            exported = datetime.fromisoformat(inventory["exported_at"].replace("Z", "+00:00"))
            if checkpoint.tzinfo is None or exported.tzinfo is None or exported < checkpoint:
                raise ValueError("Independent journal export is older than the data checkpoint")
        except (KeyError, TypeError, AttributeError) as error:
            raise ValueError("Independent journal export time is missing or invalid") from error
    entries = inventory.get("entries")
    if not isinstance(entries, list):
        raise ValueError("Independent journal inventory is missing")
    indexed, actual = {}, {}
    for entry in entries:
        identity = identifier(entry["id"])
        sequence = entry["sequence"]
        if type(sequence) is not int or sequence <= 0 or identity in indexed or sequence in indexed.values():
            raise ValueError("Independent journal inventory has invalid or duplicate identities")
        indexed[identity] = sequence
    directory = root / "erasure-journal" / installation
    if json_file(directory / "installation.json", 128) != installation:
        raise ValueError("Independent erasure journal marker differs")
    for file in directory.iterdir():
        if file.name == "installation.json":
            continue
        identity = identifier(file.stem)
        value = json_file(file)
        if file.suffix != ".json" or value.get("id") != identity or value.get("installation_id") != installation:
            raise ValueError("Independent erasure journal entry differs")
        actual[identity] = value["sequence"]
    if indexed != actual:
        raise ValueError("Independent erasure journal is incomplete")
    for entry in manifest["journal_entries"]:
        if indexed.get(entry["id"]) != entry["sequence"]:
            raise ValueError("Independent journal is older than the data checkpoint")
    analytic = root / "erasure-journal" / ("analytics-" + installation)
    if json_file(analytic / "installation.json", 128) != installation:
        raise ValueError("Independent analytical journal marker differs")
    expected = set(inventory.get("analytics", []))
    files = {p.name for p in analytic.iterdir() if p.name != "installation.json"}
    if files != expected:
        raise ValueError("Independent analytical journal is incomplete")
    for name in files:
        value = json_file(analytic / name, 8192)
        if value.get("installation_id") != installation or name != identifier(value.get("id")) + ".json":
            raise ValueError("Analytical ownership journal differs")
    expected_files = {"journal-inventory.json", f"erasure-journal/{installation}/installation.json",
                      f"erasure-journal/analytics-{installation}/installation.json"}
    expected_files.update(f"erasure-journal/{installation}/{item}.json" for item in indexed)
    expected_files.update(f"erasure-journal/analytics-{installation}/{name}" for name in files)
    if {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file()} != expected_files:
        raise ValueError("Independent journal contains files outside its installation inventory")
    return inventory


def remote_chain(records, installation):
    """Remote links refer to canonical committed predecessors, including real
    sequence gaps. All links must be present in this independent retained set.
    """
    ordered = sorted(records, key=lambda r: r["entry"]["sequence"])
    previous, seen = None, set()
    for record in ordered:
        entry = record["entry"]
        identity = identifier(entry["id"])
        if (entry.get("installation_id") != installation or identity in seen
                or type(entry.get("sequence")) is not int or entry["sequence"] <= 0
                or record.get("previous") != previous):
            raise ValueError("Remote erasure journal has missing or inconsistent predecessors")
        seen.add(identity)
        previous = {"id": identity, "sequence": entry["sequence"]}
    return [r["entry"] for r in ordered]
