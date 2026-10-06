# Readable MCP connections and call inspection

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Address the user's five marked connection readability/loading concerns.
- Non-goals: New grants, automatic tool execution or arbitrary jq query evaluation.
- Delivery shape: Existing light layout with clearer data and reliable read states.

## Governing Sources

- [Connection readability amendment](../../../contracts/desktop-experience.md)
- [Catalogue](../../../contracts/mcp-catalogue-and-profiles.md)
- [Runtime](../../../contracts/mcp-runtime-and-credentials.md)
- [Visual epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Scope/runner badges, tool loading/error/empty/permission states,
  bounded safe read retry, one expanded description, JSON Format/Compact toolbar,
  structured actual call metadata/result/evidence sections and clear Test meaning.
- Out of scope: Schema/migrations, changes to capture/expiry/grants, new MCPs,
  executing a tool during health check, code or remote grammar execution.
- Blockers: None. Exa inspection compatibility is an independently specified MCP slice.

## Surface and Interface Changes

- Interfaces: Existing read/discovery, calls and anonymous inspection APIs only.
- Storage: N/A; formatting and badges are derived presentation only.
- Ownership: McpPanel, shared description/CodeBlock and call inspection components.

## Data and Authority

- Inputs: Actual catalogue, environment names, approved schema metadata and call DTO.
- Authority: Existing configure/Use grants; failures and revocations clear visible
  cached results. Current last successful call remains historical, not health.
- Blind spots: Discovery is approved cached metadata, not live tools/list or a
  guarantee that a remote tool will succeed. Tool-group pages remain paginated.

## States and Edge Cases

- Loading: Visible status while metadata pending/refreshing, no fabricated zero.
- Empty: Actual successful zero tools is distinct from unloaded or denied state.
- Error: Bounded retry only for safe transient reads; explicit Reload remains.
- Blocked: Disabled/archived/configuration and expired output preserve old gates.
- No-access: No automatic privileged metadata request; permitted group discovery
  stays available where canonical Use permits it, no person/grant exposure.
- Duplicate or replay: No tool mutation retry; descriptions appear once while open.
- Stale data: Read authority loss removes cached tools/output and pending controls.
- Reconciliation divergence: Unknown completion remains unknown; cancel/receipt/
  evidence actions preserve their separate authoritative commands and outcomes.

## Integrations and Runtime Inputs

- Providers: Installed lowlight and safe Markdown; no dependency/autoloader fetch.
- Environment: Existing local stack and actual selected Brain.
- Secrets: No original credentials in formatted artifacts; explicit copy payloads
  retain existing protected behavior. JSON formatting preserves numeric/string bytes.
- Failure handling: Lazy highlighter failure keeps readable code; invalid/bounded
  JSON stays exact. Failed tools never cause an implicit call or server approval.

## Tests and Acceptance

- Automated: Meaningful pure formatting preservation checks, production type/design/
  build and regression discovery; governance, whitespace and CodeGraph.
- Manual: CUA actual Exa/Context7 and recorded expired call; badges, loading/retry,
  descriptions/JSON controls, explicit Test semantics and independent pixel review.
- Acceptance: All five marked concerns visibly addressed without changing grants,
  retention or execution. Existing real calls are not replayed during UI proof.

## Closeout

- Planned: Readable metadata and reliable loading on the existing Connections view.
- Shipped: Scope/runner badges, bounded reliable tool reads, single descriptions, lossless JSON Format/Compact and structured actual call/result/expiry/capture sections.
- Not shipped: New permissions, code evaluation or automated tool calls.
- New blockers: None.
- Docs updated: Contract, epic, archived pack/index, handoff and [dated evidence](../../../mappings/mcp-inspection-readability-2026-10-06.md).
- Validation: Five pure formatting tests, production type/design/build, governance32, whitespace, CodeGraph and independent source/pixel acceptance. CUA loading/error/reload, Exa/Context7, exact wide viewport and actual expired call proven.
- Version: N/A.
- Commit: Uncommitted.
