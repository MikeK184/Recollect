# Recollect

Recollect is an independent engineering memory and MCP coordination product for
one person or an internal team. Brains bring together knowledge collections,
repository memories and graphs, environment views, runbooks, and authorized MCP
connections with Vault credential integration. The first local platform is implemented: a Rust API, same-origin React UI,
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
The remaining product capabilities are tracked in the 29-slice roadmap.

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
focused tests, storage and recovery. Changes remain uncommitted; no external
service deployment is included.

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
