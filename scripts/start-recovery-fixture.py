#!/usr/bin/env python3
"""Start a fresh repository-owned OpenSSH SFTP drill endpoint with generated keys."""
import json
import os
from pathlib import Path
import subprocess
import time
import uuid

from recovery_transport import Sftp, execute, private_json

ROOT = Path(__file__).resolve().parent.parent


def start():
    identity = str(uuid.uuid4())
    root = ROOT / ".cache" / ("recovery-sftp-" + identity)
    root.mkdir(mode=0o700)
    for name in ("server-key", "client-key"):
        execute(["/usr/bin/ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", root / name])
    docker = str(ROOT / "scripts/docker.sh")
    name = "recollect-recovery-" + identity
    env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "USER", "DOCKER_CONTEXT", "DOCKER_HOST")}

    def run(args):
        result = subprocess.run([docker] + args, capture_output=True, text=True, env=env, timeout=60)
        if result.returncode:
            raise ValueError("Owned recovery fixture command failed")
        return result.stdout.strip()

    run(["run", "-d", "--name", name, "--label", "recollect.fixture=recovery-sftp",
         "--label", "recollect.fixture.id=" + identity, "--memory", "128m", "--cpus", "0.5",
         "--pids-limit", "64", "--publish", "127.0.0.1::2222", "--mount",
         f"type=bind,src={root},dst=/fixture,readonly", "recollect-recovery-sftp:local"])
    port = int(run(["port", name, "2222/tcp"]).split(":")[-1])
    public = (root / "server-key.pub").read_text().split()
    hosts = root / "known_hosts"
    hosts.write_text(f"[127.0.0.1]:{port} {public[0]} {public[1]}\n")
    hosts.chmod(0o600)
    config = {"host": "127.0.0.1", "port": port, "user": "recollect", "remote_root": "/storage",
              "identity_file": str(root / "client-key"), "known_hosts_file": str(hosts)}
    private_json(root / "fixture.json", {"container": name, "root": str(root), "remote": config})
    client = Sftp(config)
    for attempt in range(30):
        try:
            client.run("pwd\n")
            break
        except ValueError:
            if attempt == 29:
                raise
            time.sleep(0.2)
    print(json.dumps({"fixture": str(root / "fixture.json"), "container": name, "port": port,
                      "sftp_connected": True}))


if __name__ == "__main__":
    start()
