---
name: recollect-doc-maintainer
description: Author, update, or audit Recollect documentation and keep ADRs, contracts, foundations, mappings, epics, execution packs, runbooks, and indexes consistent. Use for targeted docs changes, stale-doc reviews, pre-PR cleanup, and slice closeout; preserve audit-only or metadata-only scope when requested.
---

# Recollect Doc Maintainer

Adapted from Terme's scoped maintenance and severity-reporting workflow. Use
Recollect's existing Markdown records and checker; no generated docs index or
foreign frontmatter schema is required. Run commands from the repository root.

## Choose scope from the request

- **Audit:** Inspect and report when asked to check/review only. Do not mutate
  records or services to fill an evidence gap.
- **Metadata/lifecycle:** Repair supported links, indexes, ownership and status
  bookkeeping when that is the requested scope. Status changes still need
  evidence; a filename alone cannot justify changing acceptance or shipment.
- **Content and lifecycle:** For authorized authoring, fixes or implementation
  closeout, update the affected content and its lifecycle records together.

Use the user's existing authorization and path constraints. Do not turn ordinary
maintenance into a new approval ceremony or a repository-wide rewrite.

## Find authority and affected records

Read [AGENTS.md](../../../AGENTS.md), applicable nested instructions and
[docs/README.md](../../../docs/README.md). Use the [router](../recollect-doc-router/SKILL.md)
when ownership or governing sources are not already established. Inspect the
working tree, including relevant untracked files, before editing; preserve
unrelated work and keep writes inside Recollect.

For a maintenance audit, start with `./docs/tools/lint_agent_governance.sh` to
collect structural failures. For a targeted edit, inspect the owning records
and run that check after edits. The [validation runbook](../../../docs/runbooks/validation.md)
describes the checker and its limits.

Choose the existing document owner instead of adding a competing specification:

| Change | Where it belongs |
| --- | --- |
| Authorized durable product/stack/principle decision | Relevant [foundation](../../../docs/foundation/README.md); reflect actual decision status |
| Architecture decision or testable behavior | [ADR](../../../docs/adr/README.md) or [contract](../../../docs/contracts/README.md), using its template and preserving supersession history |
| Dated source, dependency or runtime observation | [Mapping](../../../docs/mappings/README.md), with method, date, confidence and limits |
| Capability ownership, dependencies or new slice | Owning [epic](../../../docs/roadmap/epics/index.md), then the [execution template](../../../docs/roadmap/execution/template.md) when required |
| Delivered operating procedure | Relevant [runbook](../../../docs/runbooks/README.md), with tested prerequisites and failure behavior |
| Navigation or developer workflow | Affected index, agent guide, role or skill; link to canonical docs rather than copying their rules |

Retain the document type's actual metadata and template fields. Do not add
Terme's current/future/done_older layout, docsctl artifacts, review-date schema,
SaaS policies or another project's release rules. Foundations and accepted
decisions change only from established user decisions or resolved authority;
an observation or cleanup request does not silently redefine the product.

## Reconcile the lifecycle

Follow [the lifecycle](../../../docs/README.md) and [pack rules](../../../docs/roadmap/execution/README.md):

- Add the slice to its owner before a required pack and finish the relevant
  decisions before implementation. Keep the pack filename equal to the unique
  slice ID and its repo-relative owning-epic field accurate.
- Keep planned, in-progress and blocked packs in active; shipped packs belong
  in archive. Update pack status, owning slice, epic status, epic index and
  active/archive indexes in the same change.
- Label missing decisions `needs-adr`/`needs-contract`; record the blocker and
  next action. Preserve independent authorized progress. Do not infer missing
  schemas, interfaces or product choices just to fill a template.
- Use the documented small-fix exception only with established authority and
  a reason in the owning epic. Foundation-only authoring does not create
  speculative product work. Accepted foundations do not replace detailed contracts.
- Close out against actual acceptance evidence: Planned, Shipped, Not shipped,
  New blockers, Docs updated, Validation, Version and Commit. Use meaningful
  `N/A` reasons. Unmet acceptance stays open; later changes normally get a new
  slice rather than rewriting a shipped pack's historical delivery.

Keep specified, implemented, locally tested and deployed/connected claims
separate. Record failed Context7/official lookups and verification limits. A
configuration stanza, graph hit or successful docs check does not prove a
working product feature. Developer skills/MCPs and runtime Brain tools have
different owners and evidence.

## Validate and report

Run focused checks appropriate to the changed records and `./scripts/validate.sh`.
For changed skill entrypoints, also validate frontmatter, local references and
host discovery; the existing governance checker does not scan skill bodies.
Check `git diff --check` and inspect new untracked files explicitly because
ordinary Git diffs omit them. Recheck affected indexes after pack moves.

Report the mode, meaningful findings, files changed, checks and outcomes,
unresolved blockers and next actions. Triage substantive issues first:

- **Blocking:** unresolved governing decisions for dependent implementation,
  false shipment/authority, credential disclosure or conflicting behavior.
- **Significant:** incorrect ownership/state/index, broken references, missing
  validation evidence or claims that confuse configuration with runtime proof.
- **Minor:** wording and navigation drift that does not alter behavior.

If a check fails, identify the file/reason and repair what the task authorizes;
do not weaken the checker or invent evidence. End with the actual version/commit
state when closing a slice. No automatic commit, push, release or deployment is
implied, and neither skill delegates to other agents without a user request.
