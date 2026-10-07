# Muse memory comparison and Recollect implementation audit

Observed: 2026-10-07
Confidence: verified

Verification scope: published-source and local-code audit. MUSE runtime behavior
was not independently verified.

## Sources and Method

The user supplied a summary and explicitly requested two independent agents,
then a fully automated implementation plan. Source review and product-fit review
inspected published primary material, current working-tree code, accepted
contracts and retained tests/validation. This is a source audit, not new runtime
acceptance. Base Git revision: `f18fc15`; relevant implementation is dirty and
uncommitted, so that revision alone does not reproduce the inspected code.

- [Meta announcement](https://about.fb.com/news/2026/09/introducing-muse-personal-ai-agent/)
  and [official technical account](https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse).
- [Peter James's single-runtime export report](https://mouse.dev/blog/muse-runtime-export/),
  its [dream screenshot](https://mouse.dev/img/blog/muse/fig7.webp) and
  [memory diagram](https://mouse.dev/img/blog/muse/memory-diagram.webp).
- Recollect's accepted autonomous, provider, capture/reconciliation, handover,
  retention and plugin contracts; exact source inspection with `rg`/bounded reads.
- Repo-local CodeGraph `status` and `sync`: up to date; bounded node/caller
  results checked against source. Literal/authority claims come from source.
- Independent reviews: `muse_source_review` examined correctness, tests and
  source confidence; `muse_recollect_fit` examined digest/context design and
  authority, scope and lifecycle constraints. Both worked read-only.
- Follow-up independent reviews: `support_consumer_review` rechecked the written
  plan and consumer boundaries; `support_storage_review` rechecked automatic
  recovery, privacy and lifetime dependencies. They confirm the research sequence,
  with implementation and real-model acceptance unfinished at that planning snapshot.
  Their later implementation closeout is recorded in the mapping linked below.

No export archive was downloaded. No customer payload, secret, provider call,
service mutation or product-code edit was needed for this audit. Documentation
validation is recorded below after the authored plan is checked.

## Observations

### External evidence

Meta describes persistent inspectable memory, a dedicated Linux VM, an isolated
runtime cell and durable Postgres outside the agent's runtime/credential store.
These are official published design descriptions, not independent runtime proof.
James reports source-checked claims, quotations/message IDs, structured claims,
supersession, nightly reflection and synthesis from one exported environment.
The export itself was not independently inspected here.

The published dream screenshot shows `prompt_hoisted: false`. The prose/diagram
does not establish `prompt_hoisted: true` for another file, system-prompt insertion
every session, full-history nightly distillation, network-offline processing or
zero context cost. File-based personal adaptation must also be distinguished from
Meta's separately described use of sanitized trajectories for future model training.

### Recollect at the initial comparison

| Finding | Exact source evidence | Verification limit |
| --- | --- | --- |
| Schema, citation bounds and canonical evidence are checked before learning publication | `crates/server/src/learning.rs:546–584`; `crates/server/src/memory.rs:48–113` | No independent semantic support check is present in these paths |
| Autonomous policy accepts general non-conflicting candidates | `crates/server/src/learning.rs:754–779` | Policy acceptance does not establish source truth or live operational success |
| Retirement grounds validate cited bounds | `crates/server/src/learning.rs:598–616` | A valid range can still be unrelated to the requested retirement |
| Proposed/qualified records can enter investigation eligibility | `crates/server/src/memory_policy.rs:226–244` | An uncertainty label alone is not an ordinary recall exclusion |
| Direct agent contributions use the same canonical claim-write path | `crates/server/src/memory.rs:232–249`; [claims contract](../contracts/memory-claims-and-time.md#independent-state-and-eligibility); `crates/server/src/mcp/agent/catalogue.rs:193–200` | Browser/device origin identifies transport, not human authorship or review; these unreviewed contributions need support assessment too |
| Handovers generate/accept prose and automatically refresh existing records | `crates/server/src/handovers.rs:334–429,462–523`; `crates/server/src/autonomous.rs:290–292` | No automatic session-digest producer or independent prose support check established |
| Reconciliation targets source lineage or exact bound session/child | `crates/server/src/autonomous.rs:35–63,91–93`; [contract](../contracts/memory-capture-reconciliation.md) | Twelve offered revisions; cross-document semantic reconciliation explicitly deferred |
| Plugin recall is query-based, scoped and bounded | `crates/agent/src/plugin_session.rs:324–416` | Six requested items/6,000 context bytes inside an 8 KiB framed output; no stable project-brief section established |
| Task closure does not itself generate a summary | `crates/server/src/workspace.rs:878–897`; `crates/agent/src/plugin_runtime.rs:515–601` | Late delivery/concurrent append remains possible; closed does not mean a complete transcript |
| Raw evidence and durable excerpts have independent retention | [retention contract](../contracts/memory-retention-and-erasure.md) | Durable briefs must not silently keep whole transcripts or infer permission to copy excerpts |
| Knowledge permissions are Brain-level | [engineering principles](../foundation/engineering-principles.md#bind-identity-scope-and-authority-explicitly) | Actor attribution/area labels do not make inferred personal preferences private |

### Test and historical validation evidence

`crates/server/tests/platform/capture_reconciliation.rs:224–260` supplies empty
provider output for assistant proposal/history cases. It meaningfully checks
canonical provenance and handling of compliant responses, not rejection of a
nonempty unsupported assertion. Existing autonomous tests also cover capacity,
atomicity, retries, budget, malformed output, erasure and restore; these should be
retained as lifecycle controls.

The [September 15 Atlas audit](atlas-implementation-audit-2026-09-15.md) and
[archived reconciliation pack](../roadmap/execution/archive/memory-capture-reconciliation.md)
record four synthetic real-Luna events using 15,010 reported tokens. Suggestion
and history preserved the old fact; explicit synthetic tool evidence revised it;
strict operational eligibility remained false. This is dated narrow success,
not a present-day broad semantic-quality corpus or newly executed proof.

## Translation and Limits

The useful mechanisms are support assessment before usable publication,
automatic directly evidenced session digests, compact scoped context and later
bounded corrections across sessions. Reuse Recollect's canonical identities,
policy, provenance and erasure model. A retrospective audit alone leaves an
initial contamination window. A learned verifier is fallible and may share
extractor errors; it does not independently verify the external world.

Both reviewers identified consumer coverage, staging erasure/replay, raw-evidence
expiry, late uploads, private/shared visibility and missing manifest authority
as necessary specification work. These cannot be fixed by a nightly timer alone.

The [implementation plan](../research/automated-memory-improvement-plan-2026-10-07.md)
was proposed and linked from the existing owners at the initial planning closeout.
The user subsequently authorized full implementation. The
[support evaluation baseline](../roadmap/execution/archive/memory-support-evaluation-baseline.md)
now has local native actual-model acceptance under the unchanged evaluation
amendment. All five phases have accepted contracts and archived execution packs.
[Subsequent implementation and evaluation](memory-source-support-staging-2026-10-07.md)
records current delivery; the initial source audit above remains historical.
The running installation and installed host have not been upgraded.

## Follow-up

Promote the selected phase into amended contracts and its owning slice/pack before
dependent code. Establish a synthetic adversarial/positive baseline; then prove
the publication gate and every affected consumer before adding summaries/context.
Full automation follows ADR 0006: standing policy, bounded work, terminal uncertainty,
optional human override and no per-record acceptance requirement.

Planning validation: `./scripts/validate.sh` passed governance lint and all 32
checker tests; `git diff --check` passed. Both agents reviewed the written plan.
Their corrections cover protected human-reviewed model-origin eligibility,
non-replayable responses before staging commit, and explicitly amended admission
deferral rather than counting budget waits as model retries, root-generation
deduplication versus linked replacement attempts, and exact-family discovery after
typed extraction for cross-session targets. Version: N/A,
planning only. Commit: uncommitted. Product implementation/deployment: not
performed by this task.

The follow-up independent recheck clarified two further implementation boundaries:
the explicit human-decision exception cannot be inferred from browser/device
origin, and auditing support does not authorize rewriting author-owned content.
The 5,000-claim capacity counts identities: revision/reuse/retirement continue at
capacity but do not free a slot for a new identity. The plan reflects both;
ordinary erasure/retention is not asserted to reclaim identity capacity.
The product-fit recheck also found that query-only recall cannot dependably choose
a session digest for “continue,” particularly during semantic fallback. The plan
now names a bounded server-validated continuation selector, with resumed lineage,
ambiguity, same-scope unrelated-session and semantic-denial controls; it shares
the existing context budget and never guesses an arbitrary latest session.
The current excerpt command adds a parent-span link and processes the new source
(`evidence.rs:989–1018`), but does not copy imported scope or capture attribution.
The new automatic producer must preserve both and exclude its copies from
independent extraction/corroboration and nested copying. The plan includes scoped
assistant-reply, raw-expiry and denied-original-transmission controls; no current
excerpt-path delivery claim is inferred from the proposal.
The human-review exception also retains required-contributor eligibility:
reviewing a parent handover cannot make an unsupported contributor usable. The
plan adds a reviewed-parent/unsupported-child test and an independently supported
reviewed control through the shared dependency assessment.

## Subsequent authorized implementation

All five phases are locally implemented and validated. The
[implementation mapping](memory-source-support-staging-2026-10-07.md) records typed
staging, independent whole-assertion checks, shared consumer guards, direct/legacy
audits, handover checks, automatic session digests, minimal durable evidence,
scoped briefs/authenticated continuation and exact-family cross-session discovery.
Two independent reviewers checked consumer authority and storage/recovery, including
a corrected human-review/source-expiry regression and current native test evidence.

The versioned actual-model comparison retains v1/v2 failures and final v3 results.
v3 held-out withheld all 15 negative cases and delivered all 11 positive controls;
calibration delivered 10/11 positives and withheld all 13 negatives. Total measured
usage across retained versions is 124 calls/148,288 tokens. This is a repeated fixed
synthetic corpus, not general truth, a fresh independent population estimate or
superiority. Deployment, commit, push and installed binary upgrade remain separate.
