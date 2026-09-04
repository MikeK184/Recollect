"""Actual supplied-certificate proxy behavior; never modifies global trust or Docker."""
import concurrent.futures
import http.client
import http.server
import json
import os
from pathlib import Path
import socket
import ssl
import subprocess
import threading
import time
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[2]
CADDY = os.environ.get("RECOLLECT_TEST_CADDY")


@unittest.skipUnless(CADDY, "Select an installed Caddy binary with RECOLLECT_TEST_CADDY")
class CaddyProof(unittest.TestCase):
    def test_real_proxy_preserves_origin_checks_trust_drains_and_omits_request_logs(self):
        path = ROOT / ".cache" / "installation-proxy-proof" / uuid.uuid4().hex
        path.mkdir(parents=True, mode=0o700)
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
                        "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost",
                        "-keyout", str(path / "key.pem"), "-out", str(path / "cert.pem")],
                       check=True, timeout=15, umask=0o077,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        started, release = threading.Event(), threading.Event()

        class Upstream(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                if self.path.startswith("/failure"):
                    self.connection.shutdown(socket.SHUT_RDWR)
                    self.connection.close()
                    return
                if self.path == "/slow":
                    started.set()
                    if not release.wait(10):
                        return
                size = int(self.headers.get("Content-Length", 0))
                if size:
                    self.rfile.read(min(size, 4096))
                body = json.dumps({"host": self.headers.get("Host"),
                                   "origin": self.headers.get("Origin")}).encode()
                self.send_response(200)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            do_POST = do_GET

        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Upstream)
        thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.05}, daemon=True)
        thread.start()
        self.addCleanup(thread.join, 2)
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        with socket.socket() as selected:
            selected.bind(("127.0.0.1", 0))
            port = selected.getsockname()[1]
        # Substitute only container paths/listeners with owned native fixture paths.
        # The production TLS, logging, shutdown and proxy directives remain intact.
        config = (ROOT / "infra/product/Caddyfile").read_text()
        replacements = {"/run/recollect-tls/cert.pem": str(path / "cert.pem"),
                        "/run/recollect-tls/key.pem": str(path / "key.pem"),
                        "}:8443": f"}}:{port}",
                        "api:8787": f"127.0.0.1:{server.server_port}"}
        for before, after in replacements.items():
            self.assertEqual(config.count(before), 1)
            config = config.replace(before, after)
        config = config.replace("    admin off", "    default_bind 127.0.0.1\n    admin off", 1)
        (path / "Caddyfile").write_text(config)
        env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "USER", "TMPDIR")}
        env.update(RECOLLECT_SITE_HOST="localhost", XDG_DATA_HOME=str(path / "data"),
                   XDG_CONFIG_HOME=str(path / "config"))
        command = [str(Path(CADDY).resolve()), "validate", "--config", str(path / "Caddyfile"),
                   "--adapter", "caddyfile"]
        with (path / "validation.log").open("w") as output:
            valid = subprocess.run(command, env=env, stdout=output, stderr=output, timeout=15)
        self.assertEqual(valid.returncode, 0, f"Actual Caddy configuration failed: {path / 'validation.log'}")
        trust = ssl.create_default_context(cafile=path / "cert.pem")

        def request(target="/", *, context=trust, hostname="localhost", headers=None, body=None):
            client = http.client.HTTPSConnection(hostname, port, context=context, timeout=8)
            try:
                client.request("POST" if body else "GET", target, body=body, headers=headers or {})
                response = client.getresponse()
                return response.status, response.read()
            finally:
                client.close()

        canary = "proxy-request-canary-" + uuid.uuid4().hex
        with (path / "runtime.log").open("w") as output:
            process = subprocess.Popen([command[0], "run", *command[2:]], env=env,
                                       stdout=output, stderr=output)
            try:
                deadline = time.monotonic() + 10
                while True:
                    self.assertIsNone(process.poll(), f"Proxy exited before readiness: {path / 'runtime.log'}")
                    try:
                        self.assertEqual(request()[0], 200)
                        break
                    except (ConnectionError, OSError):
                        if time.monotonic() >= deadline:
                            self.fail("Owned proxy did not become ready")
                        time.sleep(0.03)
                with self.assertRaises(ssl.SSLCertVerificationError):
                    request(context=ssl.create_default_context())
                # IP access may be rejected by Caddy's certificate selection
                # before the client reaches its own hostname verification.
                with self.assertRaises(ssl.SSLError):
                    request(hostname="127.0.0.1")
                origin = f"https://localhost:{port}"
                headers = {"Origin": origin, "Authorization": "Bearer " + canary}
                status, body = request("/knowledge?input=" + canary, headers=headers, body=canary.encode())
                self.assertEqual(status, 200)
                self.assertEqual(json.loads(body), {"host": f"localhost:{port}", "origin": origin})
                # The proxy must preserve a hostile Origin for the application's
                # own authorization middleware; it must never rewrite it to trusted.
                headers["Origin"] = "https://untrusted.example"
                self.assertEqual(json.loads(request(headers=headers)[1])["origin"], headers["Origin"])
                self.assertEqual(request("/failure?input=" + canary, headers=headers, body=canary.encode())[0], 502)
                with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                    pending = pool.submit(request, "/slow")
                    self.assertTrue(started.wait(5), "Upstream request did not enter before termination")
                    process.terminate()
                    try:
                        process.wait(timeout=0.2)
                        self.fail("Proxy exited before draining its active request")
                    except subprocess.TimeoutExpired:
                        pass
                    finally:
                        release.set()
                    self.assertEqual(pending.result(timeout=8)[0], 200)
                self.assertEqual(process.wait(timeout=10), 0)
            finally:
                release.set()
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=5)
        self.assertNotIn(canary, (path / "runtime.log").read_text())
        self.assertFalse((path / "config/caddy/autosave.json").exists())


if __name__ == "__main__":
    unittest.main()
