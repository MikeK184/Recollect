"""Exercise lifecycle failures in isolated copies; never touch the live docs."""

import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from lint_agent_governance import REQUIRED, GovernanceError, Validator

REPO = Path(__file__).resolve().parents[3]
SLICE = "repository-governance-bootstrap"
EPIC = "docs/roadmap/epics/repository-governance.md"
ACTIVE = f"docs/roadmap/execution/active/{SLICE}.md"
ARCHIVED = f"docs/roadmap/execution/archive/{SLICE}.md"


class GovernanceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="recollect-governance-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        # Supply the required file scaffold without importing the live roadmap
        # or its links. The real repository is checked separately by validate.sh;
        # these tests exercise one known slice, regardless of later product work.
        for file in REQUIRED:
            target = self.root / file
            target.parent.mkdir(parents=True, exist_ok=True)
            if target.suffix == ".toml" or file == ".gitignore":
                shutil.copy2(REPO / file, target)
            else:
                target.write_text("# Fixture document\n")
        for file in ("docs/adr/0001-repository-governance.md", "docs/contracts/repository-governance.md"):
            shutil.copy2(REPO / file, self.root / file)
        bootstrap = REPO / ARCHIVED if (REPO / ARCHIVED).exists() else REPO / ACTIVE
        shutil.copy2(bootstrap, self.root / ACTIVE)
        self.replace(ACTIVE, "Status: shipped", "Status: in-progress")
        self.write(ACTIVE, re.sub(r"(?m)^- Blockers:.*$", "- Blockers: None for this fixture.", (self.root / ACTIVE).read_text()))
        epic = (REPO / EPIC).read_text()
        prefix, _, table = epic.partition("## Slice Map")
        subject_rows = [
            line for line in table.splitlines()
            if line.startswith(("| Slice ID |", "| --- |")) or f"`{SLICE}`" in line
        ]
        self.write(EPIC, prefix + "## Slice Map\n\n" + "\n".join(subject_rows) + "\n")
        self.replace(EPIC, "Status: complete", "Status: active")
        self.replace(EPIC, f"`{SLICE}` | shipped", f"`{SLICE}` | in-progress")
        self.write(
            "docs/roadmap/epics/index.md",
            "# Fixture roadmap\n\n## Epic Overview\n\n"
            "| Epic | Status | Current focus |\n| --- | --- | --- |\n"
            "| [repository-governance.md](repository-governance.md) | active | Fixture slice |\n",
        )
        self.write_index("active", [(SLICE, "in-progress")])
        self.write_index("archive", [])
        for name in ("vision", "techstack", "engineering-principles"):
            self.write(f"docs/foundation/{name}.md", f"# {name}\n\nStatus: pending\n\nPending content: user decisions.\n")

    def write(self, file, text):
        (self.root / file).write_text(text)

    def replace(self, file, old, new):
        path = self.root / file
        self.write(file, path.read_text().replace(old, new))

    def write_index(self, location, rows):
        self.write(
            f"docs/roadmap/execution/{location}/README.md",
            "# Packs\n\n| Pack | Status |\n| --- | --- |\n" + "".join(
                f"| [{name}.md]({name}.md) | {status} |\n" for name, status in rows
            ),
        )

    def validate(self):
        return Validator(self.root).validate()

    def reject(self, message):
        with self.assertRaisesRegex(GovernanceError, message):
            self.validate()

    def set_state(self, state):
        self.replace(ACTIVE, "Status: in-progress", f"Status: {state}")
        self.replace(EPIC, f"`{SLICE}` | in-progress", f"`{SLICE}` | {state}")
        self.write_index("active", [(SLICE, state)])

    def test_valid_governance_with_pending_foundations(self):
        self.assertEqual(len(self.validate()), 3)

    def test_valid_shipped_pack_and_complete_epic(self):
        self.set_state("shipped")
        self.replace(ACTIVE, "Pending validation.", "Delivered and checked locally.")
        self.replace(ACTIVE, "Pending implementation and checks.", "Fixture validation passed.")
        (self.root / ACTIVE).rename(self.root / ARCHIVED)
        self.write_index("active", [])
        self.write_index("archive", [(SLICE, "shipped")])
        self.replace(EPIC, "Status: active", "Status: complete")
        self.replace("docs/roadmap/epics/index.md", "(repository-governance.md) | active", "(repository-governance.md) | complete")
        self.assertEqual(len(self.validate()), 3)

    def test_shipped_pack_needs_closeout_evidence(self):
        self.set_state("shipped")
        self.replace(ACTIVE, "- Validation:", "- Validation: TODO")
        (self.root / ACTIVE).rename(self.root / ARCHIVED)
        self.write_index("active", [])
        self.write_index("archive", [(SLICE, "shipped")])
        self.replace(ARCHIVED, "Pending validation.", "Delivered and checked locally.")
        self.reject("unfilled Closeout / Validation")

    def test_invalid_toml_has_file_context(self):
        self.write(".codex/config.toml", "[broken")
        self.reject(r".codex/config.toml: invalid TOML")

    def test_product_may_start_with_accepted_foundations(self):
        self.replace(ACTIVE, "Work type: governance", "Work type: product")
        for name in ("vision", "techstack", "engineering-principles"):
            self.write(f"docs/foundation/{name}.md", f"# {name}\n\nStatus: accepted\n\n## Baseline\n\nA settled fixture baseline for product implementation.\n")
        self.assertEqual(self.validate(), [])

    def test_missing_owner(self):
        self.replace(ACTIVE, "Owning epic:", "Removed owner:")
        self.reject("Owning epic field")

    def test_wrong_owner(self):
        self.replace(ACTIVE, f"`{EPIC}`", "`docs/roadmap/epics/missing.md`")
        self.reject("does not own")

    def test_duplicate_slice_ids(self):
        body = (self.root / EPIC).read_text()
        row = next(line for line in body.splitlines() if f"`{SLICE}`" in line)
        self.write(EPIC, body + row + "\n")
        self.reject("duplicate slice ID")

    def test_pack_filename_must_match_slice(self):
        (self.root / ACTIVE).rename(self.root / ACTIVE.replace(SLICE, "wrong-id"))
        self.reject("filename has no owning epic slice")

    def test_shipped_pack_cannot_remain_active(self):
        self.set_state("shipped")
        self.reject("does not match active/archive location")

    def test_unshipped_pack_cannot_be_archived(self):
        (self.root / ACTIVE).rename(self.root / ARCHIVED)
        self.write_index("active", [])
        self.write_index("archive", [(SLICE, "in-progress")])
        self.reject("does not match active/archive location")

    def test_index_status_mismatch(self):
        self.write_index("active", [(SLICE, "planned")])
        self.reject("index differs")

    def test_duplicate_index_entry(self):
        self.write_index("active", [(SLICE, "in-progress")] * 2)
        self.reject("duplicate or nonlocal index record")

    def test_pack_and_slice_status_mismatch(self):
        self.replace(ACTIVE, "Status: in-progress", "Status: planned")
        self.reject("pack and epic slice statuses differ")

    def test_implemented_slice_missing_required_pack(self):
        (self.root / ACTIVE).unlink()
        self.write_index("active", [])
        self.reject("implemented slice missing required pack")

    def test_valid_small_fix_exemption(self):
        (self.root / ACTIVE).unlink()
        self.write_index("active", [])
        self.replace(EPIC, "| pack |", "| small-fix: correct a documented typo only |")
        self.assertEqual(len(self.validate()), 3)

    def test_product_cannot_start_with_pending_foundations(self):
        self.replace(ACTIVE, "Work type: governance", "Work type: product")
        self.reject("product implementation requires accepted foundations")

    def test_product_may_be_planned_while_foundations_pending(self):
        self.set_state("planned")
        self.replace(ACTIVE, "Work type: governance", "Work type: product")
        self.assertEqual(len(self.validate()), 3)

    def test_pending_foundation_is_not_authority(self):
        self.replace(ACTIVE, "## Governing Sources\n", "## Governing Sources\n\n- [Vision](../../../foundation/vision.md)\n")
        self.reject("governing source is not accepted")

    def test_backtick_pending_source_cannot_bypass_check(self):
        self.replace(ACTIVE, "## Governing Sources\n", "## Governing Sources\n\n- `docs/foundation/vision.md`\n")
        self.reject("governing source is not accepted")

    def test_foundation_status_flip_does_not_accept_placeholder(self):
        self.replace("docs/foundation/vision.md", "Status: pending", "Status: accepted")
        self.reject("accepted foundation still contains")

    def test_proposed_adr_cannot_govern_implementation(self):
        self.replace("docs/adr/0001-repository-governance.md", "Status: accepted", "Status: proposed")
        self.reject("governing source is not accepted")

    def test_needs_contract_slice_cannot_start(self):
        self.replace(EPIC, "adr-backed, contract-backed", "needs-contract")
        self.reject("unresolved authority")

    def test_missing_implementation_field(self):
        self.replace(ACTIVE, "- Loading:", "- Wrong field:")
        self.reject("States and Edge Cases / Loading")

    def test_unfilled_implementation_field(self):
        self.replace(ACTIVE, "- Loading:", "- Loading: TODO")
        self.reject("unfilled States and Edge Cases / Loading")

    def test_blocked_pack_needs_actual_blocker(self):
        self.set_state("blocked")
        self.reject("blocked pack needs a concrete blocker")

    def test_valid_blocked_pack(self):
        self.set_state("blocked")
        self.replace(ACTIVE, "- Blockers: None for this fixture.", "- Blockers: Awaiting interface decision; obtain the user's answer before resuming.")
        self.assertEqual(len(self.validate()), 3)

    def test_bad_agent_registration(self):
        self.replace(".codex/config.toml", '"agents/qa.toml"', '"agents/missing.toml"')
        self.reject("invalid registration target")

    def test_role_name_mismatch(self):
        self.replace(".codex/agents/qa.toml", 'name = "qa"', 'name = "wrong"')
        self.reject("role name must match")

    def test_inline_secret_config_rejected(self):
        path = ".codex/config.toml"
        self.write(path, (self.root / path).read_text() + '\n[mcp_servers.context7.env]\nCONTEXT7_API_KEY = "fixture-only"\n')
        self.reject("secret-free Context7")

    def test_role_personal_override_rejected(self):
        path = ".codex/agents/worker.toml"
        self.write(path, (self.root / path).read_text() + '\nmodel = "fixture-model"\n')
        self.reject("inherit personal defaults")

    def test_local_link_cannot_read_cognee(self):
        self.replace(ACTIVE, "## Summary", "[Excluded checkout](../../../../cognee/README.md)\n\n## Summary")
        self.reject("excluded checkout")


if __name__ == "__main__":
    unittest.main()
