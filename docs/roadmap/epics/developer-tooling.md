# Developer Tooling

Status: complete

## Purpose

Provide verified repository-local tools for building Recollect's independent
Rust product, using Cognee and Atlas as reference material.

## Governing Sources

- [ADR 0001](../../adr/0001-repository-governance.md)
- [Repository governance contract](../../contracts/repository-governance.md)

## Dependencies and Boundaries

Developer-tool writes stay inside Recollect; the separate Cognee and Atlas
checkouts under `references/` are read-only research input and do not replace
accepted Recollect architecture or import external agent instructions. Product
architecture and runtime implementation remain governed by their own decisions.
The installed tools, host configuration and verification procedure are recorded
in the [development tooling runbook](../../runbooks/development-tooling.md),
which superseded ADRs 0002 and 0019 on 2026-10-19.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `atlas-reference-checkout` | shipped | contract-backed | small-fix: explicit user-authorized reference clone with isolated ignore/docs changes; no application code or runtime integration | Cloned Agent Memory Atlas and verified the requested pattern/comparison sources and generated HTML against the live pages |

## Current status

Repository-local Context7, Rust Analyzer MCP, Graft and the two
documentation skills are configured for Codex, OpenCode and Claude Code. Rust
semantic and Graft structural calls have retained proof (see the runbook). Their
delivery slices (CodeGraph, now retired, documentation skills, OpenCode, Rust navigation and
Claude Code setup) were collapsed into the runbook on 2026-10-19. No additional
tool installation is required for current work.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

The collapsed packs, ADRs, contracts and the earlier chronological closeouts
remain at commit
[`f1b6bb3`](https://github.com/MikeK184/Recollect/tree/f1b6bb3040dfe65b7fac90359af782cec1f7df31/docs).
Dated test/deployment observations are not a current runtime health claim.
