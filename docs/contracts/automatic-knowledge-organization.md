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

Users can rename, pin, exclude and split automatic topics through authenticated,
version-checked commands with mutation audit. Preserve these overrides across
rebuilds. Overrides do not authorize content or preserve erased derivatives.
When all support disappears, remove derived names/memberships and orphan records;
independently supported entities and topics survive with recomputed derivatives.

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
