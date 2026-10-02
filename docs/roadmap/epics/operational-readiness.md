# Operational Readiness and Integrated Proof

Status: active

## Purpose

Make the complete personal/team product operable and demonstrably useful through
reproducible deployment, recovery, integrated negative/positive evaluations and
measured resource/quality limits. This epic owns final product readiness evidence.

## Governing Sources

- [Desktop ADR](../../adr/0014-desktop-experience-and-answers.md)
- [Knowledge surface ADR](../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Tiered surface contract](../../contracts/desktop-knowledge-surface.md)
- [Brain deletion ADR](../../adr/0016-brain-deletion.md)
- [Brain deletion contract](../../contracts/platform-brain-deletion.md)
- [Desktop contract](../../contracts/desktop-experience.md)
- [Answer contract](../../contracts/retrieval-answers.md)
- [Integrated acceptance contract](../../contracts/operations-integrated-evaluations.md)
- [ADR 0013: backup and recovery](../../adr/0013-backup-and-recovery.md)
- [Recovery contract](../../contracts/operations-recovery-drills.md)
- [ADR 0011: installation](../../adr/0011-local-and-shared-installation.md)
- [Installation contract](../../contracts/operations-local-and-shared.md)
- [Vision: pilot success and limits](../../foundation/vision.md#pilot-success-and-limits)
- [Stack: delivery and verification](../../foundation/techstack.md#delivery-and-verification)
- [Stack: graph recovery](../../foundation/techstack.md#graph-computation-and-recovery)
- [Engineering principles: behavioral proof](../../foundation/engineering-principles.md#prove-useful-behavior-and-failure-boundaries)

## Dependencies and Boundaries

Deployment work can start after platform identity, durable jobs and device
pairing. Use the same Compose topology locally and on a shared private server,
with native companions, persistent volumes, explicit migrations, HTTPS for
shared access and documented operating inputs. Do not create a new topology or
mandatory infrastructure service to satisfy readiness.

Own operational logs/metrics, resource budgeting, off-machine encrypted backups,
upgrade/restore procedures, and the integrated acceptance report. Feature epics
own meaningful tests from their first slices; scope, correction and audit are
not postponed until this epic. Retention rules come from memory lifecycle, graph
reconstruction from graph intelligence, and Vault/runtime semantics from MCP.

Recovery and integrated proof wait for the relevant feature paths, including
capture, review, retrieval, graph analytics and managed observations. Apply the
current retained erasure journal and the checkpoint's canonical rejection state
before restored recall becomes available. Ordinary changes beyond the advertised
recovery point are not claimed recovered. Record what can be rebuilt from retained
evidence and what cannot.

The first usable product includes all seven product epics. A running stack or
passing documentation checker alone is insufficient. High availability, customer
SaaS federation, universal capacity guarantees and a paid database license remain
outside scope. Actual deployment targets and credentials require the later
assignment's authorized inputs; this roadmap does not select or access them.

## Decisions Before Implementation

- Specify supported host/runtime versions, packaging, volumes, operating inputs,
  shared HTTPS configuration and diagnostic/resource limits before deployment.
- Specify backup retention/expiry, recovery objectives, deletion-ledger recovery,
  migration compatibility and rollback before data-bearing shared operation.
- Specify representative corpus/workload and measurable quality/latency/cost
  acceptance before integrated evaluation. Preserve evidence boundaries and
  compare methods under equivalent models and context budgets.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `operations-public-benchmark` | shipped | contract-backed | pack | Frozen 50-question lexical/semantic evidence retrieval reports, provider usage and Atlas lifecycle proof; not a universal memory score |
| `operations-atlas-lifecycle-proof` | shipped | contract-backed | pack | Deterministic Atlas §6 deletion sequence and §7 contradiction matrix as a 40-cell pass/fail matrix through the product API with zero model calls, plus digit-for-digit HotpotQA reproduction |
| `operations-longmemeval-bench` | blocked | needs-contract | pack | Official LongMemEval-S answer-level run under the [proposed protocol](../../contracts/operations-longmemeval-protocol.md); blocked on user acceptance of that contract with explicit cost approval; three accuracies, pinned judge, frozen dataset hashes |
| `operations-local-and-shared` | shipped | adr-backed, contract-backed | pack | Actual personal/shared UI/API, HTTPS native pairing/MCP, persistent state, migration failure, dependency outage, graceful drain and diagnostics verified |
| `operations-recovery-drills` | shipped | adr-backed, contract-backed | pack | Actual encrypted SFTP, offline restore, journal continuity, erasure/replay, failed upgrade, interrupted resume, graph/recall and desktop proof |
| `operations-integrated-evaluations` | shipped | contract-backed | pack | Actual-model quality, seven-capability matrix, unchanged 50-repository/200-document/eight-caller workload, repaired admission/queries, final restore/desktop and normal upgrade verified |
| `operations-root-compose` | shipped | adr-backed, contract-backed | pack | Root Compose image/browser login, preserved inventory, full stop/start, migration-failure gate and verified fixture shutdown pass |
| `operations-proof-cleanup` | shipped | contract-backed | small-fix: explicitly authorized removal of verified disposable local fixtures; no product implementation or schema change | Removed 50 containers, 49 volumes, 9 networks and 10 fixture directories; current seven-Brain inventory and readiness preserved |
| `operations-desktop-activity` | shipped | adr-backed, contract-backed | pack | Authorized bounded Activity feeds and diagnostics with canonical detail/recovery links |
| `desktop-experience-acceptance` | shipped | adr-backed, contract-backed | pack | Real desktop/API/agent regression, answer quality/privacy, resource measurements and rollout/rollback evidence |
| `desktop-assurance-pulse` | shipped | adr-backed, contract-backed | pack | Exception-first assurance band over existing authorized feeds, with Activity reordered as its drill-down |
| `desktop-chrome-declutter` | shipped | contract-backed | pack | Assurance band hidden on a healthy Brain (renders only while something needs attention); the four Knowledge pages drop the horizontal tab strip that duplicates the contextual sidebar |

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `operations-local-and-shared` | `platform-durable-work`, `platform-team-access`, `platform-device-pairing` |
| `operations-recovery-drills` | `operations-local-and-shared`, `evidence-session-capture`, `graph-exploration`, `retrieval-investigation-ui`, `mcp-observation-capture` |
| `operations-integrated-evaluations` | `operations-recovery-drills`, `memory-capture-reconciliation` |
| `operations-public-benchmark` | `operations-integrated-evaluations` |
| `operations-atlas-lifecycle-proof` | `operations-public-benchmark` |
| `operations-longmemeval-bench` | `operations-atlas-lifecycle-proof` |
| `operations-root-compose` | `operations-local-and-shared` |
| `operations-proof-cleanup` | `operations-root-compose` |
| `operations-desktop-activity` | `evidence-desktop-workflows`, `memory-desktop-workflows`, `graph-desktop-workspace`, `mcp-desktop-setup` |
| `desktop-assurance-pulse` | `operations-desktop-activity`, `desktop-knowledge-surface` |
| `desktop-experience-acceptance` | `evidence-desktop-workflows`, `memory-desktop-workflows`, `graph-desktop-workspace`, `mcp-desktop-setup`, `retrieval-ask-experience`, `operations-desktop-activity`, `platform-brain-deletion`, `desktop-knowledge-surface`, `desktop-ask-primary`, `desktop-connection-authority`, `desktop-assurance-pulse` |
| `desktop-chrome-declutter` | `desktop-assurance-pulse`, `desktop-knowledge-surface` |

## Completion Criteria

- A documented personal install and a two-user shared install exercise actual
  UI/API/companion paths, with accurate configured/connected/unavailable states.
- Restore to an empty instance reconstructs eligible graph/search state from
  retained inputs. Rejected or erased material stays excluded after queue replay
  and backup restore; permitted unrelated evidence remains available.
- All seven capabilities have product-path evidence, including corrected-value
  re-ingestion, late evidence, unauthorized access, forged review and erased data.
- One workspace session moves across repositories/environments with concurrent
  tasks; local/private MCP calls and Vault rotation preserve scope and leases.
- Equal-budget retrieval comparisons and concurrent capture/query/analytics runs
  report useful answers, misses, false assertions, provenance accuracy, tail
  latency, write-to-readable lag, memory and cost. State tested limits rather
  than claiming the proposed VM handles an unmeasured corpus.
- Operator runbooks and implementation closeouts report what shipped, what ran,
  remaining limits and any external operator inputs. No incomplete required
  capability is relabeled as shipped to close the epic.

## Desktop implementation evidence, 2026-09-26

The [dated implementation mapping](../../mappings/desktop-experience-implementation-2026-09-26.md)
records implemented routes/assets and the verified final local image separately
from remaining current-code domain and integrated acceptance. The
[desktop guide](../../runbooks/desktop-experience.md) documents the current function
locations. Product/proof slices stay in progress until their required checks pass;
prior shipped domain records remain historical evidence rather than redesign proof.

## 2026-09-29 assurance band and deletion restore proof

[ADR 0017](../../adr/0017-desktop-knowledge-and-ask-experience.md) adds the
exception-first assurance band, delivered by `desktop-assurance-pulse` over
existing authorized feeds only; any figure without a verified response field is
omitted and recorded rather than estimated. [ADR 0016](../../adr/0016-brain-deletion.md)
extends this epic's integrated acceptance obligation with a new negative evaluation:
a deleted Brain must stay absent from every listing, retrieval, graph, audit and MCP
path, and must not reappear after restore from an older backup with the retained
journal applied. `desktop-experience-acceptance` now names all five successor slices
as predecessors.

## 2026-10-01 local closeout

[desktop-assurance-pulse](../execution/archive/desktop-assurance-pulse.md) are locally delivered with [dated validation and runtime limits](../../mappings/desktop-continuation-2026-10-01.md).
The user prioritizes normal laptop and larger desktop displays; additional
small-screen and keyboard polish is optional under the [desktop contract](../../contracts/desktop-experience.md#display-and-interaction-priority--2026-10-01).

## Final desktop acceptance — 2026-10-01

Activity and integrated desktop acceptance are shipped with [current deployment, permission, provider and visual proof](../../mappings/desktop-final-acceptance-2026-10-01.md). LongMemEval remains a separate blocked cost/protocol decision; it is not desktop acceptance.

## Desktop chrome declutter — 2026-10-03

The user's 2026-10-03 screenshot direction removed two chrome elements that
added no information: the assurance band now renders only while something
needs attention (a healthy Brain shows no strip at all), and the four Knowledge
pages dropped the horizontal Memory/Sources/Graph/Repositories switcher — the
contextual sidebar is the sole navigation, while intra-page sub-tabs and the
inspector's "Show in…" links are untouched. Shipped 2026-10-03 with live browser
proof on the healthy SWEG Brain; the [archived pack](../execution/archive/desktop-chrome-declutter.md)
and [evidence](../../mappings/agents-ask-chrome-refinements-2026-10-03.md) record
the checks. The contract amendment lands in the
[desktop experience](../../contracts/desktop-experience.md).
