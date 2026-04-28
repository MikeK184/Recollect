# 0012: Managed tool observations as canonical capture

Status: accepted
Date: 2026-09-26

## Decision

Use the existing managed-call receipt protocol as the authenticated producer for
automatic tool observations. A standing Brain capture setting explicitly enables
this producer. Freeze consent, target attribution and scope at call admission;
apply current permission, policy, retention and deletion again before publication.
Reuse canonical capture events, source artifacts, processing, learning and privacy
fences. Do not build another memory engine or require per-observation human review.

Record terminal outcomes and later reconciliation separately. Publish evidence
asynchronously from a durable, bounded outbox whose writes accompany accepted
completion receipts. Receipt and evidence retries cannot dispatch provider work.
Unknown execution remains unknown even when a later receipt reports success.
Captured tool text is reported evidence; it does not confer operational authority.

## Why

The [foundation](../foundation/vision.md#mcp-coordinator-and-vault-integration)
requires attributable managed observations under immutable scope. Existing host
hooks deliberately exclude Recollect-derived context and cannot replace a
coordinator's actual target, runner and outcome records. The runtime already owns
credential redaction, fenced completion and a durable local receipt outbox. The
capture engine already owns source retention, scoped learning, erasure and replay.
The [reference mapping](../mappings/mcp-observations-2026-09-26.md) supports reuse of
those boundaries rather than copying Cognee's runtime or interpreting host text as
an independently verified deployment.

## Consequences

Managed capture is disabled initially, including in previously enabled host-capture
policies. Once enabled it is automatic for eligible future calls. Enabling it
explicitly permits publishing sanitized results to Brain knowledge readers; it
does not grant execution, provider transmission or another person's profile Use.
Readers with execution permission may still run tools without publishing knowledge.

Browser calls without a task retain their explicitly selected Brain/environment;
they never receive an invented task or the next session's default scope. Managed
capture bindings therefore allow an absent operation/device only for this internal
producer. Existing host bindings retain their required device/operation and cannot
forge the managed producer. Actual receipt delivery has the runtime's existing
one-hour offline bound; a missing receipt becomes explicit missing coverage,
not fabricated output. Once admitted centrally, pending permitted evidence retains
the original tool-output deadline through publication failures.

The [contract](../contracts/mcp-observation-capture.md) governs publication,
shared-knowledge consent, attribution, reconciliation, deletion and proof.

## Supersession

N/A: new producer. ADRs 0008–0010 and the capture/lifecycle decisions remain in
force; no automatic effect retry, version handshake or content hashing is added.
