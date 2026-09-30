# Separated agent and tool wiring authority

Status: planned
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Make Agents the single place a person connects a coding tool, make Connections own only outbound Brain-managed MCP, explain where-a-connection-runs in plain language, and present captured sessions and working contexts as evidence of a working connection rather than as setup forms.
- Non-goals: No change to device authority, grant evaluation, profile permissions, credential handling, runner trust, capture semantics or any MCP wire contract. No new connector, host, OAuth flow or plugin packaging.
- Delivery shape: React presentation and route-resolution change across the agents and connections features plus owner-facing copy; focused browser and authority proof; reconciled epic and execution indexes.

## Governing Sources

- [ADR 0017: Separated wiring](../../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Tiered desktop surface contract](../../../contracts/desktop-knowledge-surface.md)
- [Contextual desktop experience](../../../contracts/desktop-experience.md)
- [Approved MCP catalogue and execution profiles](../../../contracts/mcp-catalogue-and-profiles.md)
- [Managed MCP calls and credentials](../../../contracts/mcp-runtime-and-credentials.md)
- [Vault credentials and private runners](../../../contracts/mcp-vault-and-private-runners.md)
- [Agent memory and workspace MCP tools](../../../contracts/mcp-memory-and-workspace-tools.md)
- [Plugin/MCP direct user authentication](../../../contracts/mcp-plugin-direct-auth.md)
- [Companion pairing and device authority](../../../contracts/platform-device-pairing.md)
- [Automatic host session capture](../../../contracts/evidence-session-capture.md)
- [Vision: MCP coordinator and Vault integration](../../../foundation/vision.md#mcp-coordinator-and-vault-integration)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Removing the Coding agents view from Connections and resolving its legacy state to Agents; promoting Agents within the Wiring tier; one complete connect flow covering host choice, pairing where required, Brain binding, workspace configuration, host setup and a real memory-read check; captured sessions and working contexts as Agents views; plain-language where-it-runs explanation for central, paired-device and private-network placement; consistent tool-group naming with independent Use, Manage and Share explained in one place; capture coverage and gap reporting that links to the single canonical Settings editor.
- Out of scope: Any change to device credential issuance, revocation or normalized-name dedupe, delivered by the shipped [successor cleanup](../archive/mcp-successor-cleanup.md) and [direct-auth repair](../archive/mcp-direct-auth-repair.md) slices. OAuth, published plugin distribution, arbitrary executable approval, new connectors or capture hooks. Devices list ordering and density, owned by `desktop-knowledge-surface`.
- Blockers: None. [ADR 0017](../../../adr/0017-desktop-knowledge-and-ask-experience.md) resolves the desktop-contract versus managed-experience placement conflict, and the managed contract carries the amendment note.

## Surface and Interface Changes

- Interfaces: No API or wire change. Route resolution changes: legacy Connections coding-agent state resolves to the Agents destination instead of rendering a second setup path.
- Storage: N/A: no schema or migration. Existing task scope, capture binding, connection, profile and grant records are unchanged.
- Ownership: MCP coordination owns outbound connection, profile, grant, runner and runtime presentation. Evidence owns session and capture semantics. Platform owns device identity and the global Devices destination. This slice moves presentation authority only and forks no handler.

## Data and Authority

- Inputs: Supported host catalogue state, paired device records, Brain grants, existing MCP definitions, connections, profiles, grants and runners, captured session and event reads, task scope records, installed-provider and capture policy status.
- Authority: Device bearer credentials continue to act as their user with current Brain permissions. Reading knowledge never implies execution, and profile Use remains separately granted. A rendered setup card, saved configuration or registered runner is not evidence of a live connection.
- Blind spots: Whether an agent is currently running is knowable only from real server observations; without them the product must say configured rather than connected. Capture completeness is never claimable from host hooks alone. Offline devices cannot be reported as having applied any change.

## States and Edge Cases

- Loading: Each view shows its own skeleton; the connection test result area stays pending until the call returns.
- Empty: Connections keeps its actionable empty state and the exact operator prerequisite for non-owner admins who cannot import a connector definition.
- Error: A failed memory-read check reports the actual failure and the remediation step; a failed or uncertain tool outcome keeps its existing reconciliation semantics and is never auto-retried.
- Blocked: Missing connector approval, missing credential reference or unreachable private target each explain the prerequisite instead of offering a disabled button.
- No-access: Readers and non-administrators see no setup affordance; other accounts' task scopes stay invisible even to Brain admins; administrative audit remains admin-only.
- Duplicate or replay: Resolving legacy coding-agent tab state must not create a second credential, token or connection record.
- Stale data: Last-updated position and refetch errors are shown for browser metadata; native workspace discovery is labelled as a separate companion operation.
- Reconciliation divergence: Configured-only placement, metadata-verified connections and successfully called connections are three distinct displayed states and must not collapse into one indicator.

## Integrations and Runtime Inputs

- Providers: Installed Codex, Claude Code and OpenCode host shapes as already supported. No new host adapter is introduced by this slice.
- Environment: Variable names only, never values. Existing `RECOLLECT_URL` and device profile patterns apply; the loopback credential broker used during documentation walkthroughs is not part of the product.
- Secrets: Tokens stay in host secret handling or the OS credential store. No secret appears in a plugin file, generated settings, URL, list, fixture or proof output. Credential references remain references and are never displayed as values.
- Failure handling: Existing transport timeout, retry, lease and idle-shutdown behavior is unchanged. No navigation, tab resolution or refetch may start a backend, wake an instance or execute a tool.

## Tests and Acceptance

- Automated: Browser cases proving Connections exposes no agent setup path, that legacy coding-agent state resolves to Agents, that the Agents flow ends in a real memory-read check, and that configured-only states never render as connected. Existing device, profile grant, runner, credential and uncertain-call regressions pass from their new locations. Web typecheck, `./scripts/validate.sh` and `git diff --check`.
- Manual: One installed-host connection through the Agents flow on a disposable Brain, recording the actual memory-read result separately from the rendered setup, and one private-placement connection whose state is reported as configured until a real call succeeds.
- Acceptance: Exactly one product path connects a coding agent; Connections owns outbound MCP only; where-a-connection-runs is explained without undefined nouns; tool-group grants remain independent and are described in one place; captured sessions and working contexts are reachable under Agents with private scope preserved; no view change widens a grant, issues a credential, starts a process or executes a tool.

## Closeout

- Planned: Coding-agents view removal, legacy state resolution, Agents promotion, single connect flow, session and context views, where-it-runs explanation and tool-group naming.
- Shipped: Not yet implemented. This pack is the specification; delivery evidence is recorded here only after the checks above run.
- Not shipped: OAuth, plugin packaging, new hosts, capture hooks and device dedupe changes remain outside this slice.
- New blockers: None recorded at authoring time.
- Docs updated: Owning epic slice map and dependencies, active execution index, desktop contract Agents and Connections rows, managed-experience amendment note, and the desktop runbook function map.
- Validation: Not yet executed for this slice.
- Version: N/A: no release policy exists.
- Commit: Uncommitted.