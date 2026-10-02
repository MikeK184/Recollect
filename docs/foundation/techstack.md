# Recollect Technology Stack

Status: accepted

October 1 integration amendment: [ADR 0018](../adr/0018-plugin-managed-agent-memory.md) packages the Rust local memory runtime inside host plugins. Separate companion setup is replaced by plugin-managed capture and recall; independent execution remains an explicit optional runner. Existing backend and evidence guarantees remain.

Current development override: [ADR 0003](../adr/0003-product-runtime.md) records
the user's later instruction to omit product hashing and strict version/pinning
gates during this implementation goal. The remaining baseline stays in force.

Revised baseline: 2026-09-13, from the approved foundation plan and product
clarification. Rust is selected for Recollect's own backend and companion.
The MCP coordinator, Vault integration, collections and graph capabilities
remain part of the product. This replaces the Cognee/Python extension stack.
The [vision](vision.md) governs behavior; the
[research mapping](../mappings/atlas-foundation-decisions-2026-09-13.md) records
sources, alternatives and limitations. No new application stack is installed
or measured by this documentation change.

## Selected components and packaging

These are selected architectural defaults, not a tested dependency lockfile.
During scaffolding, choose compatible published releases/features, pin the Rust
toolchain and container images, and commit Cargo.lock and the frontend lockfile.
Exact release pins remain implementation work; do not use floating production
images or claim current package versions form a validated compatibility matrix.

| Responsibility | Selected default |
| --- | --- |
| Product/backend and local companion | Rust in a Cargo workspace with shared domain/protocol crates |
| Async/API | Tokio, Axum, Tower/tower-http |
| Serialization and outbound HTTP | Serde, Reqwest with Rustls |
| Transactional authority | PostgreSQL through SQLx |
| Exact, lexical and semantic retrieval | PostgreSQL identifiers/full-text search and pgvector |
| Graph traversal and analytics | Neo4j Community Edition with GDS Community Edition |
| Graph client | Rust Reqwest adapter to the official Neo4j Query API |
| MCP | Official Rust SDK rmcp; stdio locally and Streamable HTTP remotely |
| Browser | React, TypeScript, Vite, Mantine, TanStack Query and Router |
| API description/client | Utoipa OpenAPI and a generated TypeScript client |
| Jobs | PostgreSQL transactional outbox, durable jobs and bounded Rust workers |
| Artifact storage | Content-addressed files on a durable volume; an internal seam permits later object storage |
| Authentication | Built-in accounts and optional single-organization OIDC |
| MCP credentials | Vault integration; OS credential store/approved environment injection for simpler installations |
| Diagnostics | Structured tracing logs and operational metrics; authoritative mutation audit stored separately |
| Deployment | OCI services under Docker Compose locally or on a shared private server |

Start with two Rust deliverables: a server with API/worker/migration roles and an
agent with companion, capture-hook, stdio MCP bridge and private-runner roles.
The companion shares identities and validation without linking server database
drivers. Separate heavy graph work from capture and interactive API capacity.
Use concrete internal adapters; no generic plugin framework or required Python
service is selected.

Build the React application into static assets served by the Rust API from the
same origin. Node is build tooling, not an additional production application
runtime. Useful Mantine components can be reused from the reference frontend;
Next.js server rendering is not selected. [Vite deployment](https://vite.dev/guide/static-deploy.html).

Official [Axum](https://docs.rs/axum/latest/axum/),
[SQLx](https://github.com/transact-rs/sqlx), and
[rmcp](https://github.com/modelcontextprotocol/rust-sdk) documentation supports
the component direction, not proof that Recollect's integration works.
SQLx query checking needs matching schema information or maintained offline
metadata and does not prove authorization. A started Tokio blocking task cannot
simply be aborted; timeouts, task cancellation and subprocess termination need
separate contracts. [Tokio blocking work](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).

Rust provides native packaging, explicit lifetimes and concurrent I/O. Model
latency, query plans, algorithms and data volume still determine much of the
observed performance. Measure rather than infer capacity from language choice.

## Storage ownership and hybrid retrieval

PostgreSQL owns principals, Brain grants, collection/source identities, versioned
claims, review/rejection rules, temporal history, manifests, publication state,
mutation audit and durable jobs. Canonical mutations, audit and outbox work
commit together. Workers project searchable/analytical views asynchronously and
expose queued, partial, failed and current states. Define read-after-write and
publication behavior in contracts before persistence implementation.

Retain permitted normalized extraction inputs and captures as immutable artifacts,
with hashes and provenance in PostgreSQL. Artifact access and deduplication must
respect Brain boundaries. A hash or missing source reference is not sufficient
to rebuild. Collections and overlapping area/environment views do not require
separate engines or duplicated authoritative memory for each content type.

PostgreSQL provides exact and lexical lookup; pgvector supplies semantic
similarity. Neo4j supplies code/configuration and knowledge-relationship queries
and graph computation. Do not duplicate vector/text indexes in Neo4j initially.
Source-derived relationships, model-inferred links and operational observations
retain different evidence classes. None independently owns truth or permissions.

Use exact eligible-vector search as the initial correctness baseline. When ANN
is justified, use Brain partitions or separate vector tables and compare scoped
recall to exact search. Shared ANN indexes can let other scopes influence recall
and speed; iterative scans do not establish isolation. Embeddings retain model,
dimensions, representation and input revision; model changes require explicit
reindexing. Do not assume every embedding dimension fits every index type.
[pgvector filtering and multitenancy](https://github.com/pgvector/pgvector).

Fuse relevant exact, lexical, semantic and graph results into one bounded context,
deduplicated by canonical identity and attributed to sources. Native PostgreSQL
full-text ranking is not BM25. Ranking/fusion parameters are tested on the target
corpus; they never widen authorization or revision applicability. Investigation
and strict modes share the same authority and evidence model.
[PostgreSQL text search](https://www.postgresql.org/docs/current/textsearch.html).

## Graph computation and recovery

Graph analytics in the first usable version justifies Neo4j/GDS over relational
edges alone. Use Community artifacts without paid database/analytics licenses;
record the exact distributions and notices. Hosting and model fees are separate.
LadybugDB remains an embedded alternative for a future deliberate stack change,
not an additional service. Original Kuzu is not the selected dependency.

GDS Community provides the algorithm suite with a four-core concurrency limit.
Its analytical projections live in memory. Begin with one heavy job at a time,
memory admission, projection eviction and visible job state. Estimation helps
avoid oversized work but cannot guarantee freedom from OOM. Broad traversal,
centrality and clustering must be exercised on representative graphs.
[GDS editions](https://neo4j.com/docs/graph-data-science/current/introduction/),
[memory estimation](https://neo4j.com/docs/graph-data-science/current/common-usage/memory-estimation/).

Persist immutable repository snapshots once; manifests select them without
copying every graph for each environment. Stage graph generations before
publishing readiness. Cross-repository links identify both endpoint snapshots,
linker version and evidence. Retain unresolved targets and partial coverage.

Every analytical input is authorized for a Brain, exact manifest and selected
relationship families. Results record algorithm/version, parameters, direction,
weights, seed where applicable, and the input eligibility/correction epoch.
Do not combine call edges, semantic similarity and deployment observations into
one calculation without a declared meaning. Changes to eligible inputs invalidate
affected current aggregates: filtering output rows cannot undo their influence
on PageRank or community assignment. Store result provenance durably and rebuild
from permitted inputs; analytics never directly promotes its findings to belief.

Keep Neo4j behind Recollect's API with controlled parameterized queries and
scope on every traversal endpoint and analytical input. CE database authorization
is not a substitute for Brain isolation. PostgreSQL RLS is defense in depth;
use non-owner application/worker roles and test privileged bypass paths.
[PostgreSQL RLS](https://www.postgresql.org/docs/current/ddl-rowsecurity.html).

Use the official [Query API](https://neo4j.com/docs/query-api/current/) through
Reqwest initially. A Rust Bolt client is an optimization candidate only after
compatibility proof against the pinned server/GDS pair. No Python graph-control
service is required by this architecture.

PostgreSQL and retained artifacts are the primary recovery inputs. Back them up
encrypted off-machine and test restores, including deletion/rejection replay
before enabling recall. Neo4j CE supports offline dump/load; its derived graphs
can instead be reconstructed. A copy of live database files is not a consistent
backup. Define backup expiry, recovery point/time targets and rebuild duration
in the deployment contract. [PostgreSQL recovery](https://www.postgresql.org/docs/current/continuous-archiving.html),
[Neo4j offline backup](https://neo4j.com/docs/operations-manual/current/backup-restore/offline-backup/).

## Extraction, capture and models

Use Enola's versioned artifact interface as the first product extraction adapter,
subject to focused language/fixture proof. The researched v0.4.19 consumer
contract covers facts.jsonl, insights.json and receipt.json. Recollect owns full
canonical origins and stable repository UUIDs rather than trusting extractor
labels; Enola receipt normalization can discard remote ports. Preserve duplicate
record handling, unresolved targets, versions/settings and hashes explicitly.
[Enola artifact contract](https://github.com/enola-labs/enola/blob/v0.4.19/docs/schema/README.md),
[receipt schema](https://github.com/enola-labs/enola/blob/v0.4.19/docs/schema/receipt.md).

Extract from isolated exact committed trees without changing the user's working
tree. HCL extraction does not evaluate Terraform expressions, resolve every
remote module or prove deployment. Kubernetes/Helm and other language coverage
needs fixture evidence. CodeGraph 1.6.0 remains installed development navigation
and an alternate adapter candidate; a worktree index is not immutable publication.
[Enola HCL limits](https://github.com/enola-labs/enola/blob/v0.4.19/docs/extraction/hcl.md).

Thin Codex/Claude host adapters sanitize supported events into a durable local
inbox, then upload/process asynchronously. Retain immutable task scope, event
identity and coverage markers. Do not rely on a stable transcript format or
parent session ID alone for subagent attribution. The 30-day sanitized-session
default and Brain overrides apply locally and centrally; retained supporting
excerpts follow their own explicit policy. Host interruption and missing-event
behavior require compatibility tests.

One model gateway enforces Brain-approved provider, model, purpose and content
class across extraction, synthesis, embeddings and external reranking. Capture
authorization is distinct from transmission authorization. Keep model/prompt/
schema/policy versions with derivations, bound budget/concurrency and deny
unapproved fallbacks. Provider credentials, regions and exact models are deployment
inputs. Standing policies accept supported routine output and govern autonomous
reconciliation; uncertainty remains qualified while unrelated work continues.
Human corrections are optional durable overrides under [ADR 0006](../adr/0006-autonomous-memory.md).

Cognee, Hindsight, Graphiti, Verel and OpenViking are reference systems and possible
narrow component sources. No additional memory engine is required. Adding one
requires compatibility with Recollect's scope, correction, retention and recovery
rules. Check each component's own license; the Atlas license does not license
the systems it describes.

## Identity, workspace and MCP coordination

The proposed .recollect/workspace.toml selects a Brain at the workspace root;
nearest-parent discovery and nested-workspace boundaries apply. Task scope is
an explicit operation binding, not mutable authentication-session state. Keep
endpoint and sign-in settings personal. Exact wire names remain contract work.

Personal setup bootstraps one local owner; small teams use invited accounts or
optional OIDC with their existing provider. Use Argon2id password hashing,
server-backed sessions, CSRF protection, throttling and operator recovery.
Local service binding defaults to loopback; shared service requires HTTPS.
Companions pair through short-lived browser approval and receive individually
revocable credentials stored in the OS credential store. Public signup is off.
Validate OIDC issuer and stable subject; email is not the authority key.

Entra is an integration target where used, not a mandatory identity service.
Group mappings yield Recollect grants; incomplete membership data cannot widen
access. Keep contributor identity separate from resource ownership. Specify
membership refresh, effective revocation and active-operation behavior before
shared identity implementation. Browser session middleware carries authentication,
not concurrently mutated Brain/task scope.

The MCP coordinator exposes Recollect's own tools and routes permitted calls to
Brain connections/profiles. Recollect owns the approved catalogue, profile use/
manage/share grants, runner placement, credentials, startup locks and operation
leases. Tool discovery uses approved cached schemas without starting dormant
backends. First authorized use starts/connects as required; 15 minutes without
useful activity permits stopping owned instances only when no work holds leases.

Keep executable/image, target, runner and credential references outside model
arguments. Local, central and private-network execution use the same permission
model; private runners establish authenticated outbound connections. Check grants
at dispatch and execution. Do not blindly retry an operation whose external side
effects may have completed; reconcile unknown outcomes.

Vault support supplies credentials to authorized MCP runners, with environment
delivery or a connector-appropriate equivalent. Integrate existing Vault delivery
mechanisms behind the credential-provider boundary; coordinate rotation with
draining and leases. Vault Agent supervision can restart a child when credentials
change, so its default behavior must not silently interrupt active MCP work.
[Vault process supervision](https://developer.hashicorp.com/vault/docs/agent-and-proxy/agent/process-supervisor).
An installation may use an OS store or approved environment injection instead;
this does not defer Vault integration. Recollect does not constrain unrelated
shell commands or credentials outside its managed paths.

## Delivery and verification

Use the same Compose topology locally and on a shared private server, with
native companions, explicit migrations, persistent volumes and resource limits.
No Kubernetes platform, mandatory SSO server or high-availability cluster is
required initially. The proposed 8-CPU/32-GB/1-TB VM is an unbenchmarked pilot
assumption, not a minimum or a capacity guarantee.

Use tracing JSON logs and operational metrics for queues, errors, latency and
resource use. Keep secret/source payloads out of telemetry. OTLP is an optional
pinned integration; OpenTelemetry Rust's maturity must be verified before use.
Mutation audit remains authoritative even when telemetry is unavailable.

Derive epics from the completed foundations. Detailed contracts must fix scope
handles, schemas, review authority, source/collection removal, replay/erasure,
provider adapters, extraction coverage, API versioning, runner lifecycle and
deployment service levels before dependent code. These refine selected behavior;
they do not reopen the Rust, storage, UI, identity or graph-analytics direction.

Prove personal/two-user flows, cross-Brain isolation, correction after rebuild,
capture expiry, exact manifests, graph correctness, Vault rotation and runtime
leases through actual product paths. Compare retrieval channels under equal
model/context budgets and measure concurrent capture/query/analytics plus restore.
Use ./scripts/validate.sh for documentation governance. Its Bash/Python tooling
and the [CodeGraph runbook](../runbooks/codegraph.md) remain independent of the
new product runtime. No library installation or product deployment is part of
this foundation update.
