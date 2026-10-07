# Automatic-memory support evaluation baseline

Status: shipped
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product

## Summary

- Goal: Reproducible synthetic evidence/support cases and scoring that distinguish semantic quality from fixture enforcement before support cutover.
- Non-goals: Production publication/eligibility changes, customer-data evaluation, model swaps, deployment and a superiority claim.
- Delivery shape: Checked-in corpus/scorer, native opt-in handler/model harness, retained versioned observations and separately audited gateway receipts.

## Governing Sources

- [Evaluation amendment](../../../contracts/operations-integrated-evaluations.md#automatic-memory-support-baseline-2026-10-07)
- [Autonomous operation](../../../adr/0006-autonomous-memory.md)
- [Claims/time](../../../contracts/memory-claims-and-time.md)
- [Provider policy](../../../contracts/memory-provider-policy-and-learning.md)
- [Memory owner](../../epics/memory-lifecycle.md)

## Scope

- In scope: Versioned synthetic calibration/held-out cases; exact quotation/bounds and attribution; positive/critical-negative coverage; missing/error/uncertain reporting; fixed cutover thresholds; request/token/latency metrics; fixture-versus-model evidence distinction.
- Out of scope: Runtime memory-schema or eligibility changes, public benchmark activation, customer content, network calls during offline validation.
- Blockers: None; actual native model baseline and receipt review are complete.

## Surface and Interface Changes

- Interfaces: `python3 scripts/memory_support_baseline.py check`; `score RESULT_JSON --partition calibration|held_out`; JSON format version 1 with exact case IDs and declared fixture/real-model mode.
- Storage: Repository synthetic fixtures/observations and private receipt artifacts. The opt-in harness creates and removes owned disposable canonical test databases; it adds no product schema or provider-response cache.
- Ownership: Memory lifecycle owns corpus semantics; the existing model gateway alone dispatches synthetic provider runs.

## Data and Authority

- Inputs: Synthetic source text, role, applicability, proposed assertion/action, exact quote/range, expected verdict and usability; observations contain completed/failed/uncertain state, verdict, usability, request/tokens/latency.
- Authority: Versioned gold labels remain outside provider inputs. Server permissions/state establish canonical usability, never a supplied result file alone.
- Blind spots: Offline scoring validates declared observations and cannot prove actual provider execution, semantic correctness beyond this corpus, or production readiness.

## States and Edge Cases

- Loading: Read bounded local UTF-8 JSON; no credential or network loading.
- Empty: Reject an empty corpus or result set; missing expected cases are reported incomplete and cannot pass quality.
- Error: Reject unknown fields/IDs, invalid dispositions, malformed bounds/quotes, duplicate observations and invalid numeric metrics.
- Blocked: Missing/failed/uncertain assessments remain explicit and cannot pass the semantic-quality gate.
- No-access: N/A for offline synthetic data; native runtime evidence uses the owned proof Brain and current provider permits.
- Duplicate or replay: One observation per case; reject duplicates rather than selecting the favorable attempt. Replacement costs remain individually accounted by the runtime harness.
- Stale data: Version mismatch rejects scoring; calibration and held-out identities remain separate and frozen.
- Reconciliation divergence: Wrong retirement/replacement, historical and scope-mismatched examples are critical negatives; supported correction is a positive control.

## Integrations and Runtime Inputs

- Providers: None during offline checks. The explicit opt-in native comparison uses the approved installed model through the canonical gateway.
- Environment: Offline operation reads no secrets; opt-in native evaluation uses existing approved runtime/provider environment without printing its values.
- Secrets: Fixtures contain synthetic engineering examples only; output metrics contain no credentials or customer evidence.
- Failure handling: No automatic provider retry in this script. Result artifacts record missing/failed/uncertain cases rather than dropping them from denominators.

## Tests and Acceptance

- Automated: Corpus schema/bounds/quotes/category coverage; oracle-shaped fixtures cannot pass quality; critical false publication fails; all-positive withholding fails usefulness; missing/uncertain cases fail completeness; duplicate/unknown records and invalid accounting reject. Run focused offline tests and `./scripts/validate.sh`.
- Manual: Independently audit native synthetic provider/gateway receipts against frozen observations, model/prompt/schema versions and canonical delivery. Completed review is recorded in closeout.
- Acceptance: Offline checks pass. The completed versioned held-out comparison has all critical negatives withheld, at least 80% positive usability, completed assessments for every case and honest actual usage/latency evidence. No fixture-only semantic claim.

## Closeout

- Planned: Deliver the authorized slice and its governed acceptance boundaries.
- Shipped: Versioned 50-case corpus, strict offline scoring, native actual-model harness and separately retained v1/v2/v3 observations/receipts. Final v3 held-out: 26 complete,15/15 negatives withheld,11/11 positives usable; calibration:24 complete,13/13 negatives withheld,10/11 positives usable. Two-agent review verifies gateway identity/accounting and frozen labels outside model inputs.
- Not shipped: General semantic accuracy, production truth, model superiority and separately governed public benchmarks are not established. No release, commit, push or deployment is included.
- New blockers: None for this local slice.
- Docs updated: Governing contracts, operating runbooks, research/evaluation evidence, owner epics and active/archive indexes.
- Validation: All 10 offline scorer tests and 50-case validation passed. Real installed-model native evaluation used 124 distinct calls/148,288 tokens across retained versions; no favorable mixing. v3 meets the unchanged zero-critical-false-usability/80%-positive/completeness gate. Final governance validation passes 32 checks. [Dated evidence and limitations](../../../mappings/memory-source-support-staging-2026-10-07.md).
- Version: N/A; no release requested.
- Commit: uncommitted; local working-tree implementation.
