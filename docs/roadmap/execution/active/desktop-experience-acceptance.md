# Desktop experience integrated acceptance

Status: in-progress
Owning epic: `docs/roadmap/epics/operational-readiness.md`
Work type: product

## Summary

- Goal: Prove the complete approved desktop experience and new read-only Ask through the running product while preserving existing data and authority.
- Non-goals: External rollout, mobile/dark-mode testing, universal performance claims, deleting user data or enabling existing Brain policies for convenience.
- Delivery shape: Focused automated/API/browser proof, local image/stack upgrade, measured evidence, operating/rollback guidance and truthful slice closeout.

## Governing Sources

- [ADR 0014](../../../adr/0014-desktop-experience-and-answers.md)
- [Desktop contract](../../../contracts/desktop-experience.md) and [answers](../../../contracts/retrieval-answers.md)
- [Integrated evaluations](../../../contracts/operations-integrated-evaluations.md)
- [Installation](../../../contracts/operations-local-and-shared.md)
- [Approved journeys](../../desktop-experience/README.md#acceptance-journeys)
- [Owning epic](../../epics/operational-readiness.md)

## Scope

- In scope: All twelve real destinations/auth states, API/agent regression and supported user journeys, answer quality/privacy/cost controls, accessibility/visual consistency, request/performance measurements and local deployment preservation.
- Out of scope: Speculative new benchmarks, replacing existing deployment/backup architecture, additional providers/host formats and automatic commit/push.
- Blockers: None for preparing fixtures/baselines. Final acceptance depends on all six feature slices plus shell/authority; incomplete required behavior remains open.

## Surface and Interface Changes

- Interfaces: No new product endpoint; validate the interfaces from owner packs and existing agent MCP. Test fixtures are isolated, attributable and cleaned up only when owned/disposable.
- Storage: Preserve current Brain count/identities, grants, sources and policies across compatible migration/image startup. New answer metadata is additive; never roll back erasure/policy state to fix visual regressions.
- Ownership: Each feature owner supplies focused tests; operations assembles integrated evidence and local runbook/rollout proof.

## Data and Authority

- Inputs: Controlled positive/negative fixtures, preserved real inventory, actual network/API/browser results and approved-provider test data only where explicitly permitted.
- Authority: Accepted contracts and user scope determine pass/fail; screenshots/typecheck cannot stand in for behavior.
- Blind spots: Fixture correctness is not production capacity or a live external connector proof. State exact environment, sampled data and failed attempts.

## States and Edge Cases

- Loading: Test pending navigation and late responses across Brain/route/selection changes.
- Empty: Verify new/empty Brain and no-match views versus disabled or unconfigured capability.
- Error: Test safe model/graph/feed/auth failures and useful recovery with no stale sensitive payload.
- Blocked: Unavailable optional provider/tool falls back or explains prerequisites; required missing implementation remains unshipped.
- No-access: Two-Brain/two-account/current-revocation/private-task/profile-grant tests retain useful permitted controls.
- Duplicate or replay: Explicit stable Ask IDs and canonical commands do not repeat paid/effectful work; test cancellation and restart uncertainty.
- Stale data: Correction/erasure/expiry and policy/grant changes suppress in-flight and displayed content across owners.
- Reconciliation divergence: Inspect actual uncertainty, missing coverage, source absence and model usage; do not relabel incomplete proof as connected.

## Integrations and Runtime Inputs

- Providers: Existing local Compose stack plus deterministic model/tool fixtures; a real configured-provider probe is reported separately and uses explicitly allowed test content/policy.
- Environment: Existing installation/build/native test inputs; record names and role, never credentials.
- Secrets: No secrets or raw customer/model payloads in screenshot fixtures, reports or commands; preserve ignored secret files.
- Failure handling: Rebuild/start the authorized root stack through its runbook. For visual regression serve the previous compatible frontend/image; disable answering separately while keeping exact/text search. Do not restore old grants/retention data.

## Tests and Acceptance

- Automated: Meaningful Rust/database/API tests from owner packs, frontend generation/typecheck/build, route/auth/domain browser regressions, selected visual comparisons, ./scripts/validate.sh and git diff --check.
- Manual: All twelve routes and auth/expired/reader/archived states at 1280x800, 1440x900/960 and 1920x1080/1200; fonts/icons/logo, focus/keyboard/zoom/reduced motion, graph list alternative, long labels and inspector return.
- Acceptance: Human question→qualified answer→exact evidence→authorized correction→subsequent recall; disabled answer→useful text search; supported agent scope/recall/contribution remains autonomous; guided permitted connection/call; bounded graph/path; Activity recovery; pending Brain switch isolation. Record mounted request counts and comparable p95 navigation/recall/answer stages/graph responsiveness without unsupported speed claims. Existing stack inventory and readiness survive upgrade. All required owner checks pass before shipped lifecycle states.

## Closeout

- Planned: Complete approved desktop implementation, governed Ask and actual local runtime proof.
- Shipped: Not yet. Final local image 5126dccc is ready at port 8787 and preserves seven Brain identities, sixteen sources, seventeen versions, ten claims, fourteen revisions and all recorded policy/grant digests. Desktop seven, real Team/OIDC two, Ask five, MCP setup three/runtime three and completed domain cases are recorded. Workspace clippy/unit, assets, backend Ask fixtures, actual provider samples and bounded timing proof passed.
- Not shipped: Corrected remaining domain runs, relocated model-policy/autonomous UI and the integrated citation/correction/subsequent-recall journey are still required. Native browser-chrome zoom, general canvas interaction p95, universal performance and external installation/recovery reruns are not claimed. Keep required incomplete checks visible; explicit product non-goals remain excluded.
- New blockers: No unresolved product decision is known. Neo4j startup temporarily interrupted two owned UI fixtures; dependency health recovered, both fixtures were reconciled/removed, and migration preflight passed at 20:29 UTC. The remaining limitation is unfinished verification, not a claimed product fix.
- Docs updated: [Desktop guide](../../../runbooks/desktop-experience.md), [implementation/assets/dependency evidence](../../../mappings/desktop-experience-implementation-2026-09-26.md), [current handoff](../../../../CONTINUE_HERE.md), affected domain runbooks, owning epic and indexes.
- Validation: Frontend typecheck/design and final image build passed; workspace clippy and 21 unit tests passed with 3 live-Vault cases explicitly ignored; governance lint and 32 tests passed. Desktop seven cases passed across a six-pass run and the repaired font-fallback targeted rerun; real Team/OIDC two, Ask five and MCP setup/runtime six passed. The dated mapping separates each owner result, initial failures, fixture/provider boundaries and remaining checks.
- Version: N/A: no release requested.
- Commit: uncommitted.
