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
SLICE = "fixture-slice"
EPIC = "docs/roadmap/epics/repository-governance.md"
ACTIVE = f"docs/roadmap/execution/active/{SLICE}.md"

FIXTURE_EPIC = f"""# Fixture epic

Status: active

## Purpose

Exercise lifecycle validation in an isolated copy.

## Governing Sources

- [ADR 0001](../../adr/0001-repository-governance.md)

## Dependencies and Boundaries

None for this fixture.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `{SLICE}` | in-progress | adr-backed, contract-backed | pack | Fixture slice |
"""

FIXTURE_PACK = f"""# Fixture slice

Status: in-progress
Owning epic: `{EPIC}`

## Summary

- Goal: Exercise lifecycle validation.
- Non-goals: Real delivery.

## Governing Sources

- [ADR 0001](../../../adr/0001-repository-governance.md)

## Scope

- In scope: One fixture slice.
- Out of scope: Everything else.
- Blockers: None for this fixture.

## Decisions

Interfaces, storage, states and failure handling are N/A: fixture only.

## Acceptance

- Automated: Governance unit tests.
- Manual: N/A: fixture.
- Acceptance: The validator accepts or rejects the fixture as the test expects.
"""

MAPPING = "docs/mappings/evidence.md"


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
            if file == ".gitignore":
                shutil.copy2(REPO / file, target)
            else:
                target.write_text("# Fixture document\n")
        for file in ("docs/adr/0001-repository-governance.md", "docs/contracts/repository-governance.md"):
            shutil.copy2(REPO / file, self.root / file)
        # The fixture has no runbook; keep that link textual in the copied files.
        for governance in ("docs/contracts/repository-governance.md", "docs/adr/0001-repository-governance.md"):
            text = (self.root / governance).read_text()
            text = re.sub(r"\[([^]]+)\]\(\.\./runbooks/development-tooling\.md\)", r"\1", text)
            self.write(governance, text)
        self.write(ACTIVE, FIXTURE_PACK)
        self.write(EPIC, FIXTURE_EPIC)
        self.write(
            "docs/roadmap/epics/index.md",
            "# Fixture roadmap\n\n## Epic Overview\n\n"
            "| Epic | Status | Current focus |\n| --- | --- | --- |\n"
            "| [repository-governance.md](repository-governance.md) | active | Fixture slice |\n",
        )
        self.write_index([(SLICE, "in-progress")])
        for name in ("vision", "techstack", "engineering-principles"):
            self.write(f"docs/foundation/{name}.md", f"# {name}\n\nStatus: pending\n\nPending content: user decisions.\n")

    def write(self, file, text):
        (self.root / file).write_text(text)

    def replace(self, file, old, new):
        path = self.root / file
        text = path.read_text()
        self.assertIn(old, text)
        self.write(file, text.replace(old, new))

    def write_index(self, rows):
        self.write(
            "docs/roadmap/execution/active/README.md",
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
        self.write_index([(SLICE, state)])

    def ship(self, execution):
        """Close the slice out: no pack, shipped row, complete epic."""
        (self.root / ACTIVE).unlink()
        self.write_index([])
        self.replace(EPIC, f"`{SLICE}` | in-progress | adr-backed, contract-backed | pack |", f"`{SLICE}` | shipped | adr-backed, contract-backed | {execution} |")
        self.replace(EPIC, "Status: active", "Status: complete")
        self.replace("docs/roadmap/epics/index.md", "(repository-governance.md) | active", "(repository-governance.md) | complete")

    def test_valid_governance_with_pending_foundations(self):
        self.assertEqual(len(self.validate()), 3)

    def test_valid_archived_slice_and_complete_epic(self):
        self.ship("archived")
        self.assertEqual(len(self.validate()), 3)

    def test_valid_shipped_inline_slice(self):
        self.ship("inline: correct a documented typo; validate.sh passes")
        self.assertEqual(len(self.validate()), 3)

    def test_archived_requires_shipped_status(self):
        (self.root / ACTIVE).unlink()
        self.write_index([])
        self.replace(EPIC, "| pack |", "| archived |")
        self.reject("archived execution requires a shipped slice")

    def test_archived_slice_cannot_keep_active_pack(self):
        self.replace(EPIC, "| in-progress | adr-backed, contract-backed | pack |", "| shipped | adr-backed, contract-backed | archived |")
        self.reject("active pack but the slice Execution is not pack")

    def test_shipped_slice_cannot_use_pack_execution(self):
        self.set_state("shipped")
        self.reject("shipped slice must be archived or inline")

    def test_pack_status_shipped_is_invalid(self):
        self.replace(ACTIVE, "Status: in-progress", "Status: shipped")
        self.reject("invalid pack status")

    def test_inline_slice_cannot_keep_active_pack(self):
        self.replace(EPIC, "| pack |", "| inline: trivial typo; validate.sh |")
        self.reject("active pack but the slice Execution is not pack")

    def test_valid_inline_in_progress_slice_without_pack(self):
        (self.root / ACTIVE).unlink()
        self.write_index([])
        self.replace(EPIC, "| pack |", "| inline: correct a documented typo only; validate.sh passes |")
        self.assertEqual(len(self.validate()), 3)

    def test_empty_inline_reason_is_invalid(self):
        (self.root / ACTIVE).unlink()
        self.write_index([])
        self.replace(EPIC, "| pack |", "| inline:  |")
        self.reject("invalid (execution value|or empty table cells)")

    def test_retired_small_fix_value_is_invalid(self):
        (self.root / ACTIVE).unlink()
        self.write_index([])
        self.replace(EPIC, "| pack |", "| small-fix: correct a documented typo only |")
        self.reject("invalid execution value")

    def test_pack_execution_requires_active_pack_file(self):
        (self.root / ACTIVE).unlink()
        self.write_index([])
        self.reject("slice declares pack but no active pack file")

    def test_pack_may_carry_extra_sections_and_work_type(self):
        self.replace(ACTIVE, "Owning epic:", "Work type: product\nOwning epic:")
        self.write(ACTIVE, (self.root / ACTIVE).read_text() + "\n## Closeout\n\n- Planned: legacy\n- Validation: TODO\n\n## Data and Authority\n\nExtra design notes <ids>.\n")
        self.assertEqual(len(self.validate()), 3)

    def test_pack_needs_slim_sections(self):
        self.replace(ACTIVE, "## Decisions", "## Design notes")
        self.reject("missing, empty, or duplicate section: Decisions")

    def test_pack_needs_acceptance_section(self):
        self.replace(ACTIVE, "## Acceptance", "## Tests and Acceptance")
        self.reject("missing, empty, or duplicate section: Acceptance")

    def test_pack_does_not_require_work_type_or_closeout(self):
        self.assertNotIn("Work type", (self.root / ACTIVE).read_text())
        self.assertNotIn("## Closeout", (self.root / ACTIVE).read_text())
        self.validate()

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

    def test_index_status_mismatch(self):
        self.write_index([(SLICE, "planned")])
        self.reject("index differs")

    def test_duplicate_index_entry(self):
        self.write_index([(SLICE, "in-progress")] * 2)
        self.reject("duplicate or nonlocal index record")

    def test_pack_and_slice_status_mismatch(self):
        self.replace(ACTIVE, "Status: in-progress", "Status: planned")
        self.reject("pack and epic slice statuses differ")

    def test_pending_foundation_is_not_authority(self):
        self.replace(ACTIVE, "## Governing Sources\n", "## Governing Sources\n\n- [Vision](../../../foundation/vision.md)\n")
        self.reject("governing source is not accepted")

    def test_missing_references_checkout_link_is_not_checked(self):
        self.write(MAPPING, "# Evidence\n\nObserved: 2026-10-19\n\n[Atlas](../../references/agent-memory-atlas/content/overview.md)\n")
        self.validate()

    def test_broken_link_outside_references_is_rejected(self):
        self.write(MAPPING, "# Evidence\n\nObserved: 2026-10-19\n\n[Gone](../../docs/missing.md)\n")
        self.reject("broken local link")

    def test_mapping_needs_observed_line(self):
        self.write(MAPPING, "# Evidence\n\nConfidence: observed-once\n\n## Observations\n\nText.\n")
        self.reject("docs/mappings/evidence.md: need exactly one non-empty Observed field")

    def test_mapping_with_observed_line_passes(self):
        self.write(MAPPING, "# Evidence\n\nObserved: 2026-10-19\n\n## Observations\n\nText.\n")
        validator = Validator(self.root)
        validator.validate()
        self.assertEqual(validator.warnings, [])

    def test_long_mapping_warns_without_failing(self):
        self.write(MAPPING, "# Evidence\n\nObserved: 2026-10-19\n\n## Observations\n\n" + "word " * 5001)
        validator = Validator(self.root)
        self.assertEqual(len(validator.validate()), 3)
        self.assertEqual(len(validator.warnings), 1)
        self.assertIn("docs/mappings/evidence.md", validator.warnings[0])

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
        self.replace(ACTIVE, "- Goal:", "- Wrong field:")
        self.reject("Summary / Goal")

    def test_unfilled_implementation_field(self):
        self.replace(ACTIVE, "- Goal:", "- Goal: TODO")
        self.reject("unfilled Summary / Goal")

    def test_unfilled_decisions_in_progress(self):
        self.replace(ACTIVE, "Interfaces, storage, states and failure handling are N/A: fixture only.", "TBD")
        self.reject("unfilled Decisions")

    def test_planned_pack_may_leave_decisions_open(self):
        self.set_state("planned")
        self.replace(ACTIVE, "Interfaces, storage, states and failure handling are N/A: fixture only.", "TBD")
        self.assertEqual(len(self.validate()), 3)

    def test_blocked_pack_needs_actual_blocker(self):
        self.set_state("blocked")
        self.reject("blocked pack needs a concrete blocker")

    def test_valid_blocked_pack(self):
        self.set_state("blocked")
        self.replace(ACTIVE, "- Blockers: None for this fixture.", "- Blockers: Awaiting interface decision; obtain the user's answer before resuming.")
        self.assertEqual(len(self.validate()), 3)

    def test_local_link_cannot_read_cognee(self):
        self.replace(ACTIVE, "## Summary", "[Excluded checkout](../../../../cognee/README.md)\n\n## Summary")
        self.reject("excluded checkout")


if __name__ == "__main__":
    unittest.main()
