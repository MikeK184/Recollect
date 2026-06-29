# Encrypted backup, upgrade and recovery drills

Status: accepted

## Source

[ADR 0013](../adr/0013-backup-and-recovery.md),
[installation](operations-local-and-shared.md),
[retention/erasure](memory-retention-and-erasure.md),
[review rules](memory-review-and-corrections.md),
[managed observations](mcp-observation-capture.md),
[graph projections](graph-projection-and-traversal.md),
[analytics](graph-analytics.md) and the
[owning epic](../roadmap/epics/operational-readiness.md).

## Contract

### Ownership and configuration

Extend the repository's Python installation/operator tooling for named personal
and shared installations. Commands select one saved installation, validate its
Compose project/volumes and use its explicit credentials. They never select a
customer machine, reuse ambient Vault authentication, reset a volume or overwrite
another installation. Native development can continue independently.

An ignored private recovery configuration records public age recipients, a local
owned backup directory and optional SFTP host/user/port/root, identity-file and
known-hosts-file references. Secret values never enter arguments, manifests,
diagnostics or repository configuration. Noninteractive SFTP uses explicit host
verification and disables inherited proxies, forwarding and arbitrary SSH config.
Each remote installation has a fixed UUID directory and ownership marker. Writes,
listing and pruning stay within that directory; no remote shell commands are needed.
For source-machine loss, a fresh saved destination can configure these references
using the known source installation UUID and verify its existing remote owner,
then fetch the encrypted archive/current journals without contacting the lost DB.

Use age's existing authenticated encryption format and PostgreSQL 17's installed
`pg_dump`/`pg_restore`; record tool/image and applied-migration observations. Use
UUID identities and timestamps for backup records, not new content hashes or
strict application-format versions. An available configured tool is distinct from
an actual successful encrypt/transfer/restore.

### Consistent backup and retention

The backup operation checks space and destination prerequisites before stopping
the selected API/worker/proxy, then drains them using the installation's existing
bounded shutdown. Database services and unrelated installations stay running.
The application roles must be stopped before copying mutable application files.
Run pending privacy maintenance/reconciliation, then capture a full custom-format
database dump, canonical artifact tree, account credential file and owned central
receipt store. Keep current deletion and analytical ownership journals separately;
they cannot be replaced by older bundled copies on restore.

Include the canonical checkpoint time/installation UUID, applied migrations,
database major version, artifact identities/lengths, journal sequence and exact
source-installation identity in the encrypted manifest. Provider values and
deployment credentials remain operator configuration; the procedure inventories
required references without exposing them or silently redirecting connections.
The restored owner/account credential file preserves existing account identity;
database/graph/origin settings belong to the destination installation.
Record whether deletion acknowledgements require the optional remote mirror.
Restore that requirement before preparation/release, using the explicitly supplied
current operator key/path references; never put the SSH identity in the archive
or silently downgrade a mirrored installation to local-only acknowledgements.
API, worker and operator roles must use the same mirror reference. The container
hostname may be overridden explicitly only with matching pinned host trust.

Private temporary files are removed after completion or a handled failure. A
retry removes abandoned restore staging only when its local ownership marker
matches the selected installation. Never
expose a partial archive as complete. Encrypt before transport; publish the final
remote file only after the entire upload and supported remote fsync/rename succeed.
Source application roles resume only if the backup operation itself stopped them,
with failure and resume outcomes reported separately. Interrupted work leaves a
content-free status record; retry cannot overwrite a completed unrelated archive.

A whole-installation archive uses the shortest included Brain backup duration,
initially seven days (one to 365). The deadline is capped by that admitted window;
current shorter policy can expire it sooner. Longer policy applies to new backups
and cannot recover deleted files. Pruning removes only owned expired complete or
abandoned partial archive identities; retain a content-free pruned status record.
Never prune the independent
current deletion journal. A remote prune failure stays visible and retries; no
claim is made about deleting external operator copies.

Provide creation, list/status, transfer/fetch and prune commands plus a repeatable
scheduled-operation entrypoint. Document a daily backup schedule and report latest
completed checkpoint age, local/remote completion, expiry and last safe error. An
unconfigured or failed schedule cannot claim the 24-hour recovery-point objective.

### Independent deletion durability

The existing local journal remains the canonical runtime interface. An optional
configured SFTP mirror extends `privacy_journal` export: encrypt the same minimal
entry, upload it under stable installation/sequence/request identities and confirm
remote durability before marking the canonical erasure request complete. Retry
publishes the same entry, never another privacy request. Failure preserves local
fences and pending status; capture/model work cannot see erased content meanwhile.
Initial remote setup synchronizes all acknowledged local entries and the existing
installation marker before declaring remote recovery ready.

Remote mirroring requires no decryption identity in API/worker processes and no
Vault token or lease revocation. Its supported subprocess environment and output
are bounded and credential-free. Ordinary local installs without a mirror retain
their existing local durability boundary and report that boundary explicitly.

Recovery requires the latest independently retained local journal or a verified
download/decryption of its remote replica. Refuse a missing, wrong-installation,
gapped or checkpoint-older journal. Never silently fall back to a journal copied
inside an old data backup. Retaining this latest authority is an operator storage
obligation; losing every current journal copy is a blocking recovery condition.
Analytical ownership reconciliation likewise consumes the separately retained
current journal before old scratch ownership can affect another attempt.

Completeness uses exact request identities. PostgreSQL sequence numbers can skip
values after an aborted transaction; numeric contiguity is not a valid integrity
test. Local exports carry an independent identity inventory. Each encrypted remote
entry includes its canonical predecessor's identity/sequence so fetch can reject
missing interior entries without new hashes or format versions. Losing the entire
latest suffix of every independent copy remains the stated storage obligation.
Active analytical ownership entries use the same optional encrypted transport;
confirmed closed attempts can retire after their original creation deadline, as
the existing analytical journal already permits.

### Fresh-target restore and upgrade compatibility

Restore selects an explicit fresh destination installation with its own volumes,
origin and runtime inputs. Reject populated or foreign targets before changing
their data. A durable recovery marker blocks normal API/worker/migration startup
until the controlled restoration has completed; crashes retain the marker.
The recovered installation retains the original canonical identity. Its source
application must therefore remain stopped after the final journal capture; source
volumes can stay preserved for rollback. This is one active writer, not an
active-active clone. Tooling rejects a still-running source project on the same
Docker endpoint; stopping a source on another machine remains the operator's
cutover responsibility. Previously restored copies recorded by the local operator
tooling are subject to the same stopped-writer check, including after later backup
operations replace their current status record.

Fully decrypt/authenticate in private staging before reading archive members or
executing SQL. Validate expected members, UUID paths, regular files and bounded
lengths; reject traversal, links, devices, unknown schema and missing required
artifacts. Never pass unauthenticated streamed plaintext to PostgreSQL or tar.
Preserve database grants/RLS under the existing application role. Restore with
error stopping and one database transaction; failed imports cannot start serving.
An explicit resume retires only the selected restore's tagged helper containers
and matching database import session before reading its committed checkpoint or
running preparation again. Killing the operator process cannot leave an older
helper able to modify a subsequently released installation.

Applied migration names must be a recognized prefix of the selected executable's
migrations, with the supported PostgreSQL major/extension available. Refuse newer,
unknown or gapped histories. Forward migrations and privacy reconciliation run
before application startup. A failed migration remains stopped; automatic reverse
migrations and in-place downgrade are unsupported. A rollback drill uses a fresh
target and compatible retained image/checkpoint, leaving the old installation
preserved until the operator chooses cutover.

Before lifting the marker, replay current deletion fences and clean restored copies.
Check retained artifact availability and expire due content. Preserve canonical
claims, bi-temporal history, review decisions/rules/exceptions, scope, audit and
policy at the recovered checkpoint. Current restored rejection rules govern later
intake, worker replay, exact/lexical/semantic recall and graph reconstruction.
Ordinary changes after that checkpoint remain explicit recovery-point loss; do not
pretend to reconstruct unavailable later corrections, grants or account changes.

Fence restored MCP execution ownership and mark every outstanding call uncertain,
including calls merely queued at the snapshot that could have run afterward.
Preserve original IDs and observation lineage; receipt-only reconciliation remains
permitted under current authority. Never reuse a persisted PID or repeat an effect.
Similarly, outstanding model requests become uncertain with their accounting
retained; restoration cannot silently repeat a potentially billed request.
Recovery neither copies Vault credentials into the archive nor revokes them.

Neo4j is a rebuildable projection. Requeue canonical graph/derived work through
its existing boundaries after privacy/retention and rule checks. Report unavailable
or rebuilding until actual query/projection proof succeeds; an empty fresh graph
is not proof of no relationships. Retained eligible pgvector/lexical representations
may be reused; new embeddings still require standing provider policy and budget.
Do not replay old analytical jobs or assume scratch graphs belong to the new run.

### Recovery objectives and diagnostics

The ordinary-data objective is a checkpoint no older than 24 hours under a healthy
daily schedule. Remote-enabled acknowledged erasures require their separate durable
journal copy, independent of the older data checkpoint. Availability of every copy
and universal zero-loss recovery are not claimed.

The local acceptance target is restore plus canonical graph/recall verification
within fifteen minutes for the recorded drill corpus, excluding image builds and
operator key/configuration setup. Record actual records, artifact bytes, archive
bytes, backup/restore/rebuild durations and missing/unrebuildable evidence. Do not
extrapolate that target to an unmeasured production corpus or hardware.

Content-free operator status identifies backup/remote/prune failure, held/failed
restore, incompatible schema, missing journal, unavailable artifacts and projection
rebuild state. Existing ordinary desktop/readiness views must reflect unavailable
dependencies correctly; no new per-memory human review workflow is introduced.

## Acceptance

Use owned disposable personal/shared installations and an actual isolated OpenSSH
SFTP endpoint with generated fixture keys. Prove encrypted upload/download and a
fresh-volume restore, plus wrong key, corrupt archive, failed transfer, untrusted
host, missing/older/wrong journal, nonempty target, missing artifact and incompatible
migration failures. No failure may start a partially restored application.

The corpus includes separate Brain/member boundaries, retained independent evidence,
repository/managed/session observations, qualified and rejected claims, history,
rejection rules and model/graph metadata. Preserve rejection during re-entry and
worker replay. Erase captured/derived evidence after the data checkpoint, then
restore that older checkpoint using the current journal; prove affected content
stays absent while independent evidence, scope and recall survive. Pair remote
journal failure with local denial/pending status, recovery and exactly one remote
entry. Keep every Vault token and lease untouched.

Exercise interrupted publication/queued work and an uncertain managed effect across
restore; demonstrate receipt-only reconciliation without another effect. Rebuild
eligible Neo4j state on an empty graph store and inspect actual scoped recall and
desktop evidence. Test retention pruning with a retained control, restart/resume,
supported forward migration and a failed upgrade/compatible checkpoint recovery.
Run focused meaningful tests, workspace/Clippy/API/web checks as affected and
`./scripts/validate.sh`; publish measured evidence and operating steps before closeout.

## Explicit Deferrals

External production deployment/cutover, automatic global scheduler installation,
continuous WAL/PITR or zero-loss replication, high availability, in-place reverse
migrations, customer-source writeback and universal capacity guarantees. Integrated
quality/concurrency/cost evaluation remains the final named successor.
