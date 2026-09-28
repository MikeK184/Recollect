# Contextual desktop shell and shared visual system

Status: shipped
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: product

## Summary

- Goal: Navigate twelve real desktop destinations with one consistent Atlas-inspired light visual system and original Recollect SVG identity.
- Non-goals: Domain authority replacement, mobile, dark mode, framework migration and external deployment.
- Delivery shape: Local frontend, licensed assets, focused browser proof and operating documentation.

## Governing Sources

- [ADR 0014](../../../adr/0014-desktop-experience-and-answers.md)
- [Desktop contract](../../../contracts/desktop-experience.md)
- [Approved tokens](../../desktop-experience/design-system.md)
- [Team access](../../../contracts/platform-team-access.md) and [devices](../../../contracts/platform-device-pairing.md)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: Tokens/theme/fonts/logo, shared components, global Brains/Team/Devices/auth views, contextual Brain shell and real feature routes, navigation and safe state boundaries.
- Out of scope: Domain feature semantics remain with their named slices; no persistent chat or implicit model/tool action.
- Blockers: None; explicit user approval establishes the selected design. Feature route content integrates from its owning slices.

## Surface and Interface Changes

- Interfaces: Twelve paths and validated tab/filter/detail state in the desktop contract; legacy Brain links remain aliases to Ask/search.
- Storage: N/A: shell does not change canonical data. Protected context is memory-only; no new localStorage/body cache.
- Ownership: Platform owns app router/shell/design/components/assets and global identity views. Feature owners compose pages and canonical hooks.

## Data and Authority

- Inputs: Current session, authorized Brain catalogue, route state and actual health metadata.
- Authority: Server session/grants and existing identity handlers; navigation visibility never replaces authorization.
- Blind spots: Configured hosts/services do not prove a successful call; generated concept data is never runtime content.

## States and Edge Cases

- Loading: Page skeleton/async state; no previous Brain content under a new route.
- Empty: Clear new-Brain/global-list state and authorized setup action.
- Error: Safe session/API error with bounded retry and protected-cache clearing.
- Blocked: Unavailable service or unsupported feature explains the actual prerequisite.
- No-access: Team owner boundary, Brain checks and current-session loss clear content.
- Duplicate or replay: Navigation creates no mutation/model/tool request; existing command idempotency remains.
- Stale data: Cancel superseded queries; logout/Brain switch/epoch/expiry invalidates applicable payload.
- Reconciliation divergence: Labels distinguish configured, observed and degraded status; no invented status/count.

## Integrations and Runtime Inputs

- Providers: Existing same-origin Rust API; Lucide, Mantine, TanStack and licensed self-hosted fonts.
- Environment: Existing Vite/static service inputs; N/A for new production runtime variables because no new service is added.
- Secrets: Existing session transport only; no secrets or private paths in URLs/assets/logs.
- Failure handling: Preserve existing domain retries and validity polling; route failures do not repeat effectful operations.

## Tests and Acceptance

- Automated: Frontend typecheck/build, navigation/direct-link/Back/forward and role fixtures, protected-state cancellation, SVG/font asset checks and `./scripts/validate.sh`.
- Manual: All twelve paths and auth flows at 1280/1440/1920 desktop widths; font loading, shared heading sizes/icons, keyboard/focus/zoom/reduced motion and logo legibility.
- Acceptance: Existing identity/global actions work in the new shell; only active route feature polling mounts; no policy or data reset; route-specific acceptance closes in owner packs.

## Closeout

- Planned: Complete shell/design/global route behavior specified above.
- Shipped: Shared light tokens/components, four pinned font files and notices, original SVG marks/wordmark, Vite-only component reference, three global and nine contextual Brain routes, exact resource/filter history and protected-state clearing. All seven desktop cases passed, plus two device and two owned-Dex Team/OIDC cases. The final local image is ready at port 8787 with preserved recorded Brain inventory and policies.
- Not shipped: The explicit product non-goals remain excluded: mobile, dark mode, framework replacement and external rollout. Native browser-chrome zoom was not independently exercised; documented emulated scaling and 200% text/reflow checks are the evidence boundary. Domain workflow acceptance closes in its own packs.
- New blockers: None for this shell slice. Remaining domain/integrated verification is tracked in the dated mapping and active acceptance pack.
- Docs updated: [Desktop guide](../../../runbooks/desktop-experience.md), [implementation/assets/dependency evidence](../../../mappings/desktop-experience-implementation-2026-09-26.md), [current handoff](../../../../CONTINUE_HERE.md), affected domain runbooks, owning epic and indexes.
- Validation: Frontend typecheck/design and final image build passed; workspace clippy and 21 unit tests passed with 3 live-Vault cases explicitly ignored; governance lint and 32 tests passed. Desktop seven cases passed across a six-pass run and the repaired font-fallback targeted rerun; real Team/OIDC two, Ask five and MCP setup/runtime six passed. The dated mapping separates each owner result, initial failures, fixture/provider boundaries and remaining checks.
- Version: N/A: no release requested.
- Commit: uncommitted.
