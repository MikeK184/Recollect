# Procedures and multi-repository handovers

Status: accepted

Current operating-model amendment: [autonomous maintenance](memory-autonomous-maintenance.md)
adds policy acceptance and automatic refresh of machine-maintained handovers.
The proposal-first generation described below remains explicit-mode behavior;
it is not a mandatory human approval workflow.

## Source

[Canonical memory ADR](../adr/0005-canonical-claims-and-time.md), the accepted
[memory forms and Brain vision](../foundation/vision.md), [claims](memory-claims-and-time.md),
[review](memory-review-and-corrections.md), [retention](memory-retention-and-erasure.md)
and [model gateway](memory-provider-policy-and-learning.md). The full implementation
goal authorizes these routine details within the accepted baseline.

## Contract

### One memory lifecycle

Extend canonical claim revisions with kinds `procedure` and `handover`. Existing
`claim` and `decision` records and APIs remain usable. Optional typed
`procedure`/`handover` details are absent on older records and prohibited on other
kinds. Authoring, stale-base checks, device operation scope, human review/correction,
rejection rules, withdrawal, fact/knowledge time, retention and Erase use existing
memory command and eligibility paths. No parallel acceptance or execution authority
is introduced. Model-generated handovers start proposed with actual actor/device
and model provenance, no reviewer, unknown fact time and declared operational state.

Procedures have a title in subject, purpose/summary in value, conditions (up to
2,000 characters), 1–20 ordered steps (1,000 each), expected outcome (2,000) and
0–20 observations. Every observation records success or failure, observation time,
tested conditions (2,000), result description (2,000) and 1–20 supporting evidence
IDs drawn from the revision's exact supports. The revision's repository, area,
environment and optional manifest define each observation's applicability. A test
in another environment belongs in another scoped revision/record. Empty observations
are explicitly **Untested**. Neither authored steps nor recorded success execute
commands or authorize tools. Strict operational eligibility additionally requires a
latest successful observation; failure/untested procedures remain inspectable.
Existing operational fields remain attributed assessments, not service attestations.

Handovers have a title/summary plus completed work, next steps and risks/open questions,
each a list of at most 12 nonempty strings up to 1,000 characters. They preserve
1–12 distinct exact contributing claim revision IDs. Inputs must be current,
investigation-eligible claims, decisions or procedures in the same Brain; handovers
cannot contribute to another handover. Direct self-reference fails. Every input
must have the same nullable environment. Output repositories/areas are exactly the
union of contributor selections, retaining each contributor's original selection,
manifest, validity and trust state in inspection. This does not infer a graph path,
deployment, shared Git revision or continuous validity. Different environments require
separate handovers. A handover always has declared operational state.

All contributors' exact evidence supports must be present in the handover, retaining
line spans. Additional manual supports are allowed within the existing 20-support
limit. Conflicting spans for the same evidence ID fail instead of dropping provenance.
An optional output manifest must satisfy existing evidence/manifest validation.
Manual handovers can be written without a model or approved transmission.

### Contribution eligibility and removal

Persist contribution links as Brain-scoped foreign keys alongside each canonical
revision. Inspection returns each exact contributor's permitted revision and
eligibility or an unavailable marker. Contributor states are never flattened into
the handover's accepted status. For selected knowledge time, changes after the
contributing revision qualify freshness as needing verification. Rejected, withdrawn,
rule-blocked, erased or otherwise ineligible contributors exclude the handover from
ordinary investigation; missing/expired inputs and conflicts prevent strict use.
Strict accepted handovers additionally require every contributor to be strictly
accepted. Current access, erasure and rejection rules apply to historical reads.
Contribution checks are bounded and do not recursively traverse handovers.

Explicit erasure of any contribution removes the dependent handover revision and
its review/rule payloads, even if original evidence is independently retained.
Ordinary contributor expiry qualifies the durable handover and prevents strict/model
use; it does not retain expired contributor text in an inspection.
Handovers follow claim retention. When erasing a handover, fence its known contributing
claim revisions from further model transmission while keeping independent manual
evidence readable. Include opaque input and expanded dependent revision IDs in the
erasure journal. Replay into an older database applies these even if the erased
handover was created after that backup. Old entries without the new field remain
valid. No semantic matching of unrelated reuploads is promised.

### Governed generation

`POST /api/brains/{brain}/handovers` queues one attempt with title, exact contributing
revision IDs and optional write operation. The server derives combined scope and
exact evidence; devices require an active write operation with that scope. Browser
writers may use explicit derived scope. Reads require Brain read access; archived
Brains allow reads but deny new generation/writes.

The synthesis call uses the installed gateway under enabled `synthesis` and `claim`
permissions, plus `query` for the supplied title. Shared input/output, daily-token,
concurrency, timeout and body limits apply. No source/file bodies or tools are added.
A fixed strict schema permits only summary and the three text lists; model output
cannot select contributors, scope, review or execution. The worker checks current
source revisions, canonical eligibility, policy, access, operation and lease before
transmission and publication. Changed input or authority discards the response.
The result is frozen as a typed draft and independently assessed under the
[source-support contract](memory-source-support-verification.md), with `extraction`,
`claim` and original evidence-class permissions. This separate support call receives
exact permitted original source windows and contributor applicability, not tools.
Only a supported draft passes canonical validation, rule/conflict checks, audit and
refresh in the same transaction as publication. Saved draft/verdict recovery never
repeats a completed synthesis; unsupported refreshes preserve the previous head.

`GET /handovers` pages 20 attempts with safe status, input/result identities, model
request, timestamps and failure code. States are queued, running, succeeded, failed,
cancelled or removed. Bounded support inspection adds stage state, disposition,
reason, verifier version and the separate assessment receipt/state/error. Logical
expiry or erasure hides the reason and disposition before cleanup. Failed malformed
assessments expose their safe receipt without inventing a semantic verdict.
Existing job controls cancel work; generic job retry cannot
repeat model generation. `POST /handovers/{run}/retry` creates a new explicit attempt
from still-current inputs and current policy, using a fresh device operation when
supplied. Idempotency replays the original command; ambiguous completed model calls
never silently resend. Restart before transmission resumes the lease; restart after
an uncommitted external response records unavailable/uncertain output and requires
a new attempt. Provider accounting remains separate from canonical publication.

Attempts retain identities/status; free-form title follows audit retention and is
removed with dependent erasure. No raw model response or independent content cache
is stored. Claim-owned provenance follows claim retention. Queued/in-flight work
is cancelled/suppressed on erasure and fenced after restore.

### Browser

`GET /api/brains/{brain}/handover-inputs/{revision}` resolves an exact permitted
non-handover contributor for inspection/editor initialization. It does not promote
a historical revision to current input; generation and writes still revalidate it.

The editor supports procedure and handover forms, scoped evidence, ordered steps
and success/failure observations. Contribution selection derives combined scope
and supports, with visible limits and incompatibility errors. Details show
tested/untested conditions, outcomes, contributor scopes/trust/provenance and existing
review/history/Erase. Filtering by memory kind retains time/mode/scope semantics.
Writers can request generated handovers from selected records. A bounded attempt
list exposes queued/running/results/failure/cancellation and explicit retry. Results
open the canonical proposal. Readers cannot mutate; foreign references, stale input,
policy denial, missing evidence, provider errors and mobile layouts have usable states.

## Acceptance

Use real PostgreSQL/API and controlled HTTP to prove authored outcomes,
multi-repository provenance/union and conflicting-environment denial; review,
temporal change, missing/expired evidence and contributor erasure; generation,
policy denial, changed input during a call, lease/restart/idempotency, rollback and
scoped device authority. Prove old-backup replay of the new erasure fence with an
unrelated positive control. Exercise browser authoring/review, generation/failure
and desktop/mobile. Make one bounded synthetic real-Luna handover through the
gateway. Run focused checks and `./scripts/validate.sh`, preserve normal local Brains
and reconcile docs before archival.

## Explicit Deferrals

Retrieval/ranking, graph paths, executable workflows, automatic skill learning and
session-derived handovers belong to their slices. Cross-environment and recursive
handover composition are outside this bounded initial form. No procedure execution
endpoint or tool permission is granted.
