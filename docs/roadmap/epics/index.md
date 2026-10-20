# Product status and implementation roadmap

Reviewed: 2026-10-20 against `main` at `a2338a8`, which matches `origin/main`.
No version is released, and no normal-installation update after the 2026-10-09
migration-047 proof is recorded. Runtime and benchmark numbers below are dated
observations until replacement proof is recorded.

## Epic Overview

| Epic | Status | Current focus |
| --- | --- | --- |
| [repository-governance.md](repository-governance.md) | complete | Current documentation, governance and the `recollecthub.com` product/docs site on the Light and Ink system delivered |
| [developer-tooling.md](developer-tooling.md) | complete | Repository-local Rust Analyzer MCP, Graft and documentation skills |
| [product-platform.md](product-platform.md) | complete | Accounts, Brain access, durable work, installation identity, canonical Brain deletion and one typed Brain admission |
| [evidence-and-workspaces.md](evidence-and-workspaces.md) | complete | Versioned sources, collections, workspace scope, capture and repository publication |
| [memory-lifecycle.md](memory-lifecycle.md) | active | Automatic learning, digests and correction delivered; derived mapping privacy fences remain a dependency of the deferred organization pack |
| [hybrid-retrieval.md](hybrid-retrieval.md) | complete | Delivered recall baseline, set-based read path (lexical index windows, one candidate pass per transaction, set gating), labelled large-scope semantic ranking up to 50,000 entries, and locally validated attributed-feedback offline comparison; production ranking fixed |
| [graph-intelligence.md](graph-intelligence.md) | active | Graph read reliability closeout in progress; automatic semantic organization deferred by the owner on 2026-10-20 (pack blocked) |
| [mcp-coordination.md](mcp-coordination.md) | active | Shared/native skills and cross-machine coordination delivered; direct-token and normal installed-host acceptance remain separate |
| [operational-readiness.md](operational-readiness.md) | active | Normal-installation update, graph reliability and a comparable memory-quality rerun |
| [desktop-visual-experience.md](desktop-visual-experience.md) | complete | Light and Ink redesign on Tailwind CSS and shadcn/ui ([ADR 0025](../../adr/0025-two-theme-desktop-redesign.md)) shipped on 2026-10-20 with grouped 3D, source navigation and exact generation invalidation |
| [memory-quality-and-proof.md](memory-quality-and-proof.md) | planned | Part 1: engineering-task evaluation, Atlas self-score, independent support and derivation closure, writer latency, typed time, scale and cost |
| [claim-truth-and-freshness.md](claim-truth-and-freshness.md) | planned | Part 2: revision carry-forward, freshness policy, answer-time verification, knowledge changelog, background audit and trust-aware ranking |
| [agent-identity-and-authority.md](agent-identity-and-authority.md) | planned | Part 3: agent principals, scoped short-lived credentials, configuration proposals, workload identity exchange and runner identities |
| [verified-agent-work.md](verified-agent-work.md) | planned | Part 4: receipts, claim verification, capture API, verification requests, point-of-action recall, failed attempts, prospective commitments and positioning |
| [skills-provenance-and-sync.md](skills-provenance-and-sync.md) | planned | Part 5, alongside the others: skill source provenance, upstream updates, drift report and targeting |

## Current direction

On 2026-10-20 the owner set the product focus: self-hosted, harness-agnostic
memory, skills and governed tools for engineering teams, differentiated by
verified agent work. Every agent's work should leave a receipt linking its claims
to environment-aware, correctable evidence, so headless agents (for example
cluster jobs) can later work through Recollect. The improvement work is split
into five planned epics, ordered below; each holds its slices, open decisions and
what it waits for.

Deferred by the owner on 2026-10-20: automatic semantic organization and further
graph or 3D work, MCP OAuth and gateway breadth (upstream OAuth, rate limits,
tool search, more providers), and new A2A work beyond the delivered coordination
adapter. Nothing is published to the Agent Memory Atlas upstream for now. A
hosted multi-tenant offering may follow later; the
[vision](../../foundation/vision.md) still excludes it.

## Improvement sequence

| Order | Epic | Why in this position | Starts when |
| --- | --- | --- | --- |
| 1 | [Memory quality and proof](memory-quality-and-proof.md) | Everything later is measured with its evaluation, and its self-score is the honest proof | Alongside the open installation-update and quality-rerun work in the operations pack |
| 2 | [Claim truth and freshness](claim-truth-and-freshness.md) | Keeps memory true for the revision and environment in use; needs no agent identity | The evaluation baseline exists |
| 3 | [Agent identity and authority](agent-identity-and-authority.md) | Receipts and headless agents need narrow agent identities | The identity ADR is accepted; may swap with part 2 |
| 4 | [Verified agent work](verified-agent-work.md) | The headline capability, built on parts 2 and 3 | Carry-forward, freshness, principals and scoped credentials exist |
| 5 | [Skills provenance and sync](skills-provenance-and-sync.md) | Independent; runs alongside the others | Provenance, updates and drift any time after part 1 starts; targeting after agent principals |

Most planned slices need an ADR or contract amendment before code, and the
authorization, external-integration and paid-evaluation slices need a pack; each
epic names which.

## Implemented baseline

Recollect owns its Rust server and host plugin, PostgreSQL authority/pgvector,
Neo4j/GDS projections and React desktop. Delivered behavior includes versioned
source evidence, manual collections/areas/environments, automatic capture and
learning, support verification, settled-session digests, scoped project briefs,
exact-family cross-session reconciliation, cited Ask/recall, repository graphs,
optional review/correction, erasure/recovery, governed MCP/private execution,
Brain-managed native skills and cross-machine agent coordination. All of it is
committed on `main`; the latest retained installation proof remains the
[2026-10-09 checkpoint](../../mappings/automatic-knowledge-mapping-progress-2026-10-09.md).

## Graph delivery status

| Capability | State | What remains |
| --- | --- | --- |
| Graph reliability | Off-lock preparation and checked publication implemented. Local mixed 10k proof passes 602/602 reads, zero errors, p95 0.460 s; both normal-data clone series pass 40/40 below two seconds. | Final integrated closeout and a verified normal-installation update |
| Semantic worker lease admission | Six native cancellation/reclaim/healthy regressions pass in 5.47 s; stale work creates zero requests or publications. | Rollout; cancellation API, Linux runtime and cancellation after network admission are not established |
| Automatic organization | Worker, exact spans, scoped entities, reversible aliases, native topics, overlays and navigation implemented and locally tested. The latest frozen 105-call campaign fails: fact mentions 51.67% precision / 8.73% recall and reconciliation 40.63% / 3.66% against 95% / 80%; hybrid topic pair F1 26.36%; canonical behavior 4/6. | Deferred by the owner on 2026-10-20; next action and activation gates are in the [organization pack](../execution/active/graph-automatic-knowledge-mapping.md) |
| Grouped 3D and feedback ranking | Grouped 3D renderer and a bounded attributed-feedback experiment delivered locally. | Normal-installation rollout; real feedback usefulness unproved and production RRF fixed |

No producer or recipe is activated. Frozen hashes, cost approvals, cohort
denominators and every failure are in the
[evidence receipts](../../mappings/graph-organization-implementation-2026-10-18.md).

## Other delivered work

| Capability | Delivered | Limit |
| --- | --- | --- |
| [Brain-managed skills](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/mcp-managed-skills.md) | Administrator publication, environment applicability, advisory MCP profile links, progressive fetch, plugin local copies and Agents management ([evidence](../../mappings/managed-skills-host-consumption-2026-10-18.md)) | Disposable databases and mocked browser routes; not deployed |
| [Native skill reload](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/mcp-native-skill-reload.md) | Owned workspace registration, synchronization and cross-host scope leases; isolated Codex 0.162.1, Claude Code 2.1.302 and OpenCode 2.0.22 activation and removal ([proof](../../mappings/native-skill-reload-host-proof-2026-10-18.md)) | Synthetic providers; no user host upgraded |
| [Agent coordination](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/mcp-agent-coordination.md) | Session context, goal/planned-file overlap, durable peer discussion, SSE/pull, MCP, native hooks/CLI and a narrow A2A 1.0 adapter; 15 macOS/Ubuntu checks with zero model requests ([proof](../../mappings/agent-coordination-proof-2026-10-18.md)) | No production deployment or installed-host model consumption |
| [Attributed feedback](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/retrieval-attributed-feedback.md) | Exact scoped votes, privacy controls and a frozen zero-call 20-query comparison | Two deliberately mistaken votes cause bad promotions; real helpfulness unproved |

## Remaining authorized work

| Priority | Deliverable | Current gap and completion gate | Owner |
| --- | --- | --- | --- |
| 1 | Normal-installation update and comparable memory-quality rerun | The migration-047 installation passed only 35/40 normal reads; the latest official LongMemEval result is 20/50 (40%) and later repairs have no comparable scored run. Update the installation under its owned update/recovery rules, then rerun frozen questions with retrieval, answering and full-learning effects separated. | [Operations pack](../execution/active/operations-memory-quality-first-batch.md), [benchmark report](../../mappings/openrouter-benchmark-proof-2026-10-08.md) |
| 1 | Reliable graph reads during background work | Local mixed 10k and normal-data clone gates pass; final integrated closeout and a verified installation update remain. | [Concurrency pack](../execution/active/graph-concurrent-preparation.md) |
| 2 | Direct-token and installed-host acceptance | Browser config and HTTP proof exist; fresh OpenCode/Claude launch and the rebuilt plugin's native acceptance remain, and the recorded OpenCode service timeout needs fresh verification. | [Direct-token pack](../execution/active/mcp-copy-ready-agent-config.md), [plugin evidence](../../mappings/plugin-workflow-repair-2026-10-07.md) |
| 3 | Improvement epics | Planned in five parts; see the [improvement sequence](#improvement-sequence). Part 1's engineering-task evaluation comes first. | [Memory quality and proof](memory-quality-and-proof.md) and its successors |

Priorities order the remaining work; they do not turn a partial implementation
into a shipped slice. Completed cost grants do not cover new paid calls.

## Deferred

Beyond the owner's 2026-10-20 deferrals above: small-screen polish, general
desktop SSE, learned retrieval gates/decay, advanced reranking, automatic
executable skills, marketplace publishing and a release policy. Graphify must
show additional coverage over Enola on identical inputs, and Graphiti or a local
extractor service remains an optional adapter decision; no graph engine
replacement is selected. Do not present deferrals as missing implementations of
accepted current behavior; current contracts state their exact boundaries.

## Delivery records and evidence

Each epic's slice map is the current delivery record (acceptance and validation
inline). Shipped packs were removed from the tree on 2026-10-19 and remain in
[Git history](https://github.com/MikeK184/Recollect/tree/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive).
Unfinished risky work is in the [active packs](../execution/active/README.md).
[Mappings](../../mappings/README.md) retain current baseline and open-issue
evidence. Superseded studies and concepts are available at the fixed
[pre-cleanup checkpoint](https://github.com/MikeK184/Recollect/tree/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs).
Dates, model scores and successful host connections describe their recorded runs;
no current connectivity or benchmark claim follows from documentation cleanup.
