# Procedures and handovers

## Purpose and Prerequisites

Start `./scripts/dev.sh`, sign in at `http://127.0.0.1:8787` and open a Brain.
Readers can inspect knowledge; writers can author records or request generation.
Automatic learning and refresh require the Brain's standing
[autonomous model policy](provider-learning.md). Manual forms need no model call.

Procedures record a method and its evidence. They do not run commands or grant
tool permission. Handovers combine exact current claims, decisions and procedures
while preserving their separate evidence, applicability and eligibility.

## Procedure

With autonomous memory enabled, importing permitted evidence can learn a procedure
directly. Its conditions, ordered steps and expected outcome are retained; an
unexecuted procedure remains **Untested**. Later evidence can revise machine-maintained
procedures without individual review. The browser's creation/editing form is an
optional way to supply or correct the same canonical memory.

1. In **Claims and decisions**, choose **New claim** and select Procedure. Supply
   conditions, one to twenty ordered steps, expected outcome and exact evidence.
   Record only actual success/failure observations with their time, conditions,
   result and supporting evidence. Scope selects the applicable environment and
   repository/manifest; a test elsewhere belongs in its own scoped record.
2. Select Handover to compose a manual summary with completed work, next steps and
   risks/open questions. Pick one to twelve current non-handover contributors.
   Their repositories/areas and exact support are combined. All contributors must
   share the same environment, including Brain-wide/no-environment scope.
3. Alternatively, use **Generated handovers → Generate handover**, supply a title
   and select contributors. This first request defines what the handover covers.
   Enabled synthesis, claim and query permissions allow the native model worker
   to generate it. The attempt list shows progress, safe failure and result links.
4. Inspect the result's history and exact contributing revisions. Under autonomous
   policy, supported output is accepted by policy and later contributor changes
   refresh that same handover automatically. Human review remains optional.

Inspection shows each contributor's original scope, trust, fact time and manifest.
Acceptance of the handover does not accept a disputed contributor or establish a
graph relationship. Strict operational procedures require actual successful
observations as well as ordinary eligibility; generated handovers remain declared.

## Verification

```sh
./scripts/test-platform.sh
./scripts/test-ui.sh tests/procedures.spec.ts
./scripts/validate.sh
```

The API/database proof includes multi-repository composition, conflicting-environment
denial, stale contributors, policy/access changes, cancellation and real older-backup
erasure replay. Browser proof covers forms and desktop/mobile history inspection.
The [dated evidence](../mappings/procedures-and-handovers-proof-2026-09-14.md) records
actual synthetic Luna generation and the normal-runtime autonomous demo.

## Failure and Recovery

Changing a contributor qualifies an old handover immediately, while refresh is
pending. Withdrawn, rejected or erased contributors exclude it from current use.
Expired evidence is unavailable and prevents strict/model use. A generated refresh
cannot overwrite a later human edit or publish after losing policy/access/lease.

Different environments, nested handovers and conflicting spans for the same
evidence ID are rejected. Use compatible exact inputs; selecting a handover does
not authorize cross-environment merging. If a contributor changed while editing,
reopen the form and select its current revision.

Job controls can cancel generation. Generic job retry cannot repeat a model call;
the handover's explicit retry action records a new attempt with current inputs.
Known transient automatic failures have bounded delayed retries; uncertain outcomes
require an explicit new attempt after diagnosis. See [model recovery](provider-learning.md).
Erasure cancels dependent generation and removes controlled dependent content through
the [deletion journal](retention-and-erasure.md); it is distinct from logical retirement.
