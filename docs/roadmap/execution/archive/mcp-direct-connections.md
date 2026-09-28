# Direct MCP connections and URL setup

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: connect an ordinary MCP host without requiring the companion; add an anonymous HTTP MCP server from a form.
- Non-goals: a new OAuth authorization server, published plugin marketplace, automatic installation of executables.
- Delivery shape: local implementation, focused tests and preserved-data local deployment.

## Governing Sources

[Agent MCP](../../../contracts/mcp-memory-and-workspace-tools.md),
[catalogue](../../../contracts/mcp-catalogue-and-profiles.md),
[managed experience](../../../contracts/memory-managed-experience.md) and
[epic](../../epics/mcp-coordination.md). The user's 2026-09-28 correction makes
normal MCP the primary connection; capture integration is optional.

## Scope

- In scope: name/URL form with Context7 preset, bounded metadata discovery, generated manifest approval, direct HTTP host instructions and browser-issued existing device credentials.
- Out of scope: native OAuth login, custom authentication protocols, implicit profile Use grants, full public benchmark leaderboard comparisons.
- Blockers: None. Existing bearer authentication and pairing handlers remain authoritative.

## Surface and Interface Changes

- Interfaces: owner-only POST `/api/mcp/definitions/inspect-http` returns a candidate manifest from anonymous SDK discovery. Existing approval and connection handlers save it. Direct MCP uses `/api/brains/{brain}/mcp/agent`.
- Storage: existing device pairing and catalogue tables; no schema changes.
- Ownership: SDK transport in mcp-runtime, authority in server, form and setup presentation in web.

## Data and Authority

- Inputs: user-entered server name and exact URL; remote tool metadata is untrusted data.
- Authority: only browser installation owner may initiate URL discovery and approve definitions. No tool is called by discovery, and no Use permission is granted by saving.
- Blind spots: tool listing is not successful tool execution; host configuration is not connection proof. Existing device token carries the user's allowed Brain access, not a narrower token scope.

## States and Edge Cases

- Loading: disable discovery/save while pending; bounded timeout and concurrency.
- Empty: name and URL required; offer Context7 address, optional advanced manifest import.
- Error: credential rejection, protocol failure and unsupported manifests remain visible without remote bodies or secrets.
- Blocked: authentication-required remote servers use existing credential/manifest setup.
- No-access: existing owner, Brain and device checks apply.
- Duplicate or replay: existing catalogue key conflict/idempotency preserved; pairing finish prevents later token readback.
- Stale data: edits clear discovered preview; live execution still revalidates schemas.
- Reconciliation divergence: failed approval cannot be reported as saved; unfinished connection remains visible as unverified.

## Integrations and Runtime Inputs

- Providers: anonymous Context7 at `https://mcp.context7.com/mcp`; public library documentation queries only.
- Environment: `RECOLLECT_MCP_TOKEN` for host HTTP authorization; no inline token in copied config.
- Secrets: credential exists only in React state, revealed/copied deliberately, never storage, URL, logs or proof screenshots. Device remains revocable under Devices.
- Failure handling: HTTP no redirects/proxy/retries, wire limits and deadlines, at most four metadata requests in flight. No remote tools invoked during import.

## Tests and Acceptance

- Automated: authenticated owner boundaries, invalid URL/remote schemas, no tool execution on discovery; native MCP initialize/list/recall and revocation using browser-issued credentials; focused UI and `./scripts/validate.sh`.
- Manual: real anonymous Context7 metadata and documentation call through Recollect; live UI inspection.
- Acceptance: standard MCP setup needs no binary; form generates a usable connector; tests distinguish configured, listed, protocol completion and useful documentation. An observed provider quota refusal satisfies the requested anonymous probe but must not be reported as useful documentation access.

## Closeout

- Planned: direct MCP setup, anonymous URL form, Context7 proof.
- Shipped: Direct HTTP Codex/Claude Code configuration and revocable browser-created token; anonymous name/URL form and Context7 preset; locally deployed and inspected. [Dated proof](../../../mappings/mcp-direct-connections-2026-09-28.md).
- Not shipped: OAuth provider and published plugin.
- New blockers: None.
- Docs updated: Governing contracts, setup/catalogue runbooks, dated mapping, handoff, owning epic and indexes.
- Validation: Six MCP browser cases, owner/member/device authority test, design/typecheck/build, clippy and formatting passed. Live Context7 metadata discovery and save passed; actual isolated documentation call returned provider quota exceeded. Root Compose readiness and equal seven-Brain before/after inventory verified on image 56202feeeabb.
- Version: N/A: no release policy.
- Commit: uncommitted.
