#!/usr/bin/env python3
"""Install the pinned official age CLI into this repository's ignored cache."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
VERSION = "1.3.2"
LIMIT = 64 * 1024 * 1024
# SHA-256 of each release tarball (age-v1.3.2-<system>-<machine>.tar.gz). age publishes
# no checksum file, only transparency-log `.proof` files, so these values are pinned
# here; they equal the digests GitHub reports for the release assets.
# Maintainer: when bumping VERSION, verify the new release's `.proof` once with
# sigsum-verify (see SIGSUM.md in the age repository), then update this table,
# scripts/update-local.py (the age-v1.3.2-* glob), docs/runbooks/recovery.md,
# scripts/test-recovery-runtime.py and the AGE_* ARGs in infra/product/Dockerfile together.
SHA256 = {
    ("darwin", "amd64"): "1d1e4bc66e1427edad7739ae7616157de0e79db8b6d2a1497d7d9925fb06a539",
    ("darwin", "arm64"): "e2020b073c44f692685a24d6abc378817eb81ffaaf49fd0531ef8565f767f2f5",
    ("linux", "amd64"): "cbe24006683f8eb669266162894b9a522a1af52f2665fbc63a4bb032ed26ac10",
    ("linux", "arm64"): "6b8dc4333c53a5a57c9e5834e3a48f92605d7154014cd07269ff3327db5d37f4",
}


class ChecksumError(ValueError):
    """The downloaded archive does not match its pinned SHA-256."""


def download(url, destination, expected):
    """Stream `url` to `destination`, hashing as it is written; raise before use on mismatch.

    HTTPS verification uses the host's configured trust store. Never bypass
    verification to make a failed dependency download succeed.
    """
    digest = hashlib.sha256()
    total = 0
    with urllib.request.urlopen(url, timeout=30) as response, destination.open("wb") as output:
        while block := response.read(1024 * 1024):
            total += len(block)
            if total > LIMIT:
                raise ValueError("Official tool download exceeded its size bound")
            digest.update(block)
            output.write(block)
    if digest.hexdigest() != expected:
        raise ChecksumError("Official tool download does not match its pinned SHA-256")


def install():
    system = {"Darwin": "darwin", "Linux": "linux"}.get(platform.system())
    machine = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "amd64"}.get(platform.machine())
    if not system or not machine:
        raise ValueError("Recovery tools support macOS/Linux on arm64 or amd64")
    parent = ROOT / ".cache" / "recovery-tools"
    parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if parent.is_symlink() or not parent.resolve().is_relative_to(ROOT):
        raise ValueError("Recovery tool cache must remain inside this repository")
    destination = parent / f"age-v{VERSION}-{system}-{machine}"
    if destination.is_symlink():
        raise ValueError("Recovery tool directory cannot be a symbolic link")
    if destination.exists():
        for name in ("age", "age-keygen"):
            executable = destination / name
            if not executable.is_file() or executable.is_symlink():
                raise ValueError("Existing tool directory is incomplete; it was preserved")
            observed = subprocess.check_output(
                [str(executable), "--version"], text=True, timeout=10
            ).strip()
            if observed != "v" + VERSION:
                raise ValueError("Existing tool version differs; it was preserved")
        return destination
    with tempfile.TemporaryDirectory(prefix="download-", dir=parent) as temporary:
        temporary = Path(temporary)
        archive = temporary / "age.tar.gz"
        url = (f"https://github.com/FiloSottile/age/releases/download/v{VERSION}/"
               f"age-v{VERSION}-{system}-{machine}.tar.gz")
        # Nothing is extracted or executed unless the archive matches its pinned hash.
        download(url, archive, SHA256[(system, machine)])
        staged = temporary / "installed"
        staged.mkdir(mode=0o700)
        with tarfile.open(archive, "r:gz") as bundle:
            for name in ("age", "age-keygen"):
                member = bundle.getmember(f"age/{name}")
                if not member.isfile() or not 0 < member.size <= 64 * 1024 * 1024:
                    raise ValueError("Official tool archive has an invalid executable")
                source = bundle.extractfile(member)
                if source is None:
                    raise ValueError("Official tool archive is incomplete")
                with source, (staged / name).open("xb") as output:
                    shutil.copyfileobj(source, output, length=1024 * 1024)
                (staged / name).chmod(0o700)
                observed = subprocess.check_output(
                    [str(staged / name), "--version"], text=True, timeout=10
                ).strip()
                if observed != "v" + VERSION:
                    raise ValueError("Downloaded tool does not report the selected version")
        os.rename(staged, destination)
    return destination


if __name__ == "__main__":
    try:
        location = install()
        print(json.dumps({"version": VERSION, "directory": str(location)}))
    except (OSError, ValueError, KeyError, tarfile.TarError, subprocess.SubprocessError) as error:
        raise SystemExit("Recovery tool setup failed; existing files were preserved: " + type(error).__name__)
