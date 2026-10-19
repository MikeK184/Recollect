#!/usr/bin/env python3
"""Install the verified Enola adapter in Recollect's ignored tool directory."""
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.4.19"
LIMIT = 100 * 1024 * 1024
# SHA-256 of each release tarball (enola-<VERSION>-<system>-<arch>.tar.gz). They equal the
# release's published enola-<VERSION>-<system>-<arch>.sha256 assets and the digests GitHub
# reports. Maintainer: when bumping VERSION, copy the new values from those assets here and
# update the techstack Enola links together.
SHA256 = {
    ("darwin", "amd64"): "20b35dd15aa407d2885c025ba7407598c007ef767a44f202057c234b54ea7637",
    ("darwin", "arm64"): "8893bf0277b87261ebdf76c952cfc1556295846d408c7c3d035ace80a64e56ce",
    ("linux", "amd64"): "189125c37ae1e66a498b0bafe79a5a6a5612b45c975ab4431e13f58cbef31397",
    ("linux", "arm64"): "eaecc987cf4b4f9aedb641cad161304370f86ced8f975c271f76b1b7b01d4840",
}


def fetch(url, expected):
    """Download `url` and return its bytes only if they match the pinned SHA-256."""
    request = urllib.request.Request(url, headers={"User-Agent": "Recollect-local-setup"})
    with urllib.request.urlopen(request, timeout=30) as response:
        archive = response.read(LIMIT + 1)
    if len(archive) > LIMIT:
        raise SystemExit("Release archive exceeded the setup limit.")
    if hashlib.sha256(archive).hexdigest() != expected:
        raise SystemExit("Release archive does not match its pinned SHA-256; nothing was extracted.")
    return archive


def install(folder, system, arch):
    binary = folder / "enola"
    if not binary.exists():
        asset = f"enola-{VERSION}-{system}-{arch}"
        archive = fetch(f"https://github.com/enola-labs/enola/releases/download/v{VERSION}/{asset}.tar.gz",
                        SHA256[(system, arch)])
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as release:
            for source, destination in [(asset, "enola"), ("LICENSE", "LICENSE"), ("NOTICE", "NOTICE")]:
                members = [m for m in release.getmembers() if m.name.removeprefix("./") == source]
                if len(members) != 1 or not members[0].isfile() or members[0].size > LIMIT:
                    raise SystemExit("Release is missing a regular expected member.")
                target = folder / destination
                if target.exists():
                    continue
                with target.open("xb") as handle:
                    os.chmod(target, 0o700 if destination == "enola" else 0o600)
                    handle.write(release.extractfile(members[0]).read())
                    handle.flush()
                    os.fsync(handle.fileno())
    result = subprocess.run([str(binary), "--version", "--json"], env={"PATH": "/usr/bin:/bin", "ENOLA_NO_UPDATE_CHECK": "1"}, cwd=folder, capture_output=True, timeout=10, check=True)
    version = json.loads(result.stdout)
    return {"binary": str(binary), "version": version.get("version"), "extractor_version": version.get("extractor_version")}


def main():
    system = {"Darwin": "darwin", "Linux": "linux"}.get(platform.system())
    arch = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "amd64"}.get(platform.machine())
    if not system or not arch:
        raise SystemExit("This setup supports macOS/Linux ARM64 or AMD64. Select an explicit RECOLLECT_ENOLA_BIN otherwise.")
    folder = ROOT / ".cache" / "enola-tools" / f"v{VERSION}"
    folder.mkdir(parents=True, exist_ok=True, mode=0o700)
    if folder.resolve() != folder:
        raise SystemExit("Tool directory must remain inside Recollect without symlink redirection.")
    print(json.dumps(install(folder, system, arch)))


if __name__ == "__main__":
    main()
