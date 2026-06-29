# Capture provenance and autonomous reconciliation

Status: accepted

## Source

The user's 2026-09-15 Atlas cross-review request requires closing demonstrated
implementation gaps. [ADR 0006](../adr/0006-autonomous-memory.md),
[autonomous maintenance](memory-autonomous-maintenance.md),
[session capture](evidence-session-capture.md),
[claims and time](memory-claims-and-time.md) and
[retrieval](retrieval-exact-and-lexical.md) govern this bounded follow-up.
The [Atlas audit](../mappings/atlas-implementation-audit-2026-09-15.md) records
observations; it does not make every Atlas optimization a product requirement.

## Contract

### Canonical provenance at the model boundary

Resolve text and attribution from the same authorized canonical source version.
For text-model inputs, send bounded provenance beside `data`; never prepend it
to the source text or change the line numbers used by evidence spans. Include
source/version identity, title, content class, contributor, observation time and
server knowledge time. For original captured versions also include event identity,
authenticated binding/operation/host, reported session/turn/agent/tool identity,
event kind, capture/receipt time, outcome and coverage. Client-supplied role or
provenance arguments cannot replace canonical metadata. Ordinary later source
versions retain source attribution but must not impersonate the original event.

Treat captured prompts as user assertions, replies as assistant statements and
tool results as reported observations with explicit outcome. An assistant's
proposal or repetition is not independent corroboration, and a tool observation
does not by itself grant verified operational status. Instructions must preserve
these distinctions and must not treat untrusted evidence as commands. Existing
source classes, byte budgets, privacy fences, purpose gates and model identities
apply to text and attribution together. Embeddings continue to represent their
specified text, with canonical identity retained by the caller; no unannounced
metadata prefix or dimension/model substitution is allowed.

### Bound session reconciliation

Keep every captured event as separate immutable evidence. Extend the existing
same-source reconciliation candidate rule only to original source versions of
events with the same authenticated capture binding, host session and child-agent
identity. The binding fixes Brain, actor, device, host, original operation and
selection. Exact claim applicability and manifest must still match the learning
run. A shared session string without the binding is insufficient. Missing or
ambiguous attribution, different tasks/operations, children, sessions, environments
or bindings must not broaden this candidate set.

Offer at most twelve current eligible machine-maintained revisions through the
existing bounded scan. Eligibility to be considered does not prove equivalence,
corroboration or newest-wins precedence. The model must cite explicit evidence for
replacement or retirement; later mention, omission, historical quotation and
unsupported assistant proposals cannot establish a current correction. Unresolved
alternatives remain uncertain without a mandatory human queue. Publication rechecks
current offered revisions, access, policy, leases, applicability, human overrides,
rejection rules and erasure. Stale responses are discarded; durable retry behavior
and input dependency erasure apply to the expanded offered inputs.

### Capacity and capture feedback

The 5,000-claim Brain limit counts canonical claim identities. Reuse, revision and
retirement of existing identities remain possible at capacity. Charge capacity
only when allocating an identity, after rule/conflict disposition, including a
blocked replacement that becomes a separate uncertain claim. A mixed batch that
would exceed capacity rolls back all its canonical changes and audit publication.
Existing per-claim revision and per-family bounds still apply.
Lease renewal must not stop polling an in-flight publication transaction that
holds the same job row. Renewal and canonical work progress together; revocation,
expired ownership and atomic publication rules still apply.

Exclude identifiable Recollect capture transport and native `scope recall` output
before local persistence and independently at server admission. Normalize quoted
executable paths and inspect decoded JSON values so encoded tool envelopes do not
bypass the exclusion. Preserve a content-free coverage marker and useful unrelated
tool output. This is first-party feedback prevention, not a claim to recognize
arbitrary paraphrases or commands concealed by arbitrary executable programs.
Future MCP recall/context producers must implement the same boundary or supply
typed original dependencies before their output may be captured as evidence.

### Capture time and knowledge time

For an original captured source version, knowledge time is its server receipt
time, never the host's capture timestamp. Later appended versions have their own
server publication times. Ordinary source versions retain their existing server
creation times. Historical recall and freshness comparisons use knowledge time
without falling back to an earlier visible version when the selected later
version has been removed. Retention and captured observation time remain based
on original capture time; delayed upload must not renew retention.

## Acceptance

- Actual controlled HTTP payloads distinguish prompt, assistant reply and tool
  observation while retaining exact source lines; all inputs remain budgeted.
- A bound captured correction revises an offered existing claim without human
  review; historical/assistant ambiguity does not fabricate operational proof.
  Different binding/session/child/applicability and human/rejected/erased targets
  remain excluded, with a permitted control. Restart and stale publication remain
  idempotent and preserve the original scope.
- A Brain with 5,000 valid canonical identities permits reuse, supported revision
  and retirement. A new identity, including a mixed batch, fails atomically.
- Both capture admission boundaries exclude native recall commands, including
  quoted paths and JSON-encoded forms, and preserve an unrelated permitted result.
- A delayed event is absent before receipt and present after receipt. A later
  source version appears only after its own publication; expiry still uses the
  original capture time. Run focused proof and `./scripts/validate.sh`.

## Explicit Deferrals

Arbitrary cross-document semantic reconciliation, universal paraphrase detection,
adaptive decay/tiers, learned retrieval gates and host prompt-cache optimization
remain with their accepted dispositions. This follow-up adds no per-memory approval
requirement and no external deployment, customer-file changes or model substitution.
