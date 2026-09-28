# Collections and retained source evidence

Status: accepted

## Source

[Evidence epic](../roadmap/epics/evidence-and-workspaces.md),
[vision](../foundation/vision.md#brain-and-scope),
[storage baseline](../foundation/techstack.md#storage-ownership-and-hybrid-retrieval),
[durable commands](platform-durable-work.md) and
[runtime ADR](../adr/0003-product-runtime.md). The current goal authorizes routine
implementation details and append-only UUID source versions without hashes or
strict version protocols.

## Contract

### Identities and organization

A source has a Brain-scoped UUID and an ordered history of immutable versions.
Each version records title, media type, optional source URI and observation time,
contributor principal/device, capture time, retained artifact identity and byte
length. Observation time may be unknown; capture time is not observation or
deployment time. The Brain owns the resource; its contributor does not acquire
ownership. An update appends a version and changes the current pointer atomically.
The optional `base_version` detects a stale editor with 409. Historical versions
remain readable under current Brain access; no external source is overwritten.

Collections, areas and environments have stable UUIDs, a fixed kind, name and
description. Names are unique ignoring case within Brain/kind. These overlapping
views associate existing source IDs; they do not copy sources. Filters intersect
the selected collection, area and environment. Removing an association or a
group preserves source/version/artifact data and other associations. A source
with no groups remains in All sources. Erasure is a separate memory-lifecycle
operation and is not implied by collection removal.

### Capture and artifacts

The first import supports UTF-8 plain text, Markdown, JSON, YAML and TOML supplied
as text or a browser file. Text is retained exactly, without rendering uploaded
HTML or executing document contents. Binary/PDF extraction and remote connectors
are outside this adapter. References may be retained without text; a reference
does not prove remote availability and is never fetched automatically.

Each import explicitly chooses retained content or reference-only metadata. The
Brain's `allow_document_content` policy defaults true for explicit user imports;
only an admin may change it. Content requires both that policy and
`retain_content=true`. Disabling future capture preserves existing permitted
versions; it does not constitute erasure. A reference-only version requires a
nonempty source URI and stores no supplied content. No model call occurs.

Bound content to 1 MiB, complete JSON upload to 2 MiB, title/group name to 120
characters, description/URI to 2,000 characters and associations to 100 per
source. Reject NUL/binary input and control characters in metadata. Reject URI
user credentials. Up to 250 groups of each kind may exist per Brain. List sources
in pages of 50 and version history in pages of 20. Blank text is invalid.

Store artifacts under `RECOLLECT_ARTIFACT_DIR`, default `.data/artifacts`, using
Brain/UUID paths created by the service. Artifact bytes are written with exclusive
creation, synchronized before canonical commit and never overwritten. Only an
authorized API reads them; the directory is not a static web root. Missing or
unreadable artifacts produce explicit availability/processing states. A UUID or
matching byte length does not claim cryptographic integrity.

Metadata, audit and queued processing commit together through the shared command
boundary. File and database commits cannot be atomic: a failed/crashed database
commit can leave an unreferenced file, which is never served or treated as an
accepted source. Do not blindly delete such files. Recovery/erasure maintenance
will reconcile controlled artifacts. Optional idempotency keys retain normalized
request input for the existing 24-hour receipt window; this controlled copy follows
the same content policy and must be included in later erasure.
Reject configured service credential values and recognizable private-key material
before storing an import or receipt. This is a bounded detector, not a guarantee
that arbitrary text contains no unknown secret; permitted document selection remains
explicit. Session/connector sanitization is owned by the capture slices.

### Processing, authority and interfaces

Readers may browse permitted source metadata/history/content. Writers/admins may
import, update, organize, remove associations/groups and request reprocessing.
Changing capture policy requires admin. Brain, source and group IDs must agree;
foreign IDs return 404. Archived Brains reject new evidence mutations with 409.
Per-request transaction/device authority, current grants, CSRF and audit apply.
An environment with retained revision-manifest history cannot be removed by
ordinary group deletion; the [publication contract](evidence-repository-publication.md)
requires an explicit conflict and preserves that history for the erasure lifecycle.
Receipt replay requires current write access; Brain-admin commands retain their
stricter entry checks.
An access-checked database helper may lock a visible Brain row without granting
writers permission to update Brain administration columns. Readers/writers and
revocation share this lock boundary; direct Brain metadata updates remain admin-only.

Every retained version queues `source.process` on the bounded capture lane. A
worker rechecks actor/device/Brain authority and lease, then reads the exact
immutable version. It produces deterministic UTF-8 chunks at most 4,096 bytes,
with half-open byte offsets and one-based inclusive line spans. Chunks are
rebuildable representations with one version identity; processing does not claim
model extraction, semantic acceptance or retrieval delivery. Old-version work
cannot overwrite a newer current pointer. Replay replaces only that version's
chunks in the same fenced transaction as completion/audit.

Show queued/running/failed/cancelled/ready, reference-only and missing-source
states from actual canonical, job and artifact evidence. A missing artifact
never becomes an invented empty document. Reprocessing rereads retained input;
restoring an absent artifact or importing a new version is explicit. Current
source content is readable before chunk processing finishes.

The API uses these Brain-scoped paths:

- `GET /api/brains/{brain}/evidence`: policy, groups and paginated source summaries;
  optional collection, area, environment and offset filters.
- `PUT /api/brains/{brain}/evidence/policy`: document capture policy.
- `POST /api/brains/{brain}/evidence/groups` and
  `PATCH|DELETE /api/brains/{brain}/evidence/groups/{group}`: organization metadata.
- `POST /api/brains/{brain}/sources`: import the first version and associations.
- `GET /api/brains/{brain}/sources/{source}/versions`: paginated version metadata.
- `POST /api/brains/{brain}/sources/{source}/versions`: append an attributable version.
- `GET /api/brains/{brain}/sources/{source}/versions/{version}`: exact content,
  availability and support spans under current access.
- `PUT /api/brains/{brain}/sources/{source}/groups`: replace validated associations.
- `POST /api/brains/{brain}/sources/{source}/process`: reprocess the current version.

The Brain UI supplies browsing, intersecting filters, group management,
import/file/reference flows, editing/history/provenance, organization/removal,
capture policy and reprocessing with loading/empty/error/access states. Render
source text as data, never executable markup. Refresh after mutations and hide
previously loaded content when access is lost.
For focused local processing, `recollect-server worker-once capture` performs
one capture-lane pass; omitting the lane retains the interactive default.

## Acceptance

Desktop list amendment, 2026-09-26: the
[desktop contract](desktop-experience.md#literal-list-search) adds optional source
`q`, trimmed to at most 200 UTF-8 bytes, for case-insensitive literal containment
over the permitted current title before pagination. It does not search retained
bodies, call a model or change exact-version/retention rules. Empty q preserves
the prior list and ordinary wildcard characters are literal.

Exercise actual handlers, PostgreSQL RLS and filesystem artifacts: writer/reader/
foreign isolation with positive controls; shared membership removal; immutable
updates and stale-editor/replay behavior; actor/device audit; denied capture and
failed storage atomicity; missing-source recovery; fenced old-version processing
and revoked worker denial. Browser proof covers import, history, overlapping
views, collection removal and error recovery. Run Rust/frontend checks and
`./scripts/validate.sh`; record dependency and local runtime evidence.

## Explicit Deferrals

Repository extraction/publication, workspace binding, session capture, semantic
claims/review/erasure, search and graph projections belong to named successors.
No remote document connector, PDF parser, model provider or external writeback
is implied by this text/reference import adapter.
