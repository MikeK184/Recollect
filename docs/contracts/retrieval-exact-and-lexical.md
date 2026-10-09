# Canonical exact and lexical recall

Status: accepted

## Source

The accepted [retrieval baseline](../foundation/techstack.md#storage-ownership-and-hybrid-retrieval),
[claims and time](memory-claims-and-time.md), [corrections](memory-review-and-corrections.md),
[retention](memory-retention-and-erasure.md), [scope](evidence-workspace-scope.md)
and [session capture](evidence-session-capture.md) govern this consumer.
The full product goal authorizes routine interface, ranking and budget decisions.

## Contract

### Request, authority and scope

`POST /api/brains/{brain}/recall` accepts a bounded query, optional exact reference,
scope selection, optional context-operation UUID, collection and exact manifest
revision, knowledge/fact time, eligibility mode, selected channels and budgets.
Unknown fields fail. Query text is at most 512 UTF-8 bytes. Either nonblank query
text or one exact `{kind,id}` reference is required. Exact kinds are `claim`,
`source_version`, `repository_fact` and `manifest_revision`. Empty queries do not
list the whole Brain. Channels initially are `exact` and `lexical`, both by default.

Readers may recall an accessible Brain, including an archived Brain. Every request
uses current account/device access and a shared Brain read lock. A paired caller
must provide its own valid `context` or `retrieval` operation; browser callers may supply one or
an explicit selection. Provided selection must equal the operation's immutable
selection. A later task default does not relabel it. Missing/foreign resources
return 404; another actor/device's operation or wrong operation kind returns 403.
Closing a task prevents new operation creation; a previously bound context may
finish under current access and its original scope. No scope change grants access.

Selection supports up to 100 repositories/areas and one environment, using existing
identities. Empty candidate associations mean Brain-wide applicability; otherwise
repository/area intersections and the selected environment filter candidates.
Current source memberships organize imported documents. Captured sources also
retain their original operation scope, including subsequent source versions.
Collection filtering requires current membership for sources, or a claim supported
by a member source. It does not invent collection membership for repository facts.

An explicitly selected manifest must be readable, match the selected environment
and contain the selected repositories. Its immutable snapshot entries determine
repository applicability; foreign/missing snapshot selections are not silently
replaced. Without a manifest, Brain-wide/repository-only investigation can use each
repository's latest retained published snapshot known at the selected knowledge
time, labeled as committed evidence. Environment-scoped repository retrieval
requires an explicit manifest and reports this gap when absent. It never guesses
desired or observed deployment from the newest Git contribution.

Manifest configuration paths also narrow raw repository facts before channel
ranking, coverage counting or graph qualification. An empty list selects the
whole exact snapshot. A nonempty list selects a fact's canonical file/path when
it equals a selector or is below that directory at a literal `/` boundary.
Paths are case-sensitive and never glob patterns; missing locations and parent
directory facts do not acquire applicability from a selected child. This shared
gate is delivered with the combined repository graph slice.

### Time, lifecycle and correction

Freeze `knowledge_at` at the supplied timestamp or the request's server time.
Select the last claim/source revision known then before checking privacy; a removed
latest revision cannot resurrect an older one. Explicit exact historical evidence
is allowed only in history mode and still must have existed by that knowledge time.
Current access, actual retained bytes, rejection/withdrawal and erasure apply even
when historical knowledge is selected. Fact-time behavior reuses canonical claim
precision, unknown-boundary and interval rules. Raw sources/facts have no inferred
fact-validity interval and cannot satisfy a supplied strict fact-time requirement.

Modes are `investigation` (default), `strict_accepted`, `strict_operational` and
`history`. Claims use the existing canonical eligibility function, including
procedure/handover dependencies, conflict IDs, reviewer/policy attribution and
freshness. A selected manifest also excludes contradictory/missing repository
snapshots. Ranking cannot change eligibility. Strict modes return qualified claims;
unreviewed raw sources, repository facts and manifest declarations are evidence,
not independently accepted or operationally verified knowledge.

Investigations can include unreviewed raw evidence, explicitly labeled as such.
They withhold raw fragments affected by an overlapping rejected/withdrawn assertion:
either the exact support/span was blocked, or the fragment contains the normalized
subject, predicate and value of a applicable durable rule. The matching boundary
uses existing whitespace normalization and case rules; it is not semantic-equivalence
classification. A new source/fact UUID or reindex cannot bypass the textual check.
When a blocked support has no span, conservatively withhold that evidence fragment.
Independently supported claims and unrelated fragments remain usable. Explicit
revalidation exceptions authorize their resulting claim revision, not all old raw
copies of that assertion. History may show retained blocked evidence with current
rule qualifications, clearly separated from current usable knowledge. Erased/expired
payloads are never returned, including history.
A claim whose own retention permits it can remain in investigation/history when
its raw support has expired, with unavailable-evidence qualifications. Strict
modes require current retained support. Before returning, recheck deadlines used
for live evidence and required handover contributors; time advancing within a
request cannot preserve strict eligibility after a dependency expires. Canonical
defaults for older stored claims apply equally to SQL prefilters and rendering.

### Canonical indexes, ranking and provenance

Use PostgreSQL's `simple` text-search configuration, weighted claim/title text,
`websearch_to_tsquery` and `ts_rank_cd`. This is PostgreSQL cover-density ranking,
not BM25. Record the algorithm name in each response. GIN indexes derive from
canonical claim revisions, source chunks/titles and repository facts. They do
not own authorization or truth. No separate text-copy projection, search cache,
external reranker or provider call is introduced. Index rebuilds operate on the
same canonical inputs and preserve correction/deletion authority.

Exact references and literal subject/title/fact-name/path matches rank before
lexical candidates. Preserve stable kind/UUID tie breaks. Deduplicate a canonical
claim or source version across channels/chunks, retaining the best matching
fragment and channel attribution. Within exact/lexical priority, consider each
canonical identity's best fragment before adjacent fragments from that identity;
one long source cannot consume the candidate cap before another matching source
is considered. Remaining rows can supply a usable alternative when its first
fragment is withheld. This is source-aware deduplication, not a claim that varied
sources are independent corroboration. A history request can distinguish source
versions explicitly. Every result includes canonical identity/version, recorded
time, applicable scope and provenance; source fragments include their exact
chunk byte/line span, and facts name their repository, commit and snapshot.
Missing or unreadable artifacts never yield stale chunk text as retained evidence.
Reference-only/pending sources may return clearly qualified metadata when their
title matches; pending processing is not complete lexical coverage.
Coverage also reports selected pending/reference-only sources when their title
does not match. Repository/manifest lexical representations include the name and
the first 65,536 characters of their canonical JSON; a selected record exceeding
that representation reports limited coverage, including on an empty result.
Exact identity remains available for these records, with the normal context limit.

### Bounded context and response states

Accept result limits 1–20 (default 10), and context budgets 1–32 KiB (default
8 KiB). Candidate selection examines at most 100 ranked rows per request after
Brain/scope/privacy filtering. Apply canonical eligibility before adding any text
to results. Before the candidate cap, also exclude explicit rejected, withdrawn
or superseded claim states in ordinary modes; strict modes exclude nonaccepted,
authority-less or explicitly stale revisions and require recorded operational
verification when selected. Keep the full canonical assessment after these
conservative prefilters. Dependency, rule, fact-time and manifest checks can
still withhold bounded candidates; this residual coverage limit is explicit.
Short/empty pages caused by withheld candidates remain explicit;
the endpoint does not claim exhaustive search. Return candidate-limit, processing,
missing-source and budget coverage flags rather than padding with ineligible data.

Each fragment is at most 2 KiB on UTF-8 boundaries, with truncation labeled.
The complete serialized model-context object, including attribution and the
data-only instruction, must fit the requested context-byte budget. Whole results
that do not fit are omitted with a budget flag; do not strip provenance to fit.
The response exposes the exact measured context size and the current Brain
eligibility counter. It returns `results`, `no_match` or `insufficient_support`
as appropriate; unavailable channels are errors rather than silent fallbacks.

Context is structured untrusted evidence with a fixed instruction to treat it as
data, preserve qualifications and not infer execution authority. Source HTML is
rendered as text. Source/model instructions cannot change request scope, mode or
policy. This slice makes no model call; full host prompt-injection behavior is
tested when the MCP bridge actually assembles host context.

Bound the request to 32 KiB, concurrent recall to four per server process, the
complete operation to ten seconds and SQL statements to two seconds. Overload
returns 429; bounded timeout returns a safe retryable error without partial context.
Read-only recall creates no mutation receipt/audit or stored raw query. Normal
access logs contain no query/body text. With no cache, each call sees canonical
changes after the writer releases the Brain lock. A client must discard its prior
context after a scope change or erasure; browser result state follows that rule.
Transaction-scoped shared/exclusive admission precedes the row lock so newly
arriving reads cannot starve an already waiting writer. Admission stays per Brain,
allows concurrent existing readers and releases on commit or rollback. The
[integrated evaluation](operations-integrated-evaluations.md) specifies the
measured failure and its required regression proof.

### Browser and native consumer

Deliver a basic Brain recall panel now: query/exact UUID, mode, current selection,
optional manifest/collection/time filters, bounded results with state/provenance,
and links into existing claim/source evidence inspection. Clear results on a
filter, scope or Brain change; clear erased content through the shared query reset.
Show loading, empty, denied, insufficient support, partial and failure states.
This is the usable baseline; the later investigation slice adds the combined
semantic/graph investigation workflow rather than deferring basic usability.

Expose native `scope recall BRAIN_UUID OPERATION_UUID QUERY` for a paired context
or retrieval operation. It calls this same handler with the operation's fixed selection and
returns the attributed bounded response. No host recall injection or alternate
truth policy is added here. Workspace handoff advertises retrieval availability
and still marks fresh context required; later MCP consumes that handoff.

## Acceptance

- Actual API/database positive controls and cross-Brain/device/operation denial,
  concurrent task defaults, selected manifests and collection/area/environment filters.
- Exact identity/literal and lexical/phrase queries find useful source/claim/fact
  evidence; controls prove no empty retriever. Record a small fixed engineering
  corpus baseline, hit coverage and latency without claiming semantic superiority.
- Correct/reject/withdraw, replay/rebuild/restart and query copied raw assertions:
  the blocked assertion and its raw copies stay out of ordinary context while replacements and unrelated
  evidence remain. Strict and historical behavior preserve independent states.
  A replacement rationale may explicitly mention the old value as qualified history.
- Late evidence, unknown validity, current/old manifests, missing source bytes,
  processing lag and erasure during active work remain accurately qualified.
- Meaningful byte/candidate bounds, hostile text as inert data, malformed queries,
  browser desktop/mobile states and native retrieval through a real API call.
- Focused tests, generated API/build, Clippy, local migration/preservation and
  `./scripts/validate.sh`; archive only after actual behavior passes.

## Explicit Deferrals

Semantic embeddings/reindexing, graph fusion, richer investigation UI and MCP host
recall belong to their named successor slices. ANN, learned ranking, query expansion
and model-generated answers are not required by this baseline. No general semantic
equivalence or universal prompt-injection prevention is claimed.

## Complete source windows — 2026-10-08

The user's improvement request authorizes this bounded first batch. Return whole
canonical source chunks up to 4,096 bytes rather than clipping at 2,048 bytes while
retaining a longer citation coordinate. Preserve distinct eligible scoring spans
as alternatives. After exact priority and source-diversity/global ranking, spare
context capacity may admit at most three nonoverlapping source windows per source.
Every window counts against the existing item and serialized byte limits. Each
retains its exact revision/byte coordinates and is independently revalidated;
retention deadlines are conservatively combined. No whole-source expansion occurs.
