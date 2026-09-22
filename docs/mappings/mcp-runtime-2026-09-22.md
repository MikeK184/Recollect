# MCP execution interfaces and lifecycle evidence

Observed: 2026-09-22
Confidence: current primary interfaces, real central/paired execution, desktop proof and verified local schema 022 upgrade.

## SDK and protocol

Context7 resolves official rmcp documentation as `/websites/rs_rmcp_rmcp`.
Its lifecycle/transport query confirms `ClientServiceExt`, `RunningService` and
custom Streamable HTTP clients. The [official rmcp 3.4.0 docs](https://docs.rs/rmcp/3.4.0/rmcp/)
and downloaded published crate source (`.cache/mcp-sdk/rmcp-3.4.0`) are inspected.
The SDK supports `ClientLifecycleMode::Auto`; `serve()` alone defaults to legacy
initialization. It uses Reqwest 0.13.2 while existing Recollect HTTP code uses 0.12;
keep the SDK adapter's compatible dependency explicit rather than assuming types
are interchangeable. The shared crate builds with rmcp 3.4.0, Reqwest 0.13.5 and
process-wrap 10.0.0 alongside the existing Reqwest 0.12 application client.

The [current transport overview](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports),
[stdio](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio),
[Streamable HTTP](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)
and [version compatibility](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)
describe modern per-request metadata and legacy session fallback. Do not treat
Cognee's initialized Python sessions as the current protocol's only lifecycle.
The old `basic/utilities/cancellation` URL failed; the linked current
[cancellation page](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/cancellation)
was accessible. Cancellation is best effort and does not establish effect rollback.

Source findings that affect the adapter:

- `service.rs` exposes per-request handles/options and bounded service close.
  Dropping a RunningService begins asynchronous cleanup; explicit close waits.
- `transport/child_process.rs` owns child cleanup, closes stdin, waits three
  seconds and kills/reaps when needed; stderr inherits by default and must be
  suppressed. It accepts maintained `process-wrap` command wrappers.
- Streamable HTTP defaults `reinit_on_expired_session` to true and may replay
  expired-session POSTs. Disable that for managed tool calls. High-level
  `call_tool` can automatically drive input-required rounds; use one explicit call.
- SDK SSE events have a configurable bound, but the built-in Reqwest JSON response
  path reads `response.bytes()` and the default async reader accumulates a line.
  Add narrow I/O bounds without rewriting protocol handling.
- Default protocol `LATEST` remains a legacy constant in this release; choose the
  SDK's explicit modern supported version for Auto probing and retain its fallback.
  This is wire compatibility, not a custom product version gate.

## Cognee and Atlas comparison

The previously user-requested read-only reference agent inspected these paths:

- Cognee `cognee-mcp/src/test_client.py:37`, deployment `mcp_harness.py:165`, and
  `tests/e2e/docker_compose/mcp_client.py:29` demonstrate useful real transport and
  call fixtures. Its local test copies the full environment; use Recollect's
  existing cleared environment pattern instead.
- Cognee `server.py:882` starts migrations before serving: failed initialization
  cannot establish a side-effect-free executable startup. Its in-memory background
  tasks and ten-second shutdown wait do not prove durable external completion.
- Cognee `cognee_client.py:82` uses one global configured token/client; this is not
  evidence of multi-principal credential/session isolation. Its pipeline recovery
  (`modules/cognify/recovery.py:17`) uses age and explicitly notes heartbeat/lease
  as a more precise alternative.
- Atlas `recoverable-background-work.md:204` and its MARM report warn that dropping
  a waiter can leave underlying work running. This is reference-reported evidence,
  not a reproduced test against that project.
- Recollect `worker.rs:57` has useful lease polling, but its general job failure
  requeue must not be inherited after an MCP side-effect dispatch. Existing device,
  scope and catalogue authority can be reused; a runner needs separate ownership.

No mature multi-user runtime coordinator was found in the inspected Cognee paths.
Use SDK transports and existing application authority; implement the required
Recollect-specific leases, dispatch disposition and credential/session isolation.
Both reference checkouts remain read-only.

## Local implementation and proof

`crates/mcp-runtime` implements SDK transports with bounded input/output, literal
stdio execution, an owned process-group supervisor, environment/OS-store binding
references, shared and exact-value redaction, and compatible instance leases.
Context7 `/watchexec/process-wrap` and its published 10.0.0 source confirm the
maintained group/KillOnDrop wrappers used by the supervisor.

`./scripts/test-mcp-runtime.sh` initially passed six real integration cases in
7.91 seconds (`.cache/mcp-runtime-transports.log`). The expanded review suite now
passes nine in 9.96 seconds (`.cache/mcp-runtime-review-proof.log`). Coverage includes modern/legacy stdio and
HTTP JSON/SSE calls, single startup under concurrency, session isolation, exact
15-minute useful-idle boundaries, overlapping leases, cancellation/abandoned waits,
credential generation changes, a uniquely owned native OS-store entry and cleanup,
bounded hostile output, lost response without replay, executor death, child reaping
and continued hosted HTTP health. The fixture uses generated transient credentials;
no real credential values are printed or stored in fixture configuration. Legacy
stdio proof explicitly rejects the modern probe before entering the SDK's legacy
server lifecycle. A protocol-error response from a modern server alone did not
exercise that fallback, and the initial fixture was corrected accordingly.

The requested reference agent found seven concrete runtime issues. The fixes add
an outer send/wait deadline independent of SDK cancellation delivery, an OS parent
monitor independent of a blocked protocol pipe, exact scalar redaction, ownership
accounting throughout transport close, modern tool-list subscriptions, explicit
protocol-rejection dispositions and additional loader/routing exclusions. The
expanded cases exercise those boundaries with actual transports; a focused shared
scalar-redaction test also passes. Context7 confirms `listen_with_capacity` and
separate modern notification consumption; the published SDK's subscription tests
provide the fixture pattern. Closing failures retain a quarantined capacity slot.

Migration 022 and `mcp/runtime` now implement durable admission, scoped Use checks,
request identity, runner/attempt fences, pre-send transitions, cancellation,
short payload retention and late receipts. Three real PostgreSQL/API/RLS cases
pass in 2.67 seconds (`.cache/mcp-coordinator-state-proof.log`), including paired
local routing and refusal to fall back to the central runner. These coordinator
tests control SDK outcomes; they do not prove the combined execution path yet.
The first run exposed missing registration in the explicit migration list;
registration was fixed before the passing run. An earlier five-case catalogue
regression run still used schema 021 and is not evidence for migration 022.

The combined executor is now wired into the server's `serve` command and native
`mcp-runner`. Six coordinator/executor cases passed in 26.85 seconds
(`.cache/mcp-native-proof.log`); the expanded four execution cases passed in 30.91
seconds (`.cache/mcp-http-production-proof.log`). Both central and actual paired
native executors made stdio and Streamable HTTP tool calls. The native proof used
a uniquely named OS-store device entry and removed it afterward. A lost effect
response produced unknown exactly once, its approved receipt lookup preserved
that original disposition, and executor restart uploaded a stored receipt without
another tool call. Session release, scope identity, compatible reuse and owned
shutdown were checked. An initial session-release test used the wrong plural
route; the native client and fixture now use the registered singular route.

Context7 `/websites/rs_rusqlite_rusqlite` confirmed busy timeouts, explicit
transactions and batch pragmas. The receipt outbox reuses pinned rusqlite 0.32.1
with private files, exact owner identity, bounded blocking work, immediate
transactions, full sync and secure deletion. Its restart/conflict/expiry/capacity
unit proof passes; receipt quarantine remains counted within the same bound.

Two additional database recovery cases passed in 2.24 seconds
(`.cache/mcp-recovery-proof.log`): 32-call admission capacity with permitted replay,
pre-dispatch deferral/fencing, queue expiry, lost pre-send acknowledgement, retained
source-backed resolution, concurrent resolution-ID conflicts and immediate masking
after actual source erasure. Explanation replay now checks current source
availability before maintenance as well as its own deadline.

Two desktop flows passed in 29.5 seconds (`.cache/mcp-desktop-runtime-proof.log`)
against the production server and actual SDK fixture: lost admission response,
single effect and distinct receipt lookup, evidence resolution/erasure, cancellation,
output expiry and Use revocation. Screenshots were visually inspected. The result
view now prioritizes the connector's structured/text result and keeps the complete
protocol response behind a disclosure. A final affected catalogue/runtime rerun
also waits for the evidence save to finish before capturing the resolution screen.
No mobile work was added. The generated schema contains 151 unique operations.

The final affected desktop rerun passed all four runtime/catalogue flows in 55.9
seconds (`.cache/mcp-desktop-final-proof.log`); completed-resolution and result
screenshots were inspected again. Workspace checks passed 16 tests and Clippy
passed in 8.29 seconds. An initial capture-hook run took 1.246 seconds while builds
overlapped; the unchanged check passed in isolation at 716 ms and in the serial
workspace run. A subsequent cold rebuild reproduced the delay at 1.207 seconds,
so that isolated pass did not resolve the regression. The MCP executor/supervisor
now lives in a sibling `recollect-mcp-runner` executable; the public companion
command executes it directly. This reduced the per-event agent from 59 to 31 MB,
and the first hook invocation after rebuilding completed in 535 ms
(`.cache/mcp-capture-helper-cold-proof.log`). Its one-second assertion was retained.
The final nine real shared
SDK cases passed in 9.93 seconds (`.cache/mcp-runtime-transports-final.log`).

A final executor review found that biased polling of a five-second heartbeat
before cleanup could starve shutdown when the control request itself took five
seconds. Completed work and stop signals now precede renewal; failed runners no
longer renew while draining. A paused-clock regression passes in 0.05 seconds
(`.cache/mcp-heartbeat-cleanup-proof.log`). Context7 `/tokio-rs/tokio` confirmed
biased select ordering and test-only clock utilities. Central startup checks its
private outbox before advertising a runner lease. Updated affected checks follow
these final changes before normal migration and closeout.

The final reference review identified two additional recovery gaps: reliable
protocol-failure receipts were fenced after recovery, and a still-live runner
could register a provider for an expired startup attempt. The coordinator now
keeps unknown plus an attributed supported failure receipt, and checks both
attempt lease and deadline before instance admission in the handler and narrow
SQL function. All six combined execution cases pass in 95.60 seconds
(`.cache/mcp-recovery-final-review-proof.log`), including both native helper
transports, held protocol-failure receipts uploaded twice after restart with one
resolution and one provider call, and expired lease/deadline attempts with zero
provider starts followed by a successful fresh attempt. The initial new test
needed a Rust ownership correction before this passing run. The remaining ten
catalogue/coordinator/recovery cases pass in 8.73 seconds
(`.cache/mcp-coordinator-final-proof.log`).

The final workspace run passes 17 tests; Clippy passes in 28.97 seconds and
governance passes all 32 tests in 1.355 seconds. Database and SDK cases are ignored
by the plain workspace command and have the separate actual proofs above.
The first post-rebuild hook invocation remains below the unchanged one-second
assertion. `./scripts/dev.sh` also rebuilds the desktop UI and generated API.

The normal service is now healthy at schema 022 with one live central runner.
The first preservation recheck detected a newly created `test` profile, so a
fresh private `pre-022-current.dump` and exact profile/grant inventory were taken
before migration; the original pre-022 snapshot remains separate historical
evidence. All seven Brain IDs, that profile and its grants, 33 model requests and
zero active jobs are preserved. There are no approved definitions, connections,
calls or provider instances in normal data. The authenticated desktop check and
exact SWEG recall returned one retained context item in 53 ms with no semantic or
graph request; no new paid model call or customer repository read occurred.
The normal desktop screenshot was visually inspected with the existing profile
and empty runtime states. Evidence is in `.cache/mcp-runtime-normal-proof/`.

Slice 23 is delivered locally under the accepted
[runtime contract](../contracts/mcp-runtime-and-credentials.md).
