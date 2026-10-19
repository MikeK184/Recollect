# Concurrent graph preparation and checked publication

Status: accepted

## Source

The user's 2026-10-09 instruction to implement the dedicated automatic mapping
plan includes repairing populated graph reads under background work. This
contract refines [ADR 0007](../adr/0007-canonical-graph-projections.md),
[canonical traversal](graph-projection-and-traversal.md),
[exploration](graph-exploration.md) and the
[engineering concurrency principles](../foundation/engineering-principles.md).

## Contract

Prepare interactive graph selections and generation descriptors in a PostgreSQL
`REPEATABLE READ READ ONLY` transaction. A dedicated transaction-local preparation
flag permits shared Brain authorization without acquiring Brain admission/tuple
locks. Preparation still checks enabled accounts, paired devices, current visible
Brain roles and all canonical source, claim, scope, correction and retention gates.
Read-only preparation cannot write canonical state, reserve work, emit model calls
or acquire an exclusive mutation boundary.

The snapshot is provisional. It never authorizes a response or publication.
Freeze the Brain analytics epoch in the snapshot, together with existing exact
generation identities and retention deadlines. Model-policy heads, model input
fences and model claim fences participate in analytics invalidation, in addition
to existing source/revision/membership/correction/retention/generation inputs.
No persistent cache of unchecked labels or evidence is introduced.

Interactive reads complete physical Neo4j verification/traversal outside Brain
locks. Before returning buffered results, open a fresh authenticated transaction,
acquire the existing fair shared admission/Brain lock, recheck Brain access,
paired scope and browser session, require the frozen analytics epoch to match,
and check current wall-clock input deadlines. Changed inputs discard the entire
buffer and return `graph_preparation_changed`; no stale prefix is returned.
For explicitly partial windowed reads, an advanced Brain epoch may reflect an
unrelated capture. Such reads may requalify every buffered entity under the fresh
fair lock and verify exact ready generation metadata and descriptor membership.
They can retain the buffer only when the complete hydrated node payloads and
scope are identical and all current deadlines/inputs remain eligible. Update
current epoch/deadline/coverage metadata before returning. This bounded check
performs no native I/O and cannot claim global path/absence or aggregate validity.
Non-windowed reads and worker publication retain strict epoch rejection.
The final lock stays held through commit. Existing request, statement, capacity,
node/edge and exact projection verification limits remain.

A ready generation can be replaced while its native read is in flight. At most
two internal full retries may discard that buffer and reprepare against the
new exact ready generation, under the original 15-second deadline and held read
permit. Retry only after proving that the old generation was superseded without
other metadata/descriptor changes and the replacement has equivalent current
scope, immutable snapshot, canonical payload and eligible input identities.
Knowledge/combined inputs retain the same semantic epoch; repository inputs
retain the same immutable snapshot. Pin any originally resolved latest snapshot.
Corruption, changed scope/support, access loss, expiry and native mismatch keep
their existing failures and cannot become a replacement retry.

Workers build descriptors outside exclusive Brain locks. Their short staging
transaction rechecks actor/device writer authority, archive state, exact lease,
generation identity, snapshot/input availability, preparation epoch and deadlines
before persisting the prepared descriptor. Native import remains outside the
transaction. A short final publication transaction repeats those checks before
readiness. Own readiness writes may advance analytics epoch; the pre-publication
comparison precedes those writes, while existing availability/deadline/lease
checks follow them. Stale worker preparations are retried under existing bounded
lease/backoff rules. A saved descriptor is reusable only at its preparation epoch;
older descriptors without that metadata must be rebuilt.

Each native import is fenced by its exact current lease token and durable native
attempt ordinal. Reset only that unpublished generation before a newer attempt;
delayed writes/setup from an older attempt and writes after sealing return zero.
Never remove shared published entities while resetting an attempt. Persist the
native phase marker before any native/schema HTTP. An exact live graph.project
claim may defer pre-import epoch contention at most eight times without consuming
an actual native attempt; only a claim with no native marker may receive that
refund. The ninth preparation deferral terminates. The existing three-attempt
ceiling still applies to actual import/final-publication failures. No timer,
expired lease, wrong job kind or partial native attempt may refund a claim.

This changes lock tenure, not evidence meaning. Existing provenance, structural
relationships and canonical qualification remain. Analytics and answer/retrieval
consumers retain their existing serialized transactions unless explicitly moved
to this complete two-phase protocol. A preparation flag cannot be used to bypass
the final publication boundary.

Canonical deadline and graph-window queries must resolve requested immutable
identities before joining their dependencies. Parameterized lateral lookups may
avoid repeated Brain-wide RLS scans, but must retain the complete exact/current
dependency closure, selected manifest entries, reviewed expired-support rules
and invoker RLS. A performance change cannot omit an unavailable dependency or
substitute the current head for an exact historical input.

## Autonomous maintenance amendment — 2026-10-18

The user resumed the complete graph implementation, including the identified
digest/handover writer-tenure repair. Autonomous scheduling must not combine
source discovery, digest preparation, handover refresh and retry discovery in
one long exclusive Brain transaction. Discover candidates and prepare complete
qualified contributor pages in consistent read-only preparation transactions.
Publish bounded work in independent short writer transactions, freshly checking
enabled actor, current role/archive state, standing policy revision, exact input
identities, privacy fences, retention, conflicts and existing work. A provisional
selection or epoch is never publication authority.

Discovery is bounded and fair across partitions/candidates, including unavailable
prefixes. Persist content-free scan progress where needed. Statement timeouts
defer unresolved candidates; they must not produce successful empty coverage,
retirement, rejection or omission judgments. Preserve complete supported inputs,
whole-assertion page budgets, digest settlement, idempotency, human overrides and
the existing uncertain-provider recovery rules. No provider call occurs during
scheduling. Record lock wait and transaction age separately from native I/O.

## Native verification amendment — 2026-10-19

The user authorized replacing the time-windowed cache of verified native items
that let repeated populated reads skip verification. A cache bounded by 20 seconds
left damage unnoticed inside its window, which contradicts failing closed.
Interactive reads now verify the physical Neo4j projection on every read and keep
no verification result between reads. [ADR 0007](../adr/0007-canonical-graph-projections.md)
is unchanged: no content hash and no strict native version handshake is added.

**Per-read seal.** PostgreSQL already holds what a ready generation looks like
natively: the lease token of the import that sealed it (`native_attempt_token`,
equal to the generation's Neo4j `import_token`) and the node and relationship
counts verified before readiness. After the preparation snapshot is released and
before the final fair lock, a read sends one parameterized Query API request for
all its projections and passes only when each generation's token equals the stored
token, its import is closed and its membership and relationship counts equal the
stored counts. Counts use relationship-degree and indexed lookups on the
generation property (the adapter creates the indexes with its constraints), so
the request does not scan other generations or Brains. A combined read whose
generations hold more than one million relationships splits into several requests.

**Exact fallback.** Any seal difference falls back to the exact selective check:
the generation exists, every selected entity is a member, and every selected
relationship exists with its canonical endpoints, family and relation. That
passes after physical erasure of entities the read does not select, where only the
counts differ, and otherwise fails closed with `graph_import_mismatch`.

**Returned relationships.** Path, exploration and graph recall also verify exactly
the relationships the native search returned, because counts cannot show which
relationships a search walked. The check is bounded by the traversal limits.

**Deep verification.** Graph maintenance fully re-verifies at most one ready
generation per Brain per pass whose last native verification
(`native_verified_at`, set at readiness) is older than 24 hours, or was cleared by
a read that failed exact verification. It checks the complete descriptor and the
seal without a Brain lock across native I/O, and stamps only the exact sealed
import it verified: a generation replaced, re-imported or flagged by a read
meanwhile is not stamped. A native outage proves nothing and leaves the stamp.
A stamp is bookkeeping, not a canonical input, and does not advance the analytics
epoch.

**Damage.** A deep check mismatch marks that ready generation
`graph_import_mismatch`, visible in graph status. Ordinary discovery under the
current writer authority, the existing 500-job cap and its dedupe then queues one
replacement for that exact input; a queued, running, failed or newer replacement
suppresses further queuing, so a persistent fault cannot cause a rebuild storm.
Until the replacement is ready, reads of the damaged generation continue to fail
closed on any damaged selected content. A read can only ask for earlier
re-verification; it never queues work.

## Acceptance

- Actual PostgreSQL preparation observes a stable snapshot while a concurrent
  same-Brain writer commits; another Brain remains independent.
- Preparation transactions cannot write or take an exclusive Brain lock.
- Correction, scope membership, policy/fence changes, erasure, generation change,
  access/session/device loss and retention expiry reject stale prepared output.
- Actual Neo4j damaged/missing projection, native reachability and eligible-path
  controls retain their fail-closed behavior with zero model requests.
- Exercise populated overview and centered reads during capture/audits/rebuilds;
  record errors, lock wait and total latency. Synthetic success does not replace
  the recorded normal-installation timeout check.
- Against actual Neo4j, a warm generation read costs one Query API request;
  deleting a selected relationship or entity out of band fails the very next read;
  erasing an unselected entity does not; a traversal verifies the relationships it
  returned. A due generation is fully re-verified and stamped without advancing the
  analytics epoch, a changed sealing token is never stamped, and damage queues
  exactly one replacement however many passes follow.
- Pause actual digest/handover preparation while a same-Brain writer commits;
  race correction, access/policy changes, erasure and expiry against publication.
  Prove restart/wrap fairness and that timeouts cannot erase valid old coverage.

## Explicit Deferrals

Automatic semantic entities/topics and optional extractors have their own
successor contracts. General model gateway/answer transaction changes and
persistent hydrated-result caching are not introduced by this contract.
