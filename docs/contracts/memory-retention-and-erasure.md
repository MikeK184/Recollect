# Retention, controlled erasure and restore enforcement

Status: accepted

## Source

[Vision](../foundation/vision.md#capture-retention-and-model-policy),
[engineering principles](../foundation/engineering-principles.md#minimize-captured-source-and-credentials),
[canonical memory](../adr/0005-canonical-claims-and-time.md),
[review](memory-review-and-corrections.md), [evidence](evidence-collections.md),
and [publication](evidence-repository-publication.md).
The full-product goal authorizes these routine implementation details. UUID/time
metadata and existing conditional edits are used without content hashes or strict
product-format versions.

## Contract

### Retention classes and policy

Each Brain has an auditable admin-managed policy. Durations use server capture time,
not an asserted observation date. The current policy governs all unerased records;
shortening it can make existing records immediately due. An extended duration never
restores removed bytes. Days are whole UTC periods of 86,400 seconds.

| Class | Default | Override |
| --- | --- | --- |
| Sanitized raw session records | 30 days | 1–3,650 days |
| Sanitized raw tool output | 30 days | 1–3,650 days |
| Explicit documents | Until erased | 1–3,650 days or until erased |
| Permitted supporting excerpts | Until erased | 1–3,650 days or until erased; separate allow flag |
| Repository snapshots/artifacts | Until erased | 1–3,650 days or until erased |
| Claims/decisions | Until erased | 1–3,650 days or until erased |
| Review decisions and assertion rules | Until associated content is explicitly erased | No automatic rule expiry |
| Detailed mutation audit | 365 days | 1–3,650 days |
| Command receipt payloads | Existing 24-hour window | Invalidated earlier by affected erasure/expiry |
| Product-managed backups | 7 days | 1–365 days; backup adapter enforces it |

The [recovery adapter](operations-recovery-drills.md) caps each whole-installation
archive at the shortest included Brain window when captured; later shortening
can expire it sooner. An extension applies to new archives and never restores
pruned files. This bounded copy lifetime differs from canonical record retention.

Existing sources are explicit documents. New source identities have an immutable
class: `document`, `raw_session`, `tool_output`, or `support_excerpt`. Ordinary source
versions inherit that class. Raw intake uses the existing bounded, sanitized source
boundary; automatic host capture and its event attribution are delivered separately.
Changing a class to evade expiry is denied. Capture permission still differs from
provider transmission permission.

Supporting excerpts are explicit permitted copies, at most 8 KiB, selected from exact
retained source lines. They retain source identity, version, line range and capture time.
The server creates their content from that source; callers cannot mislabel arbitrary
text as an excerpt. Their own retention applies after ordinary raw-source expiry.
Explicit erasure of the source also erases these copies and their dependent records.
Neither expiry nor excerpt creation silently copies an entire raw record into a durable
attachment. Durable structured claims have their separately declared retention policy.

Due content is unavailable to ordinary reads, workers and later retrieval/model adapters
at its deadline, even before physical cleanup completes. Expiry removes source/artifact
bytes and rebuildable copies. Claims retained by their own policy can remain qualified
when their sole support expires; they cannot become strict accepted context without
available permitted evidence. An independently retained excerpt may still support them.

### Erasure targets and dependency closure

Browser Brain administrators can explicitly erase a source (all its versions), claim
(all its revisions), repository snapshot, or revision manifest (all its revisions).
Collection erasure explicitly selects all associated sources; ordinary removal of a
collection or association continues to preserve shared sources. The preview names that
shared-source effect. Account/Brain deletion is not implied by these memory operations.
Archived Brains permit erasure and cleanup while continuing to deny ordinary new writes.
Device tokens cannot initiate administrative privacy erasure.

An authorized preview computes exact target identities, dependent revisions, artifact
IDs, affected pending work, and counts of retained independent records. Confirmation
supplies the preview's current Brain eligibility counter plus the same target; changed
inputs require a fresh preview. This is a conditional edit, not a content fingerprint.
No free-form deletion reason or sensitive content is copied into the deletion ledger.

Explicit erasure clears controlled source text/metadata, excerpts, chunks, repository
artifacts/facts/inventory, affected claim revisions, review payloads/rules and derived
representations. Dependency checks follow exact support and derivation identities.
A revision with any erased support is erased; another revision supported exclusively
by independently retained inputs is evaluated separately. The mere presence of a second
support on the same affected revision does not establish independent derivation.

Erasing the current revision does not promote an older one. Minimal identity/time
tombstones preserve the distinction between no record and an erased historical interval.
They retain no subject, value, source title, URI, file path, excerpt, review reason or
other controlled content. Historical selection first chooses the revision for that time,
then checks erasure; it never skips an erased interval and presents an older assertion
as current for that interval. Review decisions touching erased inputs lose their payload;
permitted independent current decisions remain separately attributable.
The unscoped historical claim list includes content-free removal markers so remaining
independent history stays reachable. Removed applicability cannot be used to fabricate
matches for repository or environment filters.

Applicable rejected-value rules are themselves controlled content. Explicit erasure
removes affected values/support from those rules instead of retaining sensitive text
indefinitely. Minimal deletion identities remain separate from assertion rejection.
Replay fences preserve source/version/claim/snapshot/manifest IDs and, for publication,
repository UUID plus exact commit. New adapter or publication IDs cannot recreate an
erased snapshot of that commit. Without content fingerprints, arbitrary future manual
uploads under unrelated identities are not claimed to be recognized as the same content.

Recent command receipts in an affected Brain may contain complete controlled inputs
and responses. Erasure/expiry invalidates their payloads conservatively while retaining
the command key and a content-free invalidated result for their original receipt window.
An old key cannot silently execute again after its response has been removed. The UI
directs the caller to inspect current state before starting a new command.

### Mutation, physical cleanup and concurrency

Privacy mutation uses the Brain write lock and a canonical request. Marking erased
identities, clearing database content/copies, invalidating receipts and eligibility,
fencing affected queued/running jobs, and minimal audit commit together. Jobs retain no
erased payload; stale leases cannot publish. Artifact files are deleted by their recorded
Brain/UUID paths, never by arbitrary caller paths. No customer checkout or unmanaged file
is changed. An unrelated source or snapshot is preserved.

Database and filesystem deletion are not one transaction. A durable cleanup queue records
opaque artifact IDs and pending/complete/error state. Canonical reads stop exposing content
before physical removal finishes. Failed or interrupted cleanup is retryable without
resurrecting database content. A request is central-complete only after controlled files
and canonical representations are reconciled. File absence is success; inaccessible
storage remains visible pending work. Expiry uses the same cleanup mechanism.

An already-authorized privacy request continues independently of its initiating user's
later logout or revocation. System cleanup may consume only canonical committed privacy
requests; an ordinary foreign principal cannot turn this into a cross-Brain mutation.
The existing worker runs bounded privacy/expiry passes as well as its work lanes.
The operator also has a one-pass maintenance command for deterministic recovery/tests.

### Durable deletion journal and restore barrier

Completed acceptance of an erasure requires its minimal request/closure to be synchronized
to the configured erasure journal outside the database backup. It contains installation,
Brain and request UUIDs, capture time, target/dependent UUIDs, publication commit fences
and cleanup state references; no controlled values, paths, reasons or raw bodies.
Initialization belongs to migration/setup, never an implicit fallback on missing storage.
When the optional remote recovery mirror is configured, acceptance also requires
durable encrypted acknowledgement there. Mirror failure preserves local denial
and pending erasure status. Local-only installations retain their explicitly
reported local journal boundary. See the [recovery contract](operations-recovery-drills.md).
Optional closure arrays may be omitted when empty. Journal replay compares those
equivalent forms while preserving the exact non-empty identities and installation
boundary; it must not reject an otherwise identical committed deletion on restart.

If journal export fails after the database commit, the request remains pending and
cannot be reported complete. Its database tombstones still block content; maintenance
retries the export. Serve/worker startup compares the retained journal and canonical
installation state, replays newer deletion entries and refuses content service while
reconciliation or journal availability is unresolved. A restored older database must use
the latest retained journal before serving content. A missing journal for an initialized
installation is an error, not a fresh empty ledger.

Keep the journal with the live installation and separately from rotating backups.
Backup expiry is an explicit policy boundary; deletion from external or uncontrolled
copies is not promised. The later operations adapter must enforce the configured backup
window and demonstrate full backup/restore. This slice proves the replay barrier against
an older database fixture and retained artifact copies.

### Local companion copies

The companion synchronizes the Brain's policy and minimal deletion entries when connected,
and applies known expiry while offline. Its cleanup is limited to product-managed prepared
publication bundles and later capture inbox records for that Brain. It never edits Git
objects or customer working trees. Publication/resume checks current deletion fences before
upload and removes affected owned prepared bundles. The explicit maintenance command can
apply the same cleanup and report its last synchronized deletion position.

Central completion is separate from offline-device acknowledgement. The API/UI exposes
that local copies require device check-in or their previously known local expiry; revoked
or offline devices cannot be reported erased merely because server cleanup succeeded.
Automatic host inbox capture implements the same policy in its owning successor slice.

### Interfaces and visible states

- `GET|PUT /api/brains/{brain}/retention`: classes, policy, current change ID and policy update.
- `POST /api/brains/{brain}/excerpts`: source/version, first/last line and destination title;
  returns an ordinary independently retained source with excerpt provenance.
- `POST /api/brains/{brain}/erasures/preview`: target kind/ID and current exact closure/counts.
- `POST /api/brains/{brain}/erasures`: target plus preview counter; optional idempotency key.
- `GET /api/brains/{brain}/erasures`: bounded metadata/status/deletion-position pages.
- `POST /api/brains/{brain}/erasures/{request}/retry`: retry already-authorized cleanup.
- Companion policy/journal sync and operator maintenance/reconcile commands use the same rules.

Readers may inspect policy and permitted minimal status. Only admins change policy or
initiate erasure; permitted writers may explicitly retain excerpts. All resource IDs and
current grants remain Brain-scoped. Preserve loading, empty, expired, erased, pending,
failed storage, stale preview, no access and independently retained states in the UI.
An unavailable source is never fabricated as an empty successful extraction.

## Acceptance

- Defaults/overrides apply by class and deadline, with retained-excerpt and unrelated
  document controls. Expiry of sole support qualifies retained claims without preserving
  forbidden full raw content. Shortening policy immediately denies due reads/work.
- Explicit source/claim/snapshot/manifest erasure clears controlled payloads, associated
  rules/receipts, artifacts and dependent representations; independent records survive.
  Historical gaps cannot become older accepted assertions. Customer files are untouched.
- Concurrent queued/running work, stale preview, interrupted cleanup and retries cannot
  resurrect erased content. Lost or inaccessible storage produces accurate progress.
- Latest deletion journal applied to an older database/artifact fixture prevents content
  serving until reconciliation. Missing journal fails closed; backups remain a stated limit.
- Local prepared-bundle cleanup uses exact Brain/identity fences with unrelated-profile
  controls, and distinguishes acknowledgement from offline/unknown copies.
- API/RLS/real browser positive and negative proof, generated client, local service calls,
  format/Clippy/build and `./scripts/validate.sh`; no production/customer erasure as a test.

## Explicit Deferrals

Automatic host event capture, model/vector/graph consumers and full backup adapters are
delivered by their named dependent slices and cannot claim erasure support without their
own proof. Account/Brain deletion and external-source writeback are distinct operations.
This slice delivers the current canonical, artifact, receipt, worker, journal and prepared
publication obligations rather than treating future consumers as already implemented.
