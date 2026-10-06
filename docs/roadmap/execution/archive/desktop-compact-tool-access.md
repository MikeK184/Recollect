# Compact Tool access cards

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Implement the approved tighter image with one permission row per person.
- Non-goals: Grant-policy changes, new integrations or automatic tool execution.
- Delivery shape: Local web implementation, running stack proof and independent UI review.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md)
- [Catalogue and independent grants](../../../contracts/mcp-catalogue-and-profiles.md)
- [Visual epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Compact header, filter, cards and description; one effective icon set,
  locked inherited powers and source popover with optional direct editing.
- Out of scope: Backend/storage changes, new accounts, tool calls and deployments outside local stack.
- Blockers: None; the approved contract amendment resolves prior separate-direct presentation.

## Surface and Interface Changes

- Interfaces: Existing profile and grant commands, unchanged payloads.
- Storage: N/A; derived draft presentation only.
- Ownership: McpProfileCard, shared permission icons and scoped connection CSS.

## Data and Authority

- Inputs: Authoritative member/direct/admin/group data and staged grants.
- Authority: Additive rights; Brain admins inherit Manage/Share, never Use. Main
  icons edit only removable direct powers; locked inherited powers remain allowed.
  The source popover retains explicit direct editing for overlapping grants.
- Blind spots: No external OIDC provider is configured locally; group inheritance
  is checked through focused projection tests and existing canonical source.

## States and Edge Cases

- Loading: Retain labelled pending member reads and disabled edit until loaded.
- Empty: Preserve zero MCP/member states and compatible connection picker.
- Error: Failed read hides member details; partial save retains unfinished edits.
- Blocked: Archive/pending saves disable all mutations, including portal controls.
- No-access: Manage controls configuration; Share controls grants independently.
- Duplicate or replay: Existing idempotency and acknowledged grant retry remain.
- Stale data: Retain revision conflict/reload and five-second authorization refresh.
- Reconciliation divergence: Account/group draft union reflects pending changes;
  inactive/no-Brain members retain denied effective rights and stored provenance.
  Same-name grants retained from different OIDC issuers keep authoritative
  member/individual-grant rights; ambiguous main/group editing is disabled,
  while explicit direct editing remains available in the source popover.

## Integrations and Runtime Inputs

- Providers: Existing Mantine controls; Context7 confirms accessible Popover behavior.
- Environment: Existing ready local stack; no new environment variables.
- Secrets: No credentials in source, images or proof logs.
- Failure handling: Canonical errors/retry and authority-loss draft cleanup unchanged.

## Tests and Acceptance

- Automated: Meaningful direct/group/admin draft projection checks; web type/design/
  build, focused regression expectations, governance validation and CodeGraph sync.
- Manual: CUA compact/wide UI, inherited locks/provenance, MCP/person draft edit,
  Cancel and reversible Save with no persistent permission expansion; independent review.
- Acceptance: Approved visual density, single adjacent effective set, correct source
  semantics and canonical Save/Cancel preserved on the actual local stack.

## Closeout

- Planned: Approved compact layout and independent functional/visual review.
- Shipped: Compact aligned cards/header/filter, single adjacent effective icon
  set, inherited locks, source popover/direct cleanup and private You-only reader
  row; existing canonical Save/Cancel and acknowledged partial retry preserved.
  Ready local frontend deployment and independent laptop/wide review accepted.
- Not shipped: Backend policy changes and external OIDC live proof.
- New blockers: None.
- Docs updated: Contracts, epic/index, archived pack/index, active index, handoff
  and [dated evidence](../../../mappings/desktop-compact-tool-access-2026-10-06.md).
- Validation: Eight pure permission checks, web type/design/build, 32 governance
  tests, whitespace and CodeGraph passed. Actual CUA draft/Cancel/Save/partial
  retry, inherited/source and demo reader/two-tool inspection passed. Canonical
  configuration/grants/rights/Brain/call inventory restored unchanged.
  Independent final source and 1384/2504 px review reports no remaining blocker.
- Version: N/A: no release policy.
- Commit: Uncommitted.
