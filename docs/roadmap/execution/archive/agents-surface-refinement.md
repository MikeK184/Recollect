# Agent surface refinement: used-on-Brain roster and global Agents page

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Make each Brain's agent roster show only the agents that have actually been used on this Brain, and add a global `/agents` page (third global-nav entry) listing every agent of the signed-in user with the Brains each is used in and when it was last used there.
- Non-goals: No device issuance/revocation semantics change, no per-Brain scoped revocation, no new integration kinds, no capture-policy or MCP runtime change, no multi-user tenant work.
- Delivery shape: One new account-scoped server read (`GET /api/agents`), a global Agents page in the web shell, roster filtering in the existing per-Brain roster component, one contract amendment, and live browser verification.

## Governing Sources

- [Plugin/MCP direct user authentication](../../../contracts/mcp-plugin-direct-auth.md) (amended: global agent roster read)
- [Companion pairing and device authority](../../../contracts/platform-device-pairing.md)
- [Contextual desktop experience](../../../contracts/desktop-experience.md) (amended: global nav with Agents, used-only per-Brain roster, global Agents page row)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: New `GET /api/agents` account-scoped read returning the signed-in user's agents (owner: all account devices; member: own devices) grouped by username with the per-Brain roster fields plus `brains` — the accessible Brains where that device has `mcp_calls` or capture events, each with `brain_id`, `name` and `last_used_at`, ordered by most recent use; revoked/expired devices excluded and reported as a hidden count. Web: global navigation gains Agents `/agents` between Brains and Team; new global Agents page rendering the groups with name, host kind, integration label, active state dot and a per-Brain usage list (brain name + last used); the per-Brain roster shows only rows with `last_used_on_brain_at` set by default, keeps its revoked/expired toggle for that filtered set, and gains a "view all account agents" link to `/agents`.
- Out of scope: Pairing approval and the hidden `/devices` surface (unchanged); revocation behavior (account-wide, labeled); device markers, capture bindings or MCP call paths; any authorization change.
- Blockers: None. The 2026-10-02 user direction (roster relevance plus a global agent view) is the governing decision record; contract amendments are part of this change.

## Surface and Interface Changes

- Interfaces: New `GET /api/agents` with operation id `accountAgents`; response `AccountAgentRoster { groups: [{ user_name, agents: [AccountAgent] }], hidden_count }` where `AccountAgent` carries the per-Brain roster fields plus `brains: [{ brain_id, name, last_used_at }]`. No token or credential material in any response.
- Storage: None. Reuses `devices`, `device_pairings`, `mcp_calls`, capture events and `accounts.username`.
- Ownership: MCP coordination owns the account roster read and both Agents surfaces; platform device authority is untouched.

## Data and Authority

- Inputs: `devices` (name, host_kind, integration, claimed, expires_at, revoked_at), `mcp_calls.created_at`, capture event `received_at`, `brains.name` joined only for Brains the caller can access, `accounts.username`.
- Authority: Account-scoped read for the signed-in user; the owner sees every account device, any other member sees only their own. It grants nothing and never widens a grant; a row is configuration evidence, not live-connection evidence.
- Blind spots: A device with no calls or capture events on any accessible Brain shows an empty brain list. Last-use timestamps are point-in-time evidence, not presence.

## States and Edge Cases

- Loading: The global page shows its own skeleton; the per-Brain roster filter adds no new loading state.
- Empty: No used agents on this Brain → one line "No agents have been used on this Brain yet." plus the connect options and the global-page link; a global page with no devices shows one line.
- Error: Endpoint failure renders an error state with retry; per-Brain setup tabs stay usable.
- Blocked: A failed roster read blocks only the roster card or page content, never navigation.
- No-access: Members see only their own devices; inaccessible Brains never appear in `brains`, so no brain name leaks.
- Duplicate or replay: Pure read; rendering creates nothing.
- Stale data: Both rosters refetch on mount and after a revoke; timestamps are point-in-time.
- Reconciliation divergence: Global and per-Brain lists derive from the same tables; a device present in one but not the other only occurs via concurrent revocation, which a refetch reconciles.

## Integrations and Runtime Inputs

- Providers: None new.
- Environment: No new environment variables or services.
- Secrets: None; responses contain no credential material.
- Failure handling: Standard query failure → 500 `ApiError`; UI retry state with no fallback data.

## Tests and Acceptance

- Automated: Focused platform test `account_agent_roster` proves owner grouping, per-Brain usage ordering, member scoping to own devices, revoked/expired exclusion with hidden count, inaccessible-Brain absence, and no credential material in the payload.
- Manual: Owner login at the live stack — global nav shows Brains · Agents · Team; `/agents` lists every agent with per-Brain usage; the SWEG Brain roster shows only used agents with the global-page link.
- Acceptance: Browser verification passes on real data; web typecheck and design checks clean; `./scripts/validate.sh` passes; live stack rebuilt and healthy.

## Closeout

- Planned: `GET /api/agents`, global Agents page, used-only per-Brain roster, nav entry, contract amendment, browser proof.
- Shipped: `GET /api/agents` account-scoped read (owner sees all account devices, members their own; per-agent `brains` usage list limited to accessible Brains, ordered by most recent use; revoked/expired excluded with hidden count); global `/agents` page in the web shell with per-user groups, host/integration labels, active dots and per-Brain last-use lines; per-Brain roster now shows only agents used on this Brain with a "Show revoked and expired" toggle for that filtered set and an "All account agents" link. Live proof 2026-10-03: global page lists all 12 owner agents with correct SWEG usage (Recollect plugin · OpenCode last used 02/10/2026 14:02:42); the SWEG Brain roster shows exactly its three used agents; no console errors. See [evidence](../../../mappings/agents-ask-chrome-refinements-2026-10-03.md).
- Not shipped: Nothing from this scope.
- New blockers: None.
- Docs updated: Contracts `mcp-plugin-direct-auth.md` (global roster read) and `desktop-experience.md` (global nav with Agents, used-only per-Brain roster, global Agents row); epic slice and indexes reconciled; evidence mapping added.
- Validation: Focused platform test `account_agent_roster` passes on a disposable database (grouping, usage ordering, member scoping, hidden count, no credential material); existing `pairing_markers_and_brain_agent_roster` still passes; clippy clean; web typecheck and design checks clean; `./scripts/validate.sh` passes; live stack rebuilt via `./scripts/stack.sh up --build` with api/worker healthy.
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
