# Atlas-Informed Recollect Foundation Decisions

Observed: 2026-09-13
Confidence: inferred

This mapping records the research behind the user's accepted foundation plan and
subsequent clarification of MCP/Vault, Brain areas, collections and graphs. The
[foundations](../foundation/README.md) state the selected direction; this mapping
is evidence and interpretation, not a substitute for implementation contracts.
It follows the earlier [feasibility assessment](atlas-product-feasibility-2026-09-13.md),
whose Cognee-first recommendations describe a superseded decision stage.

Operating-model amendment, 2026-09-14: [ADR 0006](../adr/0006-autonomous-memory.md)
supersedes this mapping's earlier assumption that conflicts or nonliteral learning
require human review. Normal maintenance is autonomous under standing policy;
uncertainty is a valid automatic outcome and human correction is optional. The
tables below preserve the original research decision stage. The
[maintenance proof](procedures-and-handovers-proof-2026-09-14.md) records implementation.

## Sources and Method

Three user-requested research agents divided memory patterns/lifecycle,
storage/retrieval/graph choices, and Rust/platform/host integration. The parent
agent reconciled their findings against the existing foundations, the preserved
original vision, official sources and the user's explicit preferences.

- Atlas checkout: 7eca7f7abd934c2e44bc096fc1dd0cbf94275b99; revision rechecked
  during this update. See the [checkout mapping](atlas-reference-2026-09-13.md).
- Cognee reference: c0d18c80e24b7b78918e7642c03f6f128fdd2aee, version 1.5.4;
  revision rechecked. Relevant source paths and other system pins are in the
  earlier feasibility report. No reference checkout was modified.
- Pattern coverage: all 21 pages' intent, mechanism, tradeoff and testing sections;
  the full tensions page; rubric/build guidance; relevant comparative sections;
  targeted benchmark methodology, baseline, cost/latency, correction/deletion,
  scorecard and limitation sections. Not every example paragraph or system report
  was read, and this is not an exhaustive audit of the Atlas corpus.
- The storage review compared PostgreSQL/pgvector with relational edges, Neo4j/GDS,
  LadybugDB and a Rust analytics worker. The platform review inspected official
  Rust/MCP/UI/authentication, Enola/CodeGraph, host-hook and operations guidance.
- Context7 resolved and returned documentation for /neo4j/graph-data-science,
  /websites/developer_hashicorp_vault and /pgvector/pgvector during finalization.
  Its GDS excerpts referenced 2.13, so current official GDS documentation was
  checked before recording current edition limits. Earlier platform research
  found older Vite/TypeScript examples; exact frontend compatibility remains
  untested. No finalization lookup failed; successful lookup is not runtime proof.
- No product service, model benchmark, extractor build, new toolchain or runtime
  integration was executed. This update changes documentation and runs the
  existing governance checks only.

Primary Atlas inputs at the inspected revision are
[patterns](https://github.com/neoneye/agent-memory-atlas/tree/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/patterns),
[comparative report](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/overview.md),
[tensions](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/tensions.md),
[build guidance](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/build.md), and
[benchmark analysis](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/benchmarks.md).

Source existence and inspected behavior are observations. Architectural fit is
an inference accepted as the product direction; comparative superiority,
capacity, full language coverage and operational readiness remain unproven.

## Confirmed Product Decisions

| Decision | Accepted outcome |
| --- | --- |
| Ownership and language | Recollect owns its domain and Rust core/companion; Cognee is a reference, not mandatory runtime |
| Audience | One person or an internal team; customer/project content can live in Brains, without customer-user SaaS federation |
| Deployment | Same self-hosted stack locally or on a shared private server, with native companions/private runners |
| Organization | Brains enforce knowledge access; collections group sources; areas/environments are overlapping views |
| Graph scope | Repository and knowledge graphs, supported cross-repository traversal, centrality, clustering and large-graph analysis in the first usable version |
| License cost | No paid database or graph-analytics licenses; infrastructure and model charges remain possible |
| Learning | Named policies may accept routine supported facts; conflicts, overrides and high-impact claims need review |
| Capture | Automatic sanitized supported prompts, replies and tool traces under Brain policy; expose coverage gaps |
| Session retention | 30 days by default with per-Brain override; permitted supporting excerpts have separate retention |
| Retrieval | Labeled proposed/disputed material in investigations; separate strict accepted/operational mode |
| Model use | Brain-approved provider/model/purpose/content classes across generation, embeddings and external reranking |
| Identity | Local owner/invited accounts and optional OIDC, with Entra an integration target where used |
| Removal | Separate Withdraw and Erase; disclosed backup expiry and erasure enforcement on restore |
| Managed tools | MCP coordinator, Brain/environment profiles, separate execution grants, local/central/private runners and 15-minute useful-idle lifecycle |
| Credentials | Vault integration retained; OS-store/environment alternatives avoid mandatory Vault installation for personal use |
| Sequence | Foundations first, then derived product epics and detailed implementation contracts/packs |

The user explicitly reaffirmed dataset-like capabilities and both graph forms.
Cognee's datasets group documents and processed knowledge and anchor its grants.
Recollect retains the required organization/import/process/search/share/remove
capabilities without inheriting that physical schema or claiming every Cognee
connector and optimization. Collections initially inherit Brain knowledge access;
profile permissions govern execution independently.
[Cognee dataset concepts](https://docs.cognee.ai/core-concepts/multi-user-mode/permissions-system/datasets).

## Pattern Dispositions

Names below correspond to the pages in the pinned pattern directory. Detailed
obligations are in the vision and engineering principles; the Atlas examples
are references rather than instructions automatically adopted wholesale.

| Pattern | Recollect disposition |
| --- | --- |
| Evidence before belief | Adopt permitted durable evidence or exact controlled references, with claim-to-support links |
| Scope as a first-class key | Adopt across reads, writes, workers, caches, artifacts and analytics |
| Governed write gateway | Adopt one mutation policy for API/MCP/UI/import/review/rebuild |
| Explicit write destination | Adopt one authorized destination per mutation; broad reading does not choose ownership |
| Trust-state machine | Adapt into separate review, freshness and operational states |
| Rejected-value tombstone | Adapt to normalized assertion plus Brain/subject/applicability; material new evidence has an explicit revalidation route |
| Append-only memory audit | Adopt transactional mutation history; payload retention remains subject to erasure policy |
| Bi-temporal fact validity | Adopt fact validity and knowledge history, independent of revision manifests |
| Memory as an editing surface | Adopt actionable versioned review/correction; defer external-source writeback |
| Resolve, don't just detect | Adopt typed dispositions, including different valid applicability and retraction |
| Zero-LLM capture | Adopt for sanitized permitted events; enrichment follows durable capture |
| Recoverable background work | Adopt idempotent durable jobs, bounded retries, visible lag and deletion-race handling |
| Hybrid retrieval fusion | Adopt exact/lexical baseline plus useful semantic/graph channels under one eligibility contract |
| Source-diverse context | Adopt deduplication and provenance; defer rigid quotas and elaborate diversity ranking |
| Pluggable memory provider | Adopt narrow internal component seams; defer a marketplace or competing memory authorities |
| Skills as procedural memory | Adopt runbooks with conditions/outcomes; defer automatic executable-skill learning |
| Cache-preserving injection | Use stable/dynamic separation where supported; corrections and scope changes cannot be frozen out |
| Promotion between tiers | Defer adaptive tiers; explicit publication/review already supplies the needed lifecycle |
| Gate the expensive path | Defer learned cost gates until measured; authorization/provider policy never fails open |
| Retrieval hysteresis | Defer cooldown/sticky retrieval controls that could conceal corrections or requested context |
| Decay and reinforcement | Defer continuous scoring; popularity, repetition and pins cannot establish truth |

Rejected interpretations include unconditional raw capture, newest-wins conflict
resolution, summaries replacing their sole evidence, global bans on facts valid
elsewhere, model-claimed human authority, and automatic execution permission from
a remembered procedure. These are product design choices, not claims that every
reference system behaves this way.

All seven [rubric capabilities](https://neoneye.github.io/agent-memory-atlas/methodology/atlas-rubric/#the-seven)
are required: rejected-value tombstone, explicit trust state, bi-temporal validity,
enforced scope, mutation audit, human review and negative evaluations. They must
be demonstrated through actual product paths. A schema field or UI badge is not
proof; permitted positive controls must accompany absence assertions.

## Storage and Platform Findings

| Considered architecture | Finding and disposition |
| --- | --- |
| PostgreSQL/pgvector plus relational edges | Coherent for bounded adjacency/provenance; insufficient alone for the newly selected broad analytics scope |
| PostgreSQL/pgvector plus Neo4j CE/GDS CE | Selected: broad traversal/analytics with clear separation between durable authority and graph projections |
| PostgreSQL/pgvector plus LadybugDB | Credible embedded alternative; single owning writer-process boundary and narrower verified algorithm coverage |
| PostgreSQL/pgvector plus Rust analytics | Useful future specialized option; initial graph loading, algorithm integration and resource machinery would become Recollect work |
| Several independent memory engines | Not selected; adds competing scope/correction/deletion semantics rather than supplying one governed model |

Neo4j/GDS Community fits the no-paid-database-license requirement. GDS provides
the algorithms with four-core concurrency and in-memory projections; use queued
jobs with memory limits. Neo4j CE lacks the Enterprise fine-grained database
security/HA/online-backup feature set, so Brain enforcement remains behind the
Recollect API and graph reconstruction is part of recovery.
[GDS editions](https://neo4j.com/docs/graph-data-science/current/introduction/),
[Neo4j editions](https://neo4j.com/docs/operations-manual/current/introduction/),
[offline backup](https://neo4j.com/docs/operations-manual/current/backup-restore/offline-backup/).

Pin compatible Community artifacts and their notices; Neo4j/OpenGDS source
licensing and packaged distribution contents are separate observations, not
permission to use Enterprise trials as the production plan.
[Neo4j terms](https://neo4j.com/legal-terms/),
[OpenGDS source/distribution description](https://github.com/neo4j/graph-data-science/blob/2026.08.1/README.adoc#opengds).
Use the [official Query API](https://neo4j.com/docs/query-api/current/) through
Rust first. The inspected [neo4rs README](https://github.com/neo4j-labs/neo4rs/blob/19f244ae7800ac084f0679e89c2606b799be7538/README.md)
does not establish compatibility with a current 2026 server; prove a Bolt client
before adopting it.

LadybugDB has maintained releases and Rust bindings, while original Kuzu is
archived. Ladybug's documented concurrency and algorithm coverage remain the
relevant tradeoffs; no local build or restore proof was run.
[Ladybug concurrency](https://docs.ladybugdb.com/concurrency/),
[algorithms](https://docs.ladybugdb.com/extensions/algo/),
[Kuzu repository](https://github.com/kuzudb/kuzu).

The [pgvector README](https://github.com/pgvector/pgvector) explicitly warns about
cross-tenant influence on shared approximate indexes. Select exact scoped search
first and Brain partitions/separate tables when ANN is introduced. Model identity,
dimensions and representation must be explicit; ordinary vector ANN limits are
not identical to half-precision index limits. Iterative scans do not prove access
isolation. PostgreSQL text ranking is not automatically BM25.

Corrections affect aggregate results, not just visible rows. A removed edge can
change centrality/community assignments for other nodes. This motivates a
Brain/manifest/eligibility epoch in analytics identities and invalidation of
affected current results, followed by recomputation. This is a design inference
from the product's correction contract, not an Atlas benchmark result.

The Rust service/companion, React/Vite static UI, rmcp and Compose choices keep
one application authority and a consistent local/shared topology. Latest-version
numbers were researched as compatibility candidates, not installed pins. The
scaffold must validate features/toolchain compatibility and build artifacts.
[Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk),
[Vite deployment](https://vite.dev/guide/static-deploy.html),
[Compose production guidance](https://docs.docker.com/compose/how-tos/production/).

Enola v0.4.19 supplies an explicit consumer artifact contract. Its receipt
normalization and unresolved/duplicate fact behavior require an adapter owned
by Recollect; HCL extraction does not establish evaluated or deployed semantics.
[Artifacts](https://github.com/enola-labs/enola/blob/v0.4.19/docs/schema/README.md),
[facts](https://github.com/enola-labs/enola/blob/v0.4.19/docs/schema/facts.md),
[HCL limits](https://github.com/enola-labs/enola/blob/v0.4.19/docs/extraction/hcl.md).
CodeGraph's public library API remains a candidate, but the installed navigation
index does not establish a publication contract.
[CodeGraph library usage](https://github.com/colbymchenry/codegraph/blob/v1.6.0/README.md#library-usage).

Host integrations need adapters and coverage markers. Codex events do not expose
unambiguous subagent identity for every tool event; transcript formats are not
stable, and background hooks may be cancelled. Claude has different event fields
and lifecycle behavior. Capture therefore uses short durable-inbox hooks and
does not claim complete observation.
[Codex hooks](https://learn.chatgpt.com/docs/hooks),
[Claude hooks](https://code.claude.com/docs/en/hooks).

Vault Agent can render secrets into a child process environment and restart it
on changes. Recollect must coordinate rotation/draining with its own operation
leases rather than assuming supervision supplies safe MCP lifecycle behavior.
[Vault process supervisor](https://developer.hashicorp.com/vault/docs/agent-and-proxy/agent/process-supervisor).

## Translation and Limits

The original product idea is retained: Brain/area/environment organization,
shared exact-revision knowledge, repository memories and graphs, MCP coordination,
Vault credentials, local/private execution and separate profile grants. The main
change is owning these behaviors in Rust instead of inheriting Cognee's runtime.
That creates additional engineering and maintenance obligations; better fit to
this vision is not measured superiority in recall, latency or cost.

Atlas supplies useful mechanisms and failure cases, not a selected database or
a complete reference product. Its own acceptance/deletion/contradiction
specifications remain unexecuted. However, saying it never ran any benchmark
would be inaccurate: the pinned benchmark limitations describe narrow Provem
verification, a memsem offline rerun and Perseus result recomputation. These
different evidence strengths do not certify Recollect.
[Pinned benchmark limitations](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/benchmarks.md#L3402).

Correction tests must distinguish a qualified historical mention from a stale
current assertion. Prove original ingestion settled before testing deletion;
otherwise a never-indexed record can create a false pass. Compare semantic/graph
additions with exact/lexical baselines under the same models, sources and budgets.
Do not download benchmark datasets or assume their licenses from Atlas's license.

## Follow-up

Foundation decisions are settled at product level. Derive epics next; before each
dependent implementation, turn the following responsibilities into detailed
ADRs/contracts and a decision-complete execution pack:

| Boundary | Required implementation detail |
| --- | --- |
| Identity/authorization | Ownership, local/OIDC mapping, reviewer powers, profile grants, session/device revocation and membership freshness |
| Scope/API | Task handles, workspace discovery, versioned public interfaces and errors; no session-global scope |
| Evidence/collections | Canonical source versions, collection membership/removal, claim support and bi-temporal records |
| Mutation/learning | Transaction/outbox, idempotency, expected versions, acceptance policy, conflicts and reactivation |
| Retention/erasure | Per-class rules beyond the 30-day raw default, retained excerpts, queued work, controlled copies and backup expiry/restore enforcement |
| Extraction/graphs | Fixture-supported languages, manifests, cross-repository links, generation publication and aggregate invalidation |
| Retrieval/models | Eligibility modes, context budgets, provider adapters/configuration, dimensions and measured fusion/ANN behavior |
| MCP/Vault/runners | Approved catalogue/configuration, pairing, credential delivery/rotation, leases, shutdown and unknown-completion reconciliation |
| Deployment | Exact versions, volumes/backups, supported host matrix, resource budgets and recovery service levels |

These refine the selected foundation rather than reopening its major decisions.
The first deployment's providers/models, optional OIDC target, credentials and
backup service levels are operator inputs. The proposed VM size is unbenchmarked.

The [engineering principles](../foundation/engineering-principles.md) enumerate
the behavioral proof: two-Brain positive/negative controls, replay/rebuild,
erasure/restore, temporal revisions, real review authority, provider restrictions,
host coverage/retention, graph correctness and concurrent Vault/MCP lifecycle.
Measure query/ingestion lag, tail latency, memory and cost under representative
capture/query/analytics load. Passing documentation validation proves structure
only. No product epic, implementation pack, deployment or commit is created by
this foundation update; version is N/A and changes remain uncommitted.

Validation observed during this update: ./scripts/validate.sh passed governance
lint and all 32 checker tests. A comparison against the pre-edit Markdown copies
confirmed changes to foundations, research/history labels and navigation/tooling
guidance; no product epic or execution-pack content was rewritten. Only the epic
index's obsolete foundation-direction sentence changed. Both reference checkouts
remained clean at their recorded revisions. These checks establish documentation
consistency and preserved checkout state, not product runtime behavior.
