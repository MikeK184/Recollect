#!/usr/bin/env python3
"""Install the pinned official age CLI into this repository's ignored cache."""
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
        url = f"https://dl.filippo.io/age/v{VERSION}?for={system}/{machine}"
        # HTTPS verification uses the host's configured trust store. Never
        # bypass verification to make a failed dependency download succeed.
        with urllib.request.urlopen(url, timeout=30) as response, archive.open("wb") as output:
            total = 0
            while block := response.read(1024 * 1024):
                total += len(block)
                if total > 64 * 1024 * 1024:
                    raise ValueError("Official tool download exceeded its size bound")
                output.write(block)
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
