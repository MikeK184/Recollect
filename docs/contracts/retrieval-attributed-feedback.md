# Attributed utility feedback and bounded ranking experiments

Status: accepted

## Source

The user's October 18 instruction to implement the complete graph research plan,
including its bounded feedback-ranking experiment after graph reliability.
[Canonical retrieval](retrieval-graph-fusion.md),
[automatic organization](automatic-knowledge-organization.md),
[source qualification](memory-source-support-verification.md) and
[privacy/recovery](memory-retention-and-erasure.md) remain governing.

## Contract

Record explicit helpful, unhelpful or reused feedback only through an
authenticated writer command. Bind each signal to the actor/device, task or
operation, normalized requested scope, query-context identifier and exact source
version or claim revision shown to that actor. Requalify that exact input through
canonical current retrieval gates before recording it. A source-described
semantic ID cannot masquerade as an accepted claim. Do not store query text,
source quotations, credentials or a duplicate evidence body in feedback records.

Deduplicate one actor/task/query-context/input vote; an explicit revision changes
that vote instead of adding another reward. Repeated browsing, model confidence,
clicks, generated content and the model's own success assertion are not independent
helpfulness evidence. Preserve attribution and mutation audit. Changing scope,
correction, expiry, erasure, model-input fences and old-backup replay invalidate
or physically remove exact dependent signals, including retained-source ancestry.
Independent support is retained. Rebuilds never recreate withdrawn feedback.

The first implementation is a bounded offline experiment. Keep the production
RRF baseline and its fixed `k=60`. An experimental scorer may add an attributable
utility feature only after the same complete eligibility gate, capped at five
percent of the eligible candidate's baseline score. It cannot create a candidate,
alter support/permission/freshness, accept a claim, equate identities, merge
topics, or make hidden evidence contribute to a displayed score. No online model
training, automatic policy activation or provider call is selected by this
experiment.

Compare the baseline and experimental ordering on frozen task/source relevance
fixtures with development and untouched held-out partitions. Exclude the current
task and held-out outcomes from reward fitting; keep source/version identity and
scope boundaries intact. Report all query outcomes, relevance at the same cutoff,
bad promotions, missing/withheld evidence, latency and sample size. Include
adversarial repeated votes, positive feedback on an erased/corrected input,
foreign scopes and cold-start controls. A metric improvement is not answer or
citation quality proof. Paid answer judging remains a separate decision.

## Acceptance

- Actual PostgreSQL command/idempotency/version/role/device and current canonical
  qualification proof, plus exact privacy/old-restore derivative closure.
- No reward from repeated self-feedback, unavailable support or foreign scopes;
  no feedback feature crosses the original eligibility/truth boundary.
- Frozen comparable baseline/experiment receipts, including failures and zero
  provider calls. Production ranking stays fixed until a separately accepted
  activation decision backed by the completed graph and retrieval proof.

## Explicit Deferrals

Online learned ranking activation, policy training, implicit click rewards and
paid answer/citation judging are not selected. Explicit attributed commands,
privacy closure and the bounded offline comparison remain required by this slice.
