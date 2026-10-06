# Equivalent HTTP schema declaration inspection

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Make anonymous Exa Test/setup accept semantically equivalent declarations.
- Non-goals: Weaken direct approval, transform dialect-dependent constraints or fix SDK initialization.
- Delivery shape: Bounded inspection-only conversion with pure and handler proof.

## Governing Sources

- [Equivalent declaration amendment](../../../contracts/mcp-catalogue-and-profiles.md)
- [Runtime](../../../contracts/mcp-runtime-and-credentials.md)
- [Desktop Test meaning](../../../contracts/desktop-experience.md)
- [Coordination epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Convert exact root Draft 7 declaration for audited supported schema
  keywords: type, properties, required, boolean additionalProperties, minLength,
  maxLength, minimum, single-schema items and description. Then strict validation.
  Input root stays an object. Bound each schema to existing size/depth restrictions.
- Out of scope: Unknown keywords, tuple items, refs/identifiers/header behavior,
  nested dialect changes, direct import relaxation, persistence or tool calls.
- Blockers: None; user requests compatibility explanation/fix and prior equivalence evidence exists.

## Surface and Interface Changes

- Interfaces: Existing owner-only inspect-http output; no new API or DTO.
- Storage: N/A; candidate remains unapproved/unpersisted until separate approval.
- Ownership: Server definitions inspection helper; existing offline validator retained.

## Data and Authority

- Inputs: Bounded remote schemas from the SDK's anonymous metadata inspection.
- Authority: Owner browser only, transport/network/budget boundaries unchanged.
- Blind spots: Supported subset is intentionally narrow; no universal Draft 7 migration claim.

## States and Edge Cases

- Loading: Existing asynchronous timeout/concurrency behavior remains.
- Empty: Valid zero-tool server stays supported.
- Error: Unsupported dialect-dependent/unknown/oversized/deep schema remains rejected.
- Blocked: Invalid roots and remote refs cannot be normalized around validation.
- No-access: Non-owner inspection remains forbidden; no permission expansion.
- Duplicate or replay: No persistence or audit mutation from repeat inspection.
- Stale data: No overwrite of existing approved definition or active sessions.
- Reconciliation divergence: Test success proves handshake/list only, never a tool result.

## Integrations and Runtime Inputs

- Providers: Existing rmcp/jsonschema; no dependency bump.
- Environment: Existing source/test installation and local stack; bounded rebuild
  plan must respect disk guard and preserve data, with exact delivery path recorded.
- Secrets: Anonymous inspection; no credentials retained or printed.
- Failure handling: Existing network timeout, metadata pagination and sanitization.

## Tests and Acceptance

- Automated: Actual Exa schema conversion/equivalence; exact root-only diff;
  rejection of unsupported/remote/nested/tuple/header/size/depth cases; canonical
  2020-12 local-reference schema unchanged; direct raw Draft 7 approval still fails.
  Handler fixture proves inspection has no approval/audit/tool-call persistence.
- Manual: Running Exa Test returns tools without creating/replacing definitions;
  actual call remains historical and no new tool call is dispatched.
- Acceptance: False Exa schema failure fixed within supported subset, strict
  approval retained and no schema assertions/authority/execution changed.

## Closeout

- Planned: Equivalent bounded schema declaration conversion during inspection.
- Shipped: Bounded inspection-only equivalent Draft 7 root declaration conversion, followed by the unchanged strict validator. Running anonymous Exa Test lists both tools.
- Not shipped: Broad dialect converter or SDK fallback repair.
- New blockers: None.
- Docs updated: Contract, epic, archived pack/index, handoff and [dated evidence](../../../mappings/mcp-inspection-readability-2026-10-06.md).
- Validation: Three pure schema tests, one owned PostgreSQL/HTTP fixture, clippy/rustfmt, governance32 and live Exa CUA proof. Approved manifests and SWEG call count unchanged; fresh incremental Linux release and ready local stack verified.
- Version: N/A.
- Commit: Uncommitted.
