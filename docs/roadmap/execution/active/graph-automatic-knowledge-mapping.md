# Automatic entities, reversible aliases and overlapping topics

Status: in-progress
Owning epic: `docs/roadmap/epics/graph-intelligence.md`
Work type: product
Implementation: resumed 2026-10-18 by the user's complete graph implementation request

## Summary

- Goal: Automatically organize eligible Brain evidence into typed entities,
  reversible aliases and overlapping topics with useful bounded navigation.
- Non-goals: Replace the Rust core, change truth/permission meaning, overwrite
  manual organization or add a compulsory foreign-language service.
- Delivery shape: Rust candidate/persistence/worker/protocol/API implementation,
  incremental privacy-safe reconciliation, UI and measured end-to-end proof.

### Remaining acceptance gates

Current delivery status is tracked only in the
[roadmap](../../epics/index.md#graph-delivery-status). The gates below are this
pack's acceptance; passing technical checks alone never activates a producer or
recipe.

| Part | Remaining completion gate |
| --- | --- |
| Worker, spans, entities and human alias decisions | A qualified real producer; implemented admission is not model quality. |
| Topics, human overlays, navigation and privacy | Passing automatic grouping/label quality and complete integrated organization acceptance. |
| Real extraction | At least 95% precision/80% recall under a new frozen candidate and untouched evaluation boundary. |
| Real canonical semantics | Correct historical handling, required retirement and exact citation, with actual support/publication linkage. |
| Real hybrid grouping | Frozen baseline comparison, at least 80% annotated in-scope assignment and independent label/usefulness acceptance. |
| Semantic execution lease | No remaining code change for the proven local fix; deployment and documented API/Linux/network-atomicity limits remain separate. |
| Full delivery | All contract acceptance and final integrated closeout. |

The compact [evidence receipts](../../../mappings/graph-organization-implementation-2026-10-18.md)
hold the 105-call results, earlier failures (all 92 observations, 460 mode rows
and 21 unknown topic pairs per producer), frozen input hashes and cost approvals.
The 2026-10-18 design and preparation notes are at the pinned
[full text](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/mappings/graph-organization-implementation-2026-10-18.md#execution-pack-design-and-preparation-ledger);
they are evidence of their own checkpoints, not current completion.

## Governing Sources

[ADR 0020](../../../adr/0020-automatic-knowledge-organization.md),
[organization contract](../../../contracts/automatic-knowledge-organization.md),
[canonical graphs](../../../adr/0007-canonical-graph-projections.md),
[concurrent preparation](../../../contracts/graph-concurrent-preparation.md),
[epic](../../epics/graph-intelligence.md).

## Scope

- In scope: All five product improvements in the dedicated plan, with extractor
  fixtures, entity/alias storage, topics/overrides, automatic updates, current
  qualified navigation, privacy/restore closure and acceptance reporting.
- Out of scope: Unproved mandatory Graphify/Graphiti adapters, paid cloud
  summaries, paid benchmark execution without separate approval, commits and pushes.
- Blockers: Actual extractor, grouping and canonical semantics qualification
  fails; blind label usefulness and final integrated closeout remain open.
  Independent implementation and deterministic/native proof can continue.

## Decisions

- Interfaces: Bounded optional interpretation candidates, typed semantic
  organization APIs and version-checked topic/alias overrides; no fake claims.
  `POST /organization/view` takes the authenticated `GraphSelection`, a keyset
  cursor and an optional topic/entity center, and returns a typed
  topic/entity/evidence envelope; semantic IDs are never claim revisions.
  Commands to pin, exclude, split and rename topics need browser writer authority,
  a base change ID, an idempotency key and mutation audit.
- Storage: Canonical persisted entities, mentions, supported alias/relation
  records, topics, automatic memberships, overrides and durable mapping inputs.
  Topic kinds are automatic or `manual_overlay`; edited topics are kept as
  overlays on their still-valid sources beside new automatic groups.
- Ownership: Graph intelligence owns organization; evidence owns source/manual
  groups; memory lifecycle owns corrections, erasure and restore dependency replay.

## Data and Authority

Decisions that govern the remaining work (the full design notes are in the
[implementation mapping](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/mappings/graph-organization-implementation-2026-10-18.md#execution-pack-design-and-preparation-ledger)):

- A reader qualifies at most 250 exact source contributors per page through
  canonical graph qualification, derives labels and counts only from them, marks
  partial counts as lower bounds, and shares the existing two-reader capacity.
  A final short transaction reauthenticates and checks the preparation epoch and
  the earliest retention deadline. A withdrawn center yields an empty view.
- Only worker output with exact production provenance publishes a mapping input.
  The release catalogue of qualified producers starts empty and admits an entry
  only after independent passing receipts for the exact prompt, schema and
  adapter; no client-supplied hash activates a producer. Validated but
  unpublished output is retained (at most 256 KiB per run) and revisited under
  current policy, scope and retention checks.
- Native communities are provisional until fresh canonical publication. Identity
  reconciliation reuses a topic only on a unique majority overlap of surviving
  exact contributors. Overflow is explicit and never falls back to a fabricated
  anchor, vector or provider output.
- Names hydrate only after current qualification; rename text is bound to an
  eligible input, so erasure, correction or scope loss removes it everywhere.
  Source-span mentions and canonical-field origins stay separate pipelines.
- Inputs: Exact eligible immutable source/repository/claim fields, existing
  interpretation/vectors and canonical scope; schema/adapter revisions retained.
- Authority: Rust candidate validation and final authenticated mutation with
  exact current inputs, epoch, fences, deadline and lease. Models propose only.
- Blind spots: GLiNER raw development quality failed; a successful parser/fake
  fixture does not prove actual extractor recall, grouping usefulness or UI benefit.

## States and Edge Cases

- Loading: Queued/running mapping and partial coverage are visible.
- Empty: Honest unassigned/unsupported/no-eligible-input state.
- Error: Malformed/foreign/duplicate/unbounded candidate batch rejects atomically.
- Blocked: Missing compatible extractor/profile or unavailable backend is explicit.
- No-access: Current Brain/task scope and revocation apply to every hydrated name.
- Duplicate or replay: Idempotent mapping input publication and stable identities.
- Stale data: Correction, scope/model change, expiry and erasure invalidate all
  affected derivatives; stale jobs cannot restore withdrawn content.
- Reconciliation divergence: Rebuild reproduces the current permitted view and
  human overrides; native graph presence alone does not qualify a semantic node.

## Integrations and Runtime Inputs

- Providers: Existing standing Brain model gateway and optional compatible local
  candidate adapter; PostgreSQL and native GDS. Fixtures make zero paid calls.
- Environment: Existing owned test/runtime services; no global Python/MCP changes.
- Secrets: No key/input payload printed in performance receipts or checked-in data.
- Failure handling: Bounded input/output, short checked publication, leases and
  visible blocked/failed coverage; no confidence-only scope or truth acceptance.

## Acceptance

- Automated: Native Unicode/exact span/input/endpoint gates; actual persistence,
  scope/alias and override/idempotency controls; correction/erasure/restore;
  frozen extraction/grouping fixtures; populated concurrent reads and UI flows.
- Manual: Blind source-backed label review and timed cross-session navigation;
  normal-installation graph reads and independent candidate-model quality proof.
- Acceptance: The full organization contract passes, manual groups stay intact,
  all requested read outcomes are counted and no derivative content resurrects.

## Closeout

- Planned: Full Rust automatic source-backed organization and useful graph entry.
- Shipped: N/A — full product slice remains in progress. Local code now includes
  production learning/mapping, reversible alias decisions, weighted native
  communities and matched overrides, deterministic field origins, retained
  unpublished output, qualified typed navigation and grouped 3D. Native plumbing,
  normal-data clone, paging, privacy/recovery and browser proofs are recorded in
  the implementation mapping. These are uncommitted local checks; no qualified
  real producer or community recipe is selected in the normal installation.
- Not shipped: Independent real extractor quality for each actual prompt/schema,
  passing frozen grouping/vector/label qualification, current canonical
  mutation/support quality and full closeout. The separate zero-call feedback
  slice is locally delivered with all failures retained. Source-backed manual overlays and their native,
  browser, capacity and privacy proof are locally implemented.
- New blockers: None preventing independent implementation; graph/model quality
  failures remain open acceptance rather than a reason to narrow the objective.
- Docs updated: Accepted ADR/contract and owning slice/pack/index before code.
- Validation: Current migration-063 mixed 10k proof passes 602/602 reads with
  zero errors and p95 0.459626 s during capture/audit/native import/two ready
  rebuilds. Both normal-data clones pass 40/40 with identical hydrated payloads.
  Current manual-overlay, nested split bounds, source commands and populated
  erase/Brain-delete/actual older-database restore controls pass. Qualified paging
  passes 60/60; the independent renderer and feedback slices are locally delivered.
  The approved 82-request reconciliation/vector campaign passes execution/cost/
  provenance controls but fails extraction, semantic controls and fresh grouping.
  Independent blind labels also fail selected-hybrid usefulness. No recipe or
  producer is activated. Actual support/publication on retained reconciliation-3
  outputs is measured: four of six canonical controls pass; rc01 replacement and rc03
  retirement fail. The separate six-call diagnostic completed without retries
  or activation. Exact receipts, denominators and earlier failures
  remain in the evidence receipts mapping. The separately approved 105-call current4
  campaign fails real extractor/grouping quality, with canonical behavior 4/6
  and exact required-action/citation agreement 3/6. All 92 observations, 460 mode
  rows and 21 unknown pairs remain counted; no producer or recipe is activated.
  The independent main lease fix passes all six actual-main native regressions.
  Focused checks, full repository validation
  and governance pass; further harness changes need their own focused proof.
