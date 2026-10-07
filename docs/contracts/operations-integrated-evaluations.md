# Integrated product acceptance and measured limits

Status: accepted

## Automatic-memory support baseline (2026-10-07)

The user's implementation goal for the
[automated memory plan](../research/automated-memory-improvement-plan-2026-10-07.md)
authorizes a repository-owned synthetic support corpus and offline scoring before
the new support gate. This evaluation amendment does not itself enable that gate
or change production memory eligibility. Runtime changes require their own
accepted support contract and execution pack. Ordinary operation remains automatic
under [ADR 0006](../adr/0006-autonomous-memory.md).

Keep original synthetic evidence, canonical speaker/tool attribution, exact cited
spans, proposed assertions/actions and expected support dispositions in a versioned
JSON corpus. Separate calibration from held-out cases. Cover whole-assertion
meaning, unrelated valid citations, negation, history, proposals, failed tools,
corrections, retirement, procedures/handovers and environment identity. Never
send gold labels or evaluation rationales to a provider. Fixtures prove handler
enforcement; they cannot count as semantic-model quality evidence.

Freeze thresholds before any held-out paid run: every critical negative remains
unusable, at least 80% of supported held-out controls are usable, and every held-out
case has a completed assessment. Report missing/failed/uncertain outcomes separately;
an unavailable assessor cannot pass by withholding everything. Report support
verdict accuracy separately from canonical usability, plus request count, charged
tokens and latency. A fixture run can pass fixture enforcement but must never pass
the semantic-quality gate. Supplied result files are observations, not proof that
the reported gateway/provider requests actually ran; retain and validate that
runtime evidence separately.

Offline corpus checks/scoring make no network call and load no credential. A
later separately named synthetic provider run uses the existing gateway in an
owned isolated Brain, the selected installed model, content/purpose permits,
250,000 daily tokens, output at most 1,024 tokens and concurrency one. Persist
request IDs before dispatch, do not automatically repeat unknown external
completion and preserve normal Brains and customer data. Baseline preparation is
distinct from a paid run, a production cutover or benchmark superiority.

## Public retrieval benchmark (2026-09-28)

The user requested benchmarks informed by Cognee and Atlas. A separate opt-in
HotpotQA retrieval harness may use the first 50 validation distractor rows,
frozen by SHA-256 before running, with every supplied context paragraph pooled
into one isolated Brain. Import public paragraphs only; questions, answers and
supporting-fact labels remain outside memory. Use canonical product ingestion
and recall with the same ten-item/16-KiB budget per query. Report supporting
document recall, complete supporting-document coverage, precision, latency and
index readiness. This is pooled-corpus evidence retrieval, not official HotpotQA
answer EM/F1, a BEAM memory score or a head-to-head Cognee comparison.

Keep lexical-only and semantic-enabled lanes named explicitly, recording model
usage and failures without discarding misses. Begin with the model-free baseline;
an opt-in semantic comparison may use the installed embedding model, an isolated
500,000-token daily limit and at most two concurrent calls, without extraction,
learning, generation or LLM judging. Persist request identities before dispatch
and stop uncertain attempts instead of resending. Use identical questions and
budgets without changing the retriever after seeing the gold labels. Do not
insert synthetic claims or tune the index for individual questions. Atlas lifecycle
invariants are separate production-handler positive/negative tests. Preserve
normal Brains and credentials by using the owned ephemeral proof database.

## Source

The user's full-product goal authorizes this final slice and its routine acceptance
details. The [pilot and seven capabilities](../foundation/vision.md),
[proof principles](../foundation/engineering-principles.md#prove-useful-behavior-and-failure-boundaries),
[operational epic](../roadmap/epics/operational-readiness.md),
[installation](operations-local-and-shared.md), [recovery](operations-recovery-drills.md),
[fusion](retrieval-graph-fusion.md), [handovers](memory-procedures-and-handovers.md)
and existing scope, review, retention, provider and MCP contracts govern behavior.

## Contract

### Owned evaluation boundary

Run a reproducible, opt-in operator harness against explicitly owned proof
installations. Persist private run state and request identities before dispatch,
refuse to overwrite a run or automatically repeat an uncertain paid attempt,
and emit a content-free summary plus declared synthetic evaluation evidence.
Use existing product handlers and workers; a direct database fixture is labeled
and cannot substitute for a claimed product mutation. No new benchmark service,
model gateway, graph algorithm, hash scheme or product format is needed.

Do not copy customer repository contents into evaluation fixtures. Use the already
authorized retained SWEG evidence only for a separately labeled read-only smoke
check; synthetic workload results are not customer-corpus capacity. Keep normal
Brain/model/grant state and unrelated services preserved. Vault stays optional;
no token or lease revocation is permitted. Mobile and external deployment remain
outside the task.

### Integrated workflow and seven capabilities

Exercise two accounts contributing different local checkout paths to one Brain,
with a second inaccessible Brain as a canary. Register 50 synthetic repository
identities, select at least two repositories and development/production views,
and capture concurrent task operations while changing a task's future default.
Earlier capture/recall bindings must keep their original account, repository,
environment and operation attribution. A private path must not appear in another
account's workspace. Strict and investigation recall preserve their trust boundary.

Map every required capability to concrete positive and negative API/worker/MCP/UI
evidence: durable rejected-value rules, independent trust dimensions, bi-temporal
history, enforced scope, transactional mutation audit, optional actionable human
review, and paired absence/presence evaluations. Include correction/re-ingestion,
late capture, forged/stale review, source/derived erasure, private execution grants
and replay/recovery. Authorized automatic learning/revision/forgetting still needs
no human reviewing each memory; human actions are an optional control surface.

Reuse focused production-handler regression cases and dated installed/native-host,
Vault rotation/lease and recovery proof when those paths are unchanged. Record
which checks ran in this slice and which are carried evidence. An ignored test,
configured endpoint or mocked provider cannot count as a fresh external call.
Complete a desktop workflow on the final application build, including review,
scope and provenance; preserve existing failure/unavailable behavior.

### Atlas lifecycle proof (2026-09-29)

Under the user's benchmark request, a separate opt-in deterministic harness
(`RECOLLECT_ATLAS_LIFECYCLE=1` through the owned ephemeral proof database) may
execute the agent-memory-atlas benchmarks page §6 deletion sequence and §7
contradiction matrix as a machine-scored pass/fail matrix through the product
API: recall qualifications, claim state, erasure status, conflict gates, graph
view, evidence catalogue, audit events and model-usage assertion — never an
LLM judge, never a model call. Verdicts follow the page's semantics: absent or
qualified stale values pass, unqualified current assertions and silent picks
fail, and untested paths are declared `N/A` with a reason rather than scored
zero. Results are dated mapping evidence, not a universal memory score. The
answer-level complement (LongMemEval) stays deferred to the
[proposed protocol](operations-longmemeval-protocol.md), which requires
explicit user cost approval before any dispatch.

### Fixed-corpus retrieval and generation quality

Reuse the checked-in seven-document/ten-claim fusion corpus and its eight declared
questions (six supported, two unsupported). Freeze this corpus and labels before
running. Compare exact/lexical, graph addition, semantic addition and full fusion
with identical four-item/8,192-byte context ceilings and the same installed
`text-embedding-3-large` profile at 3,072 dimensions. Use the source-diversity
preference consistently; its separate ablation already has dated evidence.

Run all recalls before generated handovers can become new retrieval candidates.
Afterward, disable automatic embedding and generate a bounded engineering handover
for each lane/question with retrieved claim contributors through the existing
governed synthesis endpoint. The title/question, installed `gpt-5.6-luna`, input
ceiling 16,384 bytes, output ceiling 1,024 tokens and fixed product prompt/schema
are identical across lanes. A lane without claim contributors reports no generated
answer, not a model success. The gateway remains the sole provider dispatcher.
No automatic model retry or alternative provider is allowed. Brain admission is
capped at 500,000 daily tokens and concurrency one for this small opt-in run.

Report target coverage, misses, unsupported-question context, irrelevant context,
exact source/revision attribution, model/returned-model usage and per-lane latency.
Review generated summaries against the frozen facts: count answered targets,
unsupported factual assertions and explicit uncertainty; suggestions phrased as
future work are distinct from assertions that work occurred. Keep those bounded
annotations with the synthetic outputs and disclose the single-reviewer limit.
Require all six full-fusion target records, at least four useful supported full-lane
handover answers and no unsupported factual completion/authority assertions.
Every returned identity/support must resolve correctly and forbidden canaries must
be absent. Unsupported semantic context is reported rather than described as a
reliable abstention mechanism. This experiment cannot establish general model
superiority or independent corroboration.

Use actual provider usage for operating cost. Report tokens separately from an
optional monetary estimate with explicit dated/user-supplied rates; never treat
an estimate as an invoice. Baselines with no model request report zero model cost.

### Concurrent workload and local envelope

Use a separate synthetic Brain so performance writes do not change quality labels.
Import 200 retained documents of approximately 1 KiB, alongside the 50-repository
catalogue and a committed structural graph fixture. Disable model transmission
for this workload. Then run four capture producers and four recall readers,
30 operations each (120 capture events and 120 recalls), while four native GDS
analyses enter the existing heavy queue. Do not bypass admission or increase
application worker/DB limits to hide saturation.

Record client wall-clock p50/p95/p99/max latency, import duration, write-to-readable
lag, final queue drain, failures/refusals, data growth and periodic memory/CPU
observations for only the selected installation. Record actual host/runtime/image,
limits, corpus count/bytes and sampling interval. Sampled memory maxima are not
continuous peak measurements; Docker Desktop and other workloads affect results.

For this bounded local workload require zero unexpected request failures, no scope
mixing or forbidden/stale numeric output, recall p95 below two seconds, capture
admission p95 below two seconds, all accepted events readable within 60 seconds
of admission and final owned work drained within 180 seconds. Canonical input
changes may legitimately stale/cancel concurrent analytics; require a fresh WCC
report to complete after writes settle, with expected graph membership. Invalid
or over-limit input must fail while a following ordinary request succeeds.
If a threshold fails, report it and repair the actual bottleneck or record the
unmet acceptance; do not silently reduce the declared workload or relax its gate.

The mixed-workload proof found continuous readers starving a waiting Brain
writer under PostgreSQL row locks. Add transaction-scoped shared/exclusive
admission before the existing Brain row lock: a new reader waits behind a queued
writer, existing readers finish, and different Brains remain independent. An
internal unique integer identity supplies the advisory key in a reserved two-int
namespace; public UUIDs, authorization, row locks and mutation audit remain the
authority. Commit/rollback releases admission automatically. No digest, timeout
increase, worker-limit increase or application-local lock is introduced. Prove
the previous starvation and the corrected ordering against real PostgreSQL, then
repeat the unchanged workload and affected runtime/recovery checks.

The same workload also exposed repeated capture-event/binding scans in canonical
recall. Materialize the selected Brain's authorized capture associations and
source knowledge once per SQL statement. Preserve the exact-version receipt
time and original source-level capture scope (including later source versions),
RLS, current retention/correction gates and final canonical qualification. This
is statement-local reuse, never a cross-request authority or content cache.
Compare the real query plan and results and repeat the existing scope/history/
erasure regressions and fixed workload without increasing limits.
Source-lineage qualification likewise uses the exact unique excerpt, parent,
capture-version and binding identities for bounded lookups. It must not repeatedly
scan a Brain's parent versions when current import statistics are incomplete.
Preserve missing-parent/opaque-source behavior and current capture metadata expiry.

The proposed 8-CPU/32-GB VM, larger corpora, more simultaneous users and sustained
production throughput remain unmeasured. Publish only the tested envelope and
the existing resource/admission limits, including single-lane heavy work.

## Acceptance

Complete the declared workflow, fixed-corpus actual-model comparison, concurrent
workload, required focused regressions and desktop proof. Reconcile an evidence
matrix with all seven capabilities and the personal/team/host/Vault/recovery
predecessors. Run required workspace/Clippy/web checks when code changes justify
them and `./scripts/validate.sh`; keep the overall goal incomplete until the
remaining criteria pass. Document local startup, measured limits and unverified
external operator obligations. No automatic release, commit or deployment.

## Explicit Deferrals

General chat/answer service, learned ranking/gating, larger-scale tuning, universal
capacity claims, high availability, external cutover and mobile views are excluded.
