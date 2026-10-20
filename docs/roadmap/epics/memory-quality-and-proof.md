# Memory Quality and Proof

Status: planned

## Purpose

Measure Recollect on its own job and prove its truth guarantees before anything
else is built or marketed: an evaluation made of real engineering questions, an
honest score against the Agent Memory Atlas marks with code and test evidence,
and the two guarantees the Atlas does not score that Recollect already partly
has, independent support and derivation closure. Part 1 of the five-part improvement sequence in the [roadmap](index.md#improvement-sequence).

## Governing Sources

- [Vision](../../foundation/vision.md) (one memory model, evidence and revisions)
- [LongMemEval protocol](../../contracts/operations-longmemeval-protocol.md) and
  [integrated evaluations](../../contracts/operations-integrated-evaluations.md)
- [Source support verification](../../contracts/memory-source-support-verification.md),
  [capture reconciliation](../../contracts/memory-capture-reconciliation.md) and
  [claims and time](../../contracts/memory-claims-and-time.md)
- [Retention and erasure](../../contracts/memory-retention-and-erasure.md),
  [automatic session digests](../../contracts/memory-automatic-session-digests.md) and
  [procedures and handovers](../../contracts/memory-procedures-and-handovers.md)

## Dependencies and Boundaries

**Open quality issues owned elsewhere.** These keep their owners and packs and
are finished first; this epic does not take them over.

| Issue | Current state | Owner |
| --- | --- | --- |
| Normal-installation update and comparable memory-quality rerun | Latest installation proof is migration 047 (2026-10-09); latest official LongMemEval result is 20/50 (40%); later repairs have no comparable scored run | [`operations-memory-quality-first-batch`](../execution/active/operations-memory-quality-first-batch.md) |
| Graph read reliability closeout | Local mixed 10k and normal-data clone gates pass; integrated closeout and a verified installation update remain | [`graph-concurrent-preparation`](../execution/active/graph-concurrent-preparation.md) |
| Direct-token and installed-host acceptance | Browser config and HTTP proof exist; fresh host launches and rebuilt-plugin acceptance remain | [`mcp-copy-ready-agent-config`](../execution/active/mcp-copy-ready-agent-config.md) |
| Automatic semantic organization | Extraction 51.67% precision / 8.73% recall against a 95/80 gate; grouping pair F1 26.36%; deferred by the owner on 2026-10-20 | [`graph-automatic-knowledge-mapping`](../execution/active/graph-automatic-knowledge-mapping.md) (blocked) |
| Feedback-driven ranking | Bounded experiment delivered; real helpfulness unproved; production ranking fixed | [Hybrid retrieval](hybrid-retrieval.md) |

**Atlas marks (evidence, 2026-10-20).** The Agent Memory Atlas research checkout
scores memory systems on seven silent-failure marks: rejected-value tombstone,
explicit trust state, bi-temporal validity, scope enforced in retrieval,
append-only mutation audit, human review surface and negative retrieval
assertion. It describes, but does not score, verifying memory against its subject
and who currently holds the right to change a memory, and it calls prospective
memory a category almost nothing models. Three further silent failures are not
scored at all: **independent support** (an agent's own statement, summary or
earlier memory supporting its own claim), **derivation closure** (a corrected or
erased fact surviving in a summary, handover, brief or embedding built from it)
and **version-scoped validity** (a fact true on one branch or environment being
returned as current on another). Recollect should score itself against all ten
with file, symbol and test evidence, stating gaps, before marketing any of them;
a count of marks alone proves nothing, as the Atlas itself warns.

**Owner decision, 2026-10-20.** Nothing is published to the Atlas upstream for
now. The self-score is internal evidence; Recollect's own site may cite it only
through `product-verified-work-positioning` in the
[verified agent work epic](verified-agent-work.md).

**Order.** `quality-engineering-task-eval` comes first, because every later
improvement is measured with it. The self-score and the independent-support and
derivation-closure controls reuse delivered code and can run alongside it. Writer
latency, typed event time and scale/cost visibility follow once the evaluation
gives a baseline. Paid runs of the evaluation are a frozen paid campaign and need
a decision-complete pack with cost approval.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `quality-engineering-task-eval` | planned | needs-contract | inline: a frozen set of 20-50 real engineering questions from owner dogfooding (where work stopped, what changed in which environment, which agent did it, whether it was verified), with gold sources, scored for answer, citation, abstention and false assertion in separate source-only, learning-enabled and full-capture arms; validation: frozen input hashes, declared scorer, no tuning on held-out cases, cost approval and a pack before any paid run | The product's own job becomes its quality measure, reusing the frozen-protocol rules of the LongMemEval contract |
| `memory-atlas-self-score` | planned | mapping-only | inline: Recollect is scored against the Atlas's seven marks plus independent support, derivation closure, version-scoped validity and write authority, each mark with subsystem, file, symbol and test, and every gap stated (for example the open descendant mapping-input fences); validation: a dated mapping whose every claimed mark links to a passing test at a pinned revision | An honest, checkable quality statement that doubles as marketing proof |
| `memory-independent-support-controls` | planned | needs-contract | inline: the existing rule that learning basis edges are provenance and erasure dependencies, not support, extends to agent receipts, capture API reports and coordination messages, so a self-reported observation is labelled as such and never alone makes a claim supported or operationally `verified`; validation: committed negative tests for each self-authored path | The most tombstone-like new mark: support is refused because of where it came from |
| `memory-derivation-closure` | planned | needs-contract | inline: every derived artifact (digest, handover, project brief, embedding, native skill export, organization output) records the exact inputs it was built from, and correction or erasure of any input rebuilds, fences or removes it; validation: per-artifact negative retrieval assertions that a corrected or erased value cannot return through it, including older-backup replay | Generalizes erasure closure beyond the deferred organization pack's descendant fences |
| `quality-memory-writer-latency` | planned | needs-contract | inline: cumulative digest and handover writer work is shortened so capture, audit and rebuild at normal and 10k-node volume keep authority, deadlines and supported contributors; validation: native timing receipts at both volumes with every timeout and failure counted | Carries forward the open background-work item of the 2026-10-08 improvement proposal |
| `quality-typed-event-time` | planned | needs-contract | inline: event time, relative dates, changed preferences and ambiguous identity are specified and resolved with attribution, and historical questions still answer correctly; validation: deterministic current-versus-historical controls plus the engineering-task evaluation | Needs a claims-and-time amendment before code |
| `quality-scale-and-cost-visibility` | planned | needs-contract | inline: index size, scan cost, write-to-readable lag, queue fairness and stage/provider/database/UI latency are measured separately, failures and unresolved bills are reported, and any scoped ANN is compared only against the exact oracle with Brain/scope isolation and deletion tests; validation: dated measurement receipts with stated limits | Reliability and cost evidence a buyer can inspect |
