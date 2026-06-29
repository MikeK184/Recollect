# Canonical graph projection and traversal

Status: accepted

## Source

[ADR 0007](../adr/0007-canonical-graph-projections.md), [graph epic](../roadmap/epics/graph-intelligence.md),
[repository publication](evidence-repository-publication.md), [canonical recall](retrieval-exact-and-lexical.md),
[corrections](memory-review-and-corrections.md), [retention](memory-retention-and-erasure.md)
and [durable work](platform-durable-work.md).

## Contract

### Inputs and projection identity

The service derives one structural input per immutable repository snapshot and
one current knowledge input per Brain memory epoch. Repository generations are
reused across selected manifests; a development publication does not replace an
older production snapshot. Graph entities have the canonical kind and revision
UUID. Fact record UUIDs retain their snapshot ordinal even when extractor IDs
repeat. Neo4j entity keys include kind, revision and Brain; internal element IDs
are never public identity. Neo4j holds no source text, names, excerpts or secrets.

A generation records UUID, Brain, kind, optional snapshot, input epoch, adapter
label, actor/job, timestamps, state, counts and unresolved/unsupported coverage.
Its frozen descriptors contain only canonical identities, edge UUIDs, typed
families, allowlisted relation names and evidence ordinal/identity. Persist at
most 100,000 nodes, 250,000 edges and 64 MiB descriptors per generation. Exceeding
a bound fails visibly without publishing a truncated graph.

Structural edges require a retained materialized source fact and a `target_id`
that identifies exactly one record in that same snapshot. Missing, absent and
duplicate targets remain unresolved/ambiguous; names cannot substitute for IDs.
Supported relations are `declares`, `imports`, `calls`, `implements`,
`depends_on`, `instantiates`, `injects`, `has_method`, `handled_by`,
`implemented_by` and `names`. They retain their distinct direction and static
extraction evidence class. Unknown relation kinds remain in canonical artifacts
and contribute unsupported coverage, with no invented path edge.

Knowledge graphs contain current claim revisions and explicitly recorded source
version, fact and manifest supports. Directed `supported_by` edges go from the
claim to its evidence. Directed `contributed_to` edges go from a required typed
handover contribution revision to the handover. Historical reasoning ancestry is
not a current contribution. These are provenance relationships, not inferred
causal links or proof of deployment. Claim trust/origin/operational state remains
canonical metadata at read time. This slice creates no model-inferred relation.

### Work, publication and recovery

Maintenance discovers materialized retained snapshots and changes to current
knowledge automatically under a current writer/admin, with no per-memory review
queue. Discover at most 100 Brains and enqueue at most ten inputs per Brain/pass,
respecting the existing 500 pending jobs/Brain limit. Unavailable authority or
input remains visible. Explicit writer rebuild queues a new generation under the
same mutation/audit boundary and supports idempotency keys.

Use the existing single heavy lane, fenced 20-second leases and bounded retries.
Freeze descriptors under the Brain lock, release PostgreSQL locks before Neo4j
I/O, import in at most 500-record batches and renew leases while waiting. Use
parameterized Query API statements, three-second server transaction bounds,
five-second HTTP bounds and bounded fully parsed responses. HTTP 202 alone is
not success. Treat Query API errors, incomplete shapes and count/descriptor
mismatches as failures. Never output backend payloads or credentials in errors.
Each import attempt has a 120-second limit. Transient backend failures use the
existing durable backoff; permanent query/configuration failures stop visibly.
Reuse the shared durable-worker heartbeat: continue polling publication while
renewal waits for the job row held by that publication. Do not duplicate a lease
loop that awaits renewal while its own work is suspended.

Import shared entities, generation memberships and generation-owned edges with
idempotent identities. Validate all expected memberships and edge endpoints,
IDs, relation families and types before readiness. Commit readiness only after
rechecking authority, lease and canonical input availability. Current knowledge
input changes cancel stale publication and trigger a new generation. Retain the
previous ready generation during rebuild, label its coverage stale/partial and
apply current canonical qualification. Old and new generation edges cannot mix.
Cancellation/revocation prevents publication. Retry cannot bypass current gates.
Cleanup removes only superseded generation-owned relationships and their
unreferenced entities; unrelated Neo4j labels/data remain untouched. Failed or
cancelled generations retain retryable descriptors/staging until a replacement
publishes, when they become superseded and eligible for cleanup. Retrying a
superseded generation conflicts instead of racing cleanup.
Cleanup first fences that generation's identity against delayed imports. Each
maintenance call removes at most ten batches of 500 relationships/memberships
per generation, retaining pending cleanup until it finishes. Shared entities
survive while another generation owns them. Discovery advances its fair-scan
position even when a Brain's queue is full.

Freeze the earliest retained-input deadline with the descriptors and recheck it
after all publication writes, together with the current lease deadline. Crossing
either boundary rolls back readiness. Expired repository generations become
removed; knowledge maintenance rebuilds expired inputs even if no ordinary
memory edit has advanced the epoch yet.

### Canonical graph reads

`POST /api/brains/{brain}/graph/view` accepts an explicit `ScopeSelection`,
optional collection and immutable manifest revision, optional bound operation,
mode (`investigation`, `strict_accepted`, `strict_operational`), graph kind
(`repository` or `knowledge`), optional exact snapshot and relation families.
Paired requests require the same immutable task operation checks as recall.
An explicit repository snapshot must belong to the selected repository and,
when an environment is selected, to the exact selected manifest. Never silently
substitute a newer snapshot. This slice queries one repository snapshot at a
time; combined structural views/linking are the immediate successor.

Reuse canonical recall scope, current claim revision, manifest, collection,
review/rejection, source availability, configured-secret and temporal gates.
Knowledge paths are current-knowledge reads; this slice does not infer historical
paths from a current graph. Fact time may be supplied for claim applicability.
Each returned entity preserves kind, revision, provenance and qualifications.
For source-version nodes, every selected fragment must be eligible; a blocked or
missing fragment withholds that whole graph vertex rather than letting its other
fragments bridge a rejected assertion. Strict modes include only their qualified
claims, so provenance evidence does not silently acquire accepted status.

Before traversal, materialize at most 5,000 selected canonical candidates and
20,000 selected edges. Overflow returns `graph_scope_too_large`, not an arbitrary
prefix. Canonical qualification determines the allowed entity/edge set before
Neo4j path selection. Generation selection and both endpoints/intermediates must
belong to that set. Neo4j results are revalidated against those exact descriptors.
No path outside scope, through removed content or through a different generation
can influence path length/selection. Retention deadlines and current authority
are checked again before returning; changes fail visibly rather than emit a
partially qualified path. The read has a 15-second bound and two concurrent slots.

The view returns paginated canonical vertices (100/page), eligible counts,
available relation families, selected generation/status and coverage. A separate
`POST /graph/path` accepts the same selection plus exact start/end entity keys,
direction (`outgoing`, `incoming`, `both`) and one to eight hops. It returns one
shortest eligible path, including ordered nodes and edges, or an explicit
`no_path_within_bound`. Equal-length ties need not select the same route. Zero
hops is returned only for an explicitly selected same start/end entity. Cycles
cannot cause unbounded work. Missing projection/outage/invalid endpoint is distinct
from a proven no-path result. A path records the relevant memory epoch and exact
generation/input selection; graph ranking does not change trust.

### Erasure and read-after-removal

PostgreSQL gates deny removed nodes and incident edges immediately. The existing
privacy journal supplies exact canonical keys for Neo4j erasure. Serialize imports
and erasure for a Brain with an owned Neo4j guard; retain content-free identity
fences and refuse subsequent imports of those keys. Delete their memberships and
incident edges. Physical graph cleanup must succeed before an erasure reports
complete, and its pending/error state is visible through the existing UI/API.
Replay the fences and deletion on reconciliation/startup so restoring an older
projection cannot restore eligibility or physical graph content. An outage keeps
cleanup retryable; it cannot undo the committed canonical erasure.

### Interfaces and UI

`GET /graph` lists paginated input/generation readiness and pending graph work;
`POST /graph/rebuild` queues an exact snapshot or current knowledge rebuild.
The browser offers graph kind and exact input selection, status/coverage, rebuild,
eligible entity browsing and bounded path queries, displaying edge meaning,
direction, revision and qualifications. Existing job cancel/retry controls remain
usable; retries preserve immutable input descriptors. No automatically refreshed
page replays a mutation or traversal. Full interactive graph canvases/impact
reports belong to graph-exploration. Existing exact/lexical/semantic recall stays
available independently; this slice makes no model request.

## Acceptance

Prove against actual local Neo4j and PostgreSQL: staged visibility, exact import
validation, replay/lost lease/cancellation/revocation, restart/rebuild, repository
duplicate-ID and unresolved coverage, typed knowledge support and contributions,
scope and exact old-manifest isolation, cycles/direction/hop bounds, removal of an
intermediate node/edge with an eligible alternative path, rejection/rebuild and
erasure/replay including an import racing physical deletion. Test count/time
admission and Neo4j error/malformed/lost responses without publishing readiness.
Verify browser status, selection, path/evidence, errors and mobile layout; run
focused checks and `./scripts/validate.sh` before closeout.

## Explicit Deferrals

Validated cross-repository linkers and combined structural views (slice 17), GDS
analytics and aggregate invalidation (18), full graph exploration (19), graph
retrieval fusion (20), historical graph queries and speculative inferred edges.
These are visible limits; none counts as delivered by basic connectivity.
