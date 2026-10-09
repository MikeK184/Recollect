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

## Explicit Deferrals

Automatic semantic entities/topics and optional extractors have their own
successor contracts. General model gateway/answer transaction changes and
persistent hydrated-result caching are not introduced by this contract.
