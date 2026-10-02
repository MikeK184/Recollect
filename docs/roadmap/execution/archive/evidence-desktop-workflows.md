# Sources, repositories and agent workspaces

Status: shipped
Owning epic: `docs/roadmap/epics/evidence-and-workspaces.md`
Work type: product

## Summary

- Goal: Inspect original evidence and published repository knowledge, and configure supported agents once through three focused routes.
- Non-goals: New import formats, browser checkout scanning, arbitrary host support, task/project management and publication of private paths.
- Delivery shape: Local capability-owned frontend/API changes and focused proof, no external deployment.

## Governing Sources

- [Desktop ADR](../../../adr/0014-desktop-experience-and-answers.md) and [desktop contract](../../../contracts/desktop-experience.md)
- [evidence-collections](../../../contracts/evidence-collections.md)
- [evidence-repository-publication](../../../contracts/evidence-repository-publication.md)
- [evidence-workspace-scope](../../../contracts/evidence-workspace-scope.md)
- [evidence-session-capture](../../../contracts/evidence-session-capture.md)
- [mcp-memory-and-workspace-tools](../../../contracts/mcp-memory-and-workspace-tools.md)
- [Owning epic](../../epics/evidence-and-workspaces.md)

## Scope

- In scope: Sources list/title search, exact source versions/import/edit/views; repositories/snapshots/environments/private checkouts; Agents onboarding, published session coverage and own task scopes. Canonical capture editing links to Settings.
- Out of scope: New import formats, browser checkout scanning, arbitrary host support, task/project management and publication of private paths.
- Blockers: None; accepted desktop/domain contracts resolve behavior. Shell integrates through its predecessor slice; final browser acceptance requires that integration.

## Surface and Interface Changes

- Interfaces: Existing evidence/publication/workspace/capture APIs; optional source-list q, trimmed up to 200 UTF-8 bytes, literal case-insensitive current-title containment before pagination. Empty q preserves existing list; filters reset offset.
- Storage: No new canonical storage; source-title filter operates on authorized current list rows. All existing immutable versions/artifact/manifest/task records remain unchanged.
- Ownership: evidence-and-workspaces owns these feature views and canonical domain handlers; platform owns shared tokens/shell/components.

## Data and Authority

- Inputs: Authorized source catalogue/details, published snapshots/manifests, own checkouts/tasks, deliberately published session sources and actual capture coverage.
- Authority: Existing Brain evidence rights plus account/device ownership for task/checkout metadata. A Brain admin cannot read another account's task just because capture evidence is shared.
- Blind spots: No browser access to arbitrary local files; unknown host events, missing bytes/extraction coverage and unobserved host activity remain unknown.

## States and Edge Cases

- Loading: Only active route and canonical open inspector fetch; importing/processing shows actual command/job state.
- Empty: Separate empty Brain, no title matches, unpublished repository and no own task/session state.
- Error: Preserve safe import/setup input; hide stale source bodies on failed refresh and show actionable canonical error.
- Blocked: Reference-only source or missing native publication/capture setup is explained without a pretend connection.
- No-access: Foreign evidence and private task/path queries fail through owner handlers; reader/archived mutation restrictions remain.
- Duplicate or replay: Existing import/publication/operation idempotency stays authoritative; navigating/onboarding does not start capture or model work.
- Stale data: Cancel search/page/Brain transitions, reset pagination on criteria change, preserve exact version and immutable task binding.
- Reconciliation divergence: Show exact committed/desired/observed revision and coverage differences; no implicit latest snapshot or scope substitution.

## Integrations and Runtime Inputs

- Providers: Existing same-origin API and optional supported native Codex/Claude integrations; no new dependency service.
- Environment: Existing installation/companion configuration only; no new browser filesystem privilege or secret variable.
- Secrets: Preserve existing credential transport/redaction; no secret values in assets, URLs, fixtures, docs or output.
- Failure handling: Keep existing command retries/resume semantics. Browser metadata retry never claims native rediscovery or repeats paid work.

## Tests and Acceptance

- Automated: Source q over multiple pages, literal wildcard characters, byte bounds and foreign Brain; existing import/version/manifest/task/capture tests through relocated views; type/build and repository validation.
- Manual: Import text/reference, inspect exact versions, edit as new version, manage grouping, inspect a snapshot/manifest, verify own tasks and native host setup instructions from real desktop routes.
- Acceptance: Existing evidence/publication/scope journeys stay usable; no private task leakage; q filters before pagination and accurate labels; canonical inspectors/expiry/cancellation proven.

## Closeout

- Planned: Sources list/title search, exact source versions/import/edit/views; repositories/snapshots/environments/private checkouts; Agents onboarding, published session coverage and own task scopes. Canonical capture editing links to Settings.
- Shipped: Sources, exact immutable versions, grouping, repositories/manifests, Agents and private workspace contexts are delivered and deployed locally. Corrected Evidence and Publication regressions pass.
- Not shipped: No remaining slice acceptance; explicit product non-goals remain excluded.
- New blockers: None for this slice.
- Docs updated: This pack, owning epic, active/archive and epic indexes, handoff, relevant runbooks and [final acceptance evidence](../../../mappings/desktop-final-acceptance-2026-10-01.md).
- Validation: The completed 65-case browser matrix includes Evidence, Publication, Workspace and Capture. Production workspace integration proves foreign-account task/path isolation; current exact-source reload, history and foreign-Brain tests pass. Final repository checks are recorded in the linked evidence.
- Version: N/A: no release requested.
- Commit: Uncommitted.
