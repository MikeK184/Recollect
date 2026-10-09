# 0020: Automatic source-backed knowledge organization in Rust

Status: accepted

## Decision

The user-authorized dedicated automatic-mapping plan adds persisted semantic
mentions, scoped entities, reversible aliases and overlapping automatic topics.
Implement the canonical pipeline, validation, identities, reconciliation,
invalidation, APIs and worker in Rust. PostgreSQL owns these records and user
overrides; Neo4j/GDS computes separately qualified navigation and grouping.
Existing manual collections/areas/environments remain independent.

Reuse eligible structural metadata, current supported claim fields, existing
learning interpretation and existing vectors before requesting more inference.
Extend the existing policy-governed learning envelope with optional source-span
candidates. A separately selected local extractor may supply the same bounded
candidate format; no mandatory Python/Cognee/Graphify/Graphiti runtime is added.
The raw GLiNER development result failed quality gates and is not selected as
unconditionally trusted production extraction. Provider candidates never write
entities, aliases, topics or native graph nodes directly.

Aliases are supported, reversible associations between original identities;
do not destructively merge their supporting mentions. Unknown instance scope
does not match a known scope. Broad technology/concept identities and actual
service/person/repository instances remain distinct. Topic identity is persisted
independently of GDS's report-local community number.

Extend canonical qualification, derivative erasure/restore replay and typed
navigation for semantic records before exposing them. Topics explain source
organization; mentions and inferred relationships do not become accepted claims
or operational truth. No source content is disclosed merely because a native
graph contains its key.

## Why

The existing graph primarily records provenance and repository structure. Its
GDS provenance communities and developer CodeGraph/Graft are not automatic
organization of conversations/documents. Reusing the manual source-organize
endpoint would erase human choices. The dedicated plan requires automatic,
inspectable organization with source evidence and incremental invalidation.

Model confidence alone failed the local precision gate. Rust must validate exact
immutable input identity, UTF-8 spans, endpoint references and canonical scope;
fixture/model quality and usefulness remain separately measured. Keeping optional
extractors outside canonical mutation preserves the accepted independent Rust
baseline and existing provider/permission/budget controls.

## Consequences

The [organization contract](../contracts/automatic-knowledge-organization.md)
governs the full product slice. Its worker prepares outside long Brain locks and
publishes under current authority, input epoch, lease and deadline checks. The
concurrent graph-read prerequisite retains its own populated/normal-runtime gate.
Offline validation and candidate implementation can proceed independently; do
not release automatic graph expansion before those performance gates pass.

No benchmark improvement follows from this decision. Measure extractor quality,
topic coverage/grouping, stability, deletion, exact scope and interactive use.
Optional code-producer adoption requires an equal-input coverage comparison.

## Supersession

Extends [ADR 0007](0007-canonical-graph-projections.md) with typed semantic
organization. Existing structural/provenance analytics remain unchanged; the
new semantic grouping recipe is separate. Rust remains the product runtime.
