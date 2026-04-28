# 0001: Repository Governance

Status: accepted

## Decision

Create Recollect as an independent Git repository with documentation authority,
epics, decision-complete execution packs, active/archive closeout, validation,
six repo-local Codex roles, and Context7. The existing `cognee/` checkout stays
separate, ignored, and untouched. Use the
[governance contract](../contracts/repository-governance.md) for exact rules.

## Why

The user approved the implementation plan on 2026-09-13 after selecting the
historical strict workflow and the new Recollect repository boundary. The
reference lifecycle was inspected at commit
`087caa107d8ae1c7611d2e99919db0a6cb380aab` (`81abc40^`) in the local Polymarket
repository. This decision adopts its documentation process, not its product
requirements, automatic releases, or integrations beyond Context7.

## Consequences

The user's same-day follow-up supplies the initial Cognee extension design and
adds foundation authoring plus upstream/tooling assessment to this bootstrap.
Application implementation and detailed runtime contracts stay deferred.
Bootstrap work may proceed from this accepted decision. Documentation tooling uses Bash and the
Python 3.11+ standard library independently of future application technology.
Keep project configuration secret-free and personal defaults outside this repo.
Remote creation, publishing, and automatic commits remain deferred.

The user's 2026-09-13 follow-up authorizes repository-local documentation router
and maintainer skills, adapting the existing Terme workflows to Recollect's
authority and lifecycle. This lifts the bootstrap's custom-skills deferral for
these two skills. Keep them in `.agents/skills/`, with normal automatic discovery
and explicit invocation, and keep product decisions in the governing documents.
Assess additional skills and MCPs against concrete implementation needs; this
does not authorize speculative integrations or import another repo's rules.

## Supersession

N/A: this is the initial repository-governance decision.
