# Irreversible Brain deletion

Status: accepted

## Source

[ADR 0016](../adr/0016-brain-deletion.md) records the decision. The user
explicitly chose total erasure on 2026-09-29. Behavior composes
[retention and erasure](memory-retention-and-erasure.md),
[durable work](platform-durable-work.md),
[bootstrap identity and grants](platform-bootstrap.md),
[team access](platform-team-access.md),
[device pairing](platform-device-pairing.md),
[graph projection](graph-projection-and-traversal.md),
[MCP runtime](mcp-runtime-and-credentials.md) and
[recovery](operations-recovery-drills.md). [Vision: capture, retention and model
policy](../foundation/vision.md#capture-retention-and-model-policy) requires
Erase to remove controlled content and dependent copies while leaving only
permitted minimal audit metadata.

## Contract

### Authority and intent

`DELETE /api/brains/{brain}` deletes one Brain and everything derived from it.
It is browser-administrator and installation-owner only. Device tokens, plugin
credentials and MCP tool callers cannot initiate it. Readers and contributors
are denied. Archived Brains can be deleted. The command is audited, idempotent
through the existing key protocol, and never performs a cross-Brain mutation.

Deleting a Brain is distinct from archive, reopen, rename, Withdraw and
per-record Erase. No endpoint aliases another action as deletion, and archive
never schedules a later delete.

### Preview and confirmation

`POST /api/brains/{brain}/deletions/preview` returns the exact target Brain
identity and name, the current closure counter, dependent identity counts by
class, pending work that will be fenced, retained artifact and backup copies
that physical cleanup must remove, and the copy-lifetime limitation. It returns
no controlled content, titles, paths or excerpts.

`DELETE` requires the preview's current closure counter plus an explicit
confirmation string equal to the Brain name. A changed counter, mismatched
confirmation or unknown Brain rejects the request and requires a fresh preview.
The confirmation prevents accidental selection of the wrong Brain; it is not an
undo mechanism, and no undo exists after commit. The response states the backup
window limitation verbatim rather than claiming absolute removal.

### Canonical closure

One accepted request commits atomically through the shared mutation boundary:
Brain row, admission key, model/capture/retention/repository policies, member
and group grants, execution-profile grants, connection and profile records,
private-runner registrations, capture bindings and device reports, sources and
versions, collection and area associations, environment manifests, repository
snapshot identities, claims and revisions, review decisions, rejected-value
rules, handovers and inputs, procedures, session and tool-output records,
supporting excerpts, semantic representations, analytics reports and epochs,
graph generation records, command receipts, and detailed mutation audit rows.

Each class is removed by its existing closure rules rather than a bespoke
statement. Dependent representations follow exact support and derivation
identities. Receipts in this Brain are invalidated conservatively, keeping the
command key and a content-free invalidated result for the original window.
Grants for this Brain are removed from every device and account; device rows,
account rows, sessions and credentials in other Brains are untouched. No Vault
token or lease is revoked; our sessions, leases and instance records are
released through the existing lifecycle only.

Minimal tombstones persist: Brain identity, deletion time, initiating actor,
disposition and content-free journal entries. Tombstones retain no name,
description, subject, value, title, URI, path, excerpt or reason. A recreated
Brain with an identical name receives a new identity and cannot inherit the
tombstone's history or grants.

### Read and recall denial

After commit and before physical cleanup finishes, the Brain is absent from
Brain listings, per-Brain reads, recall, Ask, semantic and graph search, graph
traversal and analytics, MCP memory and workspace tools, catalogue and profile
enumeration, capture ingestion, publication, Activity feeds, audit browsing and
aggregate counts. Requests naming the deleted Brain fail as unknown rather than
forbidden, so existence is not leaked to a principal that lost access. Queued
and running work for that Brain is fenced; stale leases cannot publish and jobs
retain no payload.

### Physical cleanup

Database and filesystem removal remain separate transactions. Artifact files,
prepared publication bundles, semantic vectors, Neo4j per-Brain generations and
analytics scratch are removed by recorded Brain-scoped paths and identities,
never by caller-supplied paths. The existing cleanup queue carries pending,
complete and error states, is retryable, and resurrects no database content.
File absence counts as success; inaccessible storage stays visible pending work
and is reported as pending rather than complete.

Central completion is separate from offline companion acknowledgement. A paired
device that has not checked in may still hold prepared bundles or inbox events
for the deleted Brain; it applies the deletion fence on next connection and
cannot be reported erased while offline. The companion never edits Git objects
or customer working trees, and removes only product-owned local files for that
Brain.

### Restore barrier

The deletion journal is synchronized outside the database backup before the
request is reported complete, following the existing export-failure rule. A
restored older database must replay retained journal entries and must not serve
the deleted Brain, its content, its grants or its graph generation. A missing
journal for an initialized installation remains an error, not an empty ledger.
Replay compares optional closure arrays equivalently without rejecting an
otherwise identical committed deletion on restart.

### Interface and states

- `POST /api/brains/{brain}/deletions/preview` returns the closure counter,
  per-class dependent counts and the copy-lifetime limitation.
- `DELETE /api/brains/{brain}` accepts the counter, confirmation string and an
  optional idempotency key; returns the deletion request identity and status.
- `GET /api/brains/{brain}/deletions/{request}` returns bounded status, cleanup
  progress and pending physical work for an authorized caller. Tombstones are
  not listed as Brains.
- The Settings General view exposes Delete only for an authorized administrator,
  separated from archive, and requires the preview and confirmation flow.

Preserve loading, empty, pending, partially cleaned, failed storage, no-access
and already-absent states. An error never reports success. Offline companion
copies, backup windows and uncontrolled external copies stay visibly incomplete.

## Acceptance

- Owner and Brain administrator delete an active and an archived Brain through
  the real API and browser. Readers, contributors, device tokens and MCP callers
  are denied with no mutation.
- Preview counts match actual dependent records for a Brain holding sources,
  versions, claims, revisions, review decisions, rejected rules, manifests,
  snapshots, captures, connections, profiles, grants, policies and graph
  generations. Wrong confirmation, stale counter and unknown Brain reject before
  any write.
- After commit the deleted Brain is absent from every listed read and recall
  path, returns unknown rather than forbidden, and leaves other Brains
  byte-identical in a parsed before/after inventory.
- Device rows, account rows and credentials survive with only the deleted
  Brain's grants removed. No Vault token or lease revocation call occurs,
  asserted by request capture.
- Interrupted physical cleanup resumes through the existing retry path, exposes
  pending state accurately and never resurrects database content. Absent files
  report success; inaccessible storage reports pending.
- Restoring an older database and artifact fixture with the retained journal
  does not serve the deleted Brain. Missing journal fails closed. Companion
  offline copies remain unacknowledged until check-in, then apply the fence
  without touching Git objects.
- Tombstone and audit rows contain no controlled content, verified against a
  fixture whose source title, claim value and file path must not appear in any
  retained row, journal entry, receipt or log.
- Focused Rust integration and RLS tests, generated client, web typecheck,
  affected browser specs, `./scripts/validate.sh` and `git diff --check`.

## Explicit Deferrals

Account deletion and bulk multi-Brain deletion are separate operations with no
route in this contract. Restoring a deleted Brain, an undo window and a recycle
bin are excluded by decision. Removal of content from uncontrolled external
copies, third-party backups or provider-side data is not promised. Rotating or
revoking Vault credentials remains operator work. Companion local-path cleanup
beyond product-owned bundles and prepared publication artifacts is out of scope.