# Human review, corrections and durable rejection

Status: accepted

## Source

[ADR 0005](../adr/0005-canonical-claims-and-time.md),
[claims and time](memory-claims-and-time.md),
[the correction loop](../foundation/vision.md#correction-and-learning-loop) and
[mutation principles](../foundation/engineering-principles.md#route-every-mutation-through-one-policy).
The user authorized routine decisions and implementation across all roadmap slices.

## Contract

### Review authority and mutation

Review requires an authenticated browser session with current writer/admin access
to an open Brain and CSRF. A paired device cannot claim human reviewer authority;
no input accepts reviewer ID, review timestamp, accepted state or policy identity.
The server records the actual authenticated account and browser review action. This
authenticates the action, not physical human presence. Authors may review their own
proposals; no separation-of-duties requirement is selected for this local/team scope.

Every command supplies a current base revision and a nonblank reason up to 2,000
characters. UUID conditional edits and optional idempotency keys follow the existing
command boundary. Validate all participants, content, support and scope before any
write. Revision append, decision, applicable rule/exception, mutation audit, eligibility
invalidation and durable refresh work commit atomically. Invalid/stale input leaves
all participants unchanged. Current access is rechecked on replay.

An immutable review decision records actor, action/disposition, reason, prior/result
revision IDs and optional revalidation basis. It is canonical reconstruction input,
not a disposable projection. Unchanged content preserves its original author and
`origin`; accepting device/model input does not relabel it as human-authored.
Explicitly edited replacement content records the browser author's identity, while
the decision's prior revision retains the original input and its provenance.
Review commands append knowledge revisions and record their own `reviewer_id` and
decision ID. Claim lifecycle adds `active|withdrawn`, separate from review/freshness/
operational states; old revisions without a lifecycle decode as active.

### Single-claim actions

`POST /api/brains/{brain}/claims/{claim}/review` accepts action, base revision,
reason, optional full corrected content and optional revalidation basis:

| Action | Result |
| --- | --- |
| `accept` | Proposed active claim becomes accepted with the actual reviewer. Applicable rejection/withdrawal rules or unresolved overlapping contradictions require explicit resolution/revalidation first. |
| `reject` | Appends rejected review and a durable rejected-value rule for that assertion/applicability. |
| `withdraw` | Preserves review but marks lifecycle withdrawn; records a withdrawal rule preventing ordinary re-entry under the same assertion/applicability. It makes no assertion that the value was false. |
| `correct` | Full validated content must keep normalized subject/predicate and change normalized value. Rejects the old value with a durable rule and appends the reviewed accepted replacement on the same claim identity. Remaining independent contradictions stay qualified and block strict use. |
| `revalidate` | Explicitly accepts the selected assertion using validated optional replacement content with the same normalized subject/predicate/value. Records basis `changed_evidence`, `changed_applicability` or `review_correction`, a reason, and exceptions to the matching rules for this exact resulting revision. |
| `restore` | Reactivates a withdrawn claim with an explicit reason and exceptions to matching withdrawal rules. Rejection rules still require revalidation. |

Revalidation bases describe the reviewer's decision. A new source ID or commit does
not prove material change automatically. `changed_evidence` requires different support
references; `changed_applicability` requires changed scope, exact manifest or fact validity;
`review_correction` explicitly permits reconsidering a mistaken earlier decision.
Neither a new record ID nor a merely newer timestamp implicitly supplies revalidation.
The exception is bound to the resulting revision, not a global rule deletion. A new
mutation/rejection can block it again. Ordinary edits of reviewed claims remain denied.
An already-completed action without its original idempotency key may return 409;
the original identical key replays the original result.

Acceptance records judgment independently of freshness and operational proof.
Reference-only or stale evidence can be reviewed while its strict eligibility remains
qualified. An accepted label cannot make unavailable evidence or a declared assessment
operationally verified. Review UI shows the resulting eligibility and qualifications.

### Assertion matching, conflicts and replay

Rules match Brain plus the claims contract's normalized subject, predicate and value.
They also require overlapping repository/environment and fact-time applicability.
Distinct nonempty repository selections do not overlap if disjoint; distinct explicit
environments do not overlap. Empty repository/environment selections overlap relevant
explicit ones. Areas are overlapping organizational associations and do not prove
mutually exclusive applicability. Unknown time bounds overlap conservatively;
nonoverlapping known intervals and distinct observation precision buckets do not.
This bounded textual matching is not semantic equivalence. Ambiguous paraphrases
remain review work; no hidden model matching or global value ban is introduced.

Evidence IDs and unrelated new commits are deliberately not escape keys. Rules retain
the rejected revision's exact evidence as decision provenance while matching equivalent
assertions across other source/reference IDs. A new proposal may be retained with an
explicit blocked disposition, but canonical eligibility excludes it from ordinary
investigation and strict context until legitimate revalidation. Unrelated values,
subjects, repositories, environments and periods remain usable. Withdraw rules have
the same re-entry boundary while keeping a distinct disposition and history.

Current overlapping active nonrejected/nonsuperseded assertions with the same normalized
subject/predicate and different values are contradictions. Rule-blocked candidates do
not independently create conflicts. Investigations may show disputed proposals with
conflict IDs and status; strict modes exclude unresolved contradictions. Rank, repetition,
confidence and recency cannot resolve one. Bound an assertion family to 500 claims and
rules to 10,000 per Brain; limits return 429 without partial writes.

`POST /api/brains/{brain}/claim-conflicts/resolve` accepts 2–20 distinct participant
claim/base-revision pairs in one Brain/assertion family, a reason, and disposition:

- `keep_selected`: accept the explicit selected participant and reject the others
  with durable rules. Existing rejection against the selected assertion requires
  separate revalidation, not silent override.
- `keep_both`: participants supply validated applicability/content with unchanged
  assertion values. Their resulting applicability must be pairwise disjoint; append
  accepted reviewed revisions that record those conditions.
- `retract`: withdraw all participants while preserving qualified history.
- `replace`: reject participants and create one reviewed accepted replacement with
  validated support and the same normalized subject/predicate. Its normalized value
  must differ from every rejected participant. Support-only revalidation uses the
  single-claim revalidation action instead.

Resolution concerns the explicit participants. Remaining conflicting siblings stay
visible and block strict eligibility; a partial resolution cannot claim that a whole
family is settled. A stale participant rolls back the complete resolution.

### Read surfaces and downstream enforcement

Extend canonical claim eligibility with matching rule IDs, conflicting claim IDs and
withdrawn state. Current rejection/withdrawal applies even when projections or command
receipts are older. Explicit history can display the old state and evidence with
current qualifications; it cannot silently become current accepted context.
`GET /api/brains/{brain}/claims/{claim}/review` provides paginated decisions, applicable
rule metadata and conflict summaries through current Brain access. The claim detail
and review controls expose evidence, actual decision/reviewer/reason, actionable states,
conflicts and stale-command errors. Readers can inspect but cannot act.

Initial proposals, edits and later extraction/import adapters use the same canonical
admission and eligibility functions. Check the durable rules before writing the
disposition; check them again for each model-facing/current read. Existing source and
repository artifacts remain historical evidence, not accepted claims. Future raw,
vector, graph and synthesis consumers must enforce these rules in their own slices.
No current general retriever or graph projection is falsely claimed here.

A Brain-local eligibility counter advances with canonical claim/rule decisions and
source/manifest revisions and scope-group removal. This is a projection invalidation counter, not a content
hash or strict product format version. Future caches/analytics retain this counter
and cannot call stale eligible inputs current. PostgreSQL decisions/rules remain
available through service restart, durable refresh and canonical reconstruction.

Use 128 KiB single-review bodies and 2 MiB conflict bodies, the existing text/support
limits and 20-row review history pages. Secrets are rejected before decisions or
receipts are persisted. Brain RLS covers decisions, rules, revision exceptions and
eligibility counters. Deleting sources/claims/rules for privacy remains the distinct
retention/erasure contract; no indefinite retention assertion overrides it.

## Acceptance

- Actual browser/API acceptance changes strict eligibility with available supported
  positive controls. Reader/device/forged authority, archived Brain and stale reviews
  are denied. No model-provided identity can impersonate a reviewer.
- Reject/correct/withdraw, then submit equivalent values with new evidence and claim
  IDs, restart/reconstruct and refresh: blocked assertions stay inactive, replacement
  and unrelated values/subjects/environments/periods stay usable.
- Explicit revalidation records its basis and exact revision exception; unrelated
  later inputs cannot reuse it. Review history stays distinct from ordinary supersession.
- All four conflict dispositions work; same-time conflicting values do not silently
  win by recency. Disjoint applicability can coexist. One stale participant aborts all.
- Rules/current eligibility apply to historical qualified reads and idempotent replay;
  audit, decision, current revisions and invalidation are atomic. RLS hides foreign IDs.
- Focused Rust/UI proof, actual local calls, generated API/build, rustfmt/Clippy and
  `./scripts/validate.sh`; archive only on actual acceptance evidence.

## Explicit Deferrals

Erase and retention follow in `memory-retention-and-erasure`. Model policy/automatic
acceptance, source capture, general retrieval, graph/cache rebuilds and MCP adapters
implement and test their downstream consumers in their named slices. This slice
establishes their canonical rejection, conflict, review and invalidation inputs.
