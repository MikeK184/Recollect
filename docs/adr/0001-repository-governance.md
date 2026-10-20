# 0001: Repository Governance

Status: accepted

## Decision

Create Recollect as an independent Git repository with documentation authority,
epics, decision-complete execution packs, active/archive closeout, validation,
six repo-local Codex roles, and Context7. The existing `references/cognee/` checkout stays
separate, ignored, and untouched. Use the
[governance contract](../contracts/repository-governance.md) for exact rules.

Amended 2026-10-19: execution packs are required only for risky work and
shipped packs are not archived in the tree; see the amendment below, which
replaces the pack and active/archive rules in this paragraph.

## Why

The user approved the implementation plan on 2026-09-13 after selecting the
historical strict workflow and the new Recollect repository boundary. The
reference lifecycle was inspected at commit
`087caa107d8ae1c7611d2e99919db0a6cb380aab` (`81abc40^`) in the local Polymarket
repository. This decision adopts its documentation process, not its product
requirements, automatic releases, or integrations beyond Context7.

## Consequences

The accepted foundations and later ADRs define the independent Rust product;
the initial Cognee extension proposal is superseded. Documentation tooling uses
Bash and Python 3.11+ independently of the product runtime. Keep project
configuration secret-free and personal defaults outside this repository.
Commits, publishing and deployments require task authorization; they are not
automatically implied by a documentation change.

The user's 2026-10-18 cleanup instruction removes superseded studies and concepts
from current documentation. Exact Git references retain required historical
evidence; current authority, shipped packs and unfinished acceptance remain.

The user's 2026-09-13 follow-up authorizes repository-local documentation router
and maintainer skills, adapting the existing Terme workflows to Recollect's
authority and lifecycle. This lifts the bootstrap's custom-skills deferral for
these two skills. Keep them in `.agents/skills/`, with normal automatic discovery
and explicit invocation, and keep product decisions in the governing documents.
Assess additional skills and MCPs against concrete implementation needs; this
does not authorize speculative integrations or import another repo's rules.

## Supersession

The user's 2026-09-28 request extends the existing repository setup to OpenCode.
Keep Codex support and add an equivalent OpenCode v2 project configuration and
six role definitions. Both hosts share root `AGENTS.md`, `.agents/skills/`, the
existing Context7 integration and, at that time, the CodeGraph CLI. CodeGraph
was retired in commit `efca641`; navigation is now the repository-local Rust
Analyzer and Graft MCP servers described in the
[development tooling runbook](../runbooks/development-tooling.md). Personal
provider/model, reasoning, permission and UI settings remain inherited; do not
change home configuration or product Brain connections as part of this
developer setup.

N/A: this is the initial repository-governance decision.

## Amendment 2026-10-19: risk-proportional execution packs

The owner chose the strict workflow on 2026-09-13 (above). On 2026-10-19 the
owner decided, as final, to keep the lifecycle but scale its ceremony to risk:

1. **Packs only for risky work.** An execution pack is required only for
   (1) a schema or migration change with restore or compatibility impact;
   (2) a new external integration, or a paid or frozen evaluation campaign
   (cost approval and frozen input hashes); (3) a security, credential or
   authz change that no accepted contract already fully specifies;
   (4) multi-session work with open decisions. Everything else is an epic slice
   row with acceptance and validation inline (Execution `inline: ...`). A
   behavior change still updates the governing contract in the same change.
2. **Archive removed.** `docs/roadmap/execution/archive/` (105 shipped packs)
   is deleted from the tree. History stays in Git, pinned at commit
   [`a0aea9d`](https://github.com/MikeK184/Recollect/tree/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive).
   Inbound links use pinned blob URLs. At closeout a pack is deleted and its
   slice is marked shipped with Execution `archived`.
3. **Receipts, not ledgers.** Paid-run approvals, frozen input hashes and
   failure results stay in a compact receipts table; long narrative ledgers
   are replaced by a pinned Git link to their full text.

The original decisions above, including the earlier strict rules, remain as
history. The [governance contract](../contracts/repository-governance.md) holds
the current exact rules.
