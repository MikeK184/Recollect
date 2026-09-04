"""Bounded age/OpenSSH operations shared by named recovery commands and drills."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import uuid


def private_json(path, value):
    path = Path(path)
    temporary = path.with_name(f".{path.name}.{uuid.uuid4()}.tmp")
    try:
        with os.fdopen(os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as out:
            json.dump(value, out, indent=2)
            out.write("\n")
            out.flush()
            os.fsync(out.fileno())
        os.replace(temporary, path)
        descriptor = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    finally:
        temporary.unlink(missing_ok=True)


def regular(path):
    path = Path(path)
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise ValueError("Recovery requires an existing absolute regular file reference")
    if any(ord(c) < 32 or ord(c) == 127 for c in str(path)):
        raise ValueError("Recovery paths cannot contain control characters")
    return path


def execute(arguments, *, timeout=60, output=None):
    # Tools receive references/public recipients only. Capture/suppress diagnostics
    # because SSH/age errors can contain operator paths or decrypted content.
    try:
        result = subprocess.run([str(a) for a in arguments], stdin=subprocess.DEVNULL,
                                stdout=output if output is not None else subprocess.PIPE,
                                stderr=subprocess.PIPE, timeout=timeout,
                                env={"PATH": "/usr/bin:/bin:/usr/local/bin", "LC_ALL": "C"})
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ValueError("Recovery tool unavailable or deadline exceeded") from error
    if result.returncode:
        raise ValueError("Recovery encryption or transport did not complete")
    return result.stdout


def recipients(values):
    if (not isinstance(values, list) or not 1 <= len(values) <= 8
            or any(not isinstance(r, str) or not re.fullmatch(r"age1[a-z0-9]{58}", r) for r in values)):
        raise ValueError("Configure one to eight explicit X25519 age recipients")
    return values


def encrypt(binary, values, source, destination):
    destination = Path(destination)
    if destination.exists():
        raise ValueError("Encryption cannot overwrite an existing file")
    try:
        execute([regular(binary), "--encrypt", "--output", destination]
                + [a for r in recipients(values) for a in ("--recipient", r)]
                + [regular(source)], timeout=300)
        destination.chmod(0o600)
        with destination.open("rb") as handle:
            os.fsync(handle.fileno())
    except (ValueError, OSError):
        destination.unlink(missing_ok=True)
        raise


def decrypt(binary, identity, source, destination):
    destination = Path(destination)
    if destination.exists():
        raise ValueError("Decryption cannot overwrite an existing file")
    try:
        execute([regular(binary), "--decrypt", "--identity", regular(identity),
                 "--output", destination, regular(source)], timeout=300)
        destination.chmod(0o600)
    except (ValueError, OSError):
        # age may have written a plaintext prefix before authenticating its final
        # chunk. Never open/parse/extract any output until the process succeeds.
        destination.unlink(missing_ok=True)
        raise


def quote(path):
    value = str(path)
    if any(ord(c) < 32 or ord(c) == 127 for c in value):
        raise ValueError("SFTP paths cannot contain control characters")
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


class Sftp:
    def __init__(self, config, binary="/usr/bin/sftp"):
        expected = {"host", "port", "user", "remote_root", "identity_file", "known_hosts_file"}
        if set(config) != expected:
            raise ValueError("SFTP configuration fields are incomplete or unknown")
        self.config = dict(config)
        self.binary = regular(binary)
        if (not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]{0,252}", config["host"])
                or not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}", config["user"])
                or type(config["port"]) is not int or not 1 <= config["port"] <= 65535):
            raise ValueError("SFTP host, user or port is invalid")
        root = config["remote_root"]
        if (not re.fullmatch(r"/[a-zA-Z0-9/._-]{0,511}", root)
                or any(p in (".", "..") for p in root.split("/"))):
            raise ValueError("Use a fixed absolute SFTP root without traversal")
        regular(config["identity_file"])
        regular(config["known_hosts_file"])

    def run(self, batch):
        with tempfile.TemporaryDirectory(prefix="recollect-sftp-") as directory:
            path = Path(directory) / "commands"
            path.write_text(batch)
            path.chmod(0o600)
            c = self.config
            return execute([self.binary, "-q", "-F", "/dev/null", "-b", path,
                            "-P", c["port"], "-i", c["identity_file"],
                            "-o", f"UserKnownHostsFile={quote(c['known_hosts_file'])}",
                            "-o", "GlobalKnownHostsFile=/dev/null",
                            "-o", "StrictHostKeyChecking=yes", "-o", "BatchMode=yes",
                            "-o", "IdentitiesOnly=yes", "-o", "IdentityAgent=none",
                            "-o", "ForwardAgent=no", "-o", "ClearAllForwardings=yes",
                            "-o", "ProxyCommand=none", "-o", "ProxyJump=none",
                            "-o", "ConnectTimeout=5", "-o", "ServerAliveInterval=5",
                            "-o", "ServerAliveCountMax=2", f"{c['user']}@{c['host']}"], timeout=60)

    def directory(self, installation):
        return self.config["remote_root"].rstrip("/") + "/" + str(uuid.UUID(str(installation)))

    def owner(self, installation):
        return {"installation_id": str(installation), "kind": "recollect-recovery"}

    def check(self, installation):
        with tempfile.TemporaryDirectory(prefix="recollect-owner-") as directory:
            path = Path(directory) / "owner.json"
            self.run(f"get {quote(self.directory(installation) + '/owner.json')} {quote(path)}\n")
            if path.stat().st_size > 1024 or json.loads(path.read_text()) != self.owner(installation):
                raise ValueError("Remote recovery directory has a different owner")

    def initialize(self, installation):
        # mkdir must succeed: an existing directory is checked, never claimed or
        # overwritten. An interrupted empty directory requires operator inspection.
        try:
            self.check(installation)
            return
        except ValueError:
            pass
        remote = self.directory(installation)
        self.run(f"mkdir {quote(remote)}\n")
        with tempfile.TemporaryDirectory(prefix="recollect-owner-") as directory:
            path = Path(directory) / "owner.json"
            private_json(path, self.owner(installation))
            self.run(f"mkdir {quote(remote + '/journal')}\nmkdir {quote(remote + '/backups')}\nmkdir {quote(remote + '/analytics')}\n"
                     f"put -f {quote(path)} {quote(remote + '/owner.json')}\n")
        self.check(installation)

    def put(self, installation, source, relative):
        self.check(installation)
        target = self._path(installation, relative)
        temporary = target + ".pending-" + str(uuid.uuid4())
        self.run(f"put -f {quote(regular(source))} {quote(temporary)}\n"
                 f"rename {quote(temporary)} {quote(target)}\n")

    def get(self, installation, relative, destination):
        self.check(installation)
        if Path(destination).exists():
            raise ValueError("Fetch cannot overwrite an existing local file")
        self.run(f"get {quote(self._path(installation, relative))} {quote(destination)}\n")

    def list(self, installation, kind):
        if kind not in ("journal", "backups", "analytics"):
            raise ValueError("Unknown recovery directory")
        self.check(installation)
        root = self.directory(installation) + "/" + kind
        raw = self.run(f"ls -1 {quote(root)}\n").decode("utf-8")
        if len(raw) > 8 * 1024 * 1024:
            raise ValueError("Remote recovery directory exceeds its listing envelope")
        names = []
        for line in raw.splitlines():
            if line.startswith("sftp>") or not line.strip():
                continue
            name = line.strip().removeprefix(root + "/")
            if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]*", name):
                raise ValueError("Unexpected remote recovery filename")
            names.append(name)
        return sorted(names)

    def remove(self, installation, relative):
        self.check(installation)
        path = self._path(installation, relative)
        kind, name = relative.split("/")
        if kind != "backups":
            raise ValueError("Pruning cannot remove the independent deletion journal")
        if name in self.list(installation, kind):
            self.run(f"rm {quote(path)}\n")
        if name in self.list(installation, kind):
            raise ValueError("Remote prune was not confirmed")

    def _path(self, installation, relative):
        if (not re.fullmatch(r"(?:journal|backups|analytics)/[a-zA-Z0-9._-]+", relative)
                or relative.split("/")[-1] in (".", "..")):
            raise ValueError("Remote operation requires an owned recovery filename")
        return self.directory(installation) + "/" + relative
