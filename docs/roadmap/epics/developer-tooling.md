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

On 2026-10-18 the user asked that the repository setup also work with Claude
Code. That is host parity with the existing Codex/OpenCode configuration only.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `codegraph-local-navigation` | shipped | adr-backed, contract-backed | pack | Installed pinned local CLI, indexed Cognee from Recollect, verified a permissions flow and incremental refresh, and documented navigation |
| `atlas-reference-checkout` | shipped | contract-backed | small-fix: explicit user-authorized reference clone with isolated ignore/docs changes; no application code or runtime integration | Cloned Agent Memory Atlas and verified the requested pattern/comparison sources and generated HTML against the live pages |
| `documentation-skills` | shipped | adr-backed, contract-backed | pack | Adapted Terme's router and maintainer to Recollect, verified discovery, and assessed tooling against the accepted stack |
| `opencode-project-setup` | shipped | adr-backed, contract-backed | pack | Carry the existing project instructions, two skills, six roles and Context7 into OpenCode v2 plus secret-free chrome-devtools browser control and verify host discovery and read-only MCP calls |
| `rust-semantic-developer-navigation` | shipped | adr-backed, contract-backed | pack | Repository-local Rust Analyzer MCP and Graft structural context with actual calls and isolated runtime footprint |
| `claude-code-project-setup` | shipped | adr-backed, contract-backed | pack | Carry the shared instructions, two skills, five specialist roles, Context7 and Rust Analyzer/Graft MCPs into Claude Code and verify actual read-only calls |

## Current status

Repository-local Context7, CodeGraph, Rust Analyzer MCP and Graft are configured. Rust semantic and Graft structural calls have retained proof; the October 18 documentation audit also called both MCPs. Structural index freshness is separate from optional deep/model summaries. No additional tool installation is required for the current work.

The October 18 user-requested
[maintainer skill](../../../.agents/skills/recollect-doc-maintainer/SKILL.md)
clarification uses the documented small-fix exception: it adds partial-progress
reporting within the existing workflow, with no product interface, lifecycle
status or permission change. Project scores remain in the owning docs. Skill
validation and repository discovery checks belong to this local docs update;
the archived original skill-delivery record is unchanged.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Archived packs](../execution/archive/README.md) retain the delivered slices.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/developer-tooling.md).
Dated test/deployment observations are not a current runtime health claim.
