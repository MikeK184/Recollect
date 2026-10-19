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

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `operations-public-benchmark` | shipped | contract-backed | archived | Frozen 50-question lexical/semantic evidence retrieval reports, provider usage and Atlas lifecycle proof; not a universal memory score |
| `operations-atlas-lifecycle-proof` | shipped | contract-backed | archived | Deterministic Atlas §6 deletion sequence and §7 contradiction matrix as a 40-cell pass/fail matrix through the product API with zero model calls, plus digit-for-digit HotpotQA reproduction |
| `operations-longmemeval-bench` | shipped | contract-backed | archived | Official LongMemEval-S answer-level run under the [accepted protocol](../../contracts/operations-longmemeval-protocol.md); USD 10 OpenRouter campaign authorized October 8; three accuracies, pinned judge, frozen dataset hashes |
| `operations-local-and-shared` | shipped | adr-backed, contract-backed | archived | Actual personal/shared UI/API, HTTPS native pairing/MCP, persistent state, migration failure, dependency outage, graceful drain and diagnostics verified |
| `operations-recovery-drills` | shipped | adr-backed, contract-backed | archived | Actual encrypted SFTP, offline restore, journal continuity, erasure/replay, failed upgrade, interrupted resume, graph/recall and desktop proof |
| `operations-integrated-evaluations` | shipped | contract-backed | archived | Actual-model quality, seven-capability matrix, unchanged 50-repository/200-document/eight-caller workload, repaired admission/queries, final restore/desktop and normal upgrade verified |
| `operations-root-compose` | shipped | adr-backed, contract-backed | archived | Root Compose image/browser login, preserved inventory, full stop/start, migration-failure gate and verified fixture shutdown pass |
| `operations-memory-quality-first-batch` | in-progress | contract-backed | pack | The dated installed migration-047 proof passed only 35/40 normal reads. Latest October 18 local repairs pass sustained mixed 10k reads 602/602 and both normal-data clone variants below two seconds; normal installation update and comparable post-repair memory-quality run remain open |
| `operations-proof-cleanup` | shipped | contract-backed | inline: explicitly authorized removal of verified disposable local fixtures; no product implementation or schema change | Removed 50 containers, 49 volumes, 9 networks and 10 fixture directories; current seven-Brain inventory and readiness preserved |
| `operations-restore-statistics` | shipped | adr-backed, contract-backed | inline: authorized restore-readiness correction within the existing explicit recovery hold; no new interface or product decision | Explicit release rebuilds planner statistics after privacy reconciliation; native blocked-analysis/hold/retry proof and four recovery operator tests pass locally (2026-10-18), with no deployment |
| `operations-desktop-activity` | shipped | adr-backed, contract-backed | archived | Authorized bounded Activity feeds and diagnostics with canonical detail/recovery links |
| `maintenance-per-brain-budget` | shipped | contract-backed | inline: one Brain that cannot make progress must not stop other Brains' maintenance, as the [autonomous maintenance contract](../../contracts/memory-autonomous-maintenance.md) already requires (scheduled work proceeds while unavailable work stays visible); paused-clock unit tests plus an actual PostgreSQL held-admission-lock test; no contract change | Graph, autonomous and semantic maintenance share `worker::each_brain`: 30 s per Brain, no new Brain after 90 s of the 120 s pass, least-recently-attempted first with a timed-out Brain moved to the back, 2 s lock timeouts, and `subsystem`, `brain_id`, `outcome`, `code` and `elapsed_ms` log fields; five unit tests and the held-lock case pass (see the [verification receipts](../../mappings/graph-native-verification-2026-10-19.md) for the platform subset) |
| `desktop-experience-acceptance` | shipped | adr-backed, contract-backed | archived | Real desktop/API/agent regression, answer quality/privacy, resource measurements and rollout/rollback evidence |
| `desktop-assurance-pulse` | shipped | adr-backed, contract-backed | archived | Exception-first assurance band over existing authorized feeds, with Activity reordered as its drill-down |
| `desktop-chrome-declutter` | shipped | contract-backed | archived | Assurance band hidden on a healthy Brain (renders only while something needs attention); the four Knowledge pages drop the horizontal tab strip that duplicates the contextual sidebar |
| `platform-dependency-dedupe` | shipped | contract-backed | inline: remove duplicate Rust dependency versions that Recollect's own direct dependencies and features can remove, without semver-major API churn and without starting the separate sqlx 0.9 slice; `cargo tree -d` shows no reqwest 0.12, hashlink 0.9 or hashbrown 0.14, and the agent, mcp-runtime and server suites match their before-change results; remaining duplicates and blockers recorded in the [development tooling runbook](../../runbooks/development-tooling.md#duplicate-rust-dependencies) | rusqlite 0.32.1 to 0.40.2 (`default-features = false, features = ["bundled"]`, no source change; nothing calls `prepare_cached`), reqwest 0.12 gone through `platform-oidc-shared-tls`, and `Cargo.lock` loses hashlink 0.9, hashbrown 0.14, flume, sha1, sqlx-mysql and sqlx-sqlite; crate names with several versions fall from 32 to 30 (68 to 63 versions); the rusqlite move needed `uuid`/`chrono`/`json` enabled on `sqlx-core` and `sqlx-postgres` instead of the sqlx facade (its weak features admit sqlx-sqlite, whose `libsqlite3-sys ^0.30` conflicts with rusqlite 0.33 and later), a split to keep until sqlx 0.9; agent lib 97, mcp-runtime lib 13 plus 8 of its 9 ignored transport cases, server lib 91 and the auth/team/pairing/OIDC platform subset pass as before, and `publication` still stops at its missing Enola fixture; the remaining 30 duplicates are all pinned upstream (table in the runbook) |
| `operations-test-harness-reliability` | shipped | contract-backed | inline: make the platform and browser test harness independent of the target directory, stack size, SFTP fixture, compose file and Postgres shared memory, with a safe cleaner for leftover databases; platform suite, library tests, web build and validate.sh | Helpers and per-test state resolve from the Cargo target directory; RUST_MIN_STACK defaults to 8 MiB and the three weighted-organization tests pass at the default 2 MiB stack after boxing their publication awaits; the two SFTP cases print NOT RUN without a fixture and passed with one; test-ui.sh defaults COMPOSE_FILE and regenerating the API schema is a no-op; Postgres containers get 256 MB of shared memory (applies on recreation); scripts/clean-test-databases.py listed 733 leftover databases in a dry run (none dropped). Browser specs platform.spec.ts and evidence.spec.ts still fail on stale UI wording and navigation, and two platform tests that predate this slice still fail (graph lost-lease cancellation and the 5000-input organization publication, which exceeds its 5 s preparation bound on this loaded machine) |
| `operations-worker-reliability-followups` | shipped | contract-backed | inline: resolve the four reliability follow-ups left by the test-harness slice (graph lost-lease test, graph Brain starvation, 5000-input organization preparation bound, semantic admission cancel under load) with root cause, a fix or a justified test change and measured evidence; graph/maintenance/organization/semantic platform subsets no worse than the before baseline, fmt, clippy and validate.sh; the graph preparation and durable-work contracts record the retry ordinal, stale-worker reporting and scan stamp | The lost-lease test exposed two real graph defects, now fixed with three new actual PostgreSQL/Neo4j tests that fail without the fixes: a stale worker whose fenced native write failed reported success (it published nothing; it now reports the lost lease) and an admin retry restarted the attempt count below the native import ordinal, so retrying a started generation could not succeed (retry now keeps the ordinal and adds three attempts); the graph Brain scan stamp is committed before the work so a Brain whose maintenance always fails leaves the 100-Brain window head (a Brain whose row lock is held elsewhere is skipped by the stamp, not waited on). Preparation of 5,000 sources took 3.7 to 6.9 s per pass against the 5 s contract bound in the unoptimized test build on a shared machine (3.7 to 5.4 s in the phase profile: eligibility about 1 s, qualification 3 to 3.7 s, deadline 0.4 to 0.5 s), so that test now repeats bounded-preparation deferrals as production does and records them (5 of 5 reruns passed with none); whether the bound or the gate cost should change for the 10k-node envelope is an open product decision, and no gate was altered. The semantic cancel/reclaim admission tests failed when the 5 s lease heartbeat overlapped the staged mutation (reproduced deterministically with a 6 s stall) and now repeat such inconclusive attempts on a fresh fixture (20 of 20 under load above 100). Graph, maintenance, organization and semantic subsets: 134 of 136 pass against 129 of 133 before (2026-10-19); the two remaining failures predate this slice (the SFTP recovery fixture is absent, and the 062-schema upgrade test calls migration-070 Brain admission), fmt and validate.sh pass, and clippy reports only existing warnings. No deployment |
| `operations-supply-chain-pinning` | shipped | contract-backed | inline: every Dockerfile `FROM`, compose `image:` and script image reference is `name:tag@sha256:<index digest>`, GitHub Actions are commit-SHA pinned, and the age, Enola and rust-analyzer downloads fail closed on a SHA-256 mismatch before extraction; validation: `scripts/check-pins.py` (wired into `./scripts/validate.sh` and the `pins` job in `code.yml`), 25 unit tests, uncached recovery, agent-coordination and capture-hosts image builds and base-stage resolution for the product and runner images | Supply-chain pins for every image and downloaded tool, with an offline checker enforced in validation and CI; real verified downloads of age and Enola and a deliberately wrong pin rejected with nothing extracted |

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

## Current status

Installation, recovery, checkpointed local updates and the completed 50-question
benchmark baseline are delivered. New local mixed-workload and normal-data clone
reliability gates pass; a reviewed normal installation update and comparable
post-repair quality run remain open. The latest official LongMemEval result is
still 20/50 (40%), not a score for the later repair set. The separate real
current4 campaign fails real extractor/grouping qualification, with canonical
behavior 4/6. It does not qualify automatic semantic publication. The separate
main semantic lease repair passes six native tests; deployment remains open.
The [graph delivery summary](index.md#graph-delivery-status) distinguishes
implemented plumbing, failed quality and unperformed rollout.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Shipped packs](https://github.com/MikeK184/Recollect/tree/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive) were removed from the tree on 2026-10-19 and remain in Git history.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/operational-readiness.md).
Dated test/deployment observations are not a current runtime health claim.
