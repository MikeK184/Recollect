# 0005: Canonical claims and two time dimensions

Status: accepted

## Decision

Use PostgreSQL claim identities with append-only knowledge revisions. Each revision
retains a structured assertion, exact evidence references, applicability, independent
trust states, actual author/device/operation and a server-assigned knowledge time.
The current pointer is mutable only through the shared authorized command boundary.
UUID base-revision checks prevent stale edits; no content hashing or strict format
version gate is introduced.

Represent fact validity as unknown, a point observation with explicit precision,
or a half-open interval with independently unknown endpoints. Never infer validity
from a commit date or knowledge time. Historical reads independently select fact
time and knowledge time while always enforcing current access and availability.

Centralize canonical eligibility in the memory module. Other retrieval/projection
adapters will consume it; none can promote a proposed claim by ranking it. Manual
authoring starts proposed and does not manufacture human review. Review/correction
commands and policy-based acceptance enter in their named successor slices.

## Why

The accepted [vision](../foundation/vision.md#revisions-and-evidence) and
[principles](../foundation/engineering-principles.md#keep-trust-and-time-precise)
require late evidence, independent trust states and attributable decisions. The
user authorized resolving routine implementation details across all product slices.
Exact evidence and manifest identities are already delivered by
[repository publication](../contracts/evidence-repository-publication.md).
One mutable row would lose historical knowledge; a Git revision alone cannot answer
when a fact applied. PostgreSQL already supplies the transaction and access boundary.

## Consequences

The [claims contract](../contracts/memory-claims-and-time.md) defines bounded
manual authoring, temporal queries and evidence inspection. Appending a revision,
mutation audit and refresh work commits atomically. Read-time evidence reassessment
is scoped to the selected source versions and relevant manifest entries; moving
development cannot stale an independently selected production revision.
Missing source bytes remain a visible limitation. No model or remote source fetch
is needed for this slice. The selected Luna/embedding-large pair is retained for
the policy gateway, which follows retention and erasure.

## Supersession

N/A: new decision under the accepted independent Rust product foundation.
