# Autonomous Brain memory maintenance

Status: accepted

## Source

The user's 2026-09-14 clarification establishes autonomous operation: no person will
review learning, revision, forgetting or erasure record by record.
[ADR 0006](../adr/0006-autonomous-memory.md) resolves the operating model. Existing
[memory](memory-claims-and-time.md), [correction](memory-review-and-corrections.md),
[provider](memory-provider-policy-and-learning.md),
[procedures](memory-procedures-and-handovers.md) and
[retention](memory-retention-and-erasure.md) supply the canonical interfaces.

## Contract

### Standing authority and scheduling

Add `autonomous_memory` to Brain model policy. New policy forms select autonomous
operation by default; transmission still requires one Brain configuration naming
permitted content and budgets. Stored policies missing the field preserve previous
behavior. Enabling autonomy requires extraction, synthesis and claim permission.
Neither a model nor automatic learning grants tool execution.

A native pass runs every ten seconds with batches of at most twenty items per
Brain and no external call inside scheduling transactions. It catches up eligible
retained current sources, resumes durable queued work after restart and responds
to source/claim changes without per-record clicks. Scheduled work uses the current
policy author's standing Brain grant, preserves source contributor provenance and
rechecks permission at publication. Archived/disabled Brains, revoked authority,
unavailable evidence and exhausted budgets remain visible while other work proceeds.

Preserve explicit source/task applicability. Legacy unscoped sources use Brain
scope; never guess a current task, environment or deployment. One successful
automatic derivation suffices per exact source and policy. Changed policy can
reconsider evidence; equivalent output reuses existing canonical records.

### Learning, acceptance and revision

Permitted evidence triggers model extraction and reconciliation. Models supply
bounded assertions, exact source spans and, when revising existing machine-maintained
memory, its exact current revision identity. Only server-selected same-Brain,
same-applicability candidates are valid targets. Source lineage and cited content
determine change, not timestamps alone. Other environments and historical facts
must not be silently revised.

Supported routine output becomes accepted by policy without a human reviewer.
General text is eligible beyond the earlier literal-configuration rule. Keep
operational state independent; model confidence is not an operational test.
Reconciliation can retain assertions, append supported replacements, qualify
unresolved alternatives or retire obsolete machine-derived claims with reasons.
Ambiguity is a valid automatic outcome and creates no mandatory review task.
Keep history and input/output provenance. Models cannot override human corrections,
rejections or withdrawal, or revive erased values. Changed evidence still passes
current canonical rules.

Procedures and handovers share this policy. Extraction can produce a claim, an explicitly evidenced decision or a
procedure. Procedure extraction supplies conditions, 1–20 ordered steps and an
expected outcome; it starts with no verified observations. Models cannot invent
test results. Reconciliation may update machine-maintained procedures as well as
claims while keeping the original kind and applicability.

Handovers may be accepted
by policy while preserving every contributor's independent eligibility. Changed
contributors schedule refresh of an existing machine-maintained handover with new
exact inputs and an atomic append to its existing identity. A later human edit,
missing contributor, changed scope, erasure or lost lease suppresses stale output.
Unsupported outcomes remain untested; synthesis cannot invent success.

### Forgetting, erasure and failures

Changed/expired evidence automatically qualifies or excludes memory from current
recall. Superseded and withdrawn records cannot re-enter through summaries, indexes
or repeated extraction. This is logical forgetting. Existing class deadlines drive
physical removal through the deletion journal without per-record confirmation.
Manual Erase is an optional immediate override. Durable knowledge need not be deleted
merely because time passes or nobody retrieved it.

Scheduled work respects existing capacity, budgets, cancellation and leases. A
charged attempt never silently repeats. Known transient failures may start at most
two separately recorded replacement attempts, after five and thirty minutes.
Uncertain external outcomes stay visible and do not silently spend again. Changed
policy/input supersedes obsolete queued work. Service failures are operational
issues, not a requirement to approve each memory.

Model-input links, dispositions and revisions participate in current erasure closure
and older-database replay. Publication, audit and refresh commit atomically; billing
remains a separate durable outcome.

### Browser and acceptance

Show autonomous versus explicit mode, progress, policy-accepted results, uncertainty,
revision reasons and safe failures. Keep inspection, correction, withdrawal and Erase
reachable. Avoid implying that every result needs human review.

## Acceptance

Prove a closed loop without per-record acceptance calls: catch up a source, accept
supported prose, ingest changed evidence, revise the same canonical memory, refresh
its handover, exclude obsolete/expired content and enforce physical removal.
Also prove unrelated scope, durable human overrides, ambiguous evidence, idempotent
restart, bounded retries, in-flight erasure and older-database replay. Use controlled
HTTP and one bounded synthetic real-Luna workflow, browser inspection and the
repository validator.

## Explicit Deferrals

Adaptive retrieval-strength scoring is separate from autonomous lifecycle operation.
Executable skills, automatic procedure execution, changing customer files and
universal semantic equivalence are outside this contract. Session/MCP capture uses
this loop when its named producer slices are delivered.
