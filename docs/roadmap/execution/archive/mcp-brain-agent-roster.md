# Per-Brain agent roster and Devices navigation removal

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Make each Brain's Agents page the home of "who is connected to this Brain": a per-user roster of exactly two integration kinds (Recollect plugin, direct MCP access token) with host kind, active/idle state and brain-scoped last use; remove Devices from global navigation while keeping the pairing-approval deep link alive; strip all `recollect-agent`/companion wording from the UI; and prove end-to-end that a connected host can call an external MCP (official `everything` server) through the Brain's MCP with central and local placement.
- Non-goals: No per-Brain scoped revocation (revocation stays account-wide and is labeled as such); no new integration kinds, hosts, OAuth flow, connector, capture hook or plugin packaging change; no change to grant evaluation, credential issuance semantics, runner trust, MCP wire contracts or Brain deletion.
- Delivery shape: One small server migration (`devices.host_kind`, `devices.integration`, pairing markers), one new brain-member read endpoint, plugin and web issuer updates, a React restructure of the Agents surface plus a bounded copy cleanup, docs reconciliation, focused browser and API proof, and the live SWEG→Brain→external-MCP test.

## Governing Sources

- [Companion pairing and device authority](../../../contracts/platform-device-pairing.md) (amended: host_kind/integration markers, hidden Devices route)
- [Plugin/MCP direct user authentication](../../../contracts/mcp-plugin-direct-auth.md) (amended: issuer markers, per-Brain roster read)
- [Contextual desktop experience](../../../contracts/desktop-experience.md) (amended: global nav without Devices, Agents roster row, hidden Devices surface)
- [Approved MCP catalogue and execution profiles](../../../contracts/mcp-catalogue-and-profiles.md)
- [Managed MCP calls and credentials](../../../contracts/mcp-runtime-and-credentials.md)
- [Vault credentials and private runners](../../../contracts/mcp-vault-and-private-runners.md)
- [Plugin-managed memory ADR](../../../adr/0018-plugin-managed-agent-memory.md)
- [Separated wiring ADR](../../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Vision: MCP coordinator and Vault](../../../foundation/vision.md#mcp-coordinator-and-vault-integration)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Migration 031 adding `host_kind` (`codex`/`claude_code`/`opencode`, nullable) and `integration` (`mcp`/`plugin`, default `mcp`) to `devices` and `device_pairings`, with backfill of both from `capture_bindings`; `PairingRequest` accepting optional validated markers; approval persisting them (new insert and same-name reuse); the plugin `connect` path sending its own host kind plus `plugin`; the browser direct-token flow sending the selected host plus `mcp`; new `GET /api/brains/{brain}/agents` brain-member read returning per-username groups with `device_id`, `name`, `host_kind`, `integration`, `claimed`, `active`, `created_at`, `expires_at`, account-level `last_used_at`, brain-scoped `last_used_on_brain_at` (latest of `mcp_calls.created_at` and capture event `received_at`) and a hidden count of revoked/expired agents; web global nav losing the Devices entry while the `/devices` route stays registered for `?code=` deep links; the Agents page gaining the roster above its tabs (group header "username — N agents", one row per device, state dot, host kind, integration label, last used on this Brain, revoke action with an account-wide effect modal, explicit toggle for revoked/expired rows); retargeting the five in-app `/devices` links to the Brain's Agents surface; removing all `companion`/`recollect-agent` UI copy (setup cards reduced to one line each, green context banner removed, legacy CLI code blocks removed, "Approve a companion" → host wording) while keeping the verification prompt, capture-settings link, tool-connections link, collapsed disclosures, connection wizard, tool groups, runners and connector registration; docs reconciliation of vision body and runbooks that still say companion/`recollect-agent`; live proof from the SWEG host through the Brain's MCP to a centrally placed official `everything` stdio server (and locally placed variant where the plugin runner is available).
- Out of scope: Per-Brain scoped revocation or any authorization change; device credential issuance/revocation semantics beyond labeling; capture policy, memory lifecycle, retrieval, graph or activity behavior; new MCP transports or connector types; multi-user tenant work.
- Blockers: None. The 2026-10-02 user direction is the governing decision record; contract amendments are part of this change.

## Surface and Interface Changes

- Interfaces: `POST /api/devices/pairings` request gains optional `host_kind` and `integration` (400 on unknown values). New `GET /api/brains/{brain}/agents` with operation id `brainAgents`; response body `BrainAgentRoster { groups: [{ user_name, agents: [BrainAgent] }], hidden_count }` where `BrainAgent { device_id, name, host_kind: Option, integration, claimed, active, created_at, expires_at, last_used_at: Option, last_used_on_brain_at: Option }`. No token or credential material in any response.
- Storage: Migration 031 only — two nullable/checked columns per table (`devices`, `device_pairings`) plus backfill from `capture_bindings` (host kind and plugin integration for devices with capture bindings). No data deletion.
- Ownership: MCP coordination owns the roster read, issuer markers and the Agents surface presentation. Platform device authority (issuance, revocation, dedupe) is untouched except for persisting the two new columns.

## Data and Authority

- Inputs: `devices`, `device_pairings`, `capture_bindings`/`capture_events` (brain-scoped activity), `mcp_calls` (brain-scoped tool calls), `accounts.username` (group label).
- Authority: The roster is a Brain-member read; it exposes only safe device metadata already visible to the account. It grants nothing and never widens a grant. A roster row is configuration evidence, not live-connection evidence; "active" means unrevoked and unexpired, nothing more.
- Blind spots: `host_kind` is null for legacy devices without capture bindings and stays null (no guessing from names). Brain-scoped last use is null until the device makes a real call or publishes capture on that Brain.

## States and Edge Cases

- Loading: Roster shows its own skeleton; tabs are independent.
- Empty: No agents → one line "No agents connected to this Brain yet." plus the two connect options.
- Error: Endpoint failure renders an error state with retry; it never falls back to capture-only data (pure MCP tokens would vanish).
- Blocked: A failed roster read blocks only the roster card; setup tabs, sessions and contexts stay usable.
- No-access: Non-members get the standard Brain 404/403; members see only their own account's devices.
- Stale data: The roster refetches on mount and after a revoke; last-use timestamps are point-in-time evidence, not live presence.
- Reconciliation divergence: `host_kind` falls back to the device's latest capture binding for display only; the roster read never rewrites device records.
- Revoked/expired: Excluded from groups, surfaced as "Show revoked and expired (N)"; rows render with state and no misleading activity.
- Revoke: The modal states explicitly that revoking removes the agent from **all** Brains (account-wide `DELETE /api/devices/{id}`); repeat revocation stays harmless.
- Pairing deep link: `/devices?code=…` keeps working unauthenticated-redirect → sign-in → approve, with the page no longer in any menu.
- Duplicate or replay: Roster is a pure read; rendering it creates nothing.

## Integrations and Runtime Inputs

- Providers: No new host adapter. The plugin's existing `connect` flow gains two request fields; the browser Create action sends its selected host.
- Environment: Variable names only, never values. Existing `RECOLLECT_URL` patterns apply.
- Secrets: No token, private code or credential value appears in the roster response, UI, fixtures or proof output.
- Failure handling: Navigation, roster rendering and copy changes start no backend, wake no instance and execute no tool. The e2e proof uses real `mcp.call` dispatch with existing placement rules (no fallback between central/local/private).

## Tests and Acceptance

- Automated: Server tests for marker validation (400 on unknown values), approval persistence (new + reuse), backfill behavior, roster grouping/ordering/hidden-count and Brain-member authorization; web typecheck/build; `./scripts/validate.sh`; `git diff --check`.
- Manual: Owner login at the live stack — global nav shows no Devices; `/devices?code=` deep link still renders approval; Agents page shows the roster grouped by user with correct host kind/integration/state for the real plugin and MCP-token devices; revoke modal states the account-wide effect; all five retargeted links land on the Brain's Agents surface; no `companion`/`recollect-agent` string remains in rendered copy.
- End-to-end proof: On the SWEG Brain, register the official `everything` stdio server via manifest import, add a central-placement connection, grant Use on the "test" profile, then from the SWEG host (connected through the Recollect plugin MCP) run `workspace.start_task` → tool operation → `mcp.call echo` → `mcp.status` and verify the retained result in Activity/Tool calls; repeat with local placement where the plugin runner is live.
- Acceptance: Exactly two integration kinds are named anywhere in the UI; Devices is absent from global navigation with the deep link intact; the roster answers "who is connected to this Brain" per user with real data (no name parsing); revocation is labeled account-wide; the external MCP call through the Brain succeeds with retained output for central placement (and local where a runner is live).

## Closeout

- Planned: Migration 031, issuer markers, roster endpoint, nav restructure, Agents roster UI, copy cleanup, link retargets, docs reconciliation, SWEG→Brain→`everything` proof.
- Shipped: Migration 031 (`devices.host_kind`, `devices.integration`, pairing markers, capture-binding backfill); `PairingRequest` markers with 400 validation and approval persistence (new + same-name reuse via COALESCE); plugin issuer sends the `plugin` marker; browser Create action sends `mcp` plus the selected host; `GET /api/brains/{brain}/agents` brain-member read with per-username groups, brain-scoped last use and hidden count (`include_hidden` toggle); Devices removed from global navigation with the `/devices?code=` deep link and full account list intact; Agents page roster with two-integration rows, state dots, last-used-on-this-Brain and an account-wide labeled revoke modal; all `companion`/`recollect-agent` UI copy removed and the five in-app `/devices` links retargeted to the Brain's Agents surface. Live proof 2026-10-02: owner browser pass (nav, roster "owner — 12 agents", revoke modal wording, direct-URL Devices, zero legacy strings, clean console) and the SWEG host device token driving `workspace.start_task` → tool operation → `mcp.call resolve-library-id` → `mcp.status succeeded` against external Context7 with central placement, retained output in `mcp_call_payloads.result`, `mcp_calls` 0 → 1, visible in Activity → Tool calls. See [evidence](../../../mappings/brain-agent-roster-2026-10-02.md).
- Not shipped: The local-placement stdio variant (`everything` on the PC via a plugin runner) was not executed — the packaged dist ships no `recollect-mcp-runner` helper and the live SWEG bridge runs without `--with-runner`; it remains configuration-only here. Per-Brain scoped revocation stays deferred (account-wide, labeled).
- New blockers: None.
- Docs updated: Contracts `platform-device-pairing.md`, `mcp-plugin-direct-auth.md`, `desktop-experience.md` amended; epic slice and indexes reconciled; vision body and the device-pairing/desktop-experience runbooks reconciled to plugin wording; [evidence mapping](../../../mappings/brain-agent-roster-2026-10-02.md) added.
- Validation: `cargo check`/`clippy` clean for server, agent and protocol; web typecheck (design checks + tsc) clean; focused platform tests `pairing_markers_and_brain_agent_roster` and the existing `device_pairing_delivery_revocation_and_worker_authority` pass on disposable databases; `./scripts/validate.sh` passes; live stack rebuilt with `./scripts/stack.sh up --build` (api/worker healthy, migration 031 applied, backfill verified in DB).
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
