# Developer Tooling

Status: complete

## Purpose

Provide verified repository-local tools for building Recollect's independent
Rust product, using Cognee and Atlas as reference material.

## Governing Sources

- [ADR 0001](../../adr/0001-repository-governance.md)
- [ADR 0002](../../adr/0002-local-codegraph-navigation.md)
- [Local navigation contract](../../contracts/local-codegraph-navigation.md)
- [Rust semantic navigation ADR](../../adr/0019-rust-semantic-developer-navigation.md)
- [Rust/Graft navigation contract](../../contracts/rust-semantic-developer-navigation.md)
- [Repository governance contract](../../contracts/repository-governance.md)

## Dependencies and Boundaries

The user authorized local CodeGraph setup. Use the separate Cognee checkout as
read-only source input; keep developer-tool writes inside Recollect. Product
architecture and runtime implementation remain governed by their own decisions.

The user's 2026-09-13 follow-up also authorizes a separate Atlas reference clone
inside Recollect. Its reports are research input; they do not replace accepted
Recollect architecture or import external agent instructions into this repo.

The user also requests the documentation router and maintainer skills from the
existing private projects, adapted to Recollect, plus an assessment of further
skills and MCPs. Use Terme's available pair as reference; keep all writes here.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `codegraph-local-navigation` | shipped | adr-backed, contract-backed | pack | Installed pinned local CLI, indexed Cognee from Recollect, verified a permissions flow and incremental refresh, and documented navigation |
| `atlas-reference-checkout` | shipped | contract-backed | small-fix: explicit user-authorized reference clone with isolated ignore/docs changes; no application code or runtime integration | Cloned Agent Memory Atlas and verified the requested pattern/comparison sources and generated HTML against the live pages |
| `documentation-skills` | shipped | adr-backed, contract-backed | pack | Adapted Terme's router and maintainer to Recollect, verified discovery, and assessed tooling against the accepted stack |
| `opencode-project-setup` | shipped | adr-backed, contract-backed | pack | Carry the existing project instructions, two skills, six roles and Context7 into OpenCode v2 plus secret-free chrome-devtools browser control and verify host discovery and read-only MCP calls |
| `rust-semantic-developer-navigation` | shipped | adr-backed, contract-backed | pack | Repository-local Rust Analyzer MCP and Graft structural context with actual calls and isolated runtime footprint |

## Rust semantic navigation closeout

The later [Rust Analyzer/Graft pack](../execution/archive/rust-semantic-developer-navigation.md)
is locally delivered with a [runbook](../../runbooks/rust-semantic-navigation.md)
and [actual MCP/isolation evidence](../../mappings/rust-semantic-navigation-proof-2026-10-09.md).
It preserves existing CodeGraph and keeps product mapping/performance work separate.
The current host catalog may require a new session. No release, commit, push or
deployment was performed; version N/A and changes uncommitted.

## CodeGraph closeout

The [delivered pack](../execution/archive/codegraph-local-navigation.md) records
the exact validation and boundaries. The local index contains 3,075 files;
permissions navigation and source addition/removal refresh passed. Cognee's
tracked source/Git state is unchanged. Governance lint and 32 tests passed.
No further user action is needed for CLI use. Version: N/A; commit: uncommitted.

## Atlas reference closeout

- Planned: Full local Atlas clone, confirmation of the three requested pages,
  isolated Git ignore entry, and source pointers for future research.
- Shipped: Clean `agent-memory-atlas/` reference checkout at
  `7eca7f7abd934c2e44bc096fc1dd0cbf94275b99`, with all three Markdown sources
  and their generated HTML verified against the live website.
- Not shipped: Application changes or automated knowledge ingestion; this
  request delivers reference material for subsequent research.
- New blockers: None.
- Docs updated: Root README/agent guide, epic/index, and
  [dated reference mapping](../../mappings/atlas-reference-2026-09-13.md).
- Validation: Correct origin, full history, clean checkout, parent ignore,
  source-file presence, and three byte-for-byte HTML comparisons passed.
  Cognee remained clean; CodeGraph retained its complete 3,075-file index
  with zero pending changes/references. scripts/validate.sh passed governance
  lint and all 32 tests.
- Version: N/A: no application release.
- Commit: uncommitted in Recollect; upstream reference revision recorded above.

## Documentation skills closeout

The [delivered pack](../execution/archive/documentation-skills.md) records the
two Terme-derived skills, reconciled current governance and tooling assessment.
Both entrypoints passed skill validation, local-link/UI checks and actual Codex
catalog discovery from the root and a nested directory. Governance lint and all
32 tests passed. The [assessment](../../runbooks/development-tooling.md) identifies
future workflow-skill candidates and recommends no extra development MCP now.
No product capability, external integration or independent agent evaluation is
claimed. Version: N/A; commit: uncommitted.

## Final desktop and tooling acceptance — 2026-10-01

The owning desktop/tooling slices are shipped with [current deployed and host evidence](../../mappings/desktop-final-acceptance-2026-10-01.md). Earlier pending checks above describe the September 26 snapshot; their remaining acceptance is now complete. No release, commit or push was performed.
