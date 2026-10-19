# Repository Governance

Status: complete

## Purpose

Maintain Recollect's documentation authority, current implementation guidance,
repository-local developer configuration and public documentation site.

## Governing Sources

- [ADR 0001](../../adr/0001-repository-governance.md)
- [Repository governance contract](../../contracts/repository-governance.md)

## Dependencies and Boundaries

Accepted ADRs/contracts govern behavior; epics and packs track delivery.
Repository-local tooling and docs share the independent Rust product boundary.
Reference checkouts stay ignored and unchanged. Apache-2.0, the Git remote and
the GitHub Pages website are established; commit/push, app deployment and a new
release policy are not implied by documentation maintenance. Developer tooling
has its own epic and cannot silently register new global integrations.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `foundation-access-and-workspace-clarifications` | shipped | contract-backed | small-fix: bounded documentation clarification and fixture-isolation repair; no runtime change | Clarify dataset ownership, SSO group mapping, managed-operation enforcement, and workspace/developer configuration boundaries |
| `repository-license-and-readme` | shipped | contract-backed | small-fix: established user license choice, README and package metadata; no runtime change | Apply Apache-2.0 consistently, simplify the entry point and retain private GitHub visibility with authentic history |
| `repository-readme-discoverability` | shipped | contract-backed | small-fix: user-requested README presentation and GitHub metadata using existing product authority; no runtime or interface change | Deliver the local README presentation and peer comparison; verify the description and 19 topics on the existing public GitHub repository |
| `repository-readme-runner-workflows` | shipped | contract-backed | small-fix: bounded README and vector diagrams of accepted MCP scope, grants and execution; no runtime or interface change | Refresh host/status guidance and illustrate plugin/direct MCP, environment scope and private-runner request flow |
| `repository-readme-concept-imagery` | shipped | contract-backed | small-fix: user-requested replacement of README visuals using the accepted palette/concepts and existing behavior; no runtime or interface change | Generated product imagery matching the approved compact Recollect cards replaces the rejected README artwork |
| `repository-readme-branded-workflows` | shipped | contract-backed | small-fix: image/documentation refinement using the existing original logo and accepted MCP scope/placement behavior; no runtime or interface change | Original identity on approved layouts and a closer Tool access illustration clarify environment scope and service dispatch |
| `repository-readme-product-positioning` | shipped | contract-backed | small-fix: user-requested README copy refinement grounded in existing memory, environment, permission and runner contracts; no runtime or interface change | Lead with shared engineering memory, governed MCP tools and private execution as one self-hosted workflow |

## Current status

The governance lifecycle, repository-local developer setup, license and public
documentation site are delivered. The bootstrap, documentation-cleanup and
public-site packs were collapsed on 2026-10-19; the site contract and
[development tooling runbook](../../runbooks/development-tooling.md) carry the
current behavior. Release/version automation remains undecided.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Archived packs](../execution/archive/README.md) retain the delivered slices.
The removed packs, the superseded chronological closeouts and original research
references remain at commit
[`f1b6bb3`](https://github.com/MikeK184/Recollect/tree/f1b6bb3040dfe65b7fac90359af782cec1f7df31/docs)
and the earlier [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/repository-governance.md).
Dated test/deployment observations are not a current runtime health claim.
