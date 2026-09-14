# Recollect Product Feasibility from Agent Memory Atlas

Observed: 2026-09-13
Confidence: inferred

Historical assessment from earlier in the 2026-09-13 discussion, before the user
selected an independent Rust core and the researched stack. References below to
the accepted Cognee baseline describe that earlier decision stage. Preserve its
source findings as evidence; use the revised [foundations](../foundation/README.md)
and subsequent [decision mapping](atlas-foundation-decisions-2026-09-13.md) for
the current direction. This report is not implementation authority.

## Sources and Method

Three research agents independently examined Atlas patterns, the comparative
report and reference systems, and the current Cognee source. The parent agent
checked Recollect's requirements, inspected decisive source paths, and consulted
official MCP and extractor documentation. This was source/document research;
no memory engine, runner, model benchmark, or product prototype was executed.

- Atlas checkout: `7eca7f7abd934c2e44bc096fc1dd0cbf94275b99`.
  The [checkout mapping](atlas-reference-2026-09-13.md) records provenance.
  Primary inputs were `content/patterns/`, relevant `content/overview.md`
  sections, system reports, `content/build.md`, methodology, and the declarative
  `.agents/protocol/tests.yaml`. External agent workflows were research material.
- Cognee checkout: version 1.5.4,
  `c0d18c80e24b7b78918e7642c03f6f128fdd2aee`. The Atlas Cognee review describes
  version 1.4.0 at an older July 27 revision. Current local source takes
  precedence over that review when describing this checkout.
- Context7 successfully resolved `/colbymchenry/codegraph` and returned current
  indexing, node identity, worktree, and status documentation. Those documents
  describe current upstream; local CLI verification identified version 1.6.0.
- Official [MCP authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
  and [security guidance](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices)
  were read for transport and credential boundaries.
- Enola's [snapshot design at v0.4.12](https://github.com/enola-labs/enola/blob/v0.4.12/docs/SNAPSHOTS.md)
  was retrieved from raw GitHub after the browser fetch returned a cache miss.
  Raw document: 17,006 bytes, SHA-256
  `3860716adbec71815e40d05ab830041cc1cb82c55f4809e072690c169bfe5172`.
- Current reference source pins: Hindsight
  `e3efe5dd8b070d00129c5186d111c0bc5f992363`, Graphiti
  `c035afb7990b6077331a81e98b04efcfd9bf8184`, and Verel
  `a8cbba27a3fa61baeffa808c5929a273d176c942`. OpenViking's current official
  README was checked as of the observation date. A browser fetch of Verel's
  pinned review source failed; a raw GitHub read verified the relevant function.

Confidence by claim: repository contents and inspected code paths are verified
as source observations; external documentation is observed-once; product fit
and architecture recommendations are inferred. Operational performance,
security of a deployment, and comparative superiority are unproven.

## Assessment

There is enough material to design a coherent Recollect product and implement
a discriminating prototype. The research is particularly useful for avoiding
known mistakes in provenance, authorization, correction, retrieval, and
background work. It cannot establish that a new implementation will outperform
Cognee or another engine in retrieval quality, cost, latency, or maintenance.

A purpose-built product could fit this user's engineering workflow substantially
better: shared customer Brains, overlapping areas, environment-specific revision
views, inspectable evidence, and separately authorized execution. That is a
credible product opportunity. It is different from a measured claim of a better
general memory engine.

The Atlas [build guide](https://neoneye.github.io/agent-memory-atlas/build/)
explicitly states that its twenty tests are specifications, none were run by
the Atlas against the reviewed systems, it supplies no complete reference
implementation, and operating numbers are absent. Its
[methodology](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/methodology/atlas-rubric.md)
also identifies model-assisted source review and the limits of its rubric.
See local `content/build.md:322-342` and
`content/methodology/atlas-rubric.md:248-298` for the pinned text.

## What Transfers to This Vision

| Recollect requirement | Useful knowledge/material | Product work still required |
| --- | --- | --- |
| Customer Brains shared by people and agents | Scope in identities, reads, cache keys, jobs, and publication paths | Authenticated Brain ownership/grants, revocation, backend bypass prevention, SSO reconciliation |
| Areas such as Vault and Kubernetes, viewed across production/development | Typed context associations and explicit read/write destinations | Overlapping views rather than duplicated knowledge; areas/environments initially inherit Brain knowledge access |
| Concurrent agents that change working scope | Explicit operation context and scope propagation | Bind every event/call to its starting scope; never relabel an in-flight operation after a default changes |
| Repository and environment truth | Evidence lineage, temporal validity, immutable graph artifacts | Stable repository UUIDs, committed/desired/observed revision distinctions, environment manifests, authenticated snapshot publication |
| Explainable and correctable knowledge | Evidence before belief, controlled writes, durable correction, source-linked retrieval | Independent review/freshness/operational states; corrections included in replay; applicability-aware conflict rules |
| Code relationships across repositories | Existing extractors and graph query engines | A normalized snapshot contract, language/edge coverage tests, explicit combined revision sets and validated cross-repository linking |
| Environment MCP profiles and mixed runner placement | MCP transport/authentication standards and bounded work patterns | Profile grants, approved targets, runner trust, credential delivery, startup locks, leases, idle shutdown, useful lifecycle UI |
| Useful daily agent experience | Hybrid retrieval, bounded context, progressive detail and direct source access | Task-grounded evaluation, latency/cost budgets, clear missing-source and stale-evidence responses |

The characteristic question is: “How is Vault configured in production, which
repository revisions support that answer, when was it last observed, how does
development differ, and am I permitted to inspect production now?” Each clause
needs a different part of the product. Semantic recall alone does not answer
all of them.

## Adapt the Patterns Deliberately

1. **Retain evidence or dependable references before deriving claims.**
   [Evidence before belief](https://neoneye.github.io/agent-memory-atlas/patterns/evidence-before-belief/)
   permits references to controlled durable sources. Keep code local under
   capture policy, retain approved immutable artifacts and source identities,
   and describe unavailable source text honestly. A reference/hash does not
   recreate content after the source disappears. Rebuild guarantees must state
   exactly which material is retained and available.
2. **Keep state dimensions independent.** Atlas's simplified trust state
   machines do not replace Recollect's accepted distinctions among human
   review, freshness, and operational evidence. A reviewed plan remains a
   plan; a recent source can still be wrong; an older production revision can
   remain applicable while development advances.
3. **Persist corrections outside disposable indexes.** Replaying evidence must
   also replay applicable human decisions. Otherwise regeneration can restore
   a rejected claim. A rejection must include scope and applicability: rejecting
   a production assertion at revision A must not globally ban a legitimate
   development assertion or a later revision B.
4. **Treat patterns as arguments, not a conformance checklist.** The tombstone
   pattern permits explicit reactivation and requires deletion policy, while
   a catalogue test demands survival beyond every TTL. Recollect must resolve
   that into its own retention/applicability contract. Similarly, “only executed
   actions become memory” would discard useful design decisions; retain those
   as declared intent instead of presenting them as observed operation.
5. **Make authorization structural.** A scope supplied by the caller is not
   permission to use it. Apply identity/grants across retrieval, writes,
   consolidation, export, caches and engine APIs. Selecting production context
   cannot confer production execution rights.
6. **Keep recalled content below instruction authority.** Source documents,
   tool output and memories are data. Prompt formatting is useful but does not
   alone prove the model resists malicious recalled instructions; evaluate the
   resulting behavior too.
7. **Start with the product's actual boundaries.** The comparative report's
   illustrative staging postpones hosted tenancy and cross-device sync
   (`content/overview.md:6885-6893`). Shared Brain access is fundamental here,
   so Recollect cannot inherit that sequence wholesale. Likewise, defer
   automatic procedural learning, elaborate decay and universal backend
   portability unless a representative task demonstrates the need.

Pinned pattern locations include `content/patterns/evidence-before-belief.md:35-66`,
`content/patterns/trust-state-machine.md:24-48`, and
`content/patterns/rejected-value-tombstone.md:77-109`.

## Cognee Fit at the Inspected Revision

Current Cognee already supplies substantial useful machinery: ingestion,
graph/vector retrieval, users and dataset permissions, an MCP surface, UI,
and code graph import. Its present evidence/temporal capabilities must be
assessed from current source rather than inferred from the older Atlas marks.

Its current [edge-evidence model](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/modules/provenance/edge_evidence/models.py#L17)
links assertions to datasets, documents, chunks and pipeline runs. The typed
[retrieval evidence model](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/modules/search/models/EvidenceReference.py#L7)
distinguishes contextual material from assertion support. An opt-in
[temporal resolver](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/modules/graph/utils/temporal_conflict_resolver.py#L1)
marks superseded assertions for caller-declared single-valued relationships.
It ranks by assertion recency; that is not a complete environment-specific
truth or human-review policy.

One decisive mismatch is directly visible in the code graph importer. Its
[fact identity](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/tasks/code_graph/extract_code_graph.py#L90)
uses repository, kind and name, intentionally merging same-named facts of a
kind inside a repository. Its
[import reconciliation](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/tasks/code_graph/extract_code_graph.py#L828)
removes obsolete graph facts, and `CodeRepository` records the last snapshot.
That supports a current graph projection. Immutable environment-specific
historical views require an additional storage/identity design; accepting a
`snapshot_dir` does not supply that design by itself.

The source [Data model](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee/modules/data/models/Data.py#L9)
also documents identity surviving content changes. Consequently, the presence
of an evidence ID alone is insufficient to assert that the exact historical
source text remains available. These are extension boundaries, not proof
that Cognee cannot be used.

Authorization reuse has a cost to verify: dataset ownership and composed grants
are not equivalent to temporary Brain membership. See the official
[dataset ownership description](https://docs.cognee.ai/core-concepts/multi-user-mode/permissions-system/datasets)
and local `cognee/cognee/modules/data/methods/create_authorized_dataset.py:35`.
Removing one role is not proof that ownership or independent grants vanished.
The stock MCP [tool mode](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee-mcp/src/server.py#L160)
changes advertisement while tools stay callable by name. Tool discovery and
execution permission therefore require separate treatment.

## Reference Systems Worth Studying

This is a focused shortlist from the comparative research, not a ranking of the
entire corpus and not a recommendation to operate all four systems together.

| Reference | Mechanism worth studying | Limitation for Recollect | Proposed use |
| --- | --- | --- | --- |
| [Hindsight](https://github.com/vectorize-io/hindsight/tree/e3efe5dd8b070d00129c5186d111c0bc5f992363) | Source documents, extracted facts, derived observations, hybrid retrieval and consolidation | A bank does not supply team grants; document reprocessing resets fact curation | Strongest service alternative among the reviewed shortlist to evaluate against Cognee |
| [Graphiti](https://github.com/getzep/graphiti/tree/c035afb7990b6077331a81e98b04efcfd9bf8184) | Episode provenance and relationship validity over time | Time/partition fields do not establish source verification, deployment revision or caller authority | Temporal graph component/reference |
| [Verel](https://github.com/amitpatole/verel/tree/a8cbba27a3fa61baeffa808c5929a273d176c942) | Separate trust/confidence/retrieval strength; durable rejection and promotion checks | Large surrounding framework; reviewer attribution alone is not authenticated review authority | Correction and trust invariants |
| [OpenViking](https://github.com/volcengine/OpenViking) | Addressable context, browsing and progressive abstract/overview/content loading | A directory hierarchy cannot represent all overlapping Recollect associations or establish grants | Navigation and context-budget design |

Hindsight has advanced since its August 6 Atlas review. Current
[memory curation documentation](https://hindsight.vectorize.io/developer/api/memories)
and source support edit, invalidate and restore, with regeneration of derived
observations. The same documentation explicitly states that reprocessing the
original document resets curation. A durable review/correction layer would be
necessary for Recollect's replay requirements. Its built-in
[tenant implementations](https://github.com/vectorize-io/hindsight/blob/e3efe5dd8b070d00129c5186d111c0bc5f992363/hindsight-api-slim/hindsight_api/extensions/builtin/tenant.py#L35)
provide either no authentication or a shared API-key-to-schema mapping;
[extensions](https://hindsight.vectorize.io/developer/extensions) are the route
to more specific policies. This observation concerns these inspected built-in
implementations, not the capabilities of an uninspected hosted deployment.

Graphiti's inspected [edge model](https://github.com/getzep/graphiti/blob/c035afb7990b6077331a81e98b04efcfd9bf8184/graphiti_core/edges.py#L263)
contains episode references and `valid_at`, `invalid_at`, and `expired_at`.
Verel's [review function](https://github.com/amitpatole/verel/blob/a8cbba27a3fa61baeffa808c5929a273d176c942/src/verel/memory/review.py#L49)
refuses approval of rejected records or values present in a rejection ledger.
These are concrete implementation references, not runtime proofs.

License observations: Hindsight and Verel identify MIT, Graphiti Apache-2.0,
and OpenViking's main project AGPL-3.0 with component exceptions. The Atlas's
own MIT license does not license the referenced code. Record the exact selected
component, revision and terms before incorporating implementation, rather than
assuming every reference shares the Atlas license. This review selects ideas
and evaluation candidates; it incorporates no reference implementation code.

## Extraction and Execution Boundaries

The installed CodeGraph 1.6.0 remains a verified developer navigation tool. A
fresh status call showed a complete index, 3,075 files and no pending changes.
That verifies local indexing state; it does not establish a product publication
API or committed-snapshot guarantee.

For product use, evaluate extractors behind a Recollect contract covering
repository UUID, commit, extractor/version/settings, content hashes, source
locations, supported languages, unresolved relationships, and capture policy.
Enola 0.4.12 is the existing accepted extraction baseline and has a documented
snapshot-oriented design. CodeGraph can be evaluated as another provider for
local navigation or graph facts. There is no need to write a new parser to own
the product, and no basis yet to declare either extractor universally superior.

MCP standardizes tool interaction and transport authorization. Its HTTP
authorization specification and environment-credential approach for stdio do
not define Recollect's Brain/profile grants or runner lifecycle. The product
must authorize managed operations, keep registered configuration separate from
model arguments, isolate credentials/sessions and manage only processes it owns.
This does not restrict unrelated shell credentials on an unrestricted laptop.

## Architecture Options and Recommendation

| Option | Benefit | Principal cost/risk | Assessment |
| --- | --- | --- | --- |
| Add Recollect domain modules inside Cognee | Reuse existing APIs, auth primitives, storage interfaces and UI | Domain semantics must coexist with upstream dataset ownership and graph mutation; every ordinary API access path still matters | Credible and consistent with the accepted baseline; test the difficult boundaries early |
| Own the Recollect core and use a replaceable memory backend | Brains, evidence, revisions and execution rules have one product owner; engine storage can remain a derived projection | New domain APIs/UI integration plus synchronization, deletion, permission propagation and backend failure handling | Strong alternative to evaluate for the full vision |
| Replace Cognee wholesale with another reference service | May improve a particular retrieval or operating characteristic | Does not remove the custom Brain/revision/profile work; imports a different set of semantics | Evaluate only against a demonstrated need |
| Reimplement parsers, vector search, authentication and orchestration together | Maximum implementation control | Large maintenance surface without evidence of user benefit | Not justified by the research |

Recommendation: own the Recollect domain model and its acceptance tests; keep
retrieval and extraction replaceable. Evaluate Cognee first as the existing
candidate engine. A Recollect-owned core can initially be a well-separated
module in one application; it does not require microservices or several memory
engines running together. If the critical invariants demand pervasive upstream
patches, an independent core with a constrained backend adapter is the stronger
long-term option.

Authoritative grants, revision manifests, evidence identities and human decisions
must have an explicit owner. Search indexes and generated summaries should be
rebuildable from that authoritative material under the chosen retention policy.
An adapter still needs real enforcement and recovery semantics; the word
“adapter” does not make backend bypass or synchronization failures disappear.

## Proof Needed Before Choosing a Foundation

Use a small, representative pilot: two Brains, two engineers, three repositories,
Vault/Kubernetes areas, production/development manifests, one local companion,
and one controlled MCP connection. Keep the test data and providers fixed across
implementations. A separate provisioning identity may be needed; determine its
ownership contract before testing shared access/revocation.

| Scenario | Required result, including positive control |
| --- | --- |
| Query, direct-ID read or background work attempts to widen its Brain | Other-Brain data remains absent; the caller's permitted evidence is still returned |
| Production remains on commit A while development advances to B | Production answers still cite A; development answers can cite B; desired and observed state remain distinguishable |
| Default scope changes during two concurrent operations | Each original operation keeps its original attribution; a new operation immediately uses the newly selected scope |
| Human rejects a wrong extraction, then re-extraction/reindex/restart runs | The rejected claim remains inactive; unrelated valid claims and a legitimate later revision remain usable |
| Source or claim is deleted/retracted | Governed derived results and shared copies obey the chosen policy after rebuild; sibling material remains available |
| An engineer reads production knowledge but lacks profile-use permission | Knowledge recall works; the managed production tool call is denied; an authorized profile call succeeds |
| Profile use is revoked or calls overlap with idle shutdown | New calls/renewals follow the revocation contract; active calls follow their defined lease/cancellation behavior; startup is not duplicated |
| Contributor is offline or a source reference no longer resolves | Published permitted artifacts remain useful; unavailable source text is reported explicitly |
| A derived write, upload or import is retried after interruption | No incorrect duplicate or partial publication appears; completion and failure are observable |

The Atlas catalogue explicitly requires positive controls because an empty
retriever can pass absence-only assertions. See `.agents/protocol/tests.yaml:16-22`
and `content/build.md:252-260`.

Measure task answer accuracy and source/revision correctness, forbidden/stale
results, retrieval usefulness within a fixed context budget, p50/p95 latency,
write-to-readable lag, model cost, storage growth, recovery, and implementation
maintenance. Use stock Cognee as a baseline where it supports the task, marking
unsupported product workflows as unsupported rather than pretending they are
retrieval failures. Compare a thin Cognee extension and an independent prototype
on the same required behaviors before claiming one foundation is better.

## Follow-up and Limits

Promote the selected build/extend boundary into an ADR and reconcile foundation
documents only after the product decision. Then specify the Brain/grant model,
evidence/revision contract, publication lifecycle, correction/deletion policy,
and execution-profile/runner contract in scoped implementation packs.

This research establishes feasibility and a way to decide; it establishes no
delivery estimate, capacity guarantee, universal retrieval advantage or deployed
security result. No product code, deployment, upstream checkout or accepted
foundation was changed. Version is N/A; the research document is uncommitted.

Documentation validation: `./scripts/validate.sh` passed governance lint and
all 32 checker fixtures after this report and its index entry were added.
These validate repository documentation governance, not the proposed product
or the performance and correctness of the reference systems.
