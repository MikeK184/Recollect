# MCP Coordination, Vault and Private Runners

Status: complete

## Purpose

Expose Recollect memory/workspace operations to coding agents and coordinate
their authorized MCP tools per Brain/environment, including Vault credential
delivery, local/central/private execution and safe process lifecycle.

## Governing Sources

- [Managed observation ADR](../../adr/0012-managed-tool-observations.md)
- [Automatic managed observation capture](../../contracts/mcp-observation-capture.md)
- [Agent memory MCP ADR](../../adr/0010-agent-memory-mcp.md)
- [Agent memory/workspace tools](../../contracts/mcp-memory-and-workspace-tools.md)
- [Vault/private ADR](../../adr/0009-vault-and-private-execution.md)
- [Vault/private contract](../../contracts/mcp-vault-and-private-runners.md)
- [Managed runtime ADR](../../adr/0008-managed-mcp-runtime.md)
- [Runtime and credential contract](../../contracts/mcp-runtime-and-credentials.md)
- [Approved catalogue and profile permissions](../../contracts/mcp-catalogue-and-profiles.md)
- [Vision: MCP coordinator and Vault](../../foundation/vision.md#mcp-coordinator-and-vault-integration)
- [Vision: access and execution placement](../../foundation/vision.md#deployment-and-access-model)
- [Stack: identity, workspace and coordination](../../foundation/techstack.md#identity-workspace-and-mcp-coordination)
- [Engineering principles: process management](../../foundation/engineering-principles.md#separate-tool-selection-from-process-management)
- [Engineering principles: credentials](../../foundation/engineering-principles.md#minimize-captured-source-and-credentials)

## Dependencies and Boundaries

Consume platform identities/grant primitives, paired devices and workspace scope.
Own approved MCP definitions, connections, named execution profiles, independent
use/manage/share grants, catalogue/configuration UI and runtime state.
Definitions/configuration/discovery are distinct from successful live calls.

Own rmcp client/server adapters, local stdio and remote Streamable HTTP routing,
startup locks, leases, compatible instance reuse and the 15-minute useful-idle
shutdown rule. Runner targets, executables/images and credential references stay
outside model-controlled arguments. External hosted processes are not ours to stop.

Own the credential-provider boundary and Vault integration, including delivery,
renewal, rotation and draining. OS-store/environment and anonymous alternatives
can coexist with Vault within one Brain; Vault is optional for each connection.
Delivered Vault support remains first-usable scope. Private runners use authenticated
outbound connections and enforce grants at dispatch and execution. Do not equate
revocation with automatic cancellation or invalidate already-issued credentials
without an explicit supported mechanism.

Memory/workspace tools call existing application handlers, and managed observations
enter evidence capture with actual target, operation, outcome and scope. Reading
production knowledge or retrieving a runbook does not grant production execution.
No generic shell sandbox, unrestricted arbitrary executable registration or
universal catalogue of customer connectors is introduced.

## Decisions Before Implementation

- Specify definition/connection/profile ownership, approved schema discovery,
  permission evaluation and runtime configuration before the catalogue slice.
- Specify transport authentication, startup/reuse identity, leases, cancellation,
  idle behavior and unknown-completion reconciliation before execution slices.
- Specify Vault auth/paths/credential delivery and rotation with explicit test
  targets; define private-runner trust and supported connector fixtures. Never
  embed credentials in contracts, model arguments or captured output.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `mcp-catalogue-and-profiles` | shipped | contract-backed | pack | Operator-approved cached definitions, Brain/environment connections, independent profile use/manage/share and desktop catalogue with real RLS/browser proof |
| `mcp-runtime-and-credentials` | shipped | adr-backed, contract-backed | pack | Actual central/paired stdio and HTTP calls, isolated credentials, fenced recovery without effect replay, desktop/native workflow and verified local schema 022 upgrade |
| `mcp-vault-and-private-runners` | shipped | adr-backed, contract-backed | pack | Optional per-connection Vault Proxy delivery, real issuance/renewal/rotation, authenticated private stdio/HTTP/native/desktop execution and preserved normal migration 023 |
| `mcp-memory-and-workspace-tools` | shipped | adr-backed, contract-backed | pack | Stateless paired MCP, native bridge, scoped memory/graph/handover tools and fresh capture defaults proven through real coding hosts |
| `mcp-observation-capture` | shipped | adr-backed, contract-backed | pack | Actual central/local/private receipts become scoped canonical evidence; automatic learning, separate reconciliation, retries, native erasure synchronization, older-state replay and desktop state verified; normal migration 024 preserved |

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `mcp-catalogue-and-profiles` | `platform-durable-work`, `evidence-workspace-scope` |
| `mcp-runtime-and-credentials` | `mcp-catalogue-and-profiles` |
| `mcp-vault-and-private-runners` | `mcp-runtime-and-credentials` |
| `mcp-memory-and-workspace-tools` | `mcp-runtime-and-credentials`, `retrieval-graph-fusion`, `memory-procedures-and-handovers`, `evidence-session-capture` |
| `mcp-observation-capture` | `mcp-vault-and-private-runners`, `mcp-memory-and-workspace-tools` |

## Completion Criteria

- Discovery lists authorized capabilities without starting dormant backends;
  knowledge access never implies execution, including for administrators.
- Actual local, central and private calls select the approved target/credential
  configuration. Model argument substitution and revoked new calls are denied.
- Concurrent startup starts one compatible instance; active/long operations
  survive other sessions ending and idle timers. Only owned processes are stopped.
- Vault credential rotation and runner interruption exercise renewal/draining
  behavior without leaking values to model context, logs or retained captures.
- Unknown side-effect completion is reconciled instead of blindly retried.
  Captured success/failure preserves what was actually observed.
- Codex/Claude memory/workspace integration refreshes applicable context after
  scope changes without relabeling in-flight tasks or inventing missing events.
