# Recollect continuation and PC transfer record

Transfer prepared: 2026-09-16. Source: `/Users/mike/devops/Recollect` on macOS ARM.
Historical destination: `/home/mike/devops/Recollect`, `mike@192.168.0.41`.

Read this file first, then `AGENTS.md`, `docs/README.md`, the accepted foundations,
the relevant contracts and the active execution pack. This is a dated handoff,
not a declaration that unfinished work is shipped.

## Current continuation — 2026-09-26

Latest repository decision: Apache-2.0 is selected, with a simplified root README.
Keep GitHub private. Public visibility and the proposed private-archive rename
were explicitly deferred; the original commit history and dates stay intact.
The ignored `.cache/public-release-20260926/candidate-v2/` contains an isolated
privacy-cleaned history prepared for a possible future publication. It is not
pushed or active and must be refreshed against later commits before use. The
original history and handoff also have private backups beside that candidate.


All **29 original slices are delivered locally**, plus the Atlas capture repair.
The first-product goal is complete, with no remaining eligible slice or acceptance
blocker. The [final evaluation](docs/mappings/integrated-evaluations-2026-09-26.md)
and [archived pack](docs/roadmap/execution/archive/operations-integrated-evaluations.md)
record the seven-capability matrix, actual-model quality, workload, fixes, failed
attempts and final verification. Full fusion retrieved all six target claims and
produced five useful answers from six supported questions; unsupported semantic
context and the one answer miss remain visible. The unchanged eight-caller workload
passed with recall p95 863 ms, capture p95 441 ms, all accepted writes readable
within 8.1 seconds, zero scope leaks/unexpected failures and zero model calls.
Its 981 explicit capacity refusals and 4.9-second recall tail remain measured limits.
No customer-corpus capacity or general model-superiority claim follows from this
small synthetic run. Personal/shared installation and recovery acceptance are
complete; version is N/A. On 2026-09-26 the user separately authorized committing
and pushing the existing changes to the private `MikeK184/Recollect` repository.
See Git history for committed revisions; older closeouts retain their state at
the time of delivery. External deployment, mobile work and renewed VM transfer
remain outside this goal. Never revoke any Vault token or lease.

The requested final Cargo/Clippy cleanup reclaimed **13.71 GiB of actual free
space**, leaving **121.81 GiB** available. Only inactive `target/debug` dependency,
incremental, example, build and fingerprint directories were removed after tests.
Current server/agent/runner/bridge binaries, Linux host-test cache, Docker resources,
data and reference checkouts were preserved. Both normal process identities and
readiness passed after cleanup. Evidence:
`.cache/build-cache-cleanup-final-20260926.json`. The earlier 1.6-GiB cleanup is
separately recorded in `.cache/build-cache-cleanup-20260926.json`; future builds
will regenerate caches.

Managed observation capture (slice 26) and installation (slice 27) are accepted.
Normal UI is `http://127.0.0.1:8787`, with owner credentials in ignored `.env`.
Normal API/worker PIDs **9750/9749** are healthy on **migration 026**, after a
validated pre-upgrade dump and graceful drain. Fresh login and exact inventory
comparison preserved seven Brains, 33 model requests and existing profiles/grants,
with zero active jobs. Retained SWEG exact recall returned one item / 1,630 bytes
in 49 ms, without a new paid model call or customer file read. Current dump,
inventory, logs and verification are in `.cache/integrated-normal-20260926/`;
the earlier pre-025 record remains in `.cache/recovery-normal-proof/`.
Normal startup is `./scripts/dev.sh`; avoid a second native API/worker on the
same database. Cleanup preserved the current executables, so the live UI remains
available; a future Cargo build will rebuild removed artifacts.

Final integrated proof runs as `proof-integrated-restore-20260926` at loopback
port 19794, image `recollect-product:integrated-proof` with local ID
`4dda2d9c0d75b7f6c3010236117d44eb2c14c85a3cbf632dad285fb26693dc44`.
Actual encrypted SFTP backup took 11.278 seconds and fresh-volume restore 25.309
seconds. Installation identity, all 21 existing Brain admission keys, retained
scoped content, quality revisions and 41 actual model-request rows survived;
a new Brain received a distinct key. Final desktop review/workspace tests passed
together in 18.0 seconds against this restored image. Its source at 19792 retains
databases but has API/worker stopped: do not run both canonical copies as writers.
Private proof roots are `.cache/integrated-20260926/`,
`.cache/integrated-workload-lineage-20260926/` and
`.cache/integrated-recovery-20260926/`; never print saved credentials or run state.

Validation includes 133 distinct noninteractive platform cases with five documented
focused reruns, ten affected cases after the last lineage-query change, four actual
Linux/coding-host cases, 26 workspace passes, all-target Clippy, standard product
image/API/TypeScript/Vite build, final desktop and lifecycle checks. The aggregate
platform attempt was not uninterrupted green; the mapping preserves each failure,
fixture timing correction and unchanged rerun. Ignored cases and carried OIDC/
Enterprise checks are explicitly separated from fresh runtime proof.

Recovery (slice 28) is shipped under ADR 0013 and the
[archived pack](docs/roadmap/execution/archive/operations-recovery-drills.md).
The [runbook](docs/runbooks/recovery.md) gives verified named-installation commands;
[evidence](docs/mappings/recovery-2026-09-26.md) records every proof and its limits.
Actual age/SFTP backup, independent erasure/analytics journals, offline source loss,
fresh-volume restoration, failed upgrade and interrupted resume passed. Erased
capture stayed removed on replay; retained scoped history/recall/Neo4j queries
survived; the uncertain connector effect remained at one through receipt-only
reconciliation. The final killed-operator drill retired its lingering helper,
rolled back its tagged transaction and removed owned staging automatically.
The full interrupted restore/resume/verification took **28.848 seconds** for the
small synthetic corpus. No paid provider was used in the recovery drill.

Validation: 26 actual runtime scenarios; 26 workspace tests with runtime-dependent
ignored cases reported separately; all-target Clippy; generated API/TypeScript/Vite;
focused Python archive/configuration/TLS; final owner/member desktop 3.7 seconds;
governance and 32 checker tests. Restore's model row is an explicit synthetic
crash-state accounting control, not provider billing evidence. Production capacity,
off-machine survival and an installed daily schedule are not claimed.

The earlier recovery-slice proof remains `proof-recovery-resume-20260926`, loopback port 19791,
image `recollect-product:recovery-proof` (migration 025). Its predecessor writers
remain stopped with their volumes preserved; some deliberately contain the failed
upgrade fixture and must not be restarted blindly. The original source's databases
remain running but its API/worker are stopped. The owned SFTP fixture is
`recollect-recovery-2c32c543-16fb-4bc5-b2df-a0eaf4627f94`, loopback port 59710.
Private credentials, current journals, reports and screenshots are under
`.cache/recovery-drill-20260926/` and the matching SFTP fixture directory; never
print `state.json`, keys, runtime inputs or browser credentials. Existing personal
and shared installation proofs remain on their preceding accepted image.

## Resumed on the original Mac — 2026-09-22

Historical progression, superseded by the current continuation above. Runtime
states and unfinished-slice statements in this section describe their dated
checks and must not be used as current startup instructions.

**Docker recovery completed after explicit user approval.** Docker engine 29.7.2
answers, both normal database containers are healthy, and normal readiness is 200.
The bounded restart could not stop stuck helpers; the supported forced Desktop
stop followed by normal Desktop start succeeded. No reset, prune, volume removal
or Vault token/lease revocation occurred. The original slice-24 API/worker kept
running. Fresh inventory matches the retained pre-incident baseline: migration
023, seven Brains, 33 model requests, the exact `test` profile/grants and zero active
jobs/calls/instances/private registrations. The central lease renews. Actual SWEG
exact recall returns one item / 1,801 bytes / 74 ms, with no new paid model calls
or customer repository scans. Proof: `.cache/docker-recovery-20260922/`.

The user also requested build/Clippy cache cleanup. Removing only rebuildable
incremental state and obsolete workspace libraries reclaimed **12.0 GiB of actual
free space**, from 41.09 to 53.08 GiB immediately after cleanup. Subsequent test
builds consume some of that space. Current runnable binaries, external dependency
libraries, source, credentials and data were preserved. Details:
`.cache/build-cache-cleanup-20260922.json`.

**Historical runtime incident, 17:22 UTC:** the slice-25 Linux host build exhausted the Mac
disk. Recollect's rebuildable `target/debug/incremental` cache occupied 46 GB;
removing that cache restored about 27 GB of actual free space. Source, credentials,
databases and installed binaries were preserved. Docker's VM filesystem became
read-only (confirmed in its console/init logs), Docker API calls timed out, and the
normal service reported liveness 200/readiness 503. This outage is resolved by the
recovery above. Do not import fixture data or restart the normal app into unfinished
slice 25 before its acceptance passes.

The user approved the temporary Recollect Keychain prompt. The old native test
then failed while generating configuration against the unavailable service; it
does not establish a fresh native pass. Current configuration rendering is
credential/network-free. No credential-store gate was bypassed. The Linux fixture
uses an independent Secret Service and actual coding hosts.

The user chose to continue in `/Users/mike/devops/Recollect`. This checkout is
the active workspace again; the transfer details below are historical. Do not
resume remote work or overwrite the VM from this note. Before the incident,
PostgreSQL and Neo4j were healthy and the normal database reached migration 023.
The existing UI address is `http://127.0.0.1:8787`, using owner credentials in the
ignored `.env`; dependency readiness is healthy again. `./scripts/dev.sh` remains
the native startup command; the original running process has been preserved.

Both open analytics source-review issues have been repaired: the ownership
journal compares every immutable attempt field, and report reads serialize their
final metadata check against concurrent observed invalidation. The independent
strict-mode fixture and browser readiness/error assertions were corrected. All
93 platform integration tests pass, including eleven analytics scenarios with
actual older-database replay, interrupted attempts, replaced leases, both reader
orderings, final clock gates and real GDS termination through product cleanup.
Workspace tests (13), Clippy, API generation (125 operations), governance (32)
and analytics/existing graph browser checks pass. A real SWEG WCC report is ready
with 99 vertices, 183 relationships and confirmed scratch cleanup. Normal graph,
exact/lexical/qualified recall, autonomous replacement and capture recall are
preserved. All seven Brains and 33 model requests remain; no new model call or
customer file read was needed.

Desktop graph exploration is now delivered through Cytoscape and native Neo4j
reachability. Actual scope/direction/hop, excluded-hub, reader/device, damaged
projection, display-limit and hidden-input retention proofs pass. The browser
covers pointer/keyboard evidence inspection, shortest paths, scope/error clearing,
clock expiry and renderer disposal. A 500-node/2,000-edge renderer case takes
about 1.4 seconds here; the actual API maximum is separately proven. Normal SWEG
overview returns 99 nodes/183 edges, with a two-hop view of 74/151. No new model
request or customer file read was needed. Proof: `.cache/exploration-proof/`.
The user explicitly deferred mobile views on 2026-09-22: focus on desktop and
do not spend additional work on mobile.

Graph retrieval fusion is delivered with canonical qualification, native witnesses,
one reciprocal-rank reducer and bounded source coverage/depth. A full platform
run passed 98 cases; two additional cases passed separately, with workspace,
Clippy, API/web and desktop browser proof. The actual-model comparison ran 64
synthetic queries in an isolated database, using 33 embedding requests and 1,227
tokens. Combined semantic/graph found all six target claims, but both unsupported
questions still returned unrelated semantic context; no reliable abstention or
general superiority is claimed. Normal SWEG recall found 73 connected candidates,
returned six attributed records in 15,735 bytes / 152 ms, and preserved the same
seven Brains and 33 existing model requests without customer reads or new charges.
See the [fusion mapping](docs/mappings/retrieval-fusion-2026-09-22.md),
[runbook](docs/runbooks/graph-recall.md) and
[archived pack](docs/roadmap/execution/archive/retrieval-graph-fusion.md).

Desktop memory investigation is delivered: match/disagreement/source views,
frozen claim history, exact evidence, optional review and native same-scope graph
navigation. Seven browser scenarios and eleven affected retention scenarios
passed; four graph/investigation cases passed again after visual refinement.
Normal SWEG proof returned one exact fact (1,630 bytes, 60 ms), inspected its source
and opened 74 graph nodes/151 edges without another model call or filesystem read.
All seven Brains, 33 model requests and migration 020 are preserved. See the
[investigation runbook](docs/runbooks/memory-investigation.md) and
[dated proof](docs/mappings/retrieval-investigation-2026-09-22.md).

The approved MCP catalogue and execution profiles are delivered, including
independent Use/Manage/Share, schema-validated connections and cold discovery.
Five real database/API/RLS tests, three affected regressions, two desktop flows,
13 workspace tests, Clippy and the 136-operation generated API pass. Normal
migration 021 preserved the same seven Brains, 33 model requests and zero active
jobs; the normal catalogue is empty and no fixture was imported. SWEG exact recall
still returns one item / 1,630 bytes / 61 ms without paid model calls. See the
[catalogue runbook](docs/runbooks/mcp-catalogue.md) and
[proof](docs/mappings/mcp-catalogue-2026-09-22.md). The pre-021 dump and inventories
are in `.cache/mcp-catalogue-proof/`.

Slice 23 is delivered with accepted ADR 0008 and its runtime contract. The shared
rmcp runtime passes nine actual transport/process/credential integration cases,
including the follow-up Atlas/Cognee audit's timeout, backpressure, shutdown,
redaction and notification findings. The registered migration 022 and durable
coordinator pass three database/API/RLS tests for admission, dispatch, revocation,
local routing, expiry and late receipts. The normal database is now at 022.
The shared executor and private receipt outbox are now wired into the central
server and paired native runner/CLI. Actual central and paired-local stdio and
HTTP execution, effect-once receipt reconciliation, restart receipt-only upload,
source-retention resolution and desktop Run/cancel/output/revocation proofs pass.
The final 16 MCP integration cases, four affected desktop flows, 17 workspace
tests, Clippy and 32 governance tests pass. Recovery fixes preserve late failed
receipts, prevent expired startup from spawning providers, and avoid cleanup
starvation. The heavy MCP executor runs in a sibling `recollect-mcp-runner` binary;
install it beside `recollect-agent`. A cold capture hook completes in 535 ms.
The normal upgrade preserves seven Brains, 33 model requests and the newly created
`test` profile with its exact grants. The live central runner is healthy; there
are no approved definitions, connections or calls. Exact SWEG recall and desktop
inspection pass without new paid model calls or customer reads.
Private pre-022 dumps and inventories are in `.cache/mcp-runtime-normal-proof/`;
`pre-022-current.dump` includes the new profile, unlike the earlier snapshot.
See [current MCP evidence](docs/mappings/mcp-runtime-2026-09-22.md) and the
[archived pack](docs/roadmap/execution/archive/mcp-runtime-and-credentials.md).

**26 of 29 original slices are shipped locally**, plus the Atlas capture repair.
Slice 24 is delivered under accepted ADR 0009 and its archived pack. Registered
private runners use the paired host, exact Brain and current caller/host authority.
Actual native stdio/HTTP, other-caller execution, interrupted receipts, disabled
hosts and desktop registration/configuration pass. Seventeen database/API/SDK
cases, nine transport cases, 20 workspace tests, Clippy, generated API/web build
and 32 governance tests pass. The normal 023 upgrade preserves seven Brains,
33 model requests, zero active jobs and the exact existing `test` profile/grants.
The live central lease renews, no test calls/instances/registrations were imported,
and desktop SWEG recall returns one item/1,630 bytes/62 ms without model charges
or customer file reads. Fresh backup and inventories:
`.cache/mcp-vault-normal-proof/`. See the
[Vault/private evidence](docs/mappings/mcp-vault-private-2026-09-22.md),
[runbook](docs/runbooks/mcp-vault-and-private-runners.md) and
[archived pack](docs/roadmap/execution/archive/mcp-vault-and-private-runners.md).

**Vault is optional for every connection.** Environment, OS-store, Vault and
anonymous connections can coexist; unused/unavailable Vault configuration does
not block another provider. Three actual Vault tests prove coherent KV delivery,
static rotation, dynamic renewal beyond the original TTL and natural expiry.
The user authorized the existing Vault Enterprise connection on 2026-09-22: use
the inherited root credential only to bootstrap a new dedicated namespace, then
scoped AppRole for runtime. Existing namespaces/mounts must remain unchanged.
The owned namespace is `recollect-dev-20260922-dd66f852/` (ID `beqTP`). Its KV and
AppRole/Proxy calls pass; Enterprise access to the Mac fixture database failed
with `no route to host`, so dynamic credentials are proven against owned local
Vault/PostgreSQL instead. Leave the namespace and test setup in place.
The user explicitly prohibits revoking any Vault token. Do not call token/lease
revoke APIs or perform indirect revocation by namespace deletion/auth disablement;
leave this dedicated setup in place and use natural TTL expiry. Shell `unset`
is not a sufficient boundary; scope bootstrap requests and child credentials explicitly.

A local dev Vault fixture initially wrote its generated token to `~/.vault-token`.
This was detected and disclosed. The helper was restored to the existing
Enterprise token from the environment; an actual helper-based lookup verifies
root authentication. Original helper bytes were not recorded, so exact historical
file identity is not claimed. No token was revoked. Versioned fixture startup now
uses `-dev-no-store-token`, and another actual startup preserved the helper.
API/worker processes exclude the root token; the adapter uses Proxy HTTP and
never invokes Vault CLI/token helpers. Evidence:
`.cache/vault-enterprise-proof/helper-restoration.json`.

Slice 25 **mcp-memory-and-workspace-tools** is delivered under ADR 0010 and its
[archived pack](docs/roadmap/execution/archive/mcp-memory-and-workspace-tools.md).
Actual Codex 0.154.0 and Claude Code 2.1.270 discover/call tools and receive fresh
scope context on the next model turn. Linux Secret Service, native bridge,
managed effects/receipt recovery/cancellation, independent old/child scopes and
capture refresh failure pass. Recalled memory is excluded from new captured
evidence while real prompts remain captured. Linux Codex explicitly forwards
DBus/XDG variable names for its OS credential store. Actual PostgreSQL/Neo4j MCP
tests prove scoped synthesis, 22 permitted handovers across two pages versus two
other-child runs, graph traversal, current permissions, rejected/expired/erased
exclusion and committed scope changes with failed then recovered retrieval.
Both desktop flows pass again in 18.6 s; screenshots were inspected. Workspace
25 passed/143 explicitly skipped, all-target Clippy and governance pass.
Evidence and exact commands are in the [mapping](docs/mappings/mcp-tools-2026-09-22.md)
and [runbook](docs/runbooks/agent-memory-tools.md). The ordinary native process is
still slice 24; no automatic deployment is implied. The full goal remains unfinished.

Slice 27 **operations-local-and-shared** is now shipped locally under ADR 0011
and its [archived pack](docs/roadmap/execution/archive/operations-local-and-shared.md).
Both actual Docker installations pass owner/member isolation, retained evidence,
SIGTERM/SIGINT draining, migration-failure startup, dependency outage and restart
persistence. The shared browser approves a real Linux companion whose OS-store
credential powers fresh native processes and actual HTTPS MCP memory recall.
Untrusted TLS and wrong hostnames are denied; the browser fixture's certificate
exception is explicitly distinct from those validating native/Python clients.
Actual service environments, logs and resource limits pass private boundary checks.
Workspace 25 passed/144 explicitly skipped, all-target Clippy, TypeScript and
32 governance cases pass. Desktop screenshots were inspected; mobile is deferred.

Current evidence: [installed acceptance](docs/mappings/installation-2026-09-26.md)
and [installation runbook](docs/runbooks/installation.md). The two proof instances
are healthy at `http://127.0.0.1:18787` and `https://localhost:8443`; configuration
and owner credentials stay in their private `.data/install/<name>/runtime.env`.
Their existing data survived both installer restarts and the intervening stopped
period. The normal development databases were observed stopped on September 26,
and remain preserved. Earlier slice-24 runtime health statements are historical.

`mcp-observation-capture` is in progress under ADR 0012 and its accepted contract
and active pack. Its source research confirms reuse of runtime receipts and
canonical capture; it is not yet shipped. Next are `operations-recovery-drills` and
`operations-integrated-evaluations`, following their governing packs and the
[current roadmap](docs/roadmap/epics/index.md). No new per-slice approval is needed.
Historical private pre-020 backups remain in `.cache/analytics-proof/resume-20260922/`.
Changes remain uncommitted. The sections below retain the original transfer
snapshot and earlier failure notes as history, not current unfinished requirements.

## Transfer outcome as of 2026-09-16

**17 of the original 29 product slices are shipped locally.** An additional
Atlas-driven `memory-capture-reconciliation` repair is also shipped. Slice 18,
`graph-analytics`, has substantial implementation but remains **in progress**.
Twelve original slices remain, counting unfinished analytics. External deployment
and release are not part of the authorized goal.

The existing Git HEAD is `1cbde89b09c2f8afd7c7ed952dc2c8f4e58c721b`, subject
`manual commit`, dated 2026-09-16 16:38:18 +0200. It includes unfinished analytics.
The working tree was clean when this handoff began; these handoff documentation
changes are additional and uncommitted. Do not mistake a commit for acceptance.
Do not discard existing work or rewrite applied migrations.

The normal PostgreSQL database is at **019_combined_graphs**. Migration 020 exists
in source and has run in disposable tests, but **has not run against the normal
database**. The existing normal browser bundle is the slice-17 build. New analytics
browser source and a separate test bundle exist.

Both original Docker services were already stopped when the transfer was prepared:
PostgreSQL exited 0 at 2026-09-16 13:04:45 UTC; Neo4j exited 137 at 13:04:48 UTC,
with `OOMKilled=false`. The old API process still answered `/health/ready` with
HTTP 503 / `ready:false`. No current healthy runtime is claimed. Neither original
service was restarted to make this export.
On a later pre-transfer check, the old API/dev-parent processes had also exited.
There is no running source UI to carry across machines.
The source database containers were subsequently observed restarted at 15:55:32
UTC. That changed Neo4j's files after the stopped-state archive was made. The
destination comparison detected the difference: its initially rsynced Neo4j
directory was preserved under
`.cache/pc-handoff-2026-09-16/copied-neo4j-before-restore/`, and `.data/neo4j` was
restored from the known stopped-state archive. All retained destination files
then matched all 156 archive members byte-for-byte. A source SQL recheck at
16:14:50 UTC still showed migration 019, seven Brains, 99 facts, 33 model requests,
zero active jobs and a latest mutation-audit entry from 2026-09-15. No new source
application/worker was running. Do not overwrite the destination's restored
Neo4j directory with another live source-directory rsync.

## Implemented and validated scope

| Original slices | Delivered capability |
| --- | --- |
| 1–4 | Rust API and native companion; React/Mantine UI; PostgreSQL/pgvector and Neo4j/GDS; local owner, invited accounts and optional OIDC; Brain grants/isolation; audited idempotent commands; durable leased jobs; browser-approved, revocable native device pairing. |
| 5–7 | Retained/reference sources with immutable versions; collections, areas and environments; workspace discovery, task/subagent operation scope; exact committed repository publication through Enola; snapshot/fact/contributor provenance and immutable environment manifests. |
| 8–12 | Evidence-linked claims and separate fact/knowledge time; trust/operational state; review, corrections and durable rejected-value rules; retention, erasure and older-backup replay; governed model gateway and autonomous learning/revision/retirement; advisory procedures and refreshed handovers. |
| 13 | Native Codex/Claude session/tool hooks, sanitization, explicit host coverage, durable offline delivery, original scope and central processing. |
| 14–15 | Exact identity and lexical recall; full-dimensional semantic representations and exact eligible cosine search; reciprocal-rank fusion of existing channels; canonical scope, correction, retention and source attribution in browser/native recall. |
| 16–17 | Repository and knowledge projections, automatic processing/recovery, canonical entity/evidence views and bounded native shortest paths; exact manifest-selected combined graphs and parsed full-commit Terraform module links across repositories. |
| Additional repair | Atlas review fixes for own-recall recapture, model input attribution, delayed capture knowledge time, bounded session correction/revision, capacity accounting and associated recall/retention gates. |

Normal learning, revision, retirement and retention maintenance are autonomous
under the Brain's standing policy. Human review/correction/Erase controls are
optional overrides. Do not introduce a mandatory per-memory review queue.
Graph analytics is explicitly queued resource-intensive work; this does not
change the autonomy of memory maintenance.

The configured defaults are `gpt-5.6-luna` and `text-embedding-3-large` with
3,072 dimensions. `OPENAI_API_KEY` is present in the ignored `.env`. Actual
synthetic provider calls have been verified in previous slices. This handoff
does not independently verify current pricing or general model superiority.
No new paid requests were made for analytics or transfer preparation.

The selected product owns its Rust core. Cognee is a read-only reference,
not a mandatory Python runtime. PostgreSQL and retained artifacts are canonical;
Neo4j and semantic representations are derived consumers of current eligibility.

## Evidence and lessons from Cognee, Atlas and other sources

- The user-requested independent Atlas review read all 21 pattern files plus
  the index and tensions: 23 files / 7,212 lines. The complete disposition matrix,
  concrete repairs, theoretical deferrals and evidence limits are in
  [the implementation audit](docs/mappings/atlas-implementation-audit-2026-09-15.md).
- Cognee has a graph-provider abstraction, a Neo4j adapter, native GDS metrics,
  NetworkX ranking helpers and an existing React force-graph/D3 frontend.
  Recollect reuses **Neo4j/GDS algorithms and native Cypher**, not a custom Rust
  graph database or PageRank/community algorithm. Evaluate an established
  renderer/layout for slice 19. See
  [the reuse review](docs/mappings/graph-reuse-review-2026-09-15.md).
- Do not copy Cognee's whole-database `myGraph` projection into a shared Brain
  instance. Every contributor must satisfy the exact canonical scope before
  traversal/ranking; filtering returned entities cannot remove hidden aggregate
  influence. The inspected Cognee default was Ladybug, while its published guide
  still described Kuzu. Those were different observed source states.
- Graphiti, HippoRAG, Basic Memory, Graphify and Neo4j Agent Memory supplied
  bounded reference comparisons. Their behavior was not universally reproduced
  or benchmarked. Atlas reports do not override Recollect's accepted contracts.
- Current native GDS 2026.08.1 probes passed for projection, isolates, PageRank,
  Leiden, WCC, estimates, guarded creation and exact transaction termination.
  Context7 returned older GDS 2.13 guidance; current official docs and installed
  calls resolved that mismatch. See
  [analytics interface evidence](docs/mappings/graph-analytics-interfaces-2026-09-15.md).
- A correction or erasure must invalidate the **whole** analytical result.
  Source materialization can change eligible topology without changing the memory
  epoch; expiry of a withheld conflicting claim can admit another node. Analytics
  therefore adds its own epoch and a conservative retention deadline.
- Capture provenance must distinguish an observed tool result, a user statement,
  an assistant proposal and quoted history. Repeated recall must not manufacture
  new independent support. Controlled real-Luna session proof exercised those cases.
- Semantic similarity returned unrelated context on unsupported questions in the
  measured corpus. Similarity and graph centrality do not establish truth or
  deployed behavior. General entailment, universal abstention and arbitrary
  cross-document conflict resolution remain unproved.
- Adaptive tiers, continuous decay/reinforcement scores, retrieval hysteresis,
  learned expensive-path gates and executable learned skills are deliberate
  deferrals, not features to claim implemented merely because Atlas describes them.

Reference checkouts were clean and preserved:
`cognee/` at `c0d18c80e24b7b78918e7642c03f6f128fdd2aee`;
`agent-memory-atlas/` at `7eca7f7abd934c2e44bc096fc1dd0cbf94275b99`.

## Exact unfinished slice-18 state

Authority: [accepted contract](docs/contracts/graph-analytics.md),
[now archived pack](docs/roadmap/execution/archive/graph-analytics.md),
[owning epic](docs/roadmap/epics/graph-intelligence.md).

Implemented in source:

- Migration 020: analytical reports/attempts, dedicated Brain analytics epoch,
  canonical invalidation, job state integration, cleanup ownership and scoped functions.
- `crates/server/src/graph/analytics.rs` and `graph/analytics/{api,native,journal,work}.rs`:
  queue/list/view, fixed native GDS PageRank/Leiden/WCC, exact frozen inputs,
  10,000 candidate / 50,000 edge / 64 MiB admission limits, 120-second attempts,
  staged scores, cleanup before publication and durable attempt journals.
- Shared Query API adapter, privacy replay and worker integration. Separate
  Cypher MATCH clauses fix a discovered self-loop import defect. Batched raw-rule
  qualification and a qualification-local scope cache reduce repeated SQL queries;
  explicit fact-time queries retain the existing typed per-row matcher.
- Protocol/API generation and `web/src/AnalyticsPanel.tsx`: selection, queue,
  progress/cancel, reports, provenance, ranked/grouped rows and evidence inspection.
- Tests in `crates/server/tests/platform/graph_analytics*.rs` and
  `web/tests/graph-analytics.spec.ts`. UI tests build in `.cache/ui/web-dist`.

**Two concrete remaining backend findings from the final read-only review:**

1. `graph/analytics/journal.rs`, `entries()` around line 148, compares only
   Brain/job/lease against the canonical attempt. It must also compare immutable
   `report_id`, `privacy_sequence`, `created_at` and `deadline`. A mismatching
   sequence can otherwise skip necessary erasure cleanup. Add a regression that
   changes only the journal sequence while owned native scratch exists; erasure
   must refuse completion until correct ownership is restored.
2. `graph/analytics/api.rs`, report view around line 123, can retain ready scores
   while another shared-lock reader commits `recollect_analytics_observed_stale`.
   Add a serialized final report-state gate consistent with lock ordering, then
   prove a paused reader cannot return its saved ready rows after stale state commits.

Those findings are reviewed source-level gaps, not executed exploit claims, and
are **not fixed** in this handoff. Earlier fixes for lock order, missing journal
markers, persisted observed staleness and final deadlines are already in source.
The new findings show why the slice remains open.

Latest executed checks, with failures preserved:

| Check | Actual result and location |
| --- | --- |
| Last fully closed slice 17 | 79 platform integrations passed in 221.27s; 13 workspace tests; Clippy; two graph browser flows in 17.5s; 32 governance tests. `.cache/cross-graph-proof/` and the archived pack record this historical baseline. |
| Latest six analytics integrations | Five passed, one failed in 27.48s: native recipes/direction/isolate/self-loop/parallel edges, correction/erasure, delayed creation cancellation, malformed output/estimate refusal, and large paired scope were exercised. `.cache/analytics-proof/focused-repaired.log`. |
| Materialization follow-up | After correcting the initial node-count assertion, a later strict-mode queue returned `analytics_empty`; fixture still needs an independently accepted positive control. `.cache/analytics-proof/materialization.log`. Do not weaken strict eligibility to satisfy the fixture. |
| Browser analytics | Build/typecheck succeeded. The test reached real reports, evidence, stale invalidation, recomputation and desktop/mobile screenshots, then failed its final error-text assertion. It expects `There are no eligible vertices...`; actual API message is `No eligible vertices exist in this selection. Choose a ready input with retained evidence.` `.cache/analytics-proof/ui.log`. Full browser acceptance has **not** passed. |
| Native cancellation probe | Real GDS progress/transaction termination passed with deliberately long stress parameters. This is interface proof, not complete product-path cancellation/recovery acceptance. |
| Transfer database proof | Fresh logical dump restored successfully into an isolated database; counts match. See transfer files below. This is not completion of operational recovery slice 28. |

Desktop/mobile analytical report images were visually inspected at
`.cache/analytics-proof/{desktop,mobile}.png`. The whole latest tree has **not**
passed a full platform/Clippy/browser regression after slice-18 edits.

Finish the two backend repairs and meaningful regressions, fix the two test
fixtures, complete remaining cancellation/lost-lease/crash/older-restore and
missing-journal acceptance, then run focused tests and appropriate shared retrieval
regressions. Complete full checks, normal migration/restart/preservation proof and
documentation closeout before archiving the pack. Test databases retained after
failures are diagnostic state; the portable export contains the normal database,
not every failed fixture database.

## Remaining original slices, in authorized order

18. `graph-analytics` — finish the work above.
19. `graph-exploration` — established renderer/layout, interactive path and impact views.
20. `retrieval-graph-fusion` — eligible graph contributions, source diversity and measured fusion.
21. `retrieval-investigation-ui` — combined evidence, provenance and investigation workflow.
22. `mcp-catalogue-and-profiles` — approved Brain/environment connections and separate use/manage/share authority.
23. `mcp-runtime-and-credentials` — real transports, credential references, startup/reuse/lease/idle lifecycle.
24. `mcp-vault-and-private-runners` — Vault delivery/renewal/rotation and authenticated private execution.
25. `mcp-memory-and-workspace-tools` — agent-facing shared handlers, actual host recall/scope integration.
26. `mcp-observation-capture` — attributable managed-tool outcomes with retention and safe replay.
27. `operations-local-and-shared` — complete local/shared operating setup and resource behavior.
28. `operations-recovery-drills` — integrated fresh-instance/older-backup recovery and journal proof.
29. `operations-integrated-evaluations` — capability, quality, concurrency, latency, storage and cost proof.

The project-local Context7 MCP is developer tooling. It does **not** mean the
product's MCP coordinator, Vault integration or private runners are implemented.
No customer Vault/cluster target is selected or authorized by this transfer.

## What is being transferred

The PostgreSQL source is the Docker volume `recollect_postgres_data`; copying the
project directory alone cannot copy it. The transfer preparation mounted that
stopped volume read-only into a disposable network-disabled container, copied it
inside that container, started PostgreSQL only on the copy, dumped `recollect`,
and successfully restored the dump into a second isolated database. The temporary
container was removed. Original services and database files were not changed.

These ignored files are included in the project transfer:

| Path | Purpose |
| --- | --- |
| `.cache/pc-handoff-2026-09-16/recollect.dump` | Portable PostgreSQL custom-format dump, 1,030,632 bytes; migration 019. |
| `.cache/pc-handoff-2026-09-16/retained-data.tar.gz` | 78,536,643-byte archive of `.data/artifacts`, `.data/erasure-journal` and `.data/neo4j` including plugins. All archive members were read successfully. |
| `.cache/pc-handoff-2026-09-16/database-state.json`, `restored-state.json` | Matching source-copy/export and restored database counts. |
| `.cache/pc-handoff-2026-09-16/brain-inventory.json` | Exact seven Brain identities and per-Brain model counts. |
| `.cache/pc-handoff-2026-09-16/export.log`, `archive-check.json` | Executed export/archive verification. |
| `.env` | Existing private local credentials/provider key; copied without displaying values. Preserve permissions. |
| `.data/`, `.cache/*-proof/` and other retained evidence | Runtime data, experiment state, logs and screenshots. Preserve journals; never substitute an empty initialized journal. |
| `.git/`, `.codex/`, `.agents/`, `cognee/`, `agent-memory-atlas/` | Git state, project tooling/instructions/skills and separate reference checkouts. |

Exported state: seven Brains, one repository snapshot, 99 facts, five graph
generation records, zero queued/running jobs, and 33 recorded model requests.
SWEG's retained Brain has zero model calls. Its last full live graph proof was
99 eligible vertices / 183 edges on the exact stored snapshot and manifest.
The export was created at 2026-09-16 15:49:29 UTC. Neo4j destination startup and
cross-OS runtime behavior still require verification; its original shutdown was
an exit 137, so allow native recovery and check readiness explicitly.

Rebuildable exclusions: `/target/` (about 46 GiB), `/web/node_modules/`,
`/.codex/tools/codegraph/node_modules/`, `/.codegraph/`, `/.cache/codegraph/`,
and `/.cache/enola-tools/`. Rust, npm/CodeGraph and Enola include platform-specific
binaries. In particular, Enola setup skips an already existing binary, so a copied
Mac executable would prevent automatic installation of the Linux one. Other proof
files under `.cache` are retained. Do not exclude all hidden files or all `.cache`.

The native companion's OS credential-store entries and the old machine's personal
Codex configuration are outside the repository. Pair a new device on the new PC;
do not assume the old keychain token was transferred. Re-discover workspace paths
and regenerate applicable hook bindings for `/home/mike/...`. Historical immutable
snapshots/manifests remain valid without rewriting their provenance. The separate
customer SWEG checkout is not copied by this Recollect-only transfer.

## Restore and resume on the new PC

Prerequisites are Rust/Cargo, Node/npm, Python 3, Docker Compose and SSH access.
Destination preflight confirmed Linux x86_64, Docker 29.7.2, Compose v5.5.0,
Python 3 and Google Chrome. Rust/Cargo and Node/npm were not on the inspected SSH
path, so install or expose them before building. The final destination disk check
reported about 134 GiB free (the initial preflight showed 44 GiB). Continue to
monitor space as Rust debug/test caches grow.
Previous application proof used Rust 1.94, Node 26 and Docker 29 on macOS ARM.
Check the destination's actual tools. Browser tests expect Google Chrome through
Playwright's `chrome` channel; native keyring behavior outside macOS still needs
its own verification. Follow `.codex/README.md` for project trust and Context7;
the repo's `.env` is not automatically loaded into the Codex process.

1. Start in `/home/mike/devops/Recollect`. Inspect this handoff and Git status.
   Keep `.env`, artifacts and the latest journal. Full rsync includes `.data`;
   the retained-data archive is an additional snapshot. If restoring from the
   archive instead, extract it only into the intended fresh data location before
   starting services; do not overwrite unrelated/newer live data.
2. **Do not run `dev.sh` yet:** it automatically applies migration 020. First
   restore the normal database and complete the unfinished analytics repairs/tests.
3. On a fresh destination installation, initialize only the local services and
   restore into the empty normal database:

   ```sh
   cd /home/mike/devops/Recollect
   ./scripts/setup-local.sh
   ./scripts/docker.sh compose up -d --wait --wait-timeout 300
   test "$(./scripts/docker.sh compose exec -T postgres psql -U recollect_admin -d recollect -Atqc "SELECT count(*) FROM pg_tables WHERE schemaname='public'")" = 0 || {
     printf 'Destination database is not empty; inspect it before restoring.\n' >&2
     exit 1
   }
   ./scripts/docker.sh compose exec -T postgres pg_restore -U recollect_admin -d recollect --exit-on-error < .cache/pc-handoff-2026-09-16/recollect.dump
   ./scripts/docker.sh compose exec -T postgres psql -U recollect_admin -d recollect -Atqc "SELECT max(name) FROM recollect_migrations; SELECT count(*) FROM brains; SELECT count(*) FROM repository_facts; SELECT count(*) FROM model_requests;"
   ```

   Expected output: `019_combined_graphs`, `7`, `99`, `33`. These commands are
   destination instructions; no destination restore is claimed by this handoff.
   The dump preserves owners/ACLs; Compose initialization creates the required
   `recollect_admin` and `recollect_app` roles using the copied environment.
4. Rebuild tools for the destination and run disposable tests after the fixes:

   ```sh
   cargo build --workspace
   npm --prefix web ci
   python3 scripts/setup-enola.py
   ./scripts/setup-codegraph.sh
   set -a
   source .env
   set +a
   cargo test -p recollect-server --test platform graph_analytics_ -- --ignored --test-threads=1
   ./scripts/test-ui.sh tests/graph-analytics.spec.ts
   ./scripts/test-platform.sh
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   ./scripts/validate.sh
   ```

   Tests create isolated owned databases; they do not establish the normal
   migration/runtime result. Do not blindly replay paid model proof scripts.
   Some old `.cache` scripts contain absolute Mac paths; inspect and adapt them
   before execution. `./scripts/validate.sh` checks docs/governance, not Rust runtime.
5. Once slice-18 repairs and acceptance are ready for normal proof, preserve the
   migration-019 backup above and run `./scripts/dev.sh`. This applies 020,
   reconciles privacy/analytics ownership, builds the browser and starts API/worker.
   Open `http://127.0.0.1:8787` on that PC and verify `/health/ready`. From a separate
   laptop, `ssh -o ProxyCommand=none -L 8787:127.0.0.1:8787 mike@192.168.0.41`
   can forward its loopback UI after the destination application is running.
   Verify seven Brains, retained
   SWEG scope/graph/recall, capture/revision history and unchanged model counts.
   Do not close slice 18 on a successful build alone.
6. Pair the companion using `cargo run -p recollect-agent -- pair "New development PC"`
   and the browser approval flow when native operations are needed. Recreate host
   bindings under the correct workspace paths. Use existing runbooks; credentials
   must stay out of model arguments, source, docs and logs.
7. Reconcile the analytics contract/mapping/runbook, pack, epic and indexes only
   after acceptance passes. Archive that pack, then continue slice 19 onward.

## Executed destination transfer checks

Rsync completed successfully, preserving Git/reference repositories and private
file modes, with approximately 4.14 GB of included files and the platform-cache
exclusions above. It did not delete unrelated destination files. An additional
sync carried the final handoff edits. Follow-up syncing deliberately excludes
`.data/neo4j` because that destination now contains the verified stopped snapshot.

On Linux, the verification script confirmed the Git HEAD, required instructions
and implementation files, private environment/dump permissions, excluded native
caches, backup inventory and byte-for-byte retained-data agreement. Governance
lint and all 32 documentation tests passed on the VM; `git diff --check` passed.
Evidence is under `.cache/pc-handoff-2026-09-16/`, including
`destination-verification.json`, `destination-governance.log` and
`destination-result.txt`. This does not claim a Linux application test pass.
**The destination PostgreSQL database has not been restored, and no destination
Recollect application or database service was started by this transfer.**

## Instruction to the next agent

The user authorized completing **all 29 product slices** in dependency order,
continuing across slice/epic boundaries without per-slice approval. Routine
ADRs/contracts, decision-complete execution packs, implementation, meaningful
tests and repository-owned local services are authorized. Do not stop at planning
or at one slice. Do not mark the full goal complete before the full scope passes.

Keep work inside Recollect; preserve credentials, user commits, unrelated state,
customer checkouts and both ignored reference repositories. The user's 2026-09-26
instruction authorizes committing and pushing the completed existing changes to
private GitHub. It does not authorize external deployment. The user explicitly
requested no new product hashing or strict version gates;
follow accepted ADR 0003 while using ordinary compatible dependency lockfiles.

Use accepted foundations and docs, share existing handlers, and reuse established
engines/libraries rather than inventing replacements. The user authorized a
separate read-only Atlas/Cognee cross-reference agent; implementation delegation
was not requested. Re-read the applicable delegation rule before creating agents.
Continue automatic learning/revision/forgetting under standing policy; human
review remains optional. Preserve exact scope, correction, erasure, provenance
and recovery at every new consumer. Record actual test/runtime evidence and
unresolved limitations honestly. Follow the current resumed-on-Mac frontier above;
the original slice-18 gaps in the transfer snapshot are historical and resolved.
