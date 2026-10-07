# Native plugin workflow, learning recovery and visible limits

Observed: 2026-10-07
Confidence: verified

## Sources and Method

- Actual DLAG native Codex 0.160.1 session capture through its trusted hooks;
  the proof used a local synthetic model response and an explicitly labeled
  prompt/reply, not a paid model or customer infrastructure action.
- Canonical learning/model request metadata and bounded worker diagnostics;
  customer content, credential values and provider response bodies were not emitted.
- Repository-owned PostgreSQL and deterministic HTTP provider fixtures, the
  native stdio MCP bridge, CUA browser checks, normal Compose build/readiness,
  web type/design/build and repository governance checks.
- Current source, accepted contract amendments and the archived
  [execution pack](../roadmap/execution/archive/mcp-plugin-workflow-repair.md).

## Observations

### Supported evidence workflow

The old catalogue lacked document discovery/import/content tools. Agents attempted
undocumented HTTP/credential discovery and guessed obsolete repository paths.
The canonical catalogue now supplies `source.list`, `source.import` and
`source.inspect`. The native plugin advertises 26 tools including its local
`workspace.refresh`; its bundled skill uses returned IDs and citation spans.
Import commands derive exact applicability from an owned write operation. Scoped
list/content/recall reads, automatic learning and later imported versions preserve
that applicability. Replay uses a stable command ID without widening authority.

The installed stdio plugin initialized successfully, advertised all three source
tools, began an owned read task, found and inspected an existing retained DLAG
source with citation spans, and closed the verification task. No implementation
path lookup or private credential-file lookup was needed.

### Capture, roster and host identity

A fresh ordinary native Codex session delivered a retained prompt and reply into
DLAG at 08:00 UTC. Lifecycle events were accepted without retaining their text.
This establishes automatic Codex capture independently of model-issued memory
calls. OpenCode connectivity was verified; its automatic prompt/reply delivery
was not rerun for this workspace.

The previous roster ignored successful memory/workspace/source-only use and
preferred a reused credential's stale OpenCode label. Successful authorized
Brain device requests now record payload-free usage metadata. Actually reported
hosts and accepted capture hosts take precedence over setup hints. Shared
credentials can show several observed hosts, without inventing absent historical
telemetry. DLAG's Brain roster shows actual Codex use; the account roster can
also show OpenCode use of the same credential in another Brain.

### Learning failures and budgets

All thirteen original DLAG imported documents retained and processed successfully.
Their first learning pass succeeded for ten and failed for three: incomplete
output at a 1,024-token output limit, malformed structured output, and an invalid
generated candidate. Reprocessing/capture later overflowed a 16 KiB input allowance
when whole existing reconciliation views were added without a remaining-byte check.
Source storage, automatic session capture and model learning are separate stages;
a failed learning run does not erase its source or earlier successful memory.

New policies default to 4,096 output tokens. Existing policies change only through
the authorized save command; DLAG was saved at 4,096 through the real UI. Generated
invalid candidates are classified as result-shape failures and cannot publish.
Known incomplete/malformed completed results receive at most two separately
accounted replacements, after five/thirty minutes, with current source/policy
fences. Unknown transport completion is never silently repeated.

Reconciliation now includes only whole eligible revisions that fit after the full
primary source and provenance. An oversized primary input has a distinct
`model_input_too_large` error. The owned fixture proved optional inputs are omitted
without truncating evidence and learning can complete within the approved budget.

At the live observation, DLAG had approximately 94,630 of its 100,000 daily tokens
accounted for. The remaining allowance could not admit a full conservative learning
reservation. The cap was preserved. Budget rejection makes no provider call;
automatic learning may resume after the next UTC reset within the same two-retry
ceiling. The original current-policy budget failures were not rewritten or falsely
marked successful. Provider quality and future paid completion remain unproven.

### UI and bounded retrieval

Settings → AI permissions shows all four Processing limits beside Selected models:
output tokens, input bytes, daily allowance and concurrent calls. They participate
in the main Edit/Save/Cancel command. The running browser saved 4,096 output tokens;
a canceled 3,072-token draft left 4,096 unchanged. Non-editing values remain readable.

My agents → Access tokens uses compact rows with host, credential state, dates,
copyable identifier and revoke action. Search/no-results, current/history filters
and opening/canceling the existing revoke confirmation were verified without
revoking a live credential. Local proof images are in the ignored
`.cache/plugin-workflow-ui/` directory.

A broader model regression exposed a fresh-import semantic query choosing the
pending-work index once per candidate before refreshed statistics. The unique
representation lookup now precedes readiness filtering, retaining the 5,000-item
bound and two-second read deadline. Oversized scope is refused before a provider
call and a narrow collection remains usable. Import applicability is resolved
once per version before chunk fan-out. No timeout or permission limit was relaxed.

## Translation and Limits

This is locally implemented and deployed Compose evidence, with uncommitted source.
It does not establish customer operational correctness, universal model quality or
an OpenCode automatic-capture proof. Historical failed runs remain visible.

The newly built macOS binary blocked on existing Keychain trust. The previous
trusted installed runtime was restored, preserving connectivity and automatic
capture. Updated skills and the server-supplied 26-tool catalogue work with that
runtime and were verified directly. No Keychain ACL, credential or hook trust was
changed. The new native host-header implementation is validated in code/fixtures,
but full local acceptance of that new binary remains outside this proof.

## Follow-up

Restart existing coding-host sessions to reload the skill and tool catalogue.
Inspect eligible learning after the UTC reset, or explicitly adjust the visible
daily allowance. Do not infer successful recovery merely from a queued replacement.
A future native runtime update must preserve or re-establish normal OS trust.
