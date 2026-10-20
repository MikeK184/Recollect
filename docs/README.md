# Documentation Governance and Lifecycle

Start with the [current product status and remaining features](roadmap/epics/index.md)
for implementation state and priorities, and [runbooks](runbooks/README.md) for
operating the product. This guide governs documentation and implementation work.

For product setup and operator guides, use the
[public documentation website](https://recollecthub.com/docs/).
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
4. The owning epic's slice row and, for risky work only, its
   [execution pack](roadmap/execution/README.md).
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
| `roadmap/epics/` | Capability ownership, dependencies, and slice maps (acceptance and validation inline) |
| `roadmap/execution/active/` | Packs for planned, in-progress, and blocked risky slices; shipped packs are deleted |
| `runbooks/` | Verified operator procedures for delivered capabilities |

Templates live alongside each document type. Indexes link current records;
retain superseded decisions with replacement links. Shipped packs are not kept
in the tree: Git history holds them (see the 2026-10-19 note below). Avoid
competing current versions of the same specification.

## Current baseline and documentation maintenance

The [foundations](foundation/README.md) are accepted and the Rust product is
implemented. Use accepted ADRs/contracts for behavior, active packs for unfinished
work and dated evidence for test/deployment observations. A document's old date
does not by itself make an architecture rule obsolete.

Keep one current status entry point: the [roadmap](roadmap/epics/index.md).
Do not append chronological handoffs or repeat a completed feature as a proposal.
When behavior is corrected, update current guidance and remove the superseded
proposal, duplicate report or unused illustration. Preserve necessary historical
citations as links to an exact Git revision. Accepted decisions/contracts,
frozen evaluation inputs, paid-run approvals, current baselines and unresolved
failures remain accessible in the tree, as compact receipts rather than long
narrative ledgers. Never remove a failure to make a remaining slice appear
complete.

The 2026-10-18 documentation cleanup removed superseded studies and duplicate
reports. It implemented no product feature, changed no product decision and
refreshed no runtime or benchmark measurement. The earlier snapshot is available
at [checkpoint f2673d7](https://github.com/MikeK184/Recollect/tree/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs).
On 2026-10-19 the developer-tooling ADRs, contracts and packs were collapsed into
the [development tooling runbook](runbooks/development-tooling.md); that state is
available at [commit f1b6bb3](https://github.com/MikeK184/Recollect/tree/1e49d772360b1ea12ddf198a5c3dbbed0f714b1b/docs).

The owner changed the governance workflow on 2026-10-19 (see the
[ADR 0001 amendment](adr/0001-repository-governance.md)). Execution packs are
now required only for risky work, the 105 shipped packs were removed from the
tree, and long mapping ledgers were collapsed into receipts tables. The earlier
state, including every shipped pack and the full ledgers, is available at
[commit a0aea9d](https://github.com/MikeK184/Recollect/tree/873156174f421da0574952a5e533fb6a6133fab0/docs).
The change implemented no product feature, changed no product decision and
refreshed no runtime or benchmark measurement.

`output/` (browser screenshots, generated imagery and review captures) is
ignored and no longer tracked. Where earlier records cite `output/...`, the last
tracked copy is at Git revision `f2673d7`: read it with
`git show f2673d7:output/<path>`. Cite new evidence by revision or in a tracked
summary, not by a link into `output/`.

New foundation decisions remain pending until accepted. Pending foundations
cannot govern dependent implementation; an inline slice does not bypass that
rule. Foundation authoring does not require speculative product epics.

## Before implementation

1. Read this guide, applicable `AGENTS.md`, and task-relevant authority.
2. Verify external or dependency assumptions using Context7 and primary
   official sources. Record useful evidence and its date in mappings; link
   research checkouts under `references/` by upstream URL pinned to the
   recorded commit, since the linter does not check relative `references/` links.
3. Resolve missing ADRs/contracts before the dependent code change.
4. Add the slice to its owning epic's slice map. Record acceptance and
   validation in the row (Execution `inline: <acceptance; validation>`).
5. If the work is risky (below), create the pack, make it decision-complete and
   set it and the epic slice to `in-progress` before implementation; update
   the active index.

An execution pack is required only for risky work: (1) a schema or migration
change with restore or compatibility impact; (2) a new external integration, or
a paid or frozen evaluation campaign (cost approval and frozen input hashes);
(3) a security, credential or authz change that no accepted contract already
fully specifies; (4) multi-session work with open decisions. Everything else is
an epic slice row with acceptance and validation inline. Any behavior change
still updates the governing contract in the same change.

Scope expansions require updating governing docs (and the pack, if any) before
the dependent code. Unresolved contract/ADR gaps must be labeled `needs-contract`
or `needs-adr` and block implementation. Delegation is optional and requires
an explicit user request; a reviewer role does not create a new approval gate.

## Closeout

Run focused validation and `./scripts/validate.sh`. Review delivered behavior
against the slice acceptance and update affected contracts, evidence, and
runbooks. Use evidence, not an anticipated result.

After the agreed slice is delivered and required checks pass, set the slice row
to `shipped` with a one-line Summary that names the validation evidence. For a
packed slice set Execution to `archived`, delete the pack, and update the
active index and the epic index in the same change. Deferrals and failures go
in the slice Summary or a successor slice, never hidden; unfinished acceptance
cannot be relabeled as shipped. A blocked slice stays with its blocker and next
action (its pack stays in `active/`).

There is no application release policy yet, and slices record neither a
version nor a commit; Git holds the history. Local delivery is distinct from
deployment or publication.

## Validation boundaries

The [validator](runbooks/validation.md) checks structure, references, slice-map,
pack and index consistency, mapping `Observed:` lines, and foundation readiness. It cannot judge
whether prose is decision-complete, evidence is truthful, or code meets the
specification. Those remain implementation and review responsibilities.
