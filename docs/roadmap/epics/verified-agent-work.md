# Verified Agent Work: Receipts and Action Memory

Status: planned

## Purpose

Make every agent's work leave a receipt: who acted under which authority, in
which Brain, environment and scope, what it called, which repository revisions it
relied on, and whether its claims are supported by evidence the system recorded
itself. Use that memory at the moment of action: surface what is known about a
target before a tool runs, failed attempts before they are repeated, and
commitments when their trigger fires. Part 4 of the five-part improvement sequence in the [roadmap](index.md#improvement-sequence).

This is the product's intended public differentiation, set by the owner on
2026-10-20. AI gateways route models and tools, memory layers store facts and
host plugins remember one person's sessions; none of them links an agent's claims
to environment-aware, correctable evidence.

## Governing Sources

- [Session capture](../../contracts/evidence-session-capture.md) and
  [managed observation capture](../../contracts/mcp-observation-capture.md)
- [Source support verification](../../contracts/memory-source-support-verification.md),
  [claims and time](../../contracts/memory-claims-and-time.md) and
  [procedures and handovers](../../contracts/memory-procedures-and-handovers.md)
- [Runtime and credentials](../../contracts/mcp-runtime-and-credentials.md),
  [agent coordination](../../contracts/mcp-agent-coordination.md) and
  [plugin session memory](../../contracts/mcp-plugin-session-memory.md)
- [Public product and docs site](../../contracts/public-product-and-docs-site.md)

## Dependencies and Boundaries

Starts when [claim truth and freshness](claim-truth-and-freshness.md) has carry-forward
and the freshness policy, and [agent identity and authority](agent-identity-and-authority.md)
has principals and scoped credentials, because a receipt is only worth something
when it names which agent acted under whose authority. Point-of-action recall
needs only the claim slices and may start earlier.

**Order.** Receipts, then claim verification, then the capture API so any harness
produces receipts. Verification requests follow; Recollect starting read-only
probes itself is an authorization change and needs a pack. Point-of-action recall,
the failed-attempt ledger and prospective commitments build on receipts and
verification. Public positioning comes last, once the evaluation, the Atlas
self-score and receipts have dated evidence.

**Open decisions for the owner.**
- Do receipts become Brain evidence readable by every Brain reader, or are they visible only to the acting principal's owner and Brain administrators?
- May Recollect start read-only verification probes on its own, and under which principal?

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `agent-work-receipts` | planned | needs-adr, needs-contract | inline: each agent task yields an immutable, attributable receipt naming the principal, Brain, environment and scope, its managed tool observations, the repository revisions used and the claims it produced with their support state, reviewable and subject to existing erasure rules; validation: end-to-end receipt for a task using managed tools, plus correction, erasure and unauthorized-reader controls | The headline capability: verified agent work |
| `agent-claim-verification` | planned | needs-contract | inline: claims an agent states (deployed, tests pass, configuration value) are checked against recorded evidence as supported, unsupported or unknown, never confirmed by a model alone, and people can accept or reject them; validation: deterministic controls for each state, including a claim with no supporting observation | Reuses source support verification for agent output; operational `verified` stays an author assessment unless system-observed evidence certifies it |
| `agent-capture-api` | planned | needs-contract | inline: any harness without host hooks (an Agent SDK job, CLI or CI step) can report its session and results through one authenticated endpoint and a small CLI, under the same capture policy, sanitizer and size limits as plugin capture; validation: headless fixture reporting a session that becomes recallable and appears in its receipt | Makes harness-agnostic capture true beyond the three supported hosts |
| `claim-verification-requests` | planned | needs-adr, needs-contract | inline: an unknown, stale or contradicted claim yields a verification request answered first by a matching approved read-only probe in the claim's environment (through a private runner where needed), otherwise by a question to a connected agent or the owner through the coordination inbox or a skill; every answer is a new observation with its source, never trust in the answerer; validation: probe fixtures for supported, contradicted and unreachable outcomes, read-only grant enforcement and a question round trip | Lets the system ask the environment instead of guessing; Recollect starting tool calls itself is an authorization change, pack required |
| `memory-point-of-action-recall` | planned | needs-contract | inline: when an agent calls a managed tool against a connection or environment, Recollect attaches the current claims, open contradictions, recent failures and receipts for that target to the call's context, bounded and scoped, without changing the call or its authorization; validation: MCP fixtures showing the attached context for current, stale and contradicted targets and its absence for unauthorized readers | Memory delivered at the moment of action, which only a memory layer sitting in the tool path can do |
| `memory-failed-attempt-ledger` | planned | needs-contract | inline: an approach that failed (with its observed failure: command, exit code, test or probe result, revision and environment) becomes a negative procedure that is recalled when an agent proposes a similar action, until a later observed success supersedes it; validation: fixtures where a repeated failed fix is surfaced before execution and a later success retires it | A tombstone for actions: agents stop repeating fixes that already failed |
| `memory-prospective-commitments` | planned | needs-contract | inline: an agent or the owner can record an intention with a trigger Recollect observes (a repository publication, a deploy or configuration observation, a date), and the trigger produces a verification request, a notification through the coordination inbox or a reminder in the next session; validation: trigger fixtures for each kind, including expiry and a trigger that never fires | Prospective memory, the category the Atlas finds almost nothing models |
| `product-verified-work-positioning` | planned | needs-contract | inline: the README and recollecthub.com lead with verified agent work once receipts and the engineering-task evaluation have dated evidence, stating only measured results; validation: site build and check, Light and Ink review, and every claim linked to its dated evidence | Marketing follows proof, under a site contract amendment |
