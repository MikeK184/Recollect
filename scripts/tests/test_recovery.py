import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import install
import recovery
import recovery_archive as archive
from recovery_transport import decrypt, encrypt, execute, private_json


class RecoveryArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="recovery-archive-proof-", dir=install.ROOT / ".cache")
        self.root = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()

    def bundle(self):
        directory = self.root / "bundle"
        (directory / "app").mkdir(parents=True)
        (directory / "database.dump").write_bytes(b"synthetic dump used only for member validation")
        brain, artifact = str(uuid.uuid4()), str(uuid.uuid4())
        path = directory / "app/artifacts" / brain / (artifact + ".txt")
        path.parent.mkdir(parents=True)
        path.write_text("independent retained evidence")
        manifest = {"backup_id": str(uuid.uuid4()), "installation_id": str(uuid.uuid4()),
                    "migrations": ["001_platform"], "postgres_major": 17, "backup_days": 7,
                    "database_dump_bytes": (directory / "database.dump").stat().st_size,
                    "artifacts": [{"brain_id": brain, "id": artifact, "byte_length": path.stat().st_size}],
                    "application_files": {}, "journal_entries": [], "credential_count": 0}
        private_json(directory / "manifest.json", manifest)
        return directory, manifest, path

    def test_restore_members_require_retained_artifacts_and_known_migration_prefix(self):
        root, manifest, artifact = self.bundle()
        schema = {"migrations": ["001_platform", "002_durable_work"], "postgres_major": 17}
        self.assertEqual(archive.validate_bundle(root, schema), manifest)
        for history in (["999_future"], ["002_durable_work"], ["001_platform", "003_gap"]):
            private_json(root / "manifest.json", {**manifest, "migrations": history})
            with self.assertRaises(ValueError):
                archive.validate_bundle(root, schema)
        private_json(root / "manifest.json", manifest)
        artifact.unlink()
        with self.assertRaisesRegex(ValueError, "artifact"):
            archive.validate_bundle(root, schema)

    def test_extraction_rejects_escape_links_devices_duplicates_and_extra_roots(self):
        cases = [("../outside", tarfile.REGTYPE), ("/outside", tarfile.REGTYPE),
                 ("app/link", tarfile.SYMTYPE), ("app/hard", tarfile.LNKTYPE),
                 ("app/device", tarfile.CHRTYPE), ("runtime.env", tarfile.REGTYPE)]
        for index, (name, kind) in enumerate(cases):
            source = self.root / f"unsafe-{index}.tar"
            with tarfile.open(source, "w") as target:
                info = tarfile.TarInfo(name)
                info.type, info.linkname = kind, "/outside"
                target.addfile(info)
            output = self.root / str(index)
            output.mkdir()
            with self.subTest(name=name), self.assertRaises(ValueError):
                archive.extract(source, output, allowed={"app"})
        source = self.root / "duplicate.tar"
        with tarfile.open(source, "w") as target:
            for _ in range(2):
                target.addfile(tarfile.TarInfo("app/file"), io.BytesIO())
        output = self.root / "duplicate"
        output.mkdir()
        with self.assertRaisesRegex(ValueError, "duplicate"):
            archive.extract(source, output, allowed={"app"})
        self.assertFalse((self.root / "outside").exists())

    def test_journal_inventory_detects_loss_without_assuming_contiguous_pg_sequences(self):
        installation = str(uuid.uuid4())
        erasures = self.root / "erasure-journal" / installation
        analytics = self.root / "erasure-journal" / ("analytics-" + installation)
        erasures.mkdir(parents=True)
        analytics.mkdir()
        for directory in (erasures, analytics):
            private_json(directory / "installation.json", installation)
        entries = [{"id": str(uuid.uuid4()), "sequence": 1}, {"id": str(uuid.uuid4()), "sequence": 3}]
        for entry in entries:
            private_json(erasures / (entry["id"] + ".json"), {**entry, "installation_id": installation})
        private_json(self.root / "journal-inventory.json", {"installation_id": installation, "entries": entries, "analytics": []})
        manifest = {"installation_id": installation, "journal_entries": entries[:1]}
        archive.validate_journals(self.root, manifest)
        timed = {**manifest, "checkpoint_at": "2026-09-26T12:00:00Z"}
        private_json(self.root / "journal-inventory.json", {"installation_id": installation, "entries": entries,
                     "analytics": [], "exported_at": "2026-09-25T12:00:00Z"})
        with self.assertRaisesRegex(ValueError, "older"):
            archive.validate_journals(self.root, timed)
        private_json(self.root / "journal-inventory.json", {"installation_id": installation, "entries": entries,
                     "analytics": [], "exported_at": "2026-09-26T12:01:00Z"})
        archive.validate_journals(self.root, timed)
        chain = [{"entry": {**entry,"installation_id":installation},
                  "previous": entries[index-1] if index else None} for index,entry in enumerate(entries)]
        self.assertEqual(len(archive.remote_chain(chain, installation)),2)
        with self.assertRaisesRegex(ValueError,"predecessor"):
            archive.remote_chain(chain[1:],installation)
        (erasures / (entries[1]["id"] + ".json")).unlink()
        with self.assertRaisesRegex(ValueError, "incomplete"):
            archive.validate_journals(self.root, manifest)
        private_json(self.root / "journal-inventory.json", {"installation_id": installation, "entries": [], "analytics": []})
        (erasures / (entries[0]["id"] + ".json")).unlink()
        with self.assertRaisesRegex(ValueError, "older"):
            archive.validate_journals(self.root, manifest)
        private_json(self.root / "journal-inventory.json", {"installation_id": str(uuid.uuid4()), "entries": [], "analytics": []})
        with self.assertRaisesRegex(ValueError, "another"):
            archive.validate_journals(self.root, manifest)

    def test_real_age_failure_is_removed_before_any_restore_side_effect(self):
        tools = sorted((install.ROOT / ".cache/recovery-tools").glob("age-*/age"))
        if not tools:
            self.skipTest("Run scripts/setup-recovery-tools.py for the actual age proof")
        age = tools[0]
        identity, wrong = self.root / "identity", self.root / "wrong"
        execute([age.with_name("age-keygen"), "-o", identity])
        execute([age.with_name("age-keygen"), "-o", wrong])
        recipient = execute([age.with_name("age-keygen"), "-y", identity]).decode().strip()
        plain = self.root / "source"
        plain.write_bytes(b"controlled recovery evidence" * 4096)
        encrypted = self.root / "encrypted.age"
        encrypt(age, [recipient], plain, encrypted)
        self.assertNotIn(b"controlled recovery evidence", encrypted.read_bytes())
        decrypt(age, identity, encrypted, self.root / "roundtrip")
        self.assertEqual(plain.read_bytes(), (self.root / "roundtrip").read_bytes())
        destination = self.root / "failed-output"
        with self.assertRaises(ValueError):
            decrypt(age, wrong, encrypted, destination)
        self.assertFalse(destination.exists())
        corrupt = bytearray(encrypted.read_bytes())
        corrupt[-1] ^= 1
        encrypted.write_bytes(corrupt)
        saved = {"name": "fixture"}
        (self.root / "fixture").mkdir()
        with patch.object(install, "INSTALLS", self.root), patch.object(recovery.runtime, "schema") as schema, patch.object(recovery.runtime, "fresh") as fresh:
            with self.assertRaises(ValueError):
                recovery.restore(saved, {"age_binary": str(age)}, encrypted, encrypted, identity)
            schema.assert_not_called()
            fresh.assert_not_called()
        self.assertEqual(list((self.root / "fixture").iterdir()), [])


if __name__ == "__main__":
    unittest.main()
