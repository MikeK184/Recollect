# Attributed utility feedback and bounded ranking experiments

Status: shipped
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Deliver explicit scoped utility feedback and the bounded offline
  comparison in the user's complete October 18 graph request.
- Non-goals: Online ranking activation, model training or paid answer judging.
- Delivery shape: Local Rust/SQL commands, deterministic scorer, frozen fixtures
  and native privacy proof. Recorded graph reliability and organization baselines
  precede the offline comparison; production activation retains its separate gates.

## Governing Sources

[Attributed feedback](../../../contracts/retrieval-attributed-feedback.md),
[graph fusion](../../../contracts/retrieval-graph-fusion.md),
[organization](../../../contracts/automatic-knowledge-organization.md),
[privacy](../../../contracts/memory-retention-and-erasure.md).

## Scope

- In scope: Explicit source/claim utility commands, current qualification, vote
  revision/idempotency, scoped offline scoring and frozen comparison receipts.
- Out of scope: Implicit browsing rewards, new candidates, truth promotion,
  online model training, production ranking activation and paid judging.
- Blockers: None for independent implementation. Run the unchanged frozen
  comparison after the passing reliability proof and terminal real organization
  baseline are recorded, retaining a failed organization result as a failure.
  A production-qualified organization recipe is not a prerequisite for this
  zero-call arithmetic diagnostic. It remains a separate prerequisite to any
  later production activation under the accepted contract.

## Surface and Interface Changes

- Interfaces: A delivery command names an exact canonical source-version/claim-revision ID,
owned retrieval operation, normalized GraphSelection and query-context ID. It
returns the freshly qualified node and a server-issued context ID. A vote names
that context, an optional previous change ID and helpful/unhelpful/reused value.
The context binds the operation's current task scope and exact actor/device;
changing that task scope removes its dependent contexts. Repeated delivery of the
same actor/task/query/input returns the same context. Contexts are capped at
10,000 per actor per Brain and preserve the source's earliest
retention deadline. Idempotency receipts contain attribution, never node bodies.
Persist no query/evidence body. Browser and paired-device commands share existing
authorization and canonical qualification; current writer authority is required.
An offline comparison reads the same qualified candidate IDs/scores for baseline
and experiment, excluding current-task/held-out feedback when fitting rewards.
- Storage: Migration 056 adds content-free attributed contexts and immutable
  vote changes with source/claim/task dependencies and old-manifest replay closure.
- Ownership: Canonical retrieval owns qualification and utility commands; the
  offline script owns only bounded comparison arithmetic.

## Data and Authority

- Inputs: Exact canonical source/claim IDs, owned task/operation, normalized
  scope and explicit helpful/unhelpful/reused votes; no query/evidence body.
- Authority: PostgreSQL attribution and canonical supports/scope/retention. Final
  publication rechecks role/device, exact input epoch and wall-clock deadlines.
- Blind spots: Frozen scorer fixtures do not establish real helpfulness, answer
  quality or live canonical eligibility. Production ranking remains fixed.

## States and Edge Cases

- Loading: Bounded qualified delivery precedes an explicit vote.
- Empty: No vote is the initial state; cold starts preserve baseline order.
- Error: Missing/corrected/erased/expired inputs reject the whole command.
- Blocked: Storage capacity or bounded preparation/publication timeout is explicit.
- No-access: Foreign actor/device/task/scope or non-writer authority rejects.
- Duplicate or replay: Idempotent commands return the current head; repeated
  votes add no reward and stale revisions conflict.
- Stale data: Scope/head changes and exact privacy closure remove contexts;
  old receipts cannot reintroduce votes, including after backup replay.
- Reconciliation divergence: Ineligible or unsupported signals are withheld;
  the experiment cannot create candidates or change truth/authority.

## Integrations and Runtime Inputs

- Providers: Existing PostgreSQL/retrieval/erasure journal, plus an offline Python
  standard-library scorer; no model provider or new runtime service.
- Environment: Existing owned test database variables; no new credentials.
- Secrets: No query/evidence body or credential value enters feedback receipts.
- Failure handling: Fifteen-second operation, 1.5-second publication and 750 ms
  statement bounds; no successful partial vote or automatic ranking activation.

## Tests and Acceptance

- Automated: Actual command/role/scope/version/idempotency and populated
  erasure/restore proof; repeated vote, foreign scope, cold-start and failed-query
  scorer controls; focused checks and `./scripts/validate.sh`.
- Manual: Inspect frozen comparison receipts after the graph baseline, including
  every failed query and deliberately mistaken explicit-feedback promotion.
- Acceptance: Exact current attribution/privacy, bounded five-percent feature,
  complete outcomes and original eligibility/truth gates; no production activation.

## Closeout

- Planned: Complete scoped command, lifecycle and bounded offline experiment.
- Shipped: Local explicit delivery/vote commands, immutable attributed revisions,
  current canonical qualification, exact privacy/recovery closure, and the
  unchanged frozen 20-query offline comparison. The completed real organization
  baseline is a failed quality result; it is not accepted by this closeout.
- Not shipped: Online ranking, policy training and paid answer/citation judging
  remain explicit deferrals. This slice does not qualify the organization model.
- New blockers: None within this slice. Production activation still requires a
  separate accepted decision and completed graph/retrieval quality proof.
- Docs updated: Accepted contract, implementation ledger, owning epic and
  active/archive/epic indexes.
- Validation: Three current migration-063 PostgreSQL command/lifecycle cases pass
  in 4.00 s with provider keys unset; earlier populated ancestry/old-database
  replay and pre-feedback manifest recomputation remain recorded. Five scorer
  unit cases pass. Fixture SHA
  `c6ac16e614b9a7432da180524662026f8cb0c302a23b7b4518d8ab08edf88bcf`
  and scorer SHA
  `f6542a78901c9a74de58e840c34b67575bcafe8384ad0e8a58fefa03f724e260`
  were frozen before the first comparison. All 20 outcomes remain: 18 completed
  and two unavailable, each unavailable result scoring zero. Eighteen independent
  reward votes are retained; two evaluation-task votes and four other ineligible
  votes are rejected. Two candidates are withheld. Two mistaken explicit votes
  cause bad promotions, one in each partition. At the same cutoff, development
  NDCG/MRR/recall move from 0.166667 to 0.75 (12 queries), held-out from 0.25 to
  0.625 (eight queries), and all-query from 0.20 to 0.70. These toy recorded-
  eligibility scores are not real helpfulness, answer/citation or memory-benchmark
  improvement. Scorer-only p95 is 0.020167 ms, not retrieval/backend latency.
  The feature stays capped at five percent, production RRF stays k=60, and model
  calls are zero. Focused and full repository validation pass; see the current
  implementation ledger for exact receipts.
