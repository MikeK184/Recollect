# Author and inspect claims over time

Status: active

## Prerequisites

Start [local development](local-development.md) with `./scripts/dev.sh` and open
`http://127.0.0.1:8787`. Sign in using the local credentials in ignored `.env`.
A writer/admin can author proposals in an open Brain. Import a source or publish
repository facts first; evidence support is required. No model credential or call
is needed for this manual workflow.

Normal model learning and maintenance use the [autonomous policy](provider-learning.md).
The forms below are optional authoring and inspection controls; they do not imply
that someone must review every automatically learned memory.

## Procedure

1. Open a Brain and its **Claims and decisions** panel. Choose **New claim**.
2. Enter a subject, property/relationship, value and optional rationale. Choose
   Claim or Decision / intent. Select repositories, areas and environment as
   applicable. Empty selections apply across the Brain; they do not create access.
3. Add exact document versions, repository facts or manifest revisions with
   **Use evidence**. Repository support adds its repository to the visible selection.
   Optional document line spans are one-based and inclusive. **Inspect** shows the
   retained evidence or its explicit availability limitation. No excerpt is copied.
4. Optionally select an environment's exact manifest revision. Its snapshots must
   agree with the selected repository facts. An unrelated repository or environment
   advancing does not change these claims' freshness.
5. Choose unknown fact validity, a point with precision, or an interval with an
   excluded end. Inputs labeled UTC use UTC; rendered dates use the browser's local
   time. Unknown endpoints stay unknown. Record operational assessments separately;
   deployed/verified requires an observation time and description of the outcome.
6. Save the proposal. Its server-assigned knowledge time is independent of fact
   time. The detail shows review, recorded/effective freshness, assessment, exact
   support, authorship and why strict eligibility is allowed or denied.
7. **Revise proposal** appends knowledge without overwriting history. Another editor
   saving first produces a conflict: cancel, reopen the current proposal and apply
   the intended change. A lost response can be retried with unchanged input.
8. Select **Knowledge revision** to inspect earlier knowledge. Set separate fact
   and knowledge time filters in the panel for historical questions. An absent
   knowledge revision produces no result; a known fact interval excludes its end.

Use distinct claim identities for assertions that legitimately coexist in different
periods/environments. Revising a proposal replaces what is currently asserted by
that identity; it does not implicitly preserve a second current assertion for its
former fact interval. Prior versions remain explicit knowledge history.

## Expected states and recovery

Manual proposals are not human acceptance. Investigation includes useful proposals
with qualifications; strict accepted and strict operational remain empty until an
authorized acceptance path records the required authority. Assessment `verified`
alone cannot satisfy that requirement. [Review and corrections](review-and-corrections.md)
provides the delivered acceptance, rejection, conflict and withdrawal workflow.

A later supporting document version or relevant manifest selection makes effective
freshness **needs verification**. The recorded assessment and original support remain
visible. Reference-only evidence does not prove remote availability. Missing,
unreadable or invalid retained text is qualified immediately and excluded from
strict use; restore the owned artifact or select appropriate retained evidence.

Pages examine at most 20 scoped candidates. Temporal/eligibility filtering can leave
a shorter or empty page; **Next page** remains available when more candidates exist.
Readers can inspect permitted history but cannot edit. Archived Brains reject writes.
Current access applies to historical reads and idempotent retries as well.

## Verification and boundaries

`./scripts/test-platform.sh` includes the temporal/evidence and environment-manifest
scenarios. `./scripts/test-ui.sh tests/claims.spec.ts` exercises actual browser
authoring, retained support, interrupted save, stale editors, history, time filters
and mobile layout. [The proof mapping](../mappings/claims-and-time-proof-2026-09-14.md)
records executed checks and local runtime evidence.

The [claims contract](../contracts/memory-claims-and-time.md) owns endpoints and
limits. General retrieval, Erase, retention, model learning
and procedure workflows retain their successor-slice boundaries. The selected
`gpt-5.6-luna` and `text-embedding-3-large` pair remains recorded in the
[provider preflight](provider-preflight.md); this workflow sends it no evidence.
