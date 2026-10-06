# Setup and exception management

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: People connect once, inspect useful activity and control permissions/privacy without operating task scopes and capture receipts.
- Non-goals: New authorization, storage, agent launching, dependencies, automatic tool tests or policy expansion.
- Delivery shape: Existing frontend/read interfaces, governing amendments and local deployment with independent UI review.

## Governing Sources

- [Management amendment](../../../contracts/desktop-knowledge-surface.md#setup-and-exception-management--2026-10-04)
- [Desktop contract](../../../contracts/desktop-experience.md)
- [Autonomous memory](../../../adr/0006-autonomous-memory.md)
- [Private workspace scope](../../../contracts/evidence-workspace-scope.md)
- [Capture](../../../contracts/evidence-session-capture.md)
- [Tool grants](../../../contracts/mcp-catalogue-and-profiles.md)
- [Retention](../../../contracts/memory-retention-and-erasure.md)

## Scope

- In scope: Agents/activity, Connections/access, Settings/policy clarity, concise copy, scoped legacy links, lazy diagnostics and meaningful validation.
- Out of scope: Global roster visibility changes, backend mutation semantics, ambient TV closeout or new product runtime.
- Blockers: None; the user's approval resolves placement decisions.

## Surface and Interface Changes

- Interfaces: Existing reads and mutations; add exact optional device filtering before capture count/pagination and safe user/agent attribution, exact source processing and recorded learning outcomes. MCP catalogue adds a current-configuration successful-call timestamp under existing visibility and Use checks. No inference from a filtered client page.
- Storage: N/A: preserve existing records and schema.
- Ownership: Frontend feature owners, capture read and MCP catalogue summaries; no mutation or permission changes.

## Data and Authority

- Inputs: Existing contributor, capture, pipeline, context, tool-call and policy records.
- Authority: Server permissions and canonical policy/change IDs; scope is selection, never a grant.
- Blind spots: No inferred complete transcript, successful call or processing relation when history does not establish it.

## States and Edge Cases

- Loading: Bounded skeleton/loader in selected view only.
- Empty: No observed records in the displayed scope; no invented connection failure.
- Error: Remove stale protected data and report failed read; preserve retry.
- Blocked: Only current canonical blockers prominent; historical failures remain details.
- No-access: No foreign private tasks/checkouts or restricted source/tool output; own-only revoke.
- Duplicate or replay: No automatic mutations/tests; preserve idempotency and uncertain-outcome handling.
- Stale data: Preserve live access/policy refresh, exact selected identities and expected policy change IDs.
- Reconciliation divergence: Configured permissions are not successful processing; backend records remain unchanged.

## Integrations and Runtime Inputs

- Providers: Existing installed provider and approved MCP targets; no new calls from inspection.
- Environment: Existing local stack; no new variables.
- Secrets: Existing credential references and redaction only; never render secret values.
- Failure handling: Existing query errors/retry and server fences; no implicit execution fallback.

## Tests and Acceptance

- Automated: Existing frontend typecheck/design/build, focused existing backend acceptance where relevant, clippy if Rust changes, repository validate and whitespace. Only add focused tests for meaningful new filtering/authority behavior.
- Manual: Live local Agents/Activity/Connections/four Settings views; legacy scoped links; policy-preserving read-only navigation; short desktop layout; independent reviewer of actual screens.
- Acceptance: No normal context/session/runner tab or mandatory context setup; scoped activity and own diagnostics; one editor per policy; custom/off/configured states; independent grants, granular retention and mutation boundaries intact; no closed-panel diagnostic polling.

## Closeout

- Planned: Contributor-first Agents and contextual activity, secondary connection diagnostics, four Settings tabs and exact evidence/permission boundaries.
- Shipped: Implemented and deployed to the existing local stack. Agents has Connect, observed contributors and exact agent activity; private contexts and manual verification are secondary. Connections keeps execution placement and independent tool rights. Settings has one editor per policy, preserved granular restrictions, optional disclosed defaults and lazy diagnostics. Capture summaries and current-configuration successful-call timestamps use authorized recorded data.
- Not shipped: N/A for this slice. Ambient TV acceptance, overall visual epic closeout, new runtimes and releases remain separate.
- New blockers: None.
- Docs updated: Desktop, capture and MCP contracts; this archived pack, owning epic/index, execution indexes and `CONTINUE_HERE.md`.
- Validation: Focused backend and existing browser tests, frontend typecheck/design/build, clippy and independent live UI review passed. Final governance, whitespace and CodeGraph checks are recorded below.
- Version: N/A: no release policy.
- Commit: Uncommitted.

## Delivery evidence — 2026-10-04

- Existing backend acceptance tests passed: `capture_admission_replay_original_scope_and_standing_learning` proves exact device filtering/count/pagination beyond the first twenty receipts, safe actor/agent attribution, exact processing/learning outcomes, original scope, stale policy fences and denied Brain access. `mcp_runtime_durable_dispatch_fences_replay_cancel_and_expiry` proves observed successful-call time only under current Use and matching connection/connector-definition revisions, alongside existing durable dispatch fences. Logs: `.cache/management-capture-test.log` and `.cache/management-mcp-test.log`.
- Existing UI tests passed separately in disposable databases: `tests/mcp.spec.ts` (three cases) retains independent Use/Manage/Share, cached schemas, Brain revocation, exact runner placement and stale edits. `tests/capture.spec.ts` (one case) retains secret redaction, expiry, erasure and delivery states; its new focused concurrency case rejects an old policy draft after a refreshed change ID. Logs: `.cache/management-mcp-ui-tests.log` and `.cache/management-capture-ui-final.log`. Earlier failures exposed nested Escape handling and an outdated Devices label; both were corrected without weakening guards. UI fixtures were reconciled and cleaned by the existing harness. Failed backend fixtures remain available under the harness's diagnostic retention rule; passing fixtures cleaned themselves.
- API generation validated 179 unique operations. Frontend typecheck/design/build and `cargo clippy --workspace --all-targets -- -D warnings` passed. No dependency was added. The pre-existing large shell chunk warning remains. Logs: `.cache/management-typecheck.log`, `.cache/management-clippy.log` and the UI build logs above.
- `./scripts/stack.sh up --build` rebuilt the existing installation; `/health/ready` returned `{"ready":true}`. Read-only browser proof used the existing SWEG Brain and its real retained tool result: ready processing, last recorded succeeded learning with eight accepted outcomes, exact captured source text and an observed successful connection call. No live policy edits, revocations, provider/tool tests or new live fixtures were performed. Log: `.cache/management-stack-final.log`.
- Actual 1280 × 720 screens were independently reviewed with no remaining blocker. Final screenshots: `.cache/management-agent-activity-final.png`, `management-captured-evidence-final.png`, `management-connections-final.png`, `management-tool-access-final.png`, `management-settings-privacy-final.png` and `management-settings-ai-final.png`; earlier General, Access, retention and private-context screenshots use the same `management-` prefix. Evidence and tool-group Escape close only their inner inspectors. Advanced direct setup also keeps the main Connect flow open and restores focus when closed; its My agents link targets the account-wide roster. Screenshot: `.cache/management-connect-flow-final.png`. No credential was created. Failed capture refresh removes protected content.
- Legacy session links preserve the selected agent into Activity; the exact agent remains visibly scoped, with Show all agents. Legacy capture settings resolves to Privacy. Source/version selectors and private task identities remain exact; unavailable records never fall back to a different record. Screenshot: `.cache/management-legacy-agent-final.png`.
- Closed General was observed for 37 seconds with only Brain/list/auth/status GETs, no diagnostic history polling or mutations. A separate complete navigation window recorded 24 API GETs and zero mutations. Proof: `.cache/management-closed-feed-proof.json` and `.cache/management-navigation-proof.json`. Assurance histories run only on Dashboard or Needs attention; access/session checks remain independent, and policy refresh continues in displayed editors.
- Final `./scripts/validate.sh`, `git diff --check` and synchronized CodeGraph status complete the repository closeout. Logs: `.cache/management-validate-final.log` and `.cache/management-codegraph-final.log`. These checks establish repository consistency, not an external release.
