"""Configuration and ownership proof using actual installed Compose, without a daemon."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import ssl
import subprocess
import unittest
import uuid

SCRIPT = Path(__file__).resolve().parents[1] / "install.py"
spec = importlib.util.spec_from_file_location("recollect_install", SCRIPT)
install = importlib.util.module_from_spec(spec)
spec.loader.exec_module(install)


class InstallationProof(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.prefix = "proof-" + uuid.uuid4().hex[:12]
        cls.artifacts = install.ROOT / ".cache" / "installation-proof" / cls.prefix
        cls.artifacts.mkdir(parents=True, mode=0o700)
        cls.cert, cls.key = cls.artifacts / "cert.pem", cls.artifacts / "key.pem"
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
                        "-subj", "/CN=installation.test", "-addext", "subjectAltName=DNS:installation.test",
                        "-keyout", str(cls.key), "-out", str(cls.cert)],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=15, umask=0o077)

    def args(self, mode, suffix=""):
        class Args:
            pass
        args = Args()
        args.name, args.mode = self.prefix + "-" + mode + suffix, mode
        args.origin = "https://installation.test:19443" if mode == "shared" else None
        args.port, args.bind = (19443 if mode == "shared" else 19878), None
        args.cert, args.key = (str(self.cert), str(self.key)) if mode == "shared" else (None, None)
        return args

    def initialize(self, args):
        with contextlib.redirect_stdout(io.StringIO()):
            install.initialize(args)
        return install.load(args.name)

    def test_actual_profiles_preserve_environment_volumes_and_network_boundaries(self):
        for mode in ("personal", "shared"):
            with self.subTest(mode=mode):
                args = self.args(mode)
                saved = self.initialize(args)
                path = install.directory(args.name)
                original = {p.name: p.read_bytes() for p in path.iterdir()}
                self.initialize(args)
                self.assertTrue(original == {p.name: p.read_bytes() for p in path.iterdir()},
                                "Repeated initialization changed private configuration")
                for name in ("runtime.env", "operator.env", "database.env", "graph.env"):
                    self.assertEqual((path / name).stat().st_mode & 0o777, 0o600)
                self.assertEqual(path.stat().st_mode & 0o777, 0o700)
                summary = install.configuration(saved)
                self.assertTrue(summary["configured_only"])
                model = json.loads(install.run(saved, ["config", "--format", "json"], capture=True, timeout=15))
                services = model["services"]
                for name in ("api", "worker"):
                    self.assertNotIn("DATABASE_ADMIN_URL", services[name]["environment"])
                    self.assertNotIn("VAULT_TOKEN", services[name]["environment"])
                    self.assertEqual(services[name]["depends_on"]["migrate"]["condition"], "service_completed_successfully")
                    self.assertEqual(services[name]["stop_signal"], "SIGTERM")
                    self.assertEqual(services[name]["stop_grace_period"], "1m30s")
                self.assertIn("DATABASE_ADMIN_URL", services["migrate"]["environment"])
                for name, service in services.items():
                    self.assertGreater(int(service["mem_limit"]), 0, name)
                    self.assertGreater(float(service["cpus"]), 0, name)
                    self.assertEqual(service["logging"]["options"]["max-size"], "10m")
                    self.assertEqual(service["logging"]["options"]["max-file"], "3")
                self.assertEqual(services["api"]["volumes"][0]["source"], services["worker"]["volumes"][0]["source"])
                for volume in model["volumes"].values():
                    self.assertTrue(volume["name"].startswith(saved["project"] + "_"))
                self.assertFalse(services["postgres"].get("ports"))
                self.assertFalse(services["neo4j"].get("ports"))
                if mode == "personal":
                    self.assertEqual(services["api"]["ports"][0]["host_ip"], "127.0.0.1")
                    self.assertNotIn("proxy", services)
                else:
                    self.assertFalse(services["api"].get("ports"))
                    self.assertEqual(services["proxy"]["ports"][0]["target"], 8443)
                serialized = json.dumps(summary)
                self.assertFalse(any(value in serialized for value in services["api"]["environment"].values()
                                     if value and (len(value) > 20 and value not in (saved["origin"], "/run/recollect/mcp-bindings.json"))),
                                 "Configuration summary must not contain runtime values")

    def test_conflicts_partial_state_and_administrative_environment_are_explicit(self):
        args = self.args("personal", "-bad")
        saved = self.initialize(args)
        path = install.directory(args.name)
        original = (path / "runtime.env").read_bytes()
        args.port += 1
        with self.assertRaises(ValueError):
            self.initialize(args)
        self.assertTrue(original == (path / "runtime.env").read_bytes(), "Conflicting initialization changed credentials")
        with (path / "runtime.env").open("a") as file:
            file.write("VAULT_TOKEN='FORBIDDEN_SYNTHETIC_CANARY'\n")
        with self.assertRaisesRegex(ValueError, "administrative credential"):
            install.configuration(saved)
        (path / "runtime.env").write_bytes(original)
        (path / "overrides.yaml").write_text("volumes:\n  app_data:\n    name: unrelated_normal_volume\n")
        with self.assertRaisesRegex(ValueError, "volumes must belong"):
            install.configuration(saved)
        with self.assertRaises(ValueError):
            install.directory("../unrelated")
        (path / "installation.json").unlink()
        with self.assertRaisesRegex(ValueError, "missing or incomplete"):
            install.load(args.name)


if __name__ == "__main__":
    unittest.main()
