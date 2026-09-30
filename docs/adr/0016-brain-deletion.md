# 0016: Irreversible Brain deletion through the canonical erasure path

Status: accepted

## Decision

A Brain can be permanently deleted by an authorized human administrator.
`DELETE /api/brains/{brain}` removes the Brain and every record, projection,
artifact and grant derived from it, composing the existing canonical erasure
machinery rather than adding a parallel deletion path.

Deletion is the terminal member of the existing removal vocabulary and does not
rename or absorb the others. Archive stays the reversible, content-preserving
action; Withdraw keeps qualified history for one assertion; Erase removes one
record and its dependents; Delete removes the whole Brain. No action silently
substitutes for another, and archive remains available and recommended before
deletion.

The operation is composed from already-accepted behavior: the shared mutation
policy and audit boundary, the dependency-closure and physical-cleanup queue in
[memory retention and erasure](../contracts/memory-retention-and-erasure.md),
the durable deletion journal and restore barrier in
[ADR 0013](0013-backup-and-recovery.md), and per-Brain graph generations in
[ADR 0007](0007-canonical-graph-projections.md). Deleting a Brain is therefore
a scope question, not a new erasure algorithm: the target closure is every
identity inside that Brain.

Preserved across deletion:

- Minimal content-free tombstone and journal entries, so a restored older
  database cannot resurrect the Brain or its content, and so a newly created
  Brain with the same name receives a distinct identity.
- Minimal mutation-audit metadata: actor, time, Brain identity and disposition.
  No subject, value, title, path, excerpt, reason or other controlled content
  survives in audit, receipts or the journal.
- Global device identity. Paired devices belong to the account, not the Brain,
  so deletion removes only that Brain's grants and bindings for them and never
  revokes a device credential.
- All other Brains, their content and their admission keys.

Explicitly outside this operation: Vault token or lease revocation. Deletion
releases our own sessions, leases and connection records under the existing
[runtime contract](../contracts/mcp-runtime-and-credentials.md) lifecycle and
performs no credential revocation, which remains a separate operator action
and a standing user prohibition.

## Why

The user decided on 2026-09-29 that deleting a Brain must "simply delete all
related information, erase it", rejecting a discard-only-when-empty design.

The gap is real and visible in the product. No `DELETE` route exists for a
Brain; [platform bootstrap](../contracts/platform-bootstrap.md) offers only
rename/archive/reopen, and
[retention and erasure](../contracts/memory-retention-and-erasure.md) defers
"Account/Brain deletion" as a distinct operation with no owner. The live
installation carries fifteen Brains, nine of which are dated test fixtures
(`Plugin live-token proof` six times, `schema probe` twice, `test`) that cannot
be removed and cannot be distinguished from real work by any control the user
has. Brains is the first screen a person sees, so unrecoverable clutter is also
the first impression.

Alternatives considered and rejected: archive-only, which preserves content and
therefore cannot reclaim storage or answer the user's request; a soft delete
with an undo window, which reintroduces exactly the resurrection path the
journal barrier exists to prevent; and per-record erasure as the only route,
which cannot remove the Brain row, its policy, its grants or its graph
generation. A separate physical deletion path was rejected because it would
bypass the single mutation policy that
[engineering principles](../foundation/engineering-principles.md#route-every-mutation-through-one-policy)
require every mutation to use.

## Consequences

Storage reclamation becomes possible for the first time, and the artifact,
semantic-vector, graph-generation and analytics cleanup queues gain a
Brain-wide caller. The existing preview/counter/idempotency protocol extends to
a new target kind rather than inventing its own confirmation shape.

Because database and filesystem removal are not one transaction, a deleted
Brain must present an accurate intermediate state: canonical reads stop
exposing it immediately, while physical cleanup can remain pending, retryable
and visible. Completeness claims must distinguish server cleanup from offline
companion copies, exactly as per-record erasure already does.

Product acceptance widens. [Operations recovery drills](../contracts/operations-recovery-drills.md)
must prove the deleted Brain stays absent after restore from an older backup,
and the seven-capability matrix gains a negative evaluation: a deleted Brain
must not appear in any listing, retrieval, graph, audit or MCP path.

Backup copies remain a stated limit, unchanged from the accepted erasure
boundary: the configured product-managed backup window can still contain the
Brain until it expires, and deletion from external or uncontrolled copies is
not promised. Any user-facing confirmation must say so rather than implying
absolute removal.

## Supersession

Resolves the explicit deferral in
[retention and erasure](../contracts/memory-retention-and-erasure.md#explicit-deferrals)
that named Account/Brain deletion a distinct operation. It does not change
Withdraw, Erase, archive, per-record closure, retention classes or the deletion
journal format; it adds a Brain-wide target to the machinery those documents
already define. No accepted ADR is superseded.