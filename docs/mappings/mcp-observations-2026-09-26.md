# Managed observation reuse and interface evidence

Observed: 2026-09-26
Confidence: actual local connector/database/learning/privacy, desktop and normal
upgrade proof. No external deployment or universal host coverage is claimed.

CodeGraph `status`, `sync` and `node capture_source`, followed by direct source
checks, identify the existing ownership boundaries. `capture.rs` owns policy,
binding, replay and capture-event/source admission. `evidence::capture_source`
already writes canonical artifacts and schedules the bounded capture processing
lane. Model input provenance identifies `reported_tool_observation`, and automatic
learning inherits capture selection. Retrieval candidates and diversity lineage
join the canonical event/binding. These are the reused consumers exercised by
the integration evidence below.

`mcp/runtime/runner.rs` and migration 023 authenticate/fence completion separately
from provider dispatch. `mcp-runtime/outbox.rs` persists sanitized native receipts
with stable call IDs, 64-row capacity, secure SQLite deletion and a one-hour bound.
The central executor and paired/private clients share this boundary. Late receipts
append resolutions while leaving original unknown outcomes intact. Existing raw
runtime payloads also expire after one hour. New durable evidence publication must
not be inferred from this temporary receipt persistence.

Context7 `/modelcontextprotocol/rust-sdk` and the official
[Rust SDK README](https://github.com/modelcontextprotocol/rust-sdk/blob/main/README.md)
distinguish tool-reported failures from protocol errors. The SDK's
[model definitions](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/src/model.rs)
have text content, optional structured content and `is_error`. The current Context7
index advertises an older tagged example alongside main-branch snippets; installed
rmcp 3.4 source and actual existing transport fixtures remain the compatibility
authority. No dependency upgrade is required. SDK multi-round input negotiation
does not authorize replay of an unknown side effect.

Local Atlas `content/patterns/evidence-before-belief.md`, `zero-llm-capture.md`
and `recoverable-background-work.md` support retained provenance, capture without
a model dependency, stable retry identity and separate budgets for capture and
maintenance. They warn that durable input alone does not preserve corrections or
prevent maintenance starvation. Recollect reuses its rejection/privacy authority
and existing IDs; the user's explicit no-content-hash preference supersedes that
optional reference mechanism. Atlas is research, not repository authority.

The local Cognee `cognee-mcp/README.md` documents its memory-facing
remember/recall/forget tools, background ingestion status and selectable backend
connection modes. It does not establish Recollect's managed execution semantics.
The existing hook integration review remains in
[session capture research](session-capture-interfaces-2026-09-14.md). Both reference
checkouts were read without modification. Recollect keeps its independent Rust
canonical writer and pinned PostgreSQL/Neo4j engines.

## Local implementation and runtime proof

Migration 024 and `mcp/runtime/observations.rs` connect authenticated completion to
the existing canonical capture engine. The completion transaction records the
sanitized bounded event; a separate publication loop rechecks the original actor,
current grants, policy, scope, retention and deletion. Captured events are stable
per terminal call or reconciliation identity. Central, paired-local and registered
private execution share the same producer. Public host capture cannot forge it.

Five focused integration scenarios passed on September 26 using actual rmcp
fixture processes and HTTP coordinators, PostgreSQL and canonical artifacts:

| Path | Observed result |
| --- | --- |
| Results, filtering and reconciliation | Actual success/tool-error output reaches source/chunks/recall; pre-dispatch cancellation/failure and original unknown retain distinct outcomes; connector/evidence resolutions do not repeat the effect or copy arbitrary explanation |
| Durable publication | Artifact failure rolls back source publication, persists safe retry state and later publishes once; 1,000-record overflow preserves earlier entries; changed Use or expiry blocks publication |
| Policy and retention | Default-disabled calls are not imported later; full-envelope exclusions precede UTF-8 truncation; explicit retention extension changes published source/status deadlines from original capture time while pending deadlines remain fixed |
| Autonomous learning | A controlled HTTP model endpoint receives canonical managed call/target/scope provenance under the standing tool-output policy; learning creates a qualified claim without human review; capture itself makes zero model requests |
| Local/private scope and privacy | Delayed parent/child operations retain independent original areas despite changed task defaults; reader execution creates no knowledge write; source erasure clears native quarantined receipt bodies before resend and blocks late completion |
| Older-state recovery | Journal replay fences a pending outbox and an older database that predates the call; startup fails before replay, removed output stays absent afterward and independent evidence remains available |

The focused log is `.cache/mcp-observations-runtime-20260926.log`. Two actual
executor-restart scenarios also pass with managed capture enabled: later successful
receipts publish separately, and later protocol failure remains content-free.
Nine existing managed runtime/authority/reconciliation cases and five host capture,
standing learning, privacy and provenance cases pass. The sanitizer unit proves
exclusion beyond the truncation boundary, configured-secret removal, UTF-8 bounds
and the intersection of admitted/current capture policy. Logs use the same dated
`mcp-observations-` prefix for `late-receipts`, `runtime-regression`,
`host-regression` and `sanitizer` under ignored `.cache/`.

The first desktop pass exposed an exact-label selector mismatch. The next exposed
Escape closing both stacked dialogs. The implementation now disables the underlying
dialog's Escape/outside-click/focus ownership while the source viewer is open.
Context7's [Mantine Modal documentation](https://mantine.dev/core/modal/#control-behavior)
and the installed component source confirmed
these supported controls; the top source viewer retains its own focus and Escape
handling. The final four-case desktop run passed in 48.6 seconds, covering policy,
source/call/activity navigation, filtered and unavailable/retry states, cancellation,
expiry and Use revocation. The earlier build-overlapping run timed out waiting for
connectors; the final pass ran without concurrent compilation. Desktop screenshots
were inspected under `.cache/ui-managed-{source,call,activity}-desktop.png`.

Workspace tests passed 26 cases, with 149 explicitly ignored fixture cases;
all-target Clippy passed in 59.43 seconds. Generated API has 159 unique operations;
TypeScript/web build, governance lint/all 32 checker tests and whitespace checks
pass. The [archived pack](../roadmap/execution/archive/mcp-observation-capture.md)
records closeout. These are focused slice proofs; they do not substitute for the
remaining integrated workload/capacity evaluation.

After validation, the normal development API/worker drained and upgraded to
migration 024. Readiness is 200; before/after inventories confirm seven Brains,
33 model requests, no active jobs and unchanged profile/grant/connection records.
Existing managed capture remains disabled by default. Actual normal SWEG exact
recall returns one item, 1,630 bytes in 50 ms, with no new model request or customer
file read. The normal desktop has no browser errors. Proof, readable pre-024
database dump and final inventory are under `.cache/mcp-observations-normal-proof/`.
That dump is a scoped migration safeguard, not completion of the recovery slice.
The personal/shared installed proof image still predates migration 024 and was
left healthy; this slice does not claim those installations have been upgraded.
