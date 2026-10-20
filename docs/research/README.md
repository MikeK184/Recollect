# Current research and evaluation inputs

Accepted ADRs/contracts and [active packs](../roadmap/execution/active/README.md)
govern implementation. The [current roadmap](../roadmap/epics/index.md) is the
single status and priority entry point.

- [Frozen attributed utility experiment](attributed-utility-offline-fixtures-2026-10-18.json):
  12 development/eight held-out ranking queries, explicit attribution, repeated
  votes, correction/erasure, foreign-scope and mistaken-feedback controls. This
  tests bounded scorer behavior; it cannot establish real helpfulness quality.

- [Independent graph organization adversarial fixtures](graph-organization-adversarial-fixtures-2026-10-18.json):
  frozen 12-development/eight-held-out Unicode, long-input, alias/relation,
  negation/history, scope and overlapping-topic cases; no extractor score claimed.
- [Fresh grouping holdout](graph-organization-fresh-heldout-v1-2026-10-18.json):
  twelve separate untouched documents frozen before development parameter
  selection. Keep these out of fitting; real extraction and vector output remain
  unmeasured. Endpoint gold is unannotated, rather than implicitly empty.
- [Frozen blind label review rubric](graph-organization-blind-label-rubric-v1-2026-10-18.json):
  independent source fidelity, scope/time/authority and navigation-usefulness
  ratings, blinded arm/partition presentation and complete failure denominators.
  Authored without corpus content or model outputs; bind its hash before the
  remaining real campaign. It never substitutes for producer or grouping gates.
- [Canonical reconciliation controls](automatic-reconciliation-native-controls-v3-2026-10-18.json):
  six explicit update, historical/negative, duplicate, unrelated and protected
  human-memory cases. V3 records actual source attribution and actor metadata;
  prior frozen versions remain linked through exact supersession hashes.

- [Automatic organization study](automatic-knowledge-mapping-plan-2026-10-09.md):
  pinned upstream evaluations (Cognee, Graphify, Graphiti, GLiNER2.5), stage
  rationale and acceptance-test design behind ADR 0020 and the organization
  contract, which govern; kept because they do not repeat that evidence. Carries
  no delivery status.
- [Frozen organization fixtures](automatic-knowledge-mapping-fixtures-2026-10-09.json):
  development/held-out extraction annotations; do not change them to improve a score.
- [Frozen memory-product holdout](memory-product-holdout-manifest-2026-10-08.json):
  preserved inputs for comparable future evaluation; not evidence of a completed run.
- [Automatic support model results](automatic-memory-support-results-2026-10-07.json):
  versioned synthetic observations supporting the
  [automatic-memory proof](../mappings/memory-source-support-staging-2026-10-07.md).

Completed UI proposals, the delivered automatic-memory design and the earlier
claude-mem comparison were removed as duplicate current guidance. On 2026-10-20
the remaining 2026-10-08 improvement proposal moved into the
[memory quality and proof epic](../roadmap/epics/memory-quality-and-proof.md) as planned
slices; its full text is at [commit a2338a8](https://github.com/MikeK184/Recollect/blob/a2338a80c824f00214ee33e37596e5a92a2cc409/docs/research/memory-product-improvement-plan-2026-10-08.md).
Its evaluation rules (frozen holdout, no held-out tuning, separate arms) continue
through the [frozen memory-product holdout](memory-product-holdout-manifest-2026-10-08.json)
and the accepted [LongMemEval protocol](../contracts/operations-longmemeval-protocol.md). Their chosen
behavior is in contracts, runbooks and delivery packs; original studies remain
in the [fixed Git snapshot](https://github.com/MikeK184/Recollect/tree/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs/research).

## Research tooling

[scripts/](scripts/) holds evaluation, benchmark and probe tooling cited only by
mappings, runbooks and research notes (LongMemEval fetch/judge/summarize,
OpenRouter and OpenAI probes, integrated and retrieval-fusion evaluation, the
organization and feedback scorers). Run each from the repository root; scripts
that need operator modules put `scripts/` on `sys.path` themselves. Scorers and
corpus tools that Rust examples or tests hash and invoke by path stay under
`scripts/`.
