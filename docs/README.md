# Documentation Governance and Lifecycle

Start with the [current product status and remaining features](roadmap/epics/index.md)
for implementation state and priorities, and [runbooks](runbooks/README.md) for
operating the product. This guide governs documentation and implementation work.

For product setup and operator guides, use the
[public documentation website](https://mikek184.github.io/Recollect/docs/).
Its [site contract](contracts/public-product-and-docs-site.md) and
[maintenance guide](../site/README.md) govern publication. Repository records
below remain canonical for development authority and lifecycle.

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
| [research/](research/README.md) | Retained studies and design proposals; not implementation authority |
| `roadmap/epics/` | Capability ownership, dependencies, and slice maps |
| `roadmap/execution/active/` | Planned, in-progress, and blocked slice specifications |
| `roadmap/execution/archive/` | Shipped slices with reconciled closeout evidence |
| `runbooks/` | Verified operator procedures for delivered capabilities |

Templates live alongside each document type. Indexes link current records;
retain superseded decisions with replacement links and shipped packs in the
archive. Avoid competing current versions of the same specification.

## Current baseline and documentation maintenance

The [foundations](foundation/README.md) are accepted and the Rust product is
implemented. Use accepted ADRs/contracts for behavior, active packs for unfinished
work and dated evidence for test/deployment observations. A document's old date
does not by itself make an architecture rule obsolete.

Keep one current status entry point: the [roadmap](roadmap/epics/index.md).
Do not append chronological handoffs or repeat a completed feature as a proposal.
When behavior is corrected, update current guidance and remove the superseded
proposal, duplicate report or unused illustration. Preserve necessary historical
citations as links to an exact Git revision. Shipped packs retain scope and
acceptance history; accepted decisions/contracts, frozen evaluation inputs,
current baselines and unresolved failures remain accessible. Never remove a
failure to make a remaining slice appear complete.

The 2026-10-18 cleanup follows the user's explicit request. It does not implement
product features, change product decisions or refresh runtime/benchmark
measurements. The earlier snapshot is available at
[checkpoint f2673d7](https://github.com/MikeK184/Recollect/tree/f2673d71c4b63b536d41b3143eb39c207955b201/docs).

New foundation decisions remain pending until accepted. Pending foundations
cannot govern dependent implementation; the small-fix exception does not bypass
that rule. Foundation authoring does not require speculative product epics.

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
