# Recollect

For current local continuation and the historical PC transfer record, read
[CONTINUE_HERE.md](CONTINUE_HERE.md). Work resumed on the original Mac.

Recollect is an independent engineering memory and MCP coordination product for
one person or an internal team. Brains bring together knowledge collections,
repository memories and graphs, environment views, runbooks, and authorized MCP
connections with optional Vault credential integration. The first local platform is implemented: a Rust API, same-origin React UI,
owner/invited accounts, optional OIDC, isolated Brains and effective access controls,
mutation history, durable jobs with processing controls and live database health.
Native companions pair through browser approval and use individually revocable
credentials in the OS store; see [device pairing](docs/runbooks/device-pairing.md).
Brains also support text/reference imports, immutable source history and shared
collection, area and environment views; see [source evidence](docs/runbooks/evidence-collections.md).
Paired companions discover local workspaces and register checkout metadata;
the browser and CLI preserve independent task/subagent operation scopes. See
[workspace scope](docs/runbooks/workspace-scope.md).
Companions publish exact committed repository snapshots with optional permitted
file text, contributor history and durable fact processing. The browser selects
immutable environment revision manifests; see [repository publication](docs/runbooks/repository-publication.md).
Claims and decisions link exact evidence to immutable knowledge history, with
separate fact-time filters and explicit eligibility; see [claims and time](docs/runbooks/claims-and-time.md).
Writers can review claims, correct rejected values and resolve conflicts with durable
re-entry rules; see [review and corrections](docs/runbooks/review-and-corrections.md).
Brain admins control class retention, retain exact supporting excerpts and erase
controlled memory with visible cleanup and restore protection; see
[retention and erasure](docs/runbooks/retention-and-erasure.md).
Brain model policies govern the selected Luna and embedding-large gateway, usage
limits and autonomous source learning, evidence-based revision and retirement; see
[model learning](docs/runbooks/provider-learning.md).
Supported routine knowledge is accepted by policy without someone reviewing each
record. Procedures preserve conditions, steps and actual outcomes; generated
handovers refresh when their evidence changes. Human review, correction and Erase
remain optional controls. See [procedures and handovers](docs/runbooks/procedures-and-handovers.md).
Native Codex/Claude hooks capture supported sanitized session and tool evidence
under a standing Brain policy, with durable offline delivery, original task scope
and visible coverage. See [session capture](docs/runbooks/session-capture.md).
The browser and native companion recall exact identities and lexical evidence with
scope, manifest/time filters, canonical qualifications and bounded attributed
context. See [memory recall](docs/runbooks/exact-lexical-recall.md). Bound captured
corrections can revise existing memories automatically; the
[Atlas implementation audit](docs/mappings/atlas-implementation-audit-2026-09-15.md)
records all 21 pattern dispositions, repairs and actual-model proof.
Standing model policy can also build full-dimensional semantic representations
automatically. Browser/native recall combines exact, lexical and semantic matches
under the same scope and memory rules. See [semantic search](docs/runbooks/semantic-recall.md)
and its [measured synthetic comparison](docs/mappings/semantic-retrieval-2026-09-15.md).
Exact repository/combined graphs and canonical knowledge relationships support
native paths and queued PageRank, Leiden and connected-component reports with
current eligibility and durable scratch cleanup. See
[graph analytics](docs/runbooks/graph-analytics.md) and
[desktop graph exploration](docs/runbooks/graph-exploration.md). Recall also fuses
qualified graph relationships with text/semantic matches and source coverage;
see [graph recall](docs/runbooks/graph-recall.md). Desktop investigation compares
disagreements, inspects exact sources and frozen history, and follows scoped graph
relationships; see [memory investigation](docs/runbooks/memory-investigation.md).
Approved MCP connections and execution profiles now provide independent Use,
Manage and Share permissions and cached tool inspection without provider startup;
see the [catalogue runbook](docs/runbooks/mcp-catalogue.md).
Central and paired local MCP execution now include isolated credentials, call
history, cancellation and receipt/evidence reconciliation; see the
[runtime runbook](docs/runbooks/mcp-runtime.md).
Registered private runners and optional per-connection Vault credentials now
support actual execution, renewal, rotation and expiry; see the
[Vault/private runner runbook](docs/runbooks/mcp-vault-and-private-runners.md).
Environment, OS-store, Vault and anonymous connections can coexist.
All 29 original product slices are delivered locally, including
[coding-host memory/workspace MCP](docs/runbooks/agent-memory-tools.md) verified
with actual Codex and Claude hosts, and personal/shared installation proven through
real desktop/API/native HTTPS and operational failure paths. See the
[continuation record](CONTINUE_HERE.md) for current runtime state, the
[installation runbook](docs/runbooks/installation.md) for packaging, and the
[recovery runbook](docs/runbooks/recovery.md) for verified encrypted backup/restore.
The [integrated evaluation](docs/mappings/integrated-evaluations-2026-09-26.md)
records actual-model quality, concurrent workload results and measured limits;
its [runbook](docs/runbooks/integrated-evaluation.md) describes intentional repeats.
Current UI work targets desktop; mobile views are deferred.

The accepted [vision](docs/foundation/vision.md), [technology stack](docs/foundation/techstack.md),
and [engineering principles](docs/foundation/engineering-principles.md) select a
Rust backend/companion, React/Vite UI, PostgreSQL/pgvector and Neo4j/GDS Community.
All seven Atlas capabilities are required. The
[foundation research mapping](docs/mappings/atlas-foundation-decisions-2026-09-13.md)
records the reasoning, references and remaining implementation proof.

Start with [the documentation lifecycle](docs/README.md), then
[the roadmap](docs/roadmap/epics/index.md). Agent configuration and Context7
setup are described in [.codex/README.md](.codex/README.md).

Start the local application:

```sh
./scripts/dev.sh
```

Open `http://127.0.0.1:8787`; owner credentials are generated in the ignored `.env`
file. See [local development](docs/runbooks/local-development.md) for prerequisites,
focused tests, storage and recovery. GitHub publication was explicitly authorized
on 2026-09-26; see Git history for committed revisions. No external service
deployment is included.

Validate documentation from the repository root:

```sh
./scripts/validate.sh
```

Validation requires Bash and Python 3.11 or later, using only the Python
standard library. These are documentation-tooling prerequisites, not product
technology decisions. See [the validation runbook](docs/runbooks/validation.md).

The local `cognee/` directory is an independent upstream Git checkout excluded
from this repository. It is a design/component reference, not a required runtime,
vendored source or submodule.

The local `agent-memory-atlas/` directory is another independent, ignored
checkout, containing memory-system reports, architectural patterns, and their
generated website. Use it as research input for Recollect. The
[reference mapping](docs/mappings/atlas-reference-2026-09-13.md) records its
revision and the Markdown paths for the requested pattern/comparison pages.
