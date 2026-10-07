# Scoped project brief and authenticated continuation

Status: accepted

## Source

The user-authorized [automatic memory plan](../research/automated-memory-improvement-plan-2026-10-07.md)
adds scoped context to normal plugin recall. Existing memory authority, retrieval
scope, support, deadlines, cost and plugin latency limits remain unchanged.

## Contract

Recall may request `project_brief` and a typed continuation task/binding hint.
All returned records remain complete attributed `RecallItem`s with a delivery
section (`query`, `project_brief` or `continuation`); they share the caller's
result count and byte budget. Brief-only delivery is explicitly reported and is
not a successful query match. Historical recall never supplies a current brief.

A brief contains at most three current accepted supported decisions, conventions
or constraints. An optional canonical `context_role` labels the latter two;
legacy decisions qualify through kind, other unclassified records remain query
memory. Roles are descriptive, carry no new authority, are assessed with the
whole assertion, and may be supplied by the existing extraction call. Apply
ordinary selection, collection, manifest, validity, rejection, conflict and
privacy gates before deterministic selection: scope specificity, role, normalized
assertion family, then UUID. Canonical scope/collection/manifest/lifecycle gates
precede a bounded 500-row discovery scan; ordinary Rust eligibility is applied
before the 64-eligible-record limit. A saturated continuation scan records bounded
coverage and omits selection, rather than assuming that unseen rows are unambiguous.
Brief JSON
uses at most 1536 bytes; omit whole records that cannot fit. Query records retain
priority and at least half the available slots. Unused brief space returns to
query recall. No additional model call, inventory endpoint or persistent cache.

Continuation hints are not authorization. Require an authenticated operation;
the hinted task must be its task or a validated continuation predecessor with
same Brain, actor, device and exact selection. `continuation_of_task_id` is an
immutable closed predecessor relationship, separate from child delegation;
validate it at task creation. Traverse at most 32 predecessors. A binding hint
must belong to that task and caller. Without a hint, a single unambiguous
applicable settled episode may supply context; multiple episodes record an
ambiguity and fall back to brief/query. Never choose newest timestamps. Use
current published session digest coverage only, at most one page and one slot,
with exact nullable manifest and child boundaries. Late receipt/correction,
logical expiry or erasure invalidates delivery through the shared predicate.

Plugin recall requests this automatically. Native ended-session resume retains
its prior authenticated task as continuation predecessor. Injection stays within
8192 bytes including JSON escaping and framing, six total records, eight seconds
including four-second rich retrieval and lexical fallback. Query length is
bounded to 512 UTF-8 bytes. A partial capture or empty match remains an explicit
coverage limitation and never asks the agent to investigate Recollect's source.

## Acceptance

Acceptance proves non-query stable context, query priority, byte/slot/time bounds,
scope/manifest/collection and human-rule controls, ambiguous episode exclusion,
spoofed/cross-device/predecessor hints, corrected/erased/expired inputs, native
resume and unchanged-model-call counts.

## Explicit Deferrals

Local acceptance and the versioned synthetic support comparison are recorded in
[the implementation mapping](../mappings/memory-source-support-staging-2026-10-07.md).
Deployment remains separate from the archived execution pack. The evaluation does
not establish general semantic accuracy or every typed producer-action benchmark. No additional runtime, hidden provider, automatic command execution or
change to human authority is authorized by this contract.
