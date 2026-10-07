# Input errors, processing limits and page recovery

Observed: 2026-10-07
Confidence: verified

## Sources and Method

- Read-only PostgreSQL metadata for DLAG capture, source processing, learning and model accounting; no customer payload or credential values emitted.
- Captured input inspection emitted only marker-presence booleans and byte length.
- [Official Vite load-error guidance](https://vite.dev/guide/build.html#load-error-handling): new deployments can invalidate an open tab's lazy chunks; HTML should revalidate. Context7 discovery returned no callable tools, so this primary documentation was the dependency fallback.
- Focused static-host, gateway/semantic and pure UI model proofs; local Compose and browser acceptance recorded below.

## Observations

The screenshot's 09:13 UTC tool-result attempt recorded invalid_input and its
source was ready; private-key marker presence was confirmed without exposing
the matched text. The concurrent prompt attempt and subsequent small inputs
recorded model_budget_exhausted. Recent captures were accepted. The current day
accounted 94,844 tokens at the first inspection, against a 100,000 policy allowance; the
gateway conservatively reserves complete request bytes, overhead and output
capacity, so remaining allowance does not mean another request will fit.
These are learning/transmission limits, not failed automatic capture. Older
invalid_input outcomes remain historical; not all share an established cause.

The Explore error referenced an obsolete hashed JavaScript asset. The original
static fallback served index HTML for absent assets. The existing outer response
guard already sent no-store; the failure did not prove a missing cache policy. Semantic blocked entries were also eligible every minute
without distinguishing daily budget rejection.

## Translation and Limits

Delivered scope: main-form processing controls with on-demand help; explicit
learning/processing failure labels and explanations; model_input_sensitive
denial over the unchanged secret guard; semantic budget scheduling until the
next UTC day or policy change; missing-asset 404, revalidated entry HTML and
user-triggered page recovery preserving the URL. No automatic refresh discards
an editor, no customer text is changed, and no live paid allowance is increased.

Focused static-host and sensitive-input no-call proofs passed, as did all
20 semantic regression tests, three pure web tests, type/design/build and
Clippy with warnings denied. Normal Compose deployment is ready. The exact
obsolete chunk URL returns 404; entry HTML remains no-store. Browser failure
injection on an owned tab showed the styled panel and Reload restored the
URL's selected Repositories view. Network overrides were then removed.

The default browser screenshot confirms all four limits in the main form and
a full Edit label; opening the info icon exposes the hidden help text. Editing
output tokens to 3,072 then cancelling restores 4,096. The later browser read
showed an externally changed saved allowance of 500,000; this task never saved
a policy change. The real pipeline then showed running/queued work and
completed inputs, not a claim that every historical failure recovered. The old open
client must refresh once to receive the new error component. This does not
assert that historical failed learning has completed or that protected input
is now suitable for transmission.

## Follow-up

A later read-only metadata check found 40 new succeeded learning runs since
09:33 UTC, plus queued/running work. Remaining outcomes included provider_shape,
model_budget_exhausted, model_input_too_large and one new model_input_sensitive.
These remain actual failures; typed reasons and successful other runs do not
turn them into completed learning. The attempted browser viewport override
did not change the observed 1,924-pixel DOM viewport and was reset; no new
narrow-viewport acceptance is claimed from that attempt.

No cost increase or customer evidence cleanup is included. Legitimate budget
blocks remain subject to the saved policy and the normal UTC reset.
