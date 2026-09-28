# Review claims and correct rejected values

For the current route/menu map, see the [desktop guide](desktop-experience.md).

Status: active

These are optional human interventions. Normal learning, evidence-based revision,
retirement and expiry use [autonomous maintenance](provider-learning.md), without
individual acceptance calls. Human decisions remain durable overrides.

## Prerequisites

Start `./scripts/dev.sh`, open `http://127.0.0.1:8787`, and sign in using the
credentials in ignored `.env`. Open a Brain containing a supported claim.
Writers/admins can review in an open Brain; readers can inspect decisions.
Paired device tokens cannot perform human review. No model call is required.

## Procedure

1. Open **Memory**, select a claim, then **Review and corrections**.
   Inspect its original evidence, applicability, assessment and existing rules.
2. Choose **Accept**, **Reject value**, **Withdraw**, **Correct value**,
   **Explicitly revalidate**, or **Restore withdrawn claim** as appropriate.
3. For correction, use **Edit reviewed content** to prepare the changed value,
   complete support and applicability. **Use this content** prepares the draft;
   the decision is recorded only after a reason and **Confirm review**.
4. Revalidation preserves the assertion value and requires an explicit basis:
   different support, changed applicability, or correction of an earlier review.
   It exempts only the resulting revision from matching rules. New submissions
   do not inherit that exception.
5. For conflicts, inspect each competing claim and explicitly include participants.
   Choose one value, keep both under disjoint conditions, withdraw all selected,
   or prepare a different replacement. Keeping both requires nonoverlapping
   repositories, environments or fact-time conditions. Unselected contradictions
   remain unresolved. Provide the reason above and **Confirm conflict resolution**.
6. Inspect **Review recorded** and its eligibility qualifications. Decision history
   records the actual reviewer, action, reason, basis and before/after revision IDs.

Acceptance is separate from operational verification, freshness and evidence
availability. A reference-only or stale assertion may have a review decision while
remaining excluded from strict accepted context. Unresolved contradictions also
block strict context. Selecting a newer or more frequent assertion does not resolve it.

## Recovery and scope

Retry an interrupted command with unchanged input: the same saved decision returns
with eligibility recalculated against current rules. A competing edit returns a stale
revision error without partial changes. Return to the claim and inspect its latest
revision before starting a different decision. Historical revisions are read-only.

Rejecting/correcting a value records a rule scoped to normalized subject/property/value,
repository/environment and fact time. A new evidence ID or unrelated commit is not an
escape. Unknown time bounds and empty scope selections overlap conservatively. Areas
are overlapping organizational associations, not mutually exclusive applicability.
Matching is textual; semantic paraphrases remain review work.

Withdraw preserves history and records a re-entry rule without declaring the value
false. Restore reactivates a withdrawn claim with explicit withdrawal-rule exceptions;
rejection still requires revalidation. **Include historical states** makes blocked,
rejected and withdrawn claims inspectable, including content-free erased history
markers. [Erase](retention-and-erasure.md) separately removes controlled content
and dependent copies; it does not mean Withdraw.

## Verification and boundaries

`./scripts/test-platform.sh` includes review authority, durable rules/replay, all four
conflict dispositions, independent applicability and transaction-failure proof.
`./scripts/test-ui.sh tests/review.spec.ts tests/claims.spec.ts` exercises the real
browser/server workflow and restarted worker. See the
[dated proof](../mappings/review-and-corrections-proof-2026-09-14.md).

The [review contract](../contracts/memory-review-and-corrections.md) governs the three
new API operations and canonical eligibility. General retrieval, graph rebuilds,
automatic learning must consume these rules in their own slices. Delivered erasure
clears affected review/rule payloads while preserving permitted minimal attribution.
The selected Luna and embedding-3-large provider pair is not used by human review.
