# Guided connections, profiles and runners

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Configure approved tools in understandable steps and inspect actual call outcomes without conflating knowledge access and tool grants.
- Non-goals: Arbitrary executable/catalogue approval, secret-value forms/lists, admin self-granted Use, implicit test calls or uncertain effect retries.
- Delivery shape: Local capability-owned frontend/API changes and focused proof, no external deployment.

## Governing Sources

- [Desktop ADR](../../../adr/0014-desktop-experience-and-answers.md) and [desktop contract](../../../contracts/desktop-experience.md)
- [mcp-catalogue-and-profiles](../../../contracts/mcp-catalogue-and-profiles.md)
- [mcp-runtime-and-credentials](../../../contracts/mcp-runtime-and-credentials.md)
- [mcp-vault-and-private-runners](../../../contracts/mcp-vault-and-private-runners.md)
- [mcp-observation-capture](../../../contracts/mcp-observation-capture.md)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: Connections/Profiles/Runners, approved definition-to-target/credential/runner/review/test wizard, canonical connection/call inspector, Use/Manage/Share grants and observed runtime/credential state.
- Out of scope: Arbitrary executable/catalogue approval, secret-value forms/lists, admin self-granted Use, implicit test calls or uncertain effect retries.
- Blockers: None; accepted desktop/domain contracts resolve behavior. Shell integrates through its predecessor slice; final browser acceptance requires that integration.

## Surface and Interface Changes

- Interfaces: Existing catalogue, connections, profiles, grants, runner and call/cancel/reconcile/resolve/release endpoints; no new wire authority.
- Storage: N/A: UI reuses current versioned definitions, profiles, runtime leases and receipts.
- Ownership: mcp-coordination owns these feature views and canonical domain handlers; platform owns shared tokens/shell/components.

## Data and Authority

- Inputs: Authorized cached catalogue/schema, connection targets and credential references, profile rights, runner identity and actual call metadata.
- Authority: Server-side independent profile grants, approved targets/executables and runner/device authority; Brain admin is not automatically a tool user.
- Blind spots: Configured credentials/runner/definition are not a successful call. Test actions may have real effects and do not imply read-only safety.

## States and Edge Cases

- Loading: Wizard steps load bounded authorized choices; call inspector shows actual queued/running/disposition state.
- Empty: No approved definition explains operator setup; no connection/profile/runner offers only currently authorized creation.
- Error: Preserve safe configuration input and expose redacted canonical validation/runtime failure; never reveal provider/credential bodies.
- Blocked: Missing reference, unavailable runner or absent Use prevents actual test while showing the relevant grant/setup step.
- No-access: Keep Use/Manage/Share independent, including direct URL and stale session; hide inaccessible outputs rather than showing cached data.
- Duplicate or replay: Canonical request IDs and receipt rules prevent duplicate effect; explicit retry cannot bypass uncertain-outcome reconciliation.
- Stale data: Version/grant changes invalidate choices and outputs; recheck before save/call, preserve original immutable operation selection.
- Reconciliation divergence: Unknown side effects stay uncertain through cancel/reconcile/resolve; release does not kill another active lease.

## Integrations and Runtime Inputs

- Providers: Existing approved stdio/HTTP tools and optional Vault/native/private runners only.
- Environment: Existing credential-reference/runner configuration; no new provider or executable installed by navigation.
- Secrets: Preserve existing credential transport/redaction; no secret values in assets, URLs, fixtures, docs or output.
- Failure handling: Existing timeout, credential renewal/drain, lease and reconciliation rules; no automatic call on wizard mount.

## Tests and Acceptance

- Automated: Wizard validation and canonical mutation regression, independent permission combinations, configured versus observed status, uncertain/cancel/release and secret redaction; frontend and governance checks.
- Manual: Configure a permitted fixture connection, grant Use separately, explicitly execute a known fixture call, inspect actual result and uncertainty recovery.
- Acceptance: The guided flow preserves all current setup/recovery capability and prevents silent effectful tests, secret exposure or broadened grants.

## Closeout

- Planned: Connections/Profiles/Runners, approved definition-to-target/credential/runner/review/test wizard, canonical connection/call inspector, Use/Manage/Share grants and observed runtime/credential state.
- Shipped: Five-step approved connection setup, explicit Configure profile & test handoff, scoped profiles/runners and independent Use/Manage/Share behavior. Three catalogue/wizard/private-runner cases and three actual runtime cases passed, including one-effect replay/unknown reconciliation, captured evidence, cancellation, expiry, revocation, nested Escape/focus and session release. Coding-agent setup guidance also passed.
- Not shipped: Explicit product non-goals remain excluded: arbitrary executable/catalogue approval, secret-value UI, implicit test calls and self-granted Use. Configured connections are not advertised as verified calls; successful runtime evidence belongs to the exercised fixtures.
- New blockers: None for this setup/runtime presentation slice. Broader Activity and integrated acceptance remain with their owning packs.
- Docs updated: [Desktop guide](../../../runbooks/desktop-experience.md), [implementation/assets/dependency evidence](../../../mappings/desktop-experience-implementation-2026-09-26.md), [current handoff](../../../../CONTINUE_HERE.md), affected domain runbooks, owning epic and indexes.
- Validation: Frontend typecheck/design and final image build passed; workspace clippy and 21 unit tests passed with 3 live-Vault cases explicitly ignored; governance lint and 32 tests passed. Desktop seven cases passed across a six-pass run and the repaired font-fallback targeted rerun; real Team/OIDC two, Ask five and MCP setup/runtime six passed. The dated mapping separates each owner result, initial failures, fixture/provider boundaries and remaining checks.
- Version: N/A: no release requested.
- Commit: uncommitted.
