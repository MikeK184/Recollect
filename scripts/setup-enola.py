#!/usr/bin/env python3
"""Install the verified Enola adapter in Recollect's ignored tool directory."""
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
system = {"Darwin": "darwin", "Linux": "linux"}.get(platform.system())
arch = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "amd64"}.get(platform.machine())
if not system or not arch:
    raise SystemExit("This setup supports macOS/Linux ARM64 or AMD64. Select an explicit RECOLLECT_ENOLA_BIN otherwise.")
folder = ROOT / ".cache" / "enola-tools" / f"v{VERSION}"
folder.mkdir(parents=True, exist_ok=True, mode=0o700)
if folder.resolve() != folder:
    raise SystemExit("Tool directory must remain inside Recollect without symlink redirection.")
binary = folder / "enola"
if not binary.exists():
    asset = f"enola-{VERSION}-{system}-{arch}"
    request = urllib.request.Request(f"https://github.com/enola-labs/enola/releases/download/v{VERSION}/{asset}.tar.gz", headers={"User-Agent": "Recollect-local-setup"})
    with urllib.request.urlopen(request, timeout=30) as response:
        archive = response.read(100 * 1024 * 1024 + 1)
    if len(archive) > 100 * 1024 * 1024:
        raise SystemExit("Release archive exceeded the setup limit.")
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as release:
        for source, destination in [(asset, "enola"), ("LICENSE", "LICENSE"), ("NOTICE", "NOTICE")]:
            members = [m for m in release.getmembers() if m.name.removeprefix("./") == source]
            if len(members) != 1 or not members[0].isfile() or members[0].size > 100 * 1024 * 1024:
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
print(json.dumps({"binary": str(binary), "version": version.get("version"), "extractor_version": version.get("extractor_version")}))
