# Repository snapshots and revision manifests

Status: accepted

## Source

[ADR 0004](../adr/0004-committed-repository-publication.md),
[runtime posture](../adr/0003-product-runtime.md),
[workspace scope](evidence-workspace-scope.md),
[durable work](platform-durable-work.md),
[collections](evidence-collections.md) and the
[evidence epic](../roadmap/epics/evidence-and-workspaces.md).
The active goal authorizes routine implementation decisions and local integration
work. Existing Git revisions and opaque extractor IDs are metadata, not a new
product hashing contract.

## Contract

### Exact local inputs

Publication targets an explicit Brain, registered repository UUID, task and local
checkout. Require a paired writer/admin. Resolve the local origin using the
shared canonical normalizer and match a registered origin of that repository.
Do not fetch, reset, switch branches, update Git config/index, run hooks or invoke
project-defined commands. Observe dirty status separately; a dirty checkout may
publish its committed revision, with that distinction retained in provenance.

Resolve the requested revision, default HEAD, with `rev-parse --verify
--end-of-options REVISION^{commit}`. Bound revision input to 256 printable
characters. Read the resulting full Git object ID and recursively enumerate the
recorded tree using NUL-delimited `ls-tree`; read blobs by object ID with cat-file.
Do not apply archive attributes, checkout filters, replacement refs or worktree content. Disable lazy fetching of promisor objects. Resolve
the commit once: a later branch/HEAD movement cannot change the prepared input.

Accept at most 5,000 tree entries, 1 MiB per regular text blob and 64 MiB of
materialized text. Reject a tree-listing overflow rather than claim a complete
inventory. Materialize in stable path order and record size/budget exclusions.
Keep paths relative, UTF-8, printable, at most 2,000 characters and without empty,
dot/parent components, backslashes or absolute prefixes. Never follow symlinks
or gitlinks; retain their unavailable status and original Git mode/object ID.
Invalid paths abort preparation. Missing local objects fail explicitly; they do
not cause a fetch or fallback to the working copy. Git metadata operations have
10-second bounds; the full local preparation/extraction has a 120-second bound.

Exclude credential/state/tool directories and files before materialization:
`.git`, `.recollect`, `.codex`, `.claude`, `.ssh`, `.aws`, `.kube`, `.terraform`,
`.enola`, `.env` and `.env.*`, `.secrets*`, credential files, Terraform state and
private-key/keystore extensions. Binary/non-UTF-8 and oversized files are inventoried
without materializing text. Scan candidate text in memory for configured secrets,
private-key blocks, common access-token prefixes and literal credential-bearing
URLs/assignments. Exclude an entire matching file and record only the reason,
never matched content. This bounded exclusion is not a universal secret detector.
The server applies its configured-secret/private-key checks to accepted payloads
before artifact or command-receipt persistence.

Create the capture operation binding before reading source inputs. It fixes the
task's immutable scope for the full preparation/upload, and must include the
repository or be Brain-wide in that dimension. Preserve the operation UUID in
the bundle, contribution and processing job. CWD, parent defaults and concurrent
scope changes cannot relabel it. The uploader must be the operation's account
and device under current Brain access; an old handle is not authorization.

### Extraction and prepared bundles

Run a configured Enola executable, default the repo-owned installation, with an
explicit generated configuration outside the materialized source directory.
Use the repository UUID as the isolated source directory label. Enola receives
only the permitted committed files, a bounded PATH, Git isolation settings, a
repo-owned temp directory and disabled update checks. Do not pass HOME, user
provider credentials, repository config, external providers, renderers, history
or incremental persistence. Use `.enola` output inside that isolated tree.
Capture bounded diagnostic output without printing source contents or credentials.
A killed/failed executable leaves an explicit preparation error, not an empty
successful graph. The optional global receipt being skipped is expected.

Consume the three documented artifacts. Require parseable JSON shapes and
bounded fields, not a particular format/version label. Record adapter build and
extractor version. Keep facts and relations in source order, including duplicate
upstream IDs. An absent target ID remains unresolved; never choose an arbitrary
same-named node. Unknown kinds/properties can be retained as opaque evidence
within size/depth bounds, without claiming semantic support. Keep directory/root
locations distinct from file spans; missing locations and parse/coverage gaps
remain explicit. Filter volatile/personal/digest receipt fields from the stable
publication identity while preserving extraction quality and declared versions.

Bound the full JSON envelope to 24 MiB: up to 100,000 fact records, 10 MiB fact
serialization, 1 MiB insights, 64 KiB receipt and 8 MiB optional retained file
text. Strings and nesting are bounded; reject nonfinite metrics, malformed spans,
absolute/traversing locations and fields that exceed the envelope limits. Existing
opaque fact IDs are not recomputed. No source path in an artifact authorizes an
arbitrary local or server file read.

Retaining source text requires explicit `--retain-file` selections, at most 20
regular permitted UTF-8 files, and the Brain's repository-content policy. The
policy defaults false and is admin-managed independently of document imports.
Requested missing, excluded or oversized retained files fail preparation rather
than silently disappear. No caller can opt into retention while policy forbids
it. The server rechecks policy on upload; turning it off prevents future capture
and does not erase previously accepted evidence.

Prepared bundles contain normalized provenance, filtered extraction artifacts,
file inventory and only explicitly permitted retained text. Store them under a
repo-owned private directory with UUID identities and exclusive creation. Retain
the sanitized prepared bundle when upload fails so a same-device resume can
retry its original operation/idempotency identity without re-reading a changed
checkout. Remove successful bundles and transient materialized trees after use.
Never auto-remove unrelated paths. Crash leftovers remain private, identified
staging data for an explicit owned-directory cleanup; later erasure consumes this
local prepared-input boundary as well as backend artifacts.

### Shared admission and processing

The service owns immutable snapshot UUIDs within a Brain/repository. A snapshot
records full committed revision, normalized settings, adapter/build/extractor
labels, artifact references, file inventory, coverage and capture time. File
records have their own UUID, exact Git object/mode/size, materialization/coverage
status and optional retained artifact. Source locations reference snapshot/file
identity; missing raw text is explicit and does not make structural artifacts
unavailable. Retained file text is separate evidence from user document sources.

Normalize settings and compare stable parsed input values for duplicate admission.
Equivalent repository/revision/adapter/settings/artifacts reuse one snapshot;
different retained-file selections are settings variants. The same candidate
identity with conflicting stable artifacts returns 409 rather than overwriting
history. Every distinct accepted operation retains its authenticated contributor,
device, capture time and dirty/branch observation even when it reuses a snapshot.
Reusing that operation for the same snapshot with changed capture provenance
conflicts; it cannot silently relabel or discard the original contribution.
Same idempotency key/body replays its original result; changed body with that key
conflicts. No custom digest or strict version handshake is introduced.

Brain locks serialize admission, policy and capacity decisions. Limit snapshots
to 200 per repository and 4,000 per Brain, and contributions to 1,000 per snapshot.
Write private durable artifacts before committing canonical snapshot/file/
contributor metadata, mutation audit and queued processing atomically. Use the
existing artifact adapter's exclusive creation and flush behavior. A transaction
failure removes only its own unreferenced writes when safe; crash orphans remain
unserved and preserved for explicit reconciliation. Accepted evidence survives
contributor disconnect/revocation; current caller grants still govern every read.

Queue `repository.process` in the heavy lane. The job carries its snapshot and
original operation scope and uses existing leases, fencing, admission, bounded
retry/cancellation and current authorization checks. It validates stored records
and materializes fact rows with immutable snapshot/record ordinal identity.
Reprocessing preserves those identities and cannot switch any other snapshot or
environment selection. Brain readers can browse accepted artifacts and processing
state; pending, denied, failed or cancelled materialization cannot appear ready.
Current writers can explicitly reprocess retained artifacts under their own
current authority; original contributor history stays unchanged. Neo4j graph
projection and cross-repository resolution are not performed by this job.

### Environment revision manifests

An environment has named manifest histories. Each immutable manifest revision
records UUID, Brain, environment, name, kind, entries, actor, recorded time and
optional observation metadata. Kinds are `committed`, `desired` and
`observed_deployed`. Never infer observed deployment from a checkout branch or
HEAD. Observed-deployed submissions require an observation time and a supporting
reference; the UI identifies them as recorded observations, not an independent
runtime verification by Recollect.

Each manifest has 1–100 distinct repository entries. An entry records a registered
repository UUID, exact commit ID, optional matching snapshot UUID and up to 100
relative configuration paths. A linked snapshot must have the same Brain,
repository and commit. A missing snapshot is allowed and shown as unavailable;
it does not fabricate facts/files or silently choose a newer publication. Paths
are selectors/references, not proof that a deployed process consumed them.

Create/update the named manifest's current pointer with an observed base revision,
returning 409 for a stale editor. Keep prior immutable revisions inspectable.
An environment with manifest history cannot be removed through ordinary view
deletion; return 409 and preserve the selection history for the erasure lifecycle.
Names are at most 120 characters, notes at most 2 KiB and observation references
at most 2,000 characters. Bound each environment to 100 named manifests and each
history to 1,000 revisions. Validate environment/resource IDs, references and
current writer/admin access in the same transaction as audit and refresh work.
Browser edits capture explicit Brain/environment/repository scope at admission;
they do not invent a task/subagent attribution. Supplied task bindings, when
present, must authorize the whole selection and retain their immutable scope.

### Interfaces and UI

Use these Brain-scoped endpoints, all under `/api/brains/{brain}`:

- `GET/PUT /repositories/policy`: inspect/admin-update repository text capture.
- `POST /repositories/{repository}/snapshots`: paired, scoped immutable upload.
- `GET /repositories/{repository}/snapshots`: paginated revision variants.
- `GET /repository-snapshots/{snapshot}`: provenance, coverage and processing.
- `GET /repository-snapshots/{snapshot}/facts`: paginated preserved fact records.
- `GET /repository-snapshots/{snapshot}/files`: paginated file inventory.
- `GET /repository-snapshots/{snapshot}/files/{file}`: authorized retained text
  or an explicit unavailable result.
- `GET /repository-snapshots/{snapshot}/artifacts/{kind}`: facts, insights or
  sanitized receipt; kind is an allowlist, never a filesystem path.
- `POST /repository-snapshots/{snapshot}/process`: explicit reprocessing.
- `GET/POST /revision-manifests`: list current named selections or create one.
- `GET/PUT /revision-manifests/{manifest}`: inspect history/update current revision.
- `GET /revision-manifests/{manifest}/revisions/{revision}`: immutable history.

Use 20-row snapshot/manifest/history/contributor pages and 100-row fact/file pages.
No external source fetch occurs while browsing. The CLI provides repository
publication and prepared-bundle resume with explicit Brain/repository/task inputs,
plus snapshot/manifest inspection. The browser adds snapshot browsing to repository
rows and a revision-manifest editor/history. Display exact commit, contributor,
dirty working-copy observation, missing raw content, capture policy, processing,
unsupported formats, unresolved targets and recorded-observation provenance.
All reads/mutations use Brain RLS; private checkout paths stay account-private.

## Acceptance

Prove two exact committed revisions, dirty working-copy isolation, archive
attribute independence, path/symlink/submodule/secret exclusions, retained-content
policy, native failure/resume and real Enola artifacts. Verify Rust/TypeScript/HCL
positive facts and explicit GitOps/unknown-format gaps. Backend/RLS proof covers
duplicate contributors, conflicting artifacts, cross-Brain and revoked-device
denials, worker fencing/reprocessing, missing artifacts and immutable manifests
with distinct environments/revision kinds. Browser proof covers snapshot/file/
coverage inspection, capture policy, manifest history/stale edit and mobile layout.
Run focused checks and `./scripts/validate.sh` before closeout.

## Explicit Deferrals

Neo4j projection/linking, semantic recall, model-driven evidence/claim extraction,
automatic session capture and erasure/re-entry policy remain in their owning
slices. GitOps semantic parsing is unsupported by this Enola adapter; its files
remain inventoried and may be explicitly retained under policy for later evidence
processing. No repository publication claims remote origin control, Terraform
evaluation, Kubernetes/Helm rendering or independently verified deployment.
