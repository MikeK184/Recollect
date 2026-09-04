"""Real native clients against an owned HTTPS protocol fixture, without Keychain."""
import http.server
import json
import os
from pathlib import Path
import ssl
import subprocess
import threading
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
BINARIES = Path(os.environ.get("RECOLLECT_TEST_COMPANION_DIR", ROOT / "target/debug"))


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def reply(self, status, value=None):
        data = json.dumps(value).encode() if value is not None else b""
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        self.reply(200, {"ready": True}) if self.path == "/health/ready" else self.reply(405)

    def do_POST(self):
        size = int(self.headers.get("Content-Length", 0))
        if size > 256 * 1024:
            return self.reply(413)
        message = json.loads(self.rfile.read(size))
        if "id" not in message:
            return self.reply(202)
        if message.get("method") == "initialize":
            result = {"protocolVersion": message["params"]["protocolVersion"], "capabilities": {"tools": {}},
                      "serverInfo": {"name": "owned-tls-fixture", "version": "0"}}
        elif message.get("method") == "tools/list":
            result = {"tools": []}
        else:
            result = {}
        self.reply(200, {"jsonrpc": "2.0", "id": message["id"], "result": result})


class NativeTlsProof(unittest.TestCase):
    def test_native_health_and_mcp_require_the_selected_ca_and_correct_hostname(self):
        path = ROOT / ".cache" / "installation-tls-proof" / uuid.uuid4().hex
        path.mkdir(parents=True, mode=0o700)
        def openssl(*args):
            subprocess.run(["openssl", *args], check=True, timeout=15, umask=0o077,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, cwd=path)
        openssl("req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1", "-subj", "/CN=Owned fixture CA",
                "-addext", "basicConstraints=critical,CA:TRUE,pathlen:0",
                "-addext", "keyUsage=critical,keyCertSign,cRLSign",
                "-keyout", "ca.key", "-out", "ca.pem")
        openssl("req", "-new", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=localhost",
                "-keyout", "server.key", "-out", "server.csr")
        (path / "extensions.cnf").write_text("subjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n")
        openssl("x509", "-req", "-in", "server.csr", "-CA", "ca.pem", "-CAkey", "ca.key", "-CAcreateserial",
                "-days", "1", "-extfile", "extensions.cnf", "-out", "server.pem")
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(path / "server.pem", path / "server.key")
        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        server.socket = context.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        base = f"https://localhost:{server.server_port}"
        try:
            for label, url, trusted in [("untrusted", base, False), ("trusted", base, True),
                                        ("wrong-hostname", base.replace("localhost", "127.0.0.1"), True)]:
                with self.subTest(label=label):
                    env = {key: value for key, value in os.environ.items() if key in ("PATH", "HOME", "USER", "TMPDIR")}
                    env.update(RECOLLECT_URL=url, RECOLLECT_TEST_TLS_URL=url,
                               RECOLLECT_TEST_TLS_EXPECTED="accepted" if label == "trusted" else "rejected")
                    if trusted:
                        env["RECOLLECT_CA_FILE"] = str(path / "ca.pem")
                    health = subprocess.run([str(BINARIES / "recollect-agent"), "health"], env=env,
                                            capture_output=True, text=True, timeout=15)
                    self.assertEqual(health.returncode == 0, label == "trusted", "Native TLS health admitted the wrong trust/hostname")
                    probe = subprocess.run(["cargo", "test", "--locked", "--offline", "-p", "recollect-mcp-runtime",
                                            "--test", "agent_tls", "explicit_tls_roots_preserve_native_mcp_verification",
                                            "--", "--ignored", "--nocapture"], env=env, cwd=ROOT,
                                           capture_output=True, text=True, timeout=90)
                    (path / (label + ".log")).write_text(probe.stdout + probe.stderr)
                    self.assertEqual(probe.returncode, 0, f"Native MCP TLS fixture failed: {path / (label + '.log')}")
            # Standalone settings carry only the absolute trust-file path.
            env["RECOLLECT_URL"] = base
            config = subprocess.run([str(BINARIES / "recollect-agent"), "mcp-config", "claude",
                                     "--brain", str(uuid.uuid4()), "--directory", str(path)], env=env,
                                    capture_output=True, text=True, timeout=15)
            self.assertEqual(config.returncode, 0)
            generated = json.loads(config.stdout)
            self.assertEqual(generated["configuration"]["mcpServers"]["recollect"]["env"]["RECOLLECT_CA_FILE"], str(path / "ca.pem"))
            self.assertNotIn("BEGIN CERTIFICATE", config.stdout)
            self.assertNotIn("PRIVATE KEY", config.stdout)
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2)


if __name__ == "__main__":
    unittest.main()
