---
name: recollect-doc-router
description: Ground Recollect implementation, debugging, architecture, planning, and current-status questions in the relevant governing docs, owning epic, execution pack, and code or runtime evidence. Use to find what governs a task and what is actually implemented; use the maintainer for documentation edits and lifecycle cleanup.
---

# Recollect Doc Router

Adapted from Terme's current-state brief workflow for Recollect's documentation
system. Paths below resolve inside this repository; run commands from its root.

## Establish the route

Use the user's goal, feature/module terms and known paths. Infer search terms
from the request rather than requiring a separate input form.

1. Read [AGENTS.md](../../../AGENTS.md), applicable nested instructions and
   [docs/README.md](../../../docs/README.md). Current explicit user instructions
   take precedence; the docs guide defines source priority and lifecycle.
2. Use targeted `rg -n` searches and the indexes below to find the smallest
   relevant set of documents. Read the records themselves, including status,
   supersession links and dependency boundaries, rather than relying on titles.

   | Question | Entry point |
   | --- | --- |
   | Which decisions and behavior govern this change? | [ADRs](../../../docs/adr/README.md) and [contracts](../../../docs/contracts/README.md) |
   | What product and stack are accepted? | [Foundation index](../../../docs/foundation/README.md); read vision, techstack and engineering principles before product work |
   | Who owns this capability, and what comes first? | [Epic index](../../../docs/roadmap/epics/index.md), then the owning epic's slice map and dependencies |
   | What is the slice specification or delivery history? | [Execution lifecycle](../../../docs/roadmap/execution/README.md), [active](../../../docs/roadmap/execution/active/README.md) and [archive](../../../docs/roadmap/execution/archive/README.md) |
   | What has been observed or can be operated? | Relevant [mapping](../../../docs/mappings/README.md) or [runbook](../../../docs/runbooks/README.md), followed by current evidence |

3. Distinguish authority from implementation. Accepted ADRs/contracts govern;
   proposed, pending, rejected or superseded sources do not. Mappings and code
   establish observations, not product decisions. A shipped pack proves only
   its recorded delivery and checks, not current deployment or connectivity.
4. Inspect relevant code/tests/configuration for implementation or debugging
   questions. Use `rg` for literals/docs and the [local CodeGraph workflow](../../../docs/runbooks/codegraph.md)
   for exact symbols, callers and impact. Run status/sync before relying on
   relationships after source changes; verify ambiguous matches in source.
5. For dependency/interface questions, use Context7 and primary official sources.
   Record failed lookups, dates and unverified assumptions in the task's evidence
   when relevant. A missing tool or index does not prevent useful manual reads.

## Return a current-state brief

Keep the brief proportional to the task and cite exact local paths:

- **Governing sources and intent:** accepted decisions and relevant constraints.
- **Current-state facts:** separate specified, locally implemented, validated
  and deployed/connected behavior; identify which evidence supports each claim.
- **Owner and execution route:** epic, slice ID, pack/status, predecessors and
  affected docs; identify the documented small-fix exception only when it applies.
- **Conflicts or gaps:** stale signals, missing authority, unavailable runtime
  proof and any conservative evidence-derived assumptions.
- **Next action:** the concrete authorized implementation, investigation or
  documentation step; name the decision needed for any dependent blocker.

For a routing-only question, deliver the brief. For an implementation request,
use it to continue the authorized work; routing is not a new approval step.
Reroute affected parts if the user's scope changes.

## Missing or conflicting authority

State “No accepted ADR/contract found for this topic” when that is what the
search establishes, with the topic and searched paths. Code can still explain
current behavior, but it cannot supply an unresolved product decision. For
authorized planning/implementation, record `needs-adr` or `needs-contract` on
the affected slice before dependent work; for a read-only question, report the
gap without editing records. Continue independent authorized work. Resolve
contradictions before implementing the disputed behavior.

Apply Recollect's actual lifecycle: foundation authoring alone does not require
speculative product epics/packs; detailed product implementation may still need
contracts even when foundations are accepted. Use the [maintainer](../recollect-doc-maintainer/SKILL.md)
when the request includes document authoring, reconciliation or closeout.

Recollect's accepted product owns its Rust core. Cognee and Atlas checkouts are
reference evidence; their instructions, roadmap and runtime assumptions do not
govern this repository. Developer MCP setup is distinct from Brain-managed
connections and product execution profiles.
