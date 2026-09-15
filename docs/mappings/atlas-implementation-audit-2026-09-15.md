# Atlas implementation cross-review

Observed: 2026-09-15
Confidence: verified

## Sources and Method

At the user's explicit request, the separate reviewer agent
`/root/atlas_implementation_audit` read all 21 local Atlas pattern files in full,
the pattern index and tensions page: 23 files, 7,212 lines. It cross-referenced
Recollect's accepted foundations/ADRs/contracts, slice ownership, current source,
test assertions and proof mappings. It did not edit either repository or rerun
database/browser tests. Governance lint passed on its audit snapshot. CodeGraph
had pending source changes, so relationships were verified directly in source.

Atlas checkout: `agent-memory-atlas`, commit
`7eca7f7abd934c2e44bc096fc1dd0cbf94275b99`. The corresponding published
[patterns](https://neoneye.github.io/agent-memory-atlas/patterns/) and
[tensions](https://neoneye.github.io/agent-memory-atlas/tensions/) are the user's
references; the local Markdown is the inspected source. Atlas examples of other
systems are reported evidence, not independently reproduced benchmarks.
Cognee remains a separate read-only reference checkout. Parent inspection of its
`hybrid/ranking.py` and `hybrid/context.py` informed ranking/provenance comparisons,
without adopting upstream scores as canonical memory authority.

## Observations

The accepted canonical foundation is substantially implemented. These findings
are concrete current-consumer work; an accepted future pattern is not a shipped
feature. Historical archived packs retain their actual proof and limitations.

| Priority | Finding | Implementation owner and proof required |
| --- | --- | --- |
| P1 | `capture_own_transport` excluded capture commands but missed native `scope recall`. Its tool result could become a new source without original dependencies. | Capture reconciliation: exclude identifiable first-party output before local/server persistence, including quotes and encoded JSON, with unrelated permitted output. Downstream resurrection was not executed by the reviewer. |
| P1 | `model_gateway::source/resolve/body` transmitted source text without canonical capture kind, title, actor or event provenance. | Capture reconciliation: attributed bounded HTTP inputs and prompt/reply/tool controls. Incorrect real-model output was not reproduced. |
| P2 | `learning::run_job` charged all candidates against 5,000 identities before determining reuse/revision. | Capture reconciliation: charge actual inserts; prove revision/reuse/retirement at capacity and atomic failure for a new identity/mixed batch. |
| P2 | Autonomous targets required same source ID; distinct captured events always create distinct source IDs. A supported later correction could not use the existing replacement path. | Capture reconciliation: bounded authenticated binding/session/child lineage, exact applicability and current offered-target guards. Arbitrary cross-document resolution remains unproved. |
| P2 | Captured versions stored host capture time in `created_at`, used as knowledge time by raw recall and canonical freshness. Late uploads could appear known before receipt. | Capture reconciliation and slice 14: use exact original event's server receipt time, independently preserve later version times and capture-based TTL; future receipts use post-lock wall clock. Existing receipts cannot reconstruct historical lock waits. |
| P2 | Explicitly ineligible claims formerly consumed the recall candidate cap. | Slice 14 now filters known states before the cap. Parent's real PostgreSQL test passed with 101 ineligible proposals and an accepted positive control. Complex downstream rule/dependency/time checks remain bounded and coverage is explicit. |
| P3 | Roadmap introduction said application implementation had not begun. | Narrative corrected to 13 original slices shipped, slice 14 active and one explicit audit repair slice. Structural lint alone cannot establish narrative truth. |
| P2 | Recall applied an already-expired support's deadline to a separately retained claim, hiding it even from qualified investigation/history. | Slice 14 preserves canonical qualified eligibility, while strict/raw recall excludes expired support. Regression includes checks before/after the retention sweep and an independent accepted control. |
| P2 | SQL recall prefilters/rendering did not apply the canonical defaults for older claims without `lifecycle` or `rationale`. | Normal-runtime SWEG recall exposed the missing lifecycle default. Preserve `active`/empty rationale without rewriting history; old-format proposed and accepted controls exercise the actual endpoint. |
| P2 | Strict handover recall checked contributing claim retention initially but omitted its deadline from the final response guard. | Include required contribution and support deadlines; a delayed database read crosses expiry, with an independent positive control and qualified investigation/history afterward. |

Parent capacity proof exposed another concrete failure: a renewal awaited inside
the learning select loop could wait on its own publication's job-row lock while
that publication stopped being polled. The corrected learning and handover loops
continue polling publication during renewal. The real 5,000-identity test passed
in 33.53 seconds, proving replacement, reuse, retirement and atomic mixed/new-only
failure. Native/server-normalization protocol controls passed (four library tests).
The subsequent full platform run passed 42 tests in 108.41 seconds, including
capture lineage, temporal visibility, erasure and atomic publication. Protocol
and native library tests, Clippy, API generation and browser recall/capture/claims
passed. The closeout review then identified the additional recall retention and
legacy-format issues above. Final regression passed 44 platform tests in 99.84
seconds, plus eight native/protocol library tests, Clippy and 32 governance tests.
The latest normal runtime and actual-model observations are recorded below.

### Complete pattern matrix

Proof below means inspected implementation and test assertions unless a parent
test run is specifically identified. The reviewer did not establish running
service status. Later slices must prove each new consumer through its actual path.

| Pattern | Accepted disposition and current evidence | Limits and remaining owner |
| --- | --- | --- |
| Evidence before belief | `memory::validate`, `memory_evidence` and `memory::view` retain exact source identities/spans and availability; artifacts remain authoritative over chunks. Capture attribution and recall feedback repairs passed. | Citation existence is not entailment. Actual Luna proof covers the recorded cases, not general semantic correctness. |
| Scope as a first-class key | Immutable operations, Brain RLS, current authority and recall SQL scope filtering precede ranking; manifests are exact. Semantic 15 includes manifest filtering before count/distance, native scope and revoked-grant controls. | Graphs 16–19, fusion 20 and MCP 22–26 need consumer-specific isolation. ANN isolation is not implemented. |
| Governed write gateway | Shared canonical validators plus transactional append/audit/jobs govern review, learning and handovers. Review tests inspect rollback. | One logical policy, not literally one function. MCP 25–26 must call shared handlers. |
| Explicit write destination | Workspace operations and capture bindings fix one authorized Brain and original selection. | Broad read scope is not publication authority. MCP 25 must preserve it without new per-memory confirmation. |
| Trust-state machine | `memory_policy::eligibility` affects actual reads; learning records proposed/uncertain or policy acceptance, without inventing a reviewer. | Models remain fallible. Preserve attribution and operational independence; no mandatory human queue. |
| Rejected-value tombstone | Normalized Brain/applicability assertion rules govern claims and raw recall; the slice-14 copied-source/rebuild test includes permitted controls. | Bounded normalization is not universal semantic equivalence. Supersession is not rejection; erasure is not a global value ban. |
| Append-only memory audit | Review and canonical append retain transactional revision/disposition history; erasure removes payloads and retains minimal fences. | Detailed audit has its own retention. No hashing requirement; full recovery remains 28. |
| Bi-temporal fact validity | Fact time/precision and independent knowledge history are implemented; latest known selection precedes privacy filtering. Delayed capture knowledge time, freshness and no-fallback historical recall passed. | Commit/observation time is not deployment time. Historical pre-repair receipt lock waits cannot be reconstructed. |
| Memory as an editing surface | Canonical UI/API supports review, correction, conflict disposition, withdrawal and Erase. | Human intervention is optional. Arbitrary external writeback/pinning/merging were not adopted; investigation UI 21 extends inspection. |
| Resolve, don't just detect | Typed keep/replace/retract/different-applicability decisions and autonomous disposition affect eligibility. Bound session reconciliation passed controlled and real-Luna proof. | General cross-document resolution remains unproved. Session membership only supplies candidates. |
| Zero-LLM capture | Native normalization/inbox and server admission sanitize and retain permitted events before enrichment; host coverage is explicit. Own-recall feedback is excluded at both boundaries. | MCP 26 is an additional producer; capture completion is not model completion. |
| Recoverable background work | Durable jobs, leases, bounded retries, deletion fences and restart tests exist; charged uncertain calls are not silently resent. | Full installation recovery 28 and workload lag measurement 29 remain. |
| Hybrid retrieval fusion | Slice 14 implements exact identities plus PostgreSQL lexical ranking without a model call. Slice 15 adds exact eligible cosine and RRF, with original fragment attribution and measured real-model comparisons. | Graph fusion 20 remains. Semantic similarity returned unrelated context on unsupported questions; no BM25, universal abstention or superiority claim. |
| Source-diverse context | Current ranking admits the best fragment of each canonical identity before further fragments; parent long-source/independent-source PostgreSQL control passed. | Canonical diversity is not independent corroboration: claims may share evidence. Measure source lineage in 20/29. |
| Pluggable memory provider | Owned gateway/artifact/extraction/canonical interfaces supply narrow seams around one authority. | MCP 25 still needs real capability/scope/status/deletion mappings. A host's separate built-in memory is not automatically controlled. |
| Skills as procedural memory | Typed advisory procedures retain conditions, steps, outcomes and evidence with tested eligibility/erasure. | Untested remains untested; remembering steps grants no execution authority. Executable learning deferred. |
| Cache-preserving injection | Recall returns canonical scope and memory epoch; accepted intent is stable/dynamic separation where supported. | No host prompt-cache behavior is claimed. MCP 25 must prove placement/invalidation; improvement needs measurement. |
| Promotion between tiers | Canonical publication and policy acceptance meet present requirements. | Adaptive hot/warm/cold tiers are explicitly deferred; popularity cannot become truth or evict rejection state. |
| Gate the expensive path | Provider purpose/content/model/budget checks fail closed; exact/lexical no-match uses no LLM. | Learned relevance gates deferred until cost/miss measurements in 15/20/29; no arbitrary threshold. |
| Retrieval hysteresis | No correctness path depends on sticky session context; current authority governs recall. | Deferred cooldown must not hide requested/corrected content. Any host cache in 25 needs immediate scope/revocation/correction/erasure invalidation. |
| Decay and reinforcement | Freshness depends on evidence, dispositions and retention; automatic retirement requires a supported reason. | Continuous scoring deferred. Repetition/access counts/pins do not establish truth; every new producer must preserve the feedback boundary. |

### Tensions

| Tension | Recollect decision and remaining proof |
| --- | --- |
| Recall versus abstention | Strict/investigation and explicit insufficient support exist; 15/20/29 must measure misses and false assertions with permitted controls under equal budgets, including post-cap eligibility loss. |
| Retention versus correctability | Separate policies govern raw evidence, durable memory, rules, audit and backups. Journal replay protects older database fixtures; full empty-instance backup restore/rotation remains 28. |
| Scope strictness versus reuse | Brain ownership is hard; overlapping collections/areas view existing identities. Session IDs cannot override child/task/environment isolation. |
| Asynchronous writes versus readable lag | Capture and processing states are explicit; 29 owns measured write-to-readable and tail latency. Queue acknowledgment is not search readiness. |
| Auditability versus storage growth | Payload expiry and minimal fences exist. `privacy_journal::load` and device privacy sync currently load whole sets; 27–29 must measure growth. This is an unmeasured scaling concern, not observed failure. |
| Reinforcement versus truth | No access-count promotion exists; preserve origin and dependency identity so recaptured recall does not manufacture support. |

## Translation and Limits

The semantic consumer follow-up is recorded in the
[slice 15 implementation mapping](semantic-retrieval-2026-09-15.md). The same
explicitly requested reviewer found four additional issues: late claim-manifest
filtering, mixed-chunk score attribution, a missing handover-contributor publication
deadline and an insufficient historical-vector correction assertion. Its bounded
follow-up confirmed all four fixes and causal negative/positive controls. Parent
validation passed 61 platform tests, eight library tests, browser no-resubmit proof,
Clippy/API/build and governance. The real 3,072-dimensional synthetic corpus used
20 embedding calls and 1,036 recorded tokens, preserving all six prior Brains.
This extends current-consumer evidence; it does not close the graph/MCP successors.

Context7 `/websites/postgresql_17` and official
[CREATE VIEW](https://www.postgresql.org/docs/17/sql-createview.html) and
[time-function](https://www.postgresql.org/docs/17/functions-datetime.html)
documentation confirm invoker-view RLS and post-lock wall-clock timestamps.
Migration 016 shares the original-version knowledge lookup and preserves retention
timestamps. Existing stored receipts cannot reconstruct historical lock waits.

The [accepted foundation dispositions](atlas-foundation-decisions-2026-09-13.md)
remain applicable. No pending foundation decision was used as implementation
authority. The user's autonomous standing policy and absence of product hashing
or strict-version gates remain intact. Deferring adaptive tiers, continuous decay,
hysteresis, learned gates, executable skills and a marketplace is deliberate,
not a claim that their theoretical benefit has been disproved.

### Final parent runtime and real-model proof

The final API/worker began serving at 09:25:15 UTC after an owned restart; migration
016 had applied at 09:16:46. Fresh SWEG recall verified exact snapshot/manifest
attribution, strict exclusion of its original proposed claim, current autonomous
replacement and sanitized captured evidence. Desktop/mobile inspection passed.
The read-only check at 09:27 preserved all six current Brains (the original five
plus the new synthetic demo), with zero additional model calls/customer-file reads.
Exact/lexical SWEG calls took 68/73 ms with 1,584 bytes of exact context.

The new `Autonomous session reconciliation demo` Brain
`b2933e39-5e3a-4547-a7bf-b351c48a61eb` used the compiled native companion, browser
device approval, a separate OS credential profile, generated Codex binding, actual
hook commands and durable drain. Four original events in one bound session flowed
through the normal worker to actual `gpt-5.6-luna` requests. All four succeeded,
charging 15,010 reported total tokens. The selected embedding configuration stayed
`text-embedding-3-large`/3,072; this text-only scenario made no embedding request.

- Initial user assertion created the policy-accepted `Amber.port = 8080` claim.
- The assistant's explicitly unperformed 9090 suggestion left that revision
  unchanged. It produced qualified facts about a proposal and absence of action,
  not an accepted assertion that the current port had changed.
- An explicitly historical 2019/7070 quotation produced no replacement.
- A real `/bin/cat` of the repository-owned synthetic declaration supplied 9090
  and an explicit replacement statement. Its captured tool result revised claim
  `0c989483-88fb-4cc0-b16e-741d2b97cd0e` from revision
  `c09b866a-8ee5-4003-83c4-0c656177ae6c` to
  `c42a1b14-4e88-46e1-87b9-a95bca41e910`, without human review. Strict accepted
  eligibility was true; strict operational eligibility remained false.

No customer files were read or sent. All five original Brains retained their
request totals. The temporary native credential was revoked/removed and its task
closed. Browser recall and desktop/mobile inspection passed. The replayable proof
script at `.cache/capture-reconciliation-live/run.mjs` refuses to blindly restart
incomplete paid work and performs read-only checks after success. Its state and
screenshots retain the exact observations. These cases are empirical evidence,
not a general semantic-reliability or independent-corroboration benchmark.

Deterministic tests additionally prove the 5,000-identity maintenance boundary,
mixed atomic rollback, actual original-source erasure ancestry, same-binding
positive/negative candidate controls, receipt after lock wait, replay, later
source versions, quoted/encoded recall exclusion and publication across heartbeat
renewal/expiry. The final handover deadline test crosses a slow read without
returning expired strict context. Retained claims remain qualified after raw expiry.

## Follow-up

The [capture reconciliation](../roadmap/execution/archive/memory-capture-reconciliation.md)
repair and [exact/lexical baseline](../roadmap/execution/archive/retrieval-exact-and-lexical.md)
have passing local acceptance evidence. Fourteen original slices plus one explicit
audit repair are delivered; the original 29-slice goal remains incomplete. Carry
these obligations into each next pack:

- Semantic 15: exact model identity and 3,072 dimensions, defined representation,
  deletion/reindex, wrong-model rejection, exact eligible-vector control and channel failure.
- Graph 16–19/fusion 20: pre-rank scope, typed evidence, unresolved targets, readiness,
  invalidation of aggregate influence, and equal-budget ablations/provenance.
- MCP 25–26: actual Codex/Claude recall and scope refresh, capture exclusions,
  forget/erase routing, optional review, capabilities and hostile-instruction controls.
- Operations 27–29: full restore/newer deletion journal, missing-journal refusal,
  independent positive evidence, capacity/concurrency/storage/cost measurements and
  an explicit distinction between local delivery and external deployment.
