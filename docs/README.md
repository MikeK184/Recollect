# Documentation Governance and Lifecycle

Start here before meaningful implementation, debugging, or architecture work.

The repo-local [doc router](../.agents/skills/recollect-doc-router/SKILL.md)
finds the relevant authority and current evidence. The
[doc maintainer](../.agents/skills/recollect-doc-maintainer/SKILL.md) handles
scoped authoring, audits and lifecycle reconciliation. Both follow this guide;
skills are workflow helpers, not additional product authority.

## Source priority

Current explicit user instructions take precedence. Then read:

1. Accepted [ADRs](adr/README.md) and [contracts](contracts/README.md).
2. [Vision](foundation/vision.md), [technology stack](foundation/techstack.md),
   and [engineering principles](foundation/engineering-principles.md).
3. The [epic index](roadmap/epics/index.md) and owning epic.
4. The relevant [execution pack](roadmap/execution/README.md).
5. [Mappings](mappings/README.md) and local code/runtime evidence.

ADRs decide architecture; contracts specify behavior. Resolve contradictions
between accepted sources before dependent implementation. Proposed, rejected,
or superseded records are not current authority. Mappings are evidence only.
Code and runtime checks establish what exists, not what the product ought to
do. Current official external behavior can invalidate a local assumption;
record the evidence and reconcile the governing document.

## Lifecycle buckets

| Location | Purpose |
| --- | --- |
| `foundation/` | Durable product, stack, and engineering baseline |
| `adr/` | Architecture decisions and their status/history |
| `contracts/` | Testable behavior, interfaces, and boundaries |
| `mappings/` | Dated external and local evidence; never implementation authority |
| `roadmap/epics/` | Capability ownership, dependencies, and slice maps |
| `roadmap/execution/active/` | Planned, in-progress, and blocked slice specifications |
| `roadmap/execution/archive/` | Shipped slices with reconciled closeout evidence |
| `runbooks/` | Verified operator procedures for delivered capabilities |

Templates live alongside each document type. Indexes link current records;
retain superseded decisions with replacement links and shipped packs in the
archive. Avoid competing current versions of the same specification.

## Foundation bootstrap

Foundation documents begin with `Status: pending` and become `Status: accepted`
when the user's decisions are established. A non-empty draft remains pending
until then. The current three documents contain the accepted independent Rust
product baseline, superseding the initial mandatory Cognee extension direction.
Read their selected behavior and detailed implementation boundaries. The
[research mapping](mappings/atlas-foundation-decisions-2026-09-13.md) records the
evidence and confirmed decisions. Earlier mappings and closeouts retain their
historical context.

The user's sequence is foundations first, then product epics derived from them,
then detailed contracts and execution packs before dependent product code.
Foundation authoring does not itself require speculative product epics or packs.

Repository governance, docs, and developer-tool configuration can proceed from
the accepted bootstrap ADR/contract. Product packs can be planned or blocked,
but cannot enter `in-progress` or `shipped` while foundation documents are
pending. Pending documents may be named as blockers, never governing sources.
Do not use the small-fix exception to bypass this boundary. An accepted
foundation does not remove the need for detailed governing ADRs/contracts.

## Before implementation

1. Read this guide, applicable `AGENTS.md`, and task-relevant authority.
2. Verify external or dependency assumptions using Context7 and primary
   official sources. Record useful evidence and its date in mappings.
3. Resolve missing ADRs/contracts before the dependent code change.
4. Add the slice to its owning epic. Create a pack when required below.
5. Make the pack decision-complete and set it and the epic slice to
   `in-progress` before implementation; update the execution index.

An execution pack is required for cross-module work, persistence or schema
changes, external integrations, significant security/reliability behavior,
multiple meaningful user/error states, or ambiguity that could produce
materially different implementations. A small isolated fix with established
authority and trivial remaining decisions may proceed without a pack; record
why it qualifies in the owning epic's slice map.

Scope expansions require updating governing docs and the pack before the
dependent code. Unresolved contract/ADR gaps must be labeled `needs-contract`
or `needs-adr` and block implementation. Delegation is optional and requires
an explicit user request; a reviewer role does not create a new approval gate.

## Closeout

Run focused validation and `./scripts/validate.sh`. Review delivered behavior
against the pack and update affected authority, evidence, and runbooks. Fill
the closeout fields: Planned, Shipped, Not shipped, New blockers, Docs updated,
Validation, Version, and Commit. Use evidence, not an anticipated result.

After the agreed slice is delivered and required checks pass, mark the pack
`shipped`, move it from `active/` to `archive/`, and update the owning epic,
epic index, and active/archive indexes together. Record legitimate deferrals
and successor slices; unfinished acceptance criteria cannot be relabeled as
shipped. A blocked slice stays active with its blocker and next action.

There is no application release policy yet. Version is `N/A` and commit is
uncommitted unless actual release/commit work is separately included in the
user's request. Local delivery is distinct from deployment or publication.

## Validation boundaries

The [validator](runbooks/validation.md) checks structure, references, pack and
epic consistency, configuration, and foundation readiness. It cannot judge
whether prose is decision-complete, evidence is truthful, or code meets the
specification. Those remain implementation and review responsibilities.
