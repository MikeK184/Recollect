# Claims, evidence support and temporal history

Status: accepted

## Source

[ADR 0005](../adr/0005-canonical-claims-and-time.md), the accepted
[vision](../foundation/vision.md#revisions-and-evidence),
[mutation principles](../foundation/engineering-principles.md#route-every-mutation-through-one-policy),
[workspace scope](evidence-workspace-scope.md),
[collections](evidence-collections.md) and
[publication](evidence-repository-publication.md).

## Contract

### Identity, authorship and support

A Brain owns a stable claim UUID. Its immutable knowledge revisions have UUIDs,
server knowledge times and actual author/device identity. A revision contains kind
`claim` or `decision`, subject (256 characters), predicate (128), value (4,000),
rationale (4,000), an explicit repository/area/environment selection, optional exact
manifest revision, fact validity and 1–20 exact support references. Subject/predicate
normalization trims, collapses whitespace and lowercases; value normalization
collapses whitespace but preserves case. This is bounded textual identity, not
semantic equivalence. No model-supplied reviewer or accepted state is admitted.

The [procedure/handover contract](memory-procedures-and-handovers.md) extends the
kind set and typed details, contribution qualification and kind filtering while
preserving this shared lifecycle. Its owning pack records actual delivery.

Support kinds are `source_version`, `repository_fact` and `manifest_revision`.
IDs must exist in the same Brain. A source version can include an inclusive line
span only when retained text exists and contains that span. A repository fact
already identifies its exact snapshot and original source location; a manifest
support identifies its immutable selection and declared/observed meaning. No raw
excerpt is copied into the claim. A reference-only source is valid support with
explicitly unverified remote availability. Evidence detail shows exact provenance,
retained text or fact/manifest data, and absence states without remote fetching.

Repository support must belong to a selected repository. An optional manifest
revision must have the selected environment and contain every selected repository;
repository facts must match that manifest's selected snapshots. Without a manifest,
repository facts remain pinned to their exact snapshots; new unrelated publications
do not make them stale. Source support need not copy source group memberships:
groups organize evidence and do not independently authorize knowledge.

Browser authors choose explicit applicability. Paired device writes require an
open, valid `write` operation owned by the same actor/device and exactly the same
selection. Browser writes may include that same operation or retain their explicit
selection without claiming a task. Operation bindings and support identities survive
later task changes. New records require no base; edits require the current revision
UUID. Every edit is a new proposal and must not overwrite a prior human decision.
Review/correction commands extend that rule in the successor contract.

### Independent state and eligibility

Manual authoring sets origin `browser_authored` for browser sessions or
`device_authored` for paired clients; neither proves a person authored or reviewed
the text. Review starts
`proposed`, with no reviewer or acceptance policy. Later review can set
`accepted`/`rejected` through its own command. Freshness is independently
`current`, `needs_verification` or `superseded`. Operational state is independently
`declared`, `implemented`, `deployed` or `verified`; it records the author's
assessment, not a service-certified outcome. `deployed`/`verified` require an
observation time and nonempty description of the observed outcome. This attribution
does not cause acceptance. Decisions preserve intent as kind `decision`.

Current eligibility returns explicit reasons and three booleans: investigation,
strict accepted, and strict operational. Investigation includes supported proposals
with qualified freshness and availability. Rejected/superseded records are excluded
from ordinary results; explicit history remains inspectable. Strict accepted requires
accepted review with reviewer/policy attribution, current effective freshness, available support and applicable scope/
time. Strict operational additionally requires `verified` and observation metadata.
No manually created proposal qualifies merely because it says verified. Conflict,
withdrawal, rejection and erasure rules are added by their owning successor slices
before any automatic regeneration or general retrieval ships.

Effective freshness preserves the recorded assessment but additionally reports
`needs_verification` when a supported document has a later version at the selected
knowledge time, a relevant repository selection changed in its named manifest, or
the current evidence bytes are unavailable. Only selected repository entries matter;
notes-only or unrelated repository changes do not stale the claim. A different
environment's manifest never changes this result. Missing/deleted scope resources
block ordinary eligibility. Historical freshness compares evidence/manifests known
at the chosen knowledge time; current access and actual storage availability always
apply. Reference-only evidence remains a qualified support and cannot satisfy the
available-support requirement of strict modes.

### Fact time versus knowledge time

Fact validity is `unknown`, `point`, or `interval`. Unknown has no endpoints and
precision `unknown`. Point requires `from`, no `to`, and precision `second`,
`minute`, `hour` or `day`; the UTC precision bucket containing that instant is
the observation window, not proof of uninterrupted operation. Interval has at least
one endpoint, precision `second`, and `from < to` when both are known. Intervals
are `[from,to)`; absent bounds stay visibly unknown. Timestamps use UTC and
microsecond precision, years 1–9999. Unknown time is never inferred from a commit.

Knowledge time is assigned after the Brain write lock, monotonically increasing
by at least one microsecond per claim. A revision applies until the next revision's
knowledge time. Clients cannot backdate knowledge. Late evidence may describe an
earlier fact interval while being learned now. A query selects the latest revision
known at `knowledge_at` (default current server time), then applies `fact_at` if
supplied. No matching knowledge revision means no result. Known fact boundaries
exclude mismatches; missing bounds/unknown validity qualify investigations and
fail strict temporal selection. Without `fact_at`, no continuous-current validity
is inferred: validity remains labeled, and states/manifests select ordinary context.

### Commands, reads and limits

All paths are under `/api/brains/{brain}`:

- `POST /claims`, `PUT /claims/{claim}`: append a proposal using `ClaimInput`;
  optional Idempotency-Key replays only identical normalized input with current
  write/device/scope authority. Stale base returns 409. Unknown input fields fail.
- `GET /claims`: pages of 20; optional repository/environment/area, `fact_at`,
  `knowledge_at`, mode `investigation|strict_accepted|strict_operational|history`,
  and offset. Modes apply the same canonical eligibility used by detail.
  Each page examines at most 20 scoped candidates; ineligible candidates can leave
  a shorter page. `total_candidates` and `next_offset` describe candidate paging,
  not a promised count of eligible results.
- `GET /claims/{claim}`: selected knowledge revision, eligibility and paginated
  history with knowledge intervals. An explicit historical selection can display
  an ineligible revision without claiming it is current usable context.
- `GET /claim-evidence`: pages of 20 exact evidence choices, filtered by kind,
  optional repository and bounded literal search, including historical versions.
- `GET /claim-evidence/{kind}/{evidence}`: exact evidence detail/availability.

Readers can inspect current permitted data. Writers/admins can append proposals
unless the Brain is archived. Foreign Brain/resource IDs return 404; scope or device
authority mismatches return 403. Invalid input returns 400/422, stale edits 409,
capacity 429. Bound requests to 128 KiB, claims to 5,000 per Brain and histories to
1,000 revisions per claim. Lists use offset 0–1,000,000. Configured credentials and
recognizable private keys are rejected before claim or receipt persistence. Authoring
and evidence viewing never render source HTML or execute it.

Migration 008 stores identities, revisions and typed support links under Brain RLS.
Canonical append/current pointer, audit and durable `brain.refresh` work are atomic.
Immutable revisions cannot be updated through the application database role. Failed
commands leave no partial claim. Reads hold the shared Brain lock while canonical
writers take its exclusive lock, keeping claim and evidence selections consistent
within a response without replacing the existing authentication transaction.
No claim projection or external service owns eligibility; the local browser/API is
usable before future search or model work exists.

## Acceptance

Desktop list amendment, 2026-09-26: the
[desktop contract](desktop-experience.md#literal-list-search) adds optional memory
`q`, trimmed to at most 200 UTF-8 bytes, for case-insensitive literal containment
over subject, predicate, value and rationale before pagination. It does not alter
eligibility, revision/history reads or authorized detail. Empty q preserves the
prior list; the UI names this assertion-search scope rather than claiming Recall.

- Actual API/database and browser authoring, evidence inspection and stale editing;
  exact earlier source/version support survives later edits and service restart.
- Late evidence gives different fact-time and knowledge-time results; point precision,
  unknown endpoints and half-open boundaries do not invent continuous operation.
- Forged review fields fail; independent state labels and strict exclusion work,
  with accepted fixture controls for the canonical eligibility function.
- Production/development and unrelated manifest entry changes reassess only affected
  claims. Missing bytes, reference-only evidence, wrong Brain IDs, revoked grants,
  archived Brains and device/scope mismatches preserve explicit failures.
- Replay/concurrent stale edits preserve one canonical revision, audit and queued
  work; RLS and direct immutable-row mutation tests pass with positive controls.
- Focused Rust/UI checks, generated API, formatting, Clippy and `./scripts/validate.sh`.

## Explicit Deferrals

The [review/corrections contract](memory-review-and-corrections.md) extends this
baseline with actionable decisions, withdrawn lifecycle, durable rules and conflict
eligibility; its owning slice records implementation status.

Human acceptance/rejection/correction, Withdraw, conflict resolution, Erase,
retention, provider gateway/learning and procedures belong to subsequent memory
slices. General exact/semantic/graph recall and synthesized answers belong to their
retrieval/provider slices. This slice does not transmit evidence to OpenAI.
