#!/usr/bin/env python3
"""Owned loopback Vault/Proxy fixture. No revocation or destructive Vault cleanup."""
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import time
import shutil
import uuid
import urllib.request

os.umask(0o077)
repo = Path(__file__).resolve().parent.parent
root = (repo / ".cache" / ("vault-" + uuid.uuid4().hex[:8])).resolve()
root.mkdir(mode=0o700)
vault_binary = shutil.which("vault")
assert vault_binary, "Install the Vault CLI before starting this optional fixture"
with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    address = "http://127.0.0.1:" + str(listener.getsockname()[1])
token = secrets.token_urlsafe(36)
server = subprocess.Popen([vault_binary, "server", "-dev", "-dev-no-store-token", "-dev-listen-address=" + address.removeprefix("http://"), "-log-level=error"],
    env={"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "VAULT_DEV_ROOT_TOKEN_ID": token},
    stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
record = {"directory": str(root), "address": address, "server_pid": server.pid, "owner": "recollect-local-vault-proof", "tokens_revoked": 0, "leases_revoked": 0}
(root / "boundary.json").write_text(json.dumps(record, indent=2) + "\n")
(root / "server-token").write_text(token)

def request(path, body=None):
    assert not any(word in path for word in ["revoke", "rotate-root"])
    req = urllib.request.Request(address + "/v1/" + path,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"X-Vault-Token": token, "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=6) as response:
            raw = response.read(262145)
            return json.loads(raw) if raw else {}
    except Exception:
        raise RuntimeError("Local fixture request failed; raw diagnostics suppressed") from None

for _ in range(60):
    assert server.poll() is None, "Owned fixture exited"
    try:
        request("sys/health")
        break
    except RuntimeError:
        time.sleep(.2)
else:
    raise RuntimeError("Owned fixture did not start")
request("sys/mounts/fixture-kv", {"type": "kv", "options": {"version": "2"}})
request("sys/mounts/fixture-db", {"type": "database"})
database_password = secrets.token_urlsafe(36)
docker = str(repo / "scripts/docker.sh")
result = subprocess.run([docker, "run", "-d", "--name", "recollect-vault-db-" + uuid.uuid4().hex[:8],
    "--label", "io.recollect.owner=" + root.name, "--memory=256m", "-p", "127.0.0.1::5432", "-e", "POSTGRES_PASSWORD", "pgvector/pgvector:pg17"],
    env=dict(os.environ, POSTGRES_PASSWORD=database_password), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
assert result.returncode == 0, "Owned database start failed; diagnostics suppressed"
container = result.stdout.decode().strip()
ports = subprocess.run([docker, "inspect", container, "--format", '{{json .NetworkSettings.Ports}}'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
port = int(json.loads(ports.stdout)["5432/tcp"][0]["HostPort"])
database = {"address": "127.0.0.1", "port": port, "container": container}
record["database"] = database
(root / "boundary.json").write_text(json.dumps(record, indent=2) + "\n")
for _ in range(60):
    # The image's temporary initialization server accepts Unix-socket probes
    # before the published TCP endpoint exists. Vault connects through TCP.
    ready = subprocess.run([docker, "exec", container, "pg_isready", "-h", "127.0.0.1", "-U", "postgres"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0
    if ready: break
    time.sleep(.2)
assert ready, "Owned database not ready"
request("fixture-db/config/fixture", {
    "plugin_name": "postgresql-database-plugin", "allowed_roles": ["runner"],
    "connection_url": "postgresql://{{username}}:{{password}}@" + database["address"] + ":" + str(database["port"]) + "/postgres?sslmode=disable&connect_timeout=3",
    "username": "postgres", "password": database_password, "verify_connection": True,
})
request("fixture-db/roles/runner", {"db_name": "fixture", "default_ttl": "8s", "max_ttl": "32s",
    "creation_statements": ['CREATE ROLE "{{name}}" WITH LOGIN PASSWORD \'{{password}}\' VALID UNTIL \'{{expiration}}\';'],
    "renew_statements": ['ALTER ROLE "{{name}}" VALID UNTIL \'{{expiration}}\';'],
})
generation = secrets.token_urlsafe(24)
request("fixture-kv/data/fixture", {"data": {"username": "fixture-user-" + generation, "password": "fixture-password-" + generation}})
request("sys/auth/fixture-approle", {"type": "approle"})
policy = '''path "fixture-kv/data/fixture" { capabilities = ["read"] }
path "fixture-db/creds/runner" { capabilities = ["read"] }
path "auth/token/revoke*" { capabilities = ["deny"] }
path "sys/leases/revoke*" { capabilities = ["deny"] }
path "sys/leases/renew" {
  capabilities = ["update"]
  allowed_parameters = { "lease_id" = ["fixture-db/creds/runner/*"], "increment" = [] }
}
'''
request("sys/policies/acl/recollect-mcp", {"policy": policy})
request("auth/fixture-approle/role/runner", {"token_policies": ["recollect-mcp"], "token_ttl": "60s", "token_max_ttl": "10m", "secret_id_ttl": "24h"})
role = request("auth/fixture-approle/role/runner/role-id")["data"]["role_id"]
secret = request("auth/fixture-approle/role/runner/secret-id", {})["data"]["secret_id"]
(root / "role-id").write_text(role)
(root / "secret-id").write_text(secret)
config = f'''vault {{
 address = {json.dumps(address)}
 retry {{ num_retries = 0 }}
}}
auto_auth {{
 method "approle" {{
  mount_path = "auth/fixture-approle"
  config = {{
   role_id_file_path = {json.dumps(str(root / 'role-id'))}
   secret_id_file_path = {json.dumps(str(root / 'secret-id'))}
   remove_secret_id_file_after_reading = false
  }}
 }}
}}
api_proxy {{ use_auto_auth_token = "force" }}
listener "unix" {{
 address = {json.dumps(str(root / 'vault.sock'))}
 socket_mode = "0600"
 tls_disable = true
}}
'''
(root / "proxy.hcl").write_text(config)
proxy = subprocess.Popen([vault_binary, "proxy", "-config=" + str(root / "proxy.hcl"), "-log-level=error"],
    env={"PATH": os.environ.get("PATH", "/usr/bin:/bin")}, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
record.update(proxy_pid=proxy.pid, configured=True, runtime_has_root_token=False)
(root / "boundary.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps(record), flush=True)
print("Fixture ready. In another terminal run: ./scripts/test-mcp-vault.sh " + str(root), flush=True)
# Leave the owned fixture and its credentials in place. Stopping these local
# processes, if requested later, must never trigger a Vault cleanup command.
server.wait()
