#!/usr/bin/env python3
"""Prepare the owned Ubuntu lab image and private TLS/unlock material."""
import os
from pathlib import Path
import shutil
import subprocess
import uuid

ROOT = Path(__file__).resolve().parent.parent
PRIVATE = ROOT / ".cache/private-runner-ubuntu-demo"
MARKER = PRIVATE / ".owned-demo"
if PRIVATE.exists() and not MARKER.exists():
    raise SystemExit("Refusing to overwrite an unmarked demo directory")
PRIVATE.mkdir(mode=0o700, parents=True, exist_ok=True)
if MARKER.exists():
    assert MARKER.read_text().strip() == str(ROOT), "Demo directory belongs to another checkout"
else:
    MARKER.write_text(str(ROOT) + "\n")

context = PRIVATE / "build"
context.mkdir(exist_ok=True)
for filename in ("Cargo.toml", "Cargo.lock"):
    shutil.copy2(ROOT / filename, context / filename)
shutil.copytree(ROOT / "crates", context / "crates", dirs_exist_ok=True,
                ignore=shutil.ignore_patterns("target", ".env", ".env.*", ".cache", ".data"))
for filename in ("Dockerfile", "session.sh"):
    shutil.copy2(ROOT / "infra/private-runner-demo" / filename, context / filename)

tls = PRIVATE / "tls"
tls.mkdir(mode=0o700, exist_ok=True)
if not (tls / "relay.pem").exists():
    def openssl(*arguments):
        subprocess.run(["openssl", *arguments], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    openssl("req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "30",
            "-subj", "/CN=Recollect owned Ubuntu demo CA", "-addext", "basicConstraints=critical,CA:TRUE",
            "-keyout", str(tls / "ca.key"), "-out", str(tls / "ca.pem"))
    openssl("req", "-new", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=localhost",
            "-keyout", str(tls / "relay.key"), "-out", str(tls / "relay.csr"))
    (tls / "server.ext").write_text("subjectAltName=DNS:localhost,IP:127.0.0.1\n"
        "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\n"
        "extendedKeyUsage=serverAuth\n")
    openssl("x509", "-req", "-in", str(tls / "relay.csr"), "-CA", str(tls / "ca.pem"),
            "-CAkey", str(tls / "ca.key"), "-CAcreateserial", "-days", "30",
            "-extfile", str(tls / "server.ext"), "-out", str(tls / "relay.crt"))
    (tls / "relay.pem").write_bytes((tls / "relay.key").read_bytes() + (tls / "relay.crt").read_bytes())
for path in tls.iterdir():
    path.chmod(0o600)
runtime = PRIVATE / "runtime.env"
if not runtime.exists():
    with os.fdopen(os.open(runtime, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as output:
        output.write("RECOLLECT_URL=https://localhost:9443\nRECOLLECT_CA_FILE=/run/recollect/tls/ca.pem\n"
                     "RECOLLECT_DEMO_KEYRING_PASSWORD=" + uuid.uuid4().hex + "\n")

print("Building owned Ubuntu demo image; private build log retained", flush=True)
with (PRIVATE / "build.log").open("w") as output:
    subprocess.run([str(ROOT / "scripts/docker.sh"), "build", "-t", "recollect-private-runner-demo:local",
                    str(context)], check=True, cwd=ROOT, stdout=output, stderr=output)
print("Owned demo image and private TLS/OS-store unlock material prepared")
