# Guided setup and automatic privacy

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Achievable cream/ink/sage designs, one visible agent setup step, compact private context, one Add connection entry and automatic privacy summaries.
- Non-goals: New dependencies, launches on navigation, automatic policy expansion, remote process shutdown or ambient TV closeout.
- Delivery shape: Existing components and mutation interfaces, bounded authorized runtime observations, generated proposals and independent UI review.

## Governing Sources

- [Desktop knowledge surface](../../../contracts/desktop-knowledge-surface.md)
- [Desktop experience](../../../contracts/desktop-experience.md)
- [Autonomous memory](../../../adr/0006-autonomous-memory.md)
- [Managed MCP runtime](../../../contracts/mcp-runtime-and-credentials.md)
- [Private workspace scope](../../../contracts/evidence-workspace-scope.md)
- [Retention](../../../contracts/memory-retention-and-erasure.md)

## Scope

- In scope: Plugin/direct setup, private context rows, connector chooser, observed sessions, independent tool access and privacy overview/customization.
- Out of scope: New authorization, automated server probes, blanket Start/Stop controls, consent changes and credential persistence in the browser.
- Blockers: None; user authorized the designs and implementation without another approval.

## Surface and Interface Changes

- Interfaces: Existing policy and connection writes with revision fences. Runtime read includes an authorized current-configuration qualification for observed sessions. No write on step navigation. Approved connector reuse remains available to Brain administrators; registering new definitions requires installation ownership.
- Storage: No migration or policy reset. Direct credential remains in memory until the wizard closes.
- Ownership: Actor-scoped contexts; existing independent Use/Manage/Share authority remains server-enforced.

## Data and Authority

- Inputs: Current policy reads, authorized catalogue and bounded runtime instances.
- Authority: Actual server state; copied instructions never establish a live connection. Private checkouts are observations, not access grants or shared repository content.
- Blind spots: A bounded runtime page cannot establish absence of every instance. Remote service processes are not owned by Recollect. Session release spans the actor's exact client session, not one server.

## States and Edge Cases

- Loading: Selected surface only, bounded placeholders.
- Empty: No observed records; no inferred failure.
- Error: Remove stale protected data, show read failure and explicit retry.
- Blocked: Current permissions and archived Brain gates remain.
- No-access: Never expose foreign private context or widen connector registration rights.
- Duplicate or replay: Back/Next never creates another token; explicit token creation retains existing pairing completion/uncertain-outcome rules.
- Stale data: Preserve original change IDs, runtime lease truth and configuration revisions.
- Reconciliation divergence: Custom/off privacy remains custom/off; customization uses separate saves for independent policy APIs.

## Integrations and Runtime Inputs

- Providers: No new libraries or providers. MCP instances start on permitted calls; remote process state is unavailable.
- Environment: Existing localhost stack.
- Secrets: One-time credential only in wizard memory; never logs, fixtures or generated designs.
- Failure handling: Existing access refresh, cancellation and explicit errors. No implicit fallback or probes.

## Tests and Acceptance

- Automated: Frontend typecheck/design/build, existing focused UI checks, relevant backend proof if runtime projection changes, clippy, repository validator and whitespace. No gratuitous tests or hashes.
- Manual: Independent review of settled screenshots from the existing disposable local UI harness covering wizard steps/back, contexts, connector chooser, connection/tool-access views and policy customization at desktop bounds. Production deployment is verified separately.
- Acceptance: One step visible; actual commands; no navigation mutations; one Add entry; observable lifecycle accurately qualified; current actual privacy with optional customization; existing permission/retention semantics intact.

## Closeout

- Planned: Implement approved guided setup and controls.
- Shipped: Implemented, browser-tested and deployed to the existing local stack. One-stage plugin/direct setup, compact private context, one connection chooser, bounded observed sessions, explicit Pause/Enable use, independent tool rights and actual automatic privacy summaries with secondary canonical customization.
- Not shipped: Ambient TV and overall epic closeout remain separate.
- New blockers: None.
- Docs updated: Desktop and runtime contracts; this archived pack, owning epic/index, execution indexes and `CONTINUE_HERE.md`.
- Validation: Existing focused backend and seven local UI cases, frontend typecheck/design/build, clippy, independent source/screenshot review and final deployment readiness passed. Final governance, whitespace and synchronized CodeGraph evidence is recorded below.
- Version: N/A.
- Commit: Uncommitted.

## Delivery evidence — 2026-10-04

- Two generated cream/ink/sage proposal boards preceded implementation. They are illustrative designs, not runtime proof or product assets. Existing Mantine, Motion and CSS supply the result; no dependency or additional runtime was added. Context7's Mantine 8 Stepper and overlay guidance informed the single-stage wizard and secondary editors.
- `tests/mcp-direct.spec.ts` passed both local cases: Back/Next retains one explicit token, actual ordinary HTTP MCP reads succeed, revocation works, and server-address discovery does not call tools. The external Context7 call remains deliberately skipped without its opt-in. Log: `.cache/guided-direct-ui-final.log`.
- Existing capture and two catalogue UI cases passed with actual isolated fixture records: restrictive/off capture remains off on navigation; policy concurrency, redaction, expiry and erasure remain enforced; connector choice and Pause/Enable preserve exact configuration; Use/Manage/Share and Brain revocation stay independent. Log: `.cache/guided-management-ui-final.log`. The runner case passed separately after a navigation timeout: exact paired-device placement, offline truth and stale edits remain enforced. Log: `.cache/guided-runner-ui-final.log`. An earlier empty-device assertion was corrected to consider account devices already paired by the preceding capture test. No new test files were added.
- Focused existing backend acceptance `mcp_runtime_durable_dispatch_fences_replay_cancel_and_expiry` passed, including current-configuration qualification after connection and definition revisions change. The authorized read stays bounded to 50 sessions and retains runner lease-loss handling. Log: `.cache/guided-runtime-test.log`. Frontend design/typecheck/build, 179 unique API operation validation and clippy passed; the existing shell chunk warning remains. Logs: `.cache/guided-build.log`, `.cache/guided-typecheck-final.log`, `.cache/guided-clippy.log` and the UI build logs above.
- Independent source and actual screenshot review passed at 1440 × 960/1000. Screens: `.cache/guided-setup-install.png`, `guided-contexts.png`, `guided-connections.png`, `guided-tool-access.png`, `guided-privacy-off.png` and `guided-privacy-custom.png`. The review caught stretched Privacy badges; corrected selectors and tighter spacing were recaptured and accepted, including both footer actions. Screens contain owned UI-test data, not fabricated metrics or production records.
- The existing local stack was rebuilt with `./scripts/stack.sh up --build`; `/health/ready` returned `{"ready":true}` and the served CSS contains final badge sizing, privacy spacing and command wrapping. Log: `.cache/guided-stack-final.log`. No live grant/policy edits, revocations, server tests or fixture creation were performed. In-app browser control timed out, so visual acceptance used the repository's browser harness; it does not claim a fresh production-browser interaction.
- Existing `tests/retention.spec.ts` passed the full customization/excerpt/erasure case. Saving the actual 14-day policy closes only its editor and keeps Customize open; Brain-scoped protected content resets while root authority refreshes without unmounting the page. The excerpt modal now registers with its parent overlay; the test waits for focus restoration before the second Escape. Exact retained excerpt text, lost-response erasure recovery and unavailable historical memory remain verified. Obsolete source-history navigation was removed from this existing case because Sources already uses one inspector. Log: `.cache/guided-retention-ui-final.log`.
- Final `./scripts/validate.sh`, `git diff --check` and CodeGraph sync/status complete the lifecycle closeout. Logs: `.cache/guided-validate-final.log` and `.cache/guided-codegraph-final.log`. These are repository and local-runtime checks, not an external release; version is N/A and changes remain uncommitted.
