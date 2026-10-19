# Automatic knowledge organization

Status: accepted

## Source

The user's instruction to implement the
[dedicated plan](../research/automatic-knowledge-mapping-plan-2026-10-09.md),
with product implementation in Rust, and
[ADR 0020](../adr/0020-automatic-knowledge-organization.md).
Existing [canonical graphs](graph-projection-and-traversal.md),
[support qualification](memory-source-support-verification.md),
[retention/erasure](memory-retention-and-erasure.md),
[durable work](platform-durable-work.md) and
[concurrent preparation](graph-concurrent-preparation.md) remain governing.

## Contract

### Candidate ingestion and origin

One bounded mapping candidate batch binds an immutable canonical input/revision
and explicit schema/adapter revision. Canonical input selection, Brain and scope
come from the authenticated worker/command, never model-supplied destination IDs.
Source-span extraction v1 supports English; unsupported/missing/truncated input
and coverage are visible states rather than fabricated successful mappings.
Structured repository/claim fields retain their explicit field origins and do
not pretend a paraphrase is a literal source mention.

Source candidates use half-open UTF-8 byte offsets into the exact immutable input.
Validate unique local mention keys, supported kinds, bounded strings, finite
confidence, nonempty in-range character-boundary spans and byte-for-byte quote
equality. Reject duplicate JSON fields, foreign input IDs, unknown fields and
nonexistent relation/alias endpoints. Do not partially publish a malformed batch.
V1 limits: 2 MiB immutable input, 256 KiB candidate JSON, 128 mentions, 128 alias
candidates and 256 relationship candidates. No score threshold bypasses source
eligibility, kind/scope validation or the declared extractor quality gate.

Kinds are person, technology, service, environment, repository, concept and
setting. A relation candidate references two distinct local mention keys and an
exact source span covering their occurrences. Store this as a source-described
navigation candidate, not accepted operational truth. An assertion relation
requires an independently eligible claim revision and its canonical supports.
The declared relation vocabulary is `uses`, `configured_by`, `depends_on`,
`located_in`, `implements`, `related_to`, `does_not_use` and `has_owner`. Explicit
negative usage and stated ownership remain faithful source-described candidates;
neither becomes a positive usage assertion or an authorization decision. The
provider schema and deterministic validator share this vocabulary and the local
key grammar `[A-Za-z0-9_.-]{1,80}`. Quote anchors preserve exact raw source line
windows and occurrence selection, and pair evidence encloses both selected
mention occurrences. Unsupported source languages produce an explicit absent
mapping. These constraints do not permit fuzzy repair or partial batch admission.
Prompt/schema and adapter changes receive new identities; old receipts retain
their original meanings and never qualify a changed producer.
The next unqualified `source-facts-4` / `source-reconciliation-4` candidate uses
`learning-quote-anchors-3`. Its occurrence ordinal counts every exact substring
match, including overlapping matches, left to right within the selected inclusive
raw-line window. A null ordinal requires exactly one such occurrence. Resolve
these coordinates in bounded linear time; never choose an arbitrary first match.
Select the adapter from the actual successful request's exact prompt/schema
pair. Preserve `-2` with adapter1 and `-3` with adapter2 as distinct historical
identities and retain their prior coordinate semantics. Cross-family labels and
wrong adapters reject. The source DTO remains `source-mentions-1`; this mechanical
repair supplies no semantic qualification or release catalogue entry.

Alias candidates additionally require explicit supported equivalence evidence
and compatible kind/scope; mere similarity, co-occurrence or confidence is
insufficient. Human alias decisions are durable and independently attributed.

### Identity and alias boundaries

Persist original entity identities and supporting mentions. Exact identity
matching uses Brain, kind, a canonical instance realm and normalized label.
Technology/concept labels may normalize case and whitespace; case-sensitive
service/repository/setting identities preserve case. Do not strip accents,
punctuation or meaningful prefixes as a fuzzy merge rule.

Instance realm binds validated repository/environment/area context where known.
An underspecified instance uses its canonical source/claim origin as a conservative
namespace; it cannot collapse into DEV/PROD or another repository/customer by
name alone. Explicit environment mentions can distinguish candidates but cannot
select authorization, grant access or equate a text label with a manual group ID.
Ambiguous matching remains visible and unresolved. Proven context reconciliation
creates a reversible link, preserving the original identities/support.

An alias links compatible original entities with its independent support and
disposition. Removal/correction reverses its effect without erasing remaining
independent mentions. Representative identity matching reuses persisted records;
deletion/splitting retains surviving original IDs and records lineage. Replaying
the same eligible inputs is idempotent; GDS community numbers are never IDs.

### Automatic overlapping topics

Persist topics and automatic source memberships separately from
`evidence_groups`/`evidence_memberships`. Automatic recomputation never replaces
manual membership. The fixed semantic recipe uses eligible shared entity/concept
associations and optional compatible existing vectors, not claim-support topology
alone. Environment/person/common boilerplate do not become unweighted hub terms.
Weight shared terms by inverse source frequency and retain exact contributors.
Use native GDS Leiden/WCC for declared community calculations; do not write a new
Rust clustering engine. Entity facets allow overlapping memberships alongside
single-community results. Native scratch outputs remain provisional.

Compare entity-overlap, metadata-only and eligible-vector grouping on frozen
development/held-out pairs before selecting recipe thresholds. No new embedding
or generative call is needed for the baseline. Sources without usable inputs are
unassigned with coverage. Automatically create names from representative eligible
source-backed terms; polished summaries may reuse extraction output. Match
updated groups to persisted topic IDs using surviving contributor identity;
record merges/splits and recompute labels/counts when contributors change.

The October 18 label-path correction makes representative selection explicit.
Overview, source-detail and unpublished evaluation use the same deterministic
term ranking: most distinct currently qualified contributing source versions,
then canonical kind and normalized term identity. Repeated mentions cannot
increase a term's rank. Retain scoped identity distinctions and choose an exact
quote by source-version ID, UTF-8 start/end and quote, with mention ID only a
final identical-span tie break. A facet retains its eligible anchor; an automatic
community uses its technology/concept feature terms. For compatibility, an
unrenamed split or manual overlay may retain eligible terms of any existing
kind. A still-qualified human label keeps its existing precedence and exact
source/mapping basis. Otherwise, use a qualified canonical title only if no
eligible term remains, choosing by source-version ID. Every page ranks only
its complete qualified contributor window, independently of detail-row limits;
hidden, excluded, split-away, expired or unqualified contributors cannot affect
the label or its representative. Declare term, human-label or title origin
accurately. This correction changes neither membership nor qualification of
the failed frozen recipes, whose original labels and receipts are retained.

Users can rename, pin, exclude and split automatic topics through authenticated,
version-checked commands with mutation audit. Preserve these overrides across
rebuilds. Overrides do not authorize content or preserve erased derivatives.
When all support disappears, remove derived names/memberships and orphan records;
independently supported entities and topics survive with recomputed derivatives.

The user's 2026-10-18 choice preserves an edited automatic topic that has no
unique surviving match as a manual overlay, alongside new automatic groups.
An edited topic includes a human name, pin, exclusion or explicit split. Keep its
original topic ID, still-supported human intent and exact current contributors;
give the conversion a new version and an audited rebuild history entry. Distinguish
the stored kind `manual_overlay` and recipe `human-overlay-1`, detach its automatic
report dependency, and do not match it back into a later automatic community.
Uniquely matched edited communities keep existing identity and override behavior.
Topic cards expose the stored topic kind through an additive optional
`topic_kind` value (`facet`, `community`, `split`, `manual_overlay`); existing card
`kind` values remain unchanged and older JSON without this field still decodes.
Mark an overlay as a manual group in the UI rather than implying automatic
qualification of its label or members.

Qualification precedes conversion outside the writer: preserve only currently
eligible immutable source versions and contexts and their genuine entity
memberships. A source-only contributor does not need an entity feature or a
currently usable vector. Preserve exclusions/splits only for those exact inputs;
keep a human name only while its exact basis survives. Correction, changed
context, removed support and erasure cannot transfer intent to replacement data.
If no contributor survives, prune the overlay and its content-bearing derivatives
rather than publishing an empty preserved name. Existing bounded preparation and
membership envelopes and the 1.5-second writer limit apply to overlay conversion;
capacity failure cannot silently discard intent or publish a partial overlay.
The aggregate retained set, including pre-existing overlays, is bounded to 10,000
distinct persisted input IDs across the union of canonical source inputs and
mapping inputs, 50,000 membership/intent rows and 10,000 topics. Repeated references
to one input count once; multiple mapping origins for one source version remain
distinct inputs. Preparation fetches each bound plus one and returns
`organization_overlay_capacity` before mutation when exceeded; source
qualification uses pages of at most 250 within the existing preparation deadline.
The current actor/device, epoch, lease and deadlines are rechecked at publication.
Privacy pruning considers both source and entity memberships so an unrelated
entity deletion cannot remove an independently supported source-only overlay.

Persist the human-adopted contributor cohort independently of automatic report
results. On a version-checked human edit, prepare the complete current qualified
topic cohort outside the writer, then atomically save its exact canonical source
inputs and optional genuine mapping/entity memberships with the edit and audit.
Legacy-only topics also create or reuse canonical source-version/context inputs
for this human intent; no embedding or new inference is required. A mapping or
vector profile change must not erase the source cohort adopted by the human.
Use bounded `knowledge_topic_intent_sources` and
`knowledge_topic_intent_mappings` rows with exact Brain/topic/input/entity foreign
keys and cascading privacy removal, storing no source text or vectors. Snapshots
are human intent dependencies, never an exception to clearing invalid automatic
calculations or permission to expose an unqualified member.

Current edited memberships may seed provisional dependency rows during upgrade,
but readers/publishers must freshly qualify them before use. Uniquely matched
communities refresh intent from their new genuine qualified cohort; unmatched
communities preserve only qualified saved intent as an overlay. Include both
tables in physical privacy closure, defaulted older manifests and Brain registry
deletion. Test a source-only edited group whose automatic memberships are cleared
by vector replacement before regrouping; entity memberships must not mask lost
source intent in that proof.
Upgrade also bridges existing human entity-topic names to the exact adopted
source-version/context basis before producer replacement can delete the old
mapping input. Preserve historical valid names up to 256 UTF-8 bytes without
truncation; current rename-command limits remain unchanged.
Bridge legacy exclusions and split-child source memberships on that same exact
basis before ordinary producer replacement can remove the old input. A bridged
name stays hidden while its basis is excluded or split; an explicit unexclude
can restore the retained name. The last accepted unexclude clears equivalent
exclusion rows in both representations of that exact topic/version/context.
Never extend this to another version, context, topic or split decision.
If multiple legacy inputs assign one canonical source basis to different split
children, preserve the original rows and return `organization_intent_conflict`
before dependent reconciliation or ordinary producer replacement. Do not select
an arbitrary child. Upgrade may skip that ambiguous source-split bridge; explicit
privacy erasure, retention and Brain deletion still remove all affected data.
Preparation includes the pending edit's new controls, split-child memberships,
topic and bridge rows in the aggregate envelope before entering the writer.
That envelope includes all retained human split groups and their complete
transitive split descendants. Editing a split group cannot bypass preparation;
cycles or repeated references do not duplicate the accounting. Explicit split
groups keep their existing kind and identity during reconciliation, and never
participate in automatic community matching or overlay conversion.
Restored overlay memberships declare `explicit_overlay` or
`explicit_source_overlay`, rather than claiming a native grouping result or a
human split. Orphan pruning considers all current membership and human-intent
tables: an invalidated report may temporarily leave only nonvisible saved intent.
Exact input erasure cascades that intent before the final orphan check.

### Bounded retained-vector hybrid candidate

This candidate extends the accepted optional-vector design; it is not a qualified
production recipe. The frozen 20-document gold-mention experiment failed the
assignment gate, and its observed held-out data cannot select new parameters.
Keep that experiment and its community-only fixture correction. Implement and
test the following candidate with synthetic finite vectors only until independent
real compatibility and fresh untouched held-out quality evidence permit selection.
Do not seed a qualified recipe or make a provider call through this amendment.

Canonical source contributors are independent of an extraction producer. Persist
an immutable source-feature input containing Brain, exact source/source-version,
server-captured canonical context and creating actor. Deduplicate the same exact
version/context under the Brain writer; context changes create a new input.
This input contains no title, text, vector, fabricated mention or accepted claim.
Its eligibility uses the same complete canonical source/fragment qualification,
current artifact, scope, ancestry fences and retention as graph navigation. A
missing extraction profile cannot hide an otherwise eligible vector contributor.
Entity features still require their independently qualified mapping origin.

Reuse only the current immutable semantic profile compatible with the current
provider, embedding model, dimensions and `engineering-text-1` representation.
Capture its generation in the calculation. Current policy must permit embedding
for the source's server-derived classes; `automatic_embedding` need not be true.
This grants no new inference, query embedding, transmission, retries or model
permission. Source-chunk entries must be ready, exact, retained, unsuppressed and
bound to that profile and canonical source version/chunk. Ordinary request-detail
expiry does not invalidate an already published retained vector. Profile switches,
source/context changes, removed chunks and input fences do invalidate its use.
Retained use checks enabled purpose/content classes and model/profile compatibility
without requiring a provider credential. A missing key cannot hide already
retained evidence; a transmission-admission helper is not the retained-read gate.

The vector feature requires all materialized chunks of a qualified source to have
compatible, nontruncated retained vectors. Missing/pending/blocked entries remain
explicit feature coverage. Bound a source to 16 chunks and the complete usable
vector cohort to 250 source versions, 1,000 entries and 4,096 dimensions. An
oversized cohort is not a first-250 prefix: make the vector branch unavailable
with a budget reason while preserving the complete independently eligible entity
branch. Absence, policy/profile unavailability and a mathematically zero centroid
may use this honest fallback. Corrupt dimensions, nonfinite values, invalid stored
zero vectors or inconsistent provenance fail the calculation closed.

Compute one per-source centroid in PostgreSQL as the normalized mean of its
unit-normalized eligible chunk vectors. PostgreSQL computes exact cosine distances;
vectors and centroids never enter Neo4j, the browser, logs or a second vector
store. Do not introduce ANN or Rust clustering. Add positive source-to-source
edges only for mutual top-five neighbors with cosine at least 0.75, breaking ties
by exact source-version UUID. Scale cosine by the conventional median of one
positive IDF weight per distinct eligible term (the mean of the middle pair for
an even count), or 1.0 when no term edge exists. Keep the existing deduplicated technology/concept
IDF edges, using the full qualified source cohort as the corpus denominator;
person/environment/boilerplate remain excluded as hub terms. These provisional
parameters name `source-idf-retained-vector-leiden-1`, not measured thresholds.
Use the same native Leiden parameters, qualified GDS version, 10k-node/50k-edge
envelope, five-second readonly preparation and finite leased/journaled native
attempt. No usable edge means unassigned rather than an invented singleton topic.

Persist source memberships separately from entity memberships. A topic-source
membership references the canonical source-feature input and its actual feature
reason. Existing facet/entity rows preserve their meaning; an entity UUID is
never invented or borrowed to encode a vector-only source. Bind each calculation
to every exact contributing source input, mapping input and semantic entry/profile/
request. Native weights and group numbers remain provisional until the same
complete cohort, feature identities, profile/recipe heads, current authority,
epoch/outbox revision, exact lease and earliest deadlines are freshly rechecked.
The final writer remains capped at 1.5 seconds. Entry replacement/removal, request
suppression and profile changes coalesce invalidation; removing one influence
invalidates and physically clears the entire affected calculation/projection.
Privacy preview, Brain deletion and old-manifest replay cover these dependencies,
including vector-only sources, without removing independent facets or contributors.

Expose typed source memberships and qualified representative sources additively.
Keep existing entity/mention/connection rows and their cursor semantics. An
independent source-membership detail endpoint uses the same authenticated
`GraphSelection`, topic center, epoch/deadline and 250-source window, emitting at
most 80 rows per page. No-access, empty, partial, stale, failed and vector-feature
unavailable states remain distinct. Counts and labels derive only from the current
qualified window; full-window report expiry still bounds every page. When no
eligible term supplies a label, use a currently qualified canonical source title,
declaring field provenance instead of a fake byte span. Such provenance proves
origin, not semantic usefulness.

Add explicit source-basis rename/exclude/split commands while preserving legacy
mapping-basis commands, pin, idempotency, base-change checks and mutation audit.
Source overrides bind the immutable source-feature input. Matched rebuilds may
transfer an existing mapping-basis override only to the same exact source version
and canonical context with surviving support. Correction/erasure/context changes
never transfer intent to a different input. A source exclusion or source split
also hides that exact source-version/context contributor's entity memberships
from the same original topic. Other topics and global entity/mention identity
remain eligible; a source split may expose its child as a typed source group.
A later representation of the same contribution cannot resurrect an excluded
original-topic membership. Identity reconciliation uses unique
strict-majority surviving source-version/context contributors, one-to-one, with
lineage bound to a contributor present in both generations. Unmatched edited
topics follow the user-selected manual-overlay behavior above.

Synthetic vectors prove dimension/model/scope gates, publication, boundedness,
privacy and recovery only. For later real evaluation, fit development data only
over the predeclared grid k={3,5,8}, cosine={0.65,0.75,0.85}, scale={0.25,0.5,1.0}.
Require development pair precision at least 0.95, then maximize pair F1, assignment
coverage and finally the lexicographically smallest parameter tuple. If none meets
the precision floor, select nothing. Freeze corpus, real vector provenance,
parameters, GDS/representation identities and label rubric before evaluating a
fresh untouched held-out corpus. Keep failed/unavailable cases and all pair
denominators. No fake vector or previously observed held-out score qualifies a
production recipe; blind label usefulness remains a separate gate.
The [frozen blind rubric](../research/graph-organization-blind-label-rubric-v1-2026-10-18.json)
defines source fidelity, scope/time/authority and navigation-usefulness ratings.
Bind its bytes in the campaign before real outputs. An independent reviewer sees
the exact displayed label and complete qualified contributor evidence while arm,
parameters, gold topics, partition and scores remain hidden until ratings are
durable. Retain every failed or unreviewable group; an unavailable/empty arm is
not a usefulness pass. Report failing baselines without converting their failure
into a selected-candidate pass. Review can use local independent assessment and
requires no paid answer judge or automatic activation.

### Unpublished candidate evaluation

An internal Rust evaluator may compare validated real extraction candidates and
the same canonical retained-vector features before producer/recipe qualification.
It cannot create a producer origin, mark a learning run successful, select a
qualified head, write canonical entities/topics or change normal ranking. No HTTP
route, browser action or autonomous scheduler admits these experiments. Public
analytics list/get/results continue to accept only `purpose='topology'`.

Use an explicit currently authenticated browser evaluator separately from the
producing request's actor/device. Canonical experiment reads use the evaluator's
current Brain access; admission additionally checks the original producer's exact
actor, device, current role and authority deadlines. Never drop producing-device
attribution to obtain a default-scope read. Freshly revalidate every offered
source/claim reference, including current claim heads, supports and fences, at
admission and every preparation. An unchanged stored manifest alone does not
prove that earlier offered memory is still current. Experiment-only expiry or
candidate deletion removes only dependent experiment reports and must preserve
the same Brain's normal production report, epoch, heads and topic data.

An automatic standing-policy learning run may legitimately record no device.
For experimental offered-memory validation, a device-attributed request may bind
such a run only when the run is automatic, its actor/input0/policy/operation are
exact, and immutable canonical source-origin records prove that same producer
actor/device and declared selection/manifest: the exact capture event/binding and
its operation, or the exact imported source version and its write operation.
Keep the request's device attribution and its current claimed-device, account,
role and deadline checks. Manual runs and unrelated devices do not gain this
fallback. This does not change production scheduling or attach historical source
device expiry to ordinary standing-policy work.

Use explicit `organization_shadow` analytics reports with the existing exact
lease, native GDS worker and cleanup journal. Store bounded experimental candidates
separately, referencing the exact successful unsuppressed extraction request,
canonical source/version/context, actor and adapter/schema identity. Validate the
candidate through the real adapter and offered-input manifest; model-supplied IDs
cannot choose destinations. Keep at most 250 sources, 256 KiB per candidate and
1,000 compatible vector entries; expiry is no later than seven days or the
earliest source, authority or audit deadline. Store neither raw provider responses
nor vector values in this candidate table or exported results.

The evaluator shares production canonical qualification, PostgreSQL centroid and
exact cosine, deterministic IDF projection, and native weighted Leiden. Resolve
exact source/chunk/profile/request dependencies before work and freshly recheck
the complete cohort, authority/device, epoch, lease and earliest deadline after
native cleanup. Missing/failed features remain in experiment denominators; foreign,
stale or corrupt provenance cannot become a successful result. Only experimental
scores and qualified label provenance are persisted. There is no implicit
promotion from an experimental result to a qualified profile.

Source correction/erasure, request suppression, profile replacement, context or
policy changes invalidate the whole affected experimental result and native
projection. Include candidate/report dependencies in physical privacy closure,
Brain deletion and normalized older-manifest replay, preserving independent
controls. The development grid and fresh held-out separation above apply equally
to this evaluator. Freeze selection and provenance before the first held-out
evaluation, and keep real-candidate receipts distinct from zero-call gold proofs.

Compare five explicitly named equal-cohort arms: `native_hybrid` includes hybrid
weighted Leiden and the existing eligible-term facets; `native_weighted` includes
entity-IDF weighted Leiden and those same facets; `entity_overlap` contains only
the exact eligible-term facets; `metadata` contains overlapping groups from actual
canonical repository/environment/area references; `eligible_vector` uses retained
vector-only native Leiden without term edges or facets. All arms retain the full
declared source denominator, including missing NER/vector features. Only hybrid
parameters use the predeclared development grid. Freeze its chosen tuple before
held-out work; the vector baseline uses the same neighbor/cosine parameters and
uniform scale with IDF fallback 1.0, without separate held-out tuning. A failed
development selection prevents a qualification claim, including for other arms.

### Automatic processing, current reads and privacy

Source materialization/revision, eligible learning output, repository publication,
claim correction, scope/membership change and model/schema changes queue durable
idempotent mapping work. Prepare outside long Brain locks; final mutation rechecks
actor/device role, exact lease, input identity/epoch, current policy/fences and
retention deadlines. Stale/erased work cannot publish. Incrementally update affected
records; full rebuild must reproduce the eligible view and user overrides.

Entity/topic APIs hydrate current source-backed names, mentions, membership
explanations and counts through canonical qualification. Every contributor must
meet current Brain/task scope, exact input, correction, support and retention
requirements. Never disclose an excluded alias/name by using a global cached
label in a narrower scope. Model-input fences apply to derived outputs too.

Add exact derivative dependencies to privacy preview/physical cleanup and restore
journal replay. Remove rejected/erased contributing mentions, relation/alias
evidence, extracted payloads, topic assignments, derived names/counts and native
projections. Keep only independently eligible support and rederive survivors.
Expiry/correction invalidates entire affected aggregates, not just their rows.
Brain deletion and old-backup restore retain the same closed boundary.

### Navigation and cost

Provide typed organization records rather than fake claims/unchecked Neo4j
labels. Default entry is paginated topic/entity cards with coverage/current state;
expand a small neighborhood and hydrate selected evidence on demand. No whole
10k-node browser load is required. Names, counts and connections explain why
documents belong together; exact source links remain accessible. Loading, partial,
stale, empty, failed and no-access states remain distinct and accessible.

Reuse the existing model gateway and standing Brain policy for interpretation;
no hidden new provider or compulsory extractor service is added. Offline fixture
work uses deterministic/fake outputs and zero paid/external requests, without
altering the normal Brains' standing model policies. Synthetic payload validation
is not extractor-quality proof. Optional adapters need independent quality and
compatibility proof before production selection.

### Qualified producers and retained unpublished output

The application release owns a reviewed catalogue of exact producer definitions
and independently measured quality receipts. A browser or model cannot submit
its own qualified flag, receipt hash, provider endpoint or producer definition.
A catalogue definition binds provider, requested/returned model, prompt/schema,
adapter and source schema revisions. Catalogue changes require the same frozen
development/held-out qualification as the original producer. The catalogue may
be empty; that means extraction-based organization is visibly blocked, rather
than silently trusting a candidate fixture.

With an enabled compatible standing extraction policy, the worker may select a
single exact compatible release-qualified definition when no profile is selected.
It never changes the model policy, enables transmission, replaces an explicitly
selected profile, invokes a provider or selects between ambiguous definitions.
Selection is recorded with canonical mutation audit. Current profile withdrawal
and policy/model changes continue to hide dependent mappings immediately.

Validated output from an already authorized learning call may be retained in a
bounded unpublished stage while a qualified profile is unavailable. Limit this
to 1,000 stages per Brain, 256 KiB per stage and seven days, shortened by current
source/audit retention. Capacity exhaustion is explicit. This stage
binds the exact request, learning run, source version, immutable context, schema
and adapter; it is not a visible entity/topic or a cached provider response.
Later compatible selection can stage and enqueue that output without another
provider call, only while the original successful request, policy, actor/device,
source/context, audit retention and input fences remain eligible. Publication
still uses the existing exact lease and current-source checks. A stale,
suppressed, expired or erased stage is physically removed and cannot be retried
as inference. Legacy output that was never retained remains explicitly missing.
Privacy preview, physical cleanup, Brain deletion and old-manifest journal replay
cover these unpublished payloads, including contributor ancestry and producing
request suppression. Independent stages survive exact contributor removal.

Request admission has a persisted monotonic server order. For one exact immutable
source version and context, an older request cannot replace a newer published
mapping, whether it resumes from retained output or an already queued job. Check
this both before staging and under the final publication lock. A superseded
stage/job is an explicit no-op with physical removal of its unpublished payload;
it cannot revert the newer membership or human override. This ordering chooses
an interpretation generation, never truth or a newer source revision.

## Acceptance

- English development/untouched held-out entity and endpoint fixtures: at least
  95% precision/80% recall; preserve unsupported/failing cases in the denominator.
  Include Unicode, long inputs, negation/history, aliases and foreign scopes.
- Zero false identity merges across Brain/customer/repository/DEV/PROD controls;
  duplicate imports, rebuilds, alias reversals and splits preserve eligible IDs.
- Topic source/pair scores and frozen label rubric compare all baselines; target
  at least 80% assignment coverage of annotated in-scope sources, reporting
  unrelated/unassigned documents separately. Verify overlapping topics.
- Manual memberships and rename/pin/exclude/split overrides survive recomputation;
  correction, erasure, scope/model change and restore replay remove every affected
  derivative without deleting independent support or resurrecting content.
- Actual PostgreSQL/Neo4j qualified semantic navigation plus timed browser tasks
  find two related cross-session documents and exact support in changing scopes.
- Populated 10k-node performance with declared concurrency, capture/audits/rebuilds,
  all outcomes counted, zero busy/database/timeout failures and p95 bounded reads
  below two seconds; the recorded normal-Brain read failure also must resolve.
- Focused Rust/integration/UI checks and `./scripts/validate.sh`; record real
  provider/model quality separately. No benchmark improvement without comparable
  answer/citation proof; no paid judge is required for these acceptance gates.

## Explicit Deferrals

Graphify/Graphiti adoption is conditional on equal-input benefit; no mandatory
new code producer, Python product runtime, paid summaries or cloud benchmark run
is selected. Automatic entities/aliases/topics/navigation and invalidation are
required by this slice and are not deferred. Multilingual extractor quality is
reported as unsupported until independently verified. No release/commit/push is
implied. This contract does not relax existing privacy or claim authority.
