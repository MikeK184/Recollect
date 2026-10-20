# Claim Truth and Freshness

Status: planned

## Purpose

Keep memory as true as possible for the revision and environment in use now:
carry claims forward exactly when their support is unchanged, age what nothing
re-observes, recheck citations at answer time, report what changed in what is
known, audit claims continuously with a measured error rate, and rank what is
true here above what is merely similar. Part 2 of the five-part improvement sequence in the [roadmap](index.md#improvement-sequence).

## Governing Sources

- [Claims and time](../../contracts/memory-claims-and-time.md) and
  [source support verification](../../contracts/memory-source-support-verification.md)
- [Repository publication](../../contracts/evidence-repository-publication.md) and
  [workspace scope](../../contracts/evidence-workspace-scope.md)
- [Autonomous maintenance](../../contracts/memory-autonomous-maintenance.md) and
  [provider policy and learning](../../contracts/memory-provider-policy-and-learning.md)
- [Graph fusion retrieval](../../contracts/retrieval-graph-fusion.md) and
  [answers](../../contracts/retrieval-answers.md)

## Dependencies and Boundaries

Starts after the engineering-task evaluation in
[memory quality and proof](memory-quality-and-proof.md) exists, so each slice is
measured against a baseline. Needs no agent identity, so it is independent of
[agent identity and authority](agent-identity-and-authority.md); the two can swap
order.

**Claim truth model (proposal, 2026-10-20).** A claim states what was true for
an exact repository revision or environment observation at a recorded time; that
anchored statement never becomes false by age. What changes is whether it applies
to the revision or environment in use now, which Recollect computes. Git supplies
exact change detection: blob object IDs and line-span hashes show whether support
is unchanged at a newer commit, and commit ancestry, not timestamps, decides which
of two claims on one line of history is newer. Deploy, configuration and manifest
observations invalidate environment claims early; time-based aging applies only
to what neither can see. Doubt is resolved by the cheapest check that can decide,
in order: deterministic comparison, an approved read-only probe in the claim's
environment, a small-model match of claim to evidence, a question to a connected
agent, then the owner. A model never accepts or rejects a claim, as the
[claims contract](../../contracts/memory-claims-and-time.md) already requires.

**Order.** Revision carry-forward, then the freshness policy, then answer-time
verification and the knowledge changelog, which both reuse them. Compact delivery
and the background audit follow. Trust-aware ranking starts only if the
evaluation shows retrieval misses rather than answer misses, and the decision-model
provider only after that. The provider is a new external integration and needs a
pack; a hosted provider additionally needs explicit owner consent because memory
content leaves the installation. Automatic semantic organization stays deferred
in [graph intelligence](graph-intelligence.md).

**Open decisions for the owner.**
- How long may each claim kind go unverified (for example configuration values versus versions), or should environment claims age only on events?
- Which model and monthly budget may the background auditor use?
- May a hosted decision model (for example TypeSafe's Jev, a closed hosted API) receive memory content for ranking or claim matching?

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `claim-revision-carry-forward` | planned | needs-contract | inline: when a published snapshot or the owner's working revision changes, a repository-backed claim is re-evaluated without a model call: unchanged supporting blob IDs or line-span hashes carry its support forward to the new commit, changed support marks it `needs_verification` for that line of history only, commit ancestry decides newer versus older, and claims on diverged branches or environments stay independently valid; validation: Git fixtures covering unchanged, edited, moved, deleted, rebased and diverged support, plus committed negative retrieval assertions that a claim is not returned as current on a revision or environment where its support differs | Makes "is this still true at the revision I am on" exact and free; extends today's pinned-snapshot rule rather than replacing it |
| `claim-freshness-policy` | planned | needs-contract | inline: each claim kind has a declared aging rule (revision-bound facts never age, only their applicability; environment observations carry a verification interval and are invalidated early by deploy, configuration or manifest observations; decisions never age but are flagged when observed state contradicts them), and aging only changes freshness to `needs_verification` or a new contradicted state, never rejecting or deleting; validation: deterministic clock and event fixtures per kind | Facts that are not re-observed lose current status instead of silently staying true; a deterministic policy, not the deferred learned decay |
| `answer-time-verification` | planned | needs-contract | inline: before Ask or recall returns a cited claim, its applicability to the requested revision and environment is rechecked with the deterministic carry-forward comparison, and each citation is labelled current, needs verification or stale at answer time; validation: fixtures where support changed between learning and answering | Answers carry their own freshness, at near-zero cost |
| `memory-knowledge-changelog` | planned | needs-contract | inline: for two revisions, deploys or dates in one Brain and environment, Recollect lists which claims became current, stale, contradicted, superseded or rejected, each with its evidence; validation: fixtures across a commit range and a deploy pair with exact expected change sets | "What changed in what we know since the last release" for engineers and for handovers |
| `claim-compact-agent-delivery` | planned | needs-contract | inline: recall delivers claims to agents as compact structured records (kind, subject, predicate, value, scope, environment, revision, observed time, freshness, support state, evidence IDs) with exact evidence fetched on demand, instead of prose; validation: the engineering-task evaluation compares answer quality and context tokens against current delivery | Claims are already structured; this makes what agents receive structured too |
| `claim-background-audit` | planned | needs-contract | inline: a small-model auditor continuously samples claims, randomly and weighted toward frequently recalled, old, production and never re-checked ones, within a configured budget; it checks deterministically first, uses the model only to match a claim to evidence, and records each verdict as a new attributed assessment that can lower freshness or flag contradiction but never accepts, rejects or deletes; validation: a labelled audit set measuring auditor precision and recall, budget exhaustion and provider-policy denial; sampling also weights check-worthiness (production, security, configuration values and decisions), as in the CLEF CheckThat! fact-checking labs | The always-on fact checker, with its own error rate measured rather than assumed |
| `retrieval-trust-aware-ranking` | planned | needs-contract | inline: after the engineering-task evaluation shows retrieval misses, candidates are first tiered by support state, freshness and revision/environment applicability and only then ordered by fused relevance, and any reranker runs on the shortlist with gold-evidence retention measured and the current RRF order as fallback; validation: equal-budget comparison against the fixed production ranking on the frozen evaluation, reporting recall@k, gold retention and latency | Ranks what is true here above what is merely similar; reuses the deferred advanced-reranking boundary |
| `retrieval-decision-model-provider` | planned | needs-adr, needs-contract | inline: an optional typed-decision provider (a local model by default; a hosted one such as Jev only with explicit owner consent under provider policy, because memory content leaves the installation) can rank shortlists, match claims to evidence and score check-worthiness, with its probabilities used only as inputs to declared state transitions and never stored as trust; validation: provider-policy denial, outage fallback to RRF and deterministic checks, and accuracy on the frozen evaluation rather than vendor benchmarks | Cheap fast decisions for ranking and auditing without making a closed service mandatory; new external integration, pack required |
