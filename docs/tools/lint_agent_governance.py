#!/usr/bin/env python3
"""Validate Recollect's documented governance format without third-party packages."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

if sys.version_info < (3, 11):
    raise SystemExit("Governance lint requires Python 3.11 or later.")

ROOT = Path(__file__).resolve().parents[2]
ROLES = ("default", "explorer", "architect", "worker", "qa", "reviewer")
FOUNDATIONS = ("vision", "techstack", "engineering-principles")
SLICE_STATES = {"planned", "in-progress", "blocked", "shipped"}
PACK_STATES = {"planned", "in-progress", "blocked"}  # a pack is deleted when its slice ships
EPIC_STATES = {"planned", "active", "blocked", "complete"}
EVIDENCE = {"adr-backed", "contract-backed", "mapping-only", "needs-adr", "needs-contract"}
UNRESOLVED = {"mapping-only", "needs-adr", "needs-contract"}
RESERVED = {"README.md", "index.md", "template.md"}
MAPPING_WORD_WARNING = 5000
LINK = re.compile(r"\[[^\]\n]+\]\(([^\s)]+)\)")
PLACEHOLDER = re.compile(r"<[A-Za-z][A-Za-z0-9 _./-]*>|\b(?:TODO|TBD)\b|Pending (?:validation|implementation)")
PACK_FIELDS = {
    "Summary": ("Goal", "Non-goals"),
    "Governing Sources": (),
    "Scope": ("In scope", "Out of scope", "Blockers"),
    "Decisions": (),
    "Acceptance": ("Automated", "Manual", "Acceptance"),
}  # packs may carry further sections; only these are required
REQUIRED = (
    "AGENTS.md", "README.md", ".gitignore", ".codex/README.md", ".codex/config.toml",
    "docs/README.md", "docs/foundation/README.md", "docs/roadmap/README.md",
    "docs/roadmap/epics/index.md", "docs/roadmap/epics/template.md",
    "docs/roadmap/execution/README.md", "docs/roadmap/execution/template.md",
    "docs/roadmap/execution/active/README.md",
    "docs/runbooks/validation.md", "docs/tools/lint_agent_governance.sh",
    "docs/tools/lint_agent_governance.py", "docs/tools/tests/test_governance.py",
    "scripts/validate.sh",
) + tuple(f"docs/foundation/{name}.md" for name in FOUNDATIONS) + tuple(
    f"docs/{bucket}/{name}.md"
    for bucket in ("adr", "contracts", "mappings", "runbooks")
    for name in ("README", "template")
) + tuple(f".codex/agents/{role}.toml" for role in ROLES)


class GovernanceError(ValueError):
    """A document or configuration breaks the declared governance contract."""


def require(condition: bool, file: str, message: str) -> None:
    if not condition:
        raise GovernanceError(f"{file}: {message}")


class Validator:
    def __init__(self, root: Path):
        self.root = root.resolve()
        self.documents: dict[str, str] = {}
        self.warnings: list[str] = []

    def path(self, file: str) -> Path:
        path = self.root / file
        relative = Path(file)
        require(not relative.is_absolute() and ".." not in relative.parts, file, "use a repo-relative path")
        resolved = path.resolve()
        require(resolved.is_relative_to(self.root), file, "path leaves this repository")
        require(
            resolved.relative_to(self.root).parts[:1] not in (("cognee",), (".git",)),
            file, "path enters an excluded checkout or Git internals",
        )
        return path

    def read(self, file: str) -> str:
        if file not in self.documents:
            path = self.path(file)
            require(path.is_file(), file, "missing required file")
            self.documents[file] = path.read_text(encoding="utf-8")
        return self.documents[file]

    def records(self, directory: str) -> list[str]:
        return [
            str(path.relative_to(self.root))
            for path in sorted(self.path(directory).glob("*.md"))
            if path.name not in RESERVED
        ]

    def metadata(self, file: str, key: str) -> str:
        header = self.read(file).split("\n## ", 1)[0]
        values = re.findall(rf"^{re.escape(key)}:[ \t]*(.*)$", header, re.MULTILINE)
        require(len(values) == 1 and bool(values[0].strip()), file, f"need exactly one non-empty {key} field")
        return values[0].strip().strip("`")

    def section(self, file: str, heading: str) -> str:
        parts = re.split(r"^## (.+)$", self.read(file), flags=re.MULTILINE)
        matches = [parts[i + 1].strip() for i in range(1, len(parts), 2) if parts[i] == heading]
        require(len(matches) == 1 and bool(matches[0]), file, f"missing, empty, or duplicate section: {heading}")
        return matches[0]

    def fields(self, file: str, heading: str, names: tuple[str, ...], filled: bool) -> dict[str, str]:
        body = self.section(file, heading)
        if not names:  # free-text section: only presence is required
            return {}
        pairs = re.findall(r"^- ([^:\n]+):[ \t]*(.*?)(?=^- |\Z)", body, re.MULTILINE | re.DOTALL)
        values = dict(pairs)
        require(len(values) == len(pairs), file, f"duplicate field in {heading}")
        for name in names:
            value = values.get(name, "").strip()
            require(bool(value), file, f"missing or empty {heading} / {name}")
            if filled:
                require(not PLACEHOLDER.search(value), file, f"unfilled {heading} / {name}")
                require(value != "N/A", file, f"{heading} / {name} needs an N/A reason")
        return values

    def table(self, file: str, heading: str | None, columns: tuple[str, ...]) -> list[list[str]]:
        body = self.section(file, heading) if heading else self.read(file)
        rows = [
            [cell.strip().strip("`") for cell in line.strip().strip("|").split("|")]
            for line in body.splitlines() if line.strip().startswith("|")
        ]
        require(len(rows) >= 2 and rows[0] == list(columns), file, f"expected table columns: {', '.join(columns)}")
        require(
            len(rows[1]) == len(columns) and all(re.fullmatch(r":?-{3,}:?", c) for c in rows[1]),
            file, "invalid table separator",
        )
        require(all(len(row) == len(columns) and all(row) for row in rows[2:]), file, "invalid or empty table cells")
        return rows[2:]

    def local_target(self, file: str, url: str) -> str | None:
        parsed = urlsplit(url)
        if parsed.scheme or parsed.netloc or not parsed.path:
            return None
        target = (self.root / file).parent / unquote(parsed.path)
        resolved = target.resolve()
        require(resolved.is_relative_to(self.root), file, f"link leaves repository: {url}")
        relative = str(resolved.relative_to(self.root))
        if resolved.is_relative_to(self.root / "references"):
            return None  # ignored research checkouts exist only on maintainer machines
        require(self.path(relative).exists(), file, f"broken local link: {url}")
        return relative

    def index(self, file: str, heading: str | None, columns: tuple[str, ...], expected: dict[str, str]) -> None:
        actual = {}
        for row in self.table(file, heading, columns):
            match = LINK.fullmatch(row[0])
            require(match is not None, file, "index records must use Markdown links")
            target = self.local_target(file, match.group(1))
            require(target is not None and target not in actual, file, "duplicate or nonlocal index record")
            actual[target] = row[1]
        require(actual == expected, file, "index differs from actual files/statuses; reconcile all rows")

    def governing(self, file: str, accepted: set[str]) -> None:
        body = self.section(file, "Governing Sources")
        targets = {self.local_target(file, url) for url in LINK.findall(body)}
        authority = {
            target for target in targets if target and
            target.startswith(("docs/adr/", "docs/contracts/", "docs/foundation/"))
        }
        # Also inspect repo-relative code paths so a backtick reference cannot
        # conceal pending authority. Use Markdown links for actual governing sources.
        references = set(re.findall(r"`(docs/(?:adr|contracts|foundation)/[^`]+\.md)`", body))
        for source in authority | references:
            require(source in accepted, file, f"governing source is not accepted: {source}")
        require(bool(authority), file, "Governing Sources needs a link to accepted authority")

    def validate(self) -> list[str]:
        for file in REQUIRED:
            self.read(file)
        require("/references/" in self.read(".gitignore").splitlines(), ".gitignore", "ignore the separate /references/ research checkouts")

        accepted: set[str] = set()
        pending = []
        for name in FOUNDATIONS:
            file = f"docs/foundation/{name}.md"
            status = self.metadata(file, "Status")
            require(status in {"pending", "accepted"}, file, "foundation status must be pending or accepted")
            if status == "pending":
                pending.append(file)
            else:
                body = re.sub(r"(?m)^(# .*|Status:.*)$", "", self.read(file)).strip()
                require(bool(body) and "Pending content:" not in body and not PLACEHOLDER.search(body), file, "accepted foundation still contains empty or placeholder content")
                accepted.add(file)

        # Other foundation documents (for example the design system) carry their
        # own Status and may govern once accepted.
        for file in self.records("docs/foundation"):
            if Path(file).stem not in FOUNDATIONS:
                status = self.metadata(file, "Status")
                require(status in {"pending", "accepted"}, file, "foundation status must be pending or accepted")
                if status == "pending":
                    pending.append(file)
                else:
                    accepted.add(file)

        for directory, headings in (
            ("docs/adr", ("Decision", "Why", "Consequences", "Supersession")),
            ("docs/contracts", ("Source", "Contract", "Acceptance", "Explicit Deferrals")),
        ):
            for file in self.records(directory):
                status = self.metadata(file, "Status")
                require(status in {"proposed", "accepted", "superseded", "rejected"}, file, "invalid authority status")
                for heading in headings:
                    body = self.section(file, heading)
                    if status == "accepted":
                        require(not PLACEHOLDER.search(body), file, f"accepted authority has placeholders in {heading}")
                if status == "accepted":
                    accepted.add(file)

        epics: dict[str, str] = {}
        slices: dict[str, tuple[str, str, str]] = {}
        for file in self.records("docs/roadmap/epics"):
            state = self.metadata(file, "Status")
            require(state in EPIC_STATES, file, "invalid epic status")
            epics[file] = state
            self.section(file, "Purpose")
            self.section(file, "Dependencies and Boundaries")
            # Planned/blocked epics may identify decisions still needing authority.
            if state in {"active", "complete"}:
                self.governing(file, accepted)
            else:
                self.section(file, "Governing Sources")
            rows = self.table(file, "Slice Map", ("Slice ID", "Status", "Evidence", "Execution", "Summary"))
            require(bool(rows), file, "epic needs at least one slice")
            for slice_id, status, evidence, execution, _ in rows:
                require(bool(re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", slice_id)) and slice_id + ".md" not in RESERVED, file, f"invalid slice ID: {slice_id}")
                require(slice_id not in slices, file, f"duplicate slice ID: {slice_id}")
                require(status in SLICE_STATES, file, f"invalid slice status: {slice_id}")
                labels = {value.strip() for value in evidence.split(",")}
                require(labels <= EVIDENCE, file, f"invalid evidence labels: {slice_id}")
                require(
                    execution in {"pack", "archived"} or (execution.startswith("inline: ") and bool(execution.removeprefix("inline: ").strip())),
                    file, f"invalid execution value (use pack, inline: <acceptance; validation> or archived): {slice_id}",
                )
                if execution == "pack":
                    require(status != "shipped", file, f"shipped slice must be archived or inline, not pack: {slice_id}")
                if execution == "archived":
                    require(status == "shipped", file, f"archived execution requires a shipped slice: {slice_id}")
                if status in {"in-progress", "shipped"}:
                    require(not (labels & UNRESOLVED), file, f"unresolved authority on implemented slice: {slice_id}")
                    self.governing(file, accepted)
                if status == "in-progress":
                    require(state == "active", file, "in-progress slice requires an active epic")
                if state == "complete":
                    require(status == "shipped", file, "complete epic has unfinished slices")
                slices[slice_id] = (file, status, execution)
        self.index("docs/roadmap/epics/index.md", "Epic Overview", ("Epic", "Status", "Current focus"), epics)

        packs: set[str] = set()
        expected = {}
        for file in self.records("docs/roadmap/execution/active"):
            slice_id = Path(file).stem
            packs.add(slice_id)
            status = self.metadata(file, "Status")
            require(status in PACK_STATES, file, "invalid pack status (planned, in-progress or blocked; shipped packs are deleted)")
            owner = self.metadata(file, "Owning epic")
            self.path(owner)
            require(slice_id in slices, file, "filename has no owning epic slice")
            require(slices[slice_id][0] == owner, file, "Owning epic does not own this slice")
            require(slices[slice_id][2] == "pack", file, "active pack but the slice Execution is not pack")
            require(slices[slice_id][1] == status, file, "pack and epic slice statuses differ")
            implementing = status == "in-progress"
            # Even a draft pack must not cite pending documents as authority.
            self.governing(file, accepted)
            for heading, names in PACK_FIELDS.items():
                values = self.fields(file, heading, names, implementing)
                if status == "blocked" and heading == "Scope":
                    blocker = values["Blockers"].strip()
                    require(not blocker.lower().startswith(("none", "n/a")) and not PLACEHOLDER.search(blocker), file, "blocked pack needs a concrete blocker and next action")
            if implementing:
                require(not PLACEHOLDER.search(self.section(file, "Decisions")), file, "unfilled Decisions")
            expected[file] = status
        self.index("docs/roadmap/execution/active/README.md", None, ("Pack", "Status"), expected)
        for slice_id, (file, status, execution) in slices.items():
            if execution == "pack":
                require(slice_id in packs, file, f"slice declares pack but no active pack file exists: {slice_id}")

        for file in self.records("docs/mappings"):
            self.metadata(file, "Observed")
            words = len(self.read(file).split())
            if words > MAPPING_WORD_WARNING:
                self.warnings.append(f"{file}: {words} words exceeds the {MAPPING_WORD_WARNING}-word mapping guideline; keep a compact receipts table and link the full text at a pinned revision")

        # Only scan this repo's governed docs/config; do not walk the repo root.
        markdown = ["README.md", "AGENTS.md", ".codex/README.md"] + [
            str(path.relative_to(self.root)) for path in self.path("docs").rglob("*.md")
        ]
        for file in markdown:
            if Path(file).name != "template.md":
                for url in LINK.findall(self.read(file)):
                    self.local_target(file, url)
        return pending


def main() -> int:
    if len(sys.argv) > 1:
        print("Usage: lint_agent_governance.py (validates its own repository)", file=sys.stderr)
        return 2
    try:
        validator = Validator(ROOT)
        pending = validator.validate()
    except (GovernanceError, OSError, UnicodeError) as error:
        print(f"Governance lint failed: {error}", file=sys.stderr)
        return 1
    print("Governance lint passed")
    for warning in validator.warnings:
        print(f"Warning: {warning}", file=sys.stderr)
    if pending:
        print("Pending foundations (not implementation authority): " + ", ".join(Path(p).name for p in pending))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
