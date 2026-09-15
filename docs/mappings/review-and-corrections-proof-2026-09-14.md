# Review, correction and replay proof

Observed: 2026-09-14
Confidence: verified

## Sources and method

Implemented the accepted [review contract](../contracts/memory-review-and-corrections.md)
on the existing PostgreSQL 17, SQLx 0.8.6, Axum and React/Mantine stack. Context7's
`/launchbadge/sqlx` lookup redirected to `/transact-rs/sqlx`; the subsequent query
returned the upstream [transaction implementation](https://github.com/transact-rs/sqlx/blob/main/sqlx-core/src/transaction.rs)
and documented commit/drop rollback behavior. Local pinned interfaces compiled.
CodeGraph status/sync and source inspection located the existing Brain lock,
transaction, authorization, idempotency and evidence helpers. No new dependency,
content hashing or strict product format gate was introduced.

## API and persistence evidence

The core integration suite passed 13 scenarios in 9.88 seconds, with optional live
OIDC excluded. Three focused review scenarios subsequently passed in 2.82 seconds:

- Actual browser acceptance recorded the authenticated reviewer and enabled strict
  accepted context with retained support. Readers, devices, foreign principals,
  invalid CSRF and forged reviewer fields failed; configured secret material was
  rejected before receipt persistence. Unchanged content retained its author/origin.
- Concurrent identical review commands returned one immutable decision. Correction
  accepted the new value and blocked old accepted receipts and historical views
  using current rules. New evidence and claim IDs did not revive the rejected value.
  Reconstructed application state read the same durable rules without projection work.
- Changed-evidence revalidation required different references and recorded an exact
  revision exception. Later submissions remained blocked. A later rejection, withdrawal
  and explicit restoration retained distinct states and effective eligibility.
- Keep selected, keep both with disjoint time conditions, retract and replacement
  all worked. An unselected third sibling kept strict eligibility blocked until its
  independent review. One stale participant left every participant and decision unchanged.
- Environment, repository, unrelated value/subject and nonoverlapping time controls
  remained admissible. Overlapping observation-day and unknown-global applicability
  remained blocked. Application-role RLS hid review tables from foreign principals;
  historical rules could not be updated through that role.
- An injected audit constraint failure returned the existing database-unavailable
  response and rolled back revisions, decisions, exceptions and the eligibility
  counter; the original request key succeeded after removing the test failure.
  Scope-group removal also advances the counter and makes absent scope ineligible;
  the final expanded scope/failure scenario passed in 0.76 seconds with Clippy/build.

An initial test expected HTTP 500 for the injected database failure; the application's
established SQLx error mapping is 503. The assertion was corrected. Its disposable
database and fixture files were removed only after verifying the database ownership
comment. A Clippy boolean simplification was applied without changing interval behavior.

## Browser and local runtime

The final combined claim/review browser run passed both workflows in 14.0 seconds,
and a restarted native worker drained their persisted queue. Review proof exercised
retained evidence inspection, actual acceptance/correction, a committed request with
its response interrupted, successful replay after the UI observed the changed revision,
keep-selected and disjoint-period conflict resolution, and a stale competing reviewer.
Review and proposal forms retain the revision opened by the editor. A background refresh
cannot silently turn an old draft into a write against a newer base. The original
eligibility text assertion needed its own rendered text element; the recorded backend
eligibility was already correct. Desktop/mobile screenshots were inspected, with no
horizontal overflow and no uncaught browser errors.

The normal local API/worker restarted with migration 009 at
`2026-09-14T12:40:57Z`. Authenticated normal-runtime proof at
`2026-09-14T12:44:04Z` returned readiness and showed the new review controls/history
for the existing SWEG test claim `5571b985-652e-44b1-ab77-857c9c0bb514` in Brain
`5c054930-d266-4c18-a42b-942729f942aa`. It remained proposed with zero review decisions.
The probe used retained publication evidence, read no customer files and made no model
calls. Synthetic review decisions ran only in disposable integration/browser databases.

Generated OpenAPI has 84 unique operations. Frontend build, rustfmt, Clippy with warnings
denied, governance lint and all 32 checker tests passed. Vite reports the existing main
bundle above its configured 700 kB warning threshold; that warning is not a failed build.
No release, commit or external deployment was performed.

## Handoff

Migration 009 supplies immutable decisions/rules, exact-revision exceptions and the
Brain eligibility counter. Initial proposal admission and current/historical canonical
views use these inputs immediately. Future raw/vector/graph/model consumers remain
responsible for enforcing and testing them. Retention and erasure is the next slice;
this proof does not assert downstream consumers or automatic learning are implemented.
