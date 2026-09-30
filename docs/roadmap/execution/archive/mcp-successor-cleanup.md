# Successor cleanup: device dedupe, review-UI removal, Ask/Search simplification

Status: shipped
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Ship the three decided successor slices in one change: device-identity dedupe stops Devices list growth, review/proposal/overwrite UI is removed in favor of the autonomous learning log, and Ask/Search becomes read-only simplified with Search default and explicit empty states.
- Non-goals: OAuth/broader SSO, published marketplace, OpenCode capture hooks, public-server deployment, any change to what a model may do with an approved tool, any change to canonical review/rejection rules or retrieval eligibility.
- Delivery shape: Local Rust server change plus React UI changes, focused proof, runbook updates, and reconciled epic/pack indexes.

## Governing Sources

- [Direct plugin user auth ADR](../../../adr/0015-direct-plugin-user-auth.md)
- [Plugin direct-auth contract](../../../contracts/mcp-plugin-direct-auth.md)
- [Companion pairing and device authority](../../../contracts/platform-device-pairing.md)
- [Human review and durable corrections](../../../contracts/memory-review-and-corrections.md)
- [Evidence-backed temporary answers](../../../contracts/retrieval-answers.md)
- [Autonomous memory ADR](../../../adr/0006-autonomous-memory.md)
- [Owning epic](../../epics/mcp-coordination.md)
- User instruction 2026-09-29 authorizing all three successor slices in one go.

## Scope

- In scope: device approve dedupe by normalized name per account with token rotation; removal of review/proposal/overwrite mutation UI while keeping read-only history and server APIs unchanged; Ask page Search-default plus explicit no-evidence empty states with next steps.
- Out of scope: changing scope enforcement, execution-profile evaluation, capture behavior, canonical rejection/conflict rules, retrieval ranking/eligibility, or migration of historical revoked rows beyond stopping new duplicates.
- Blockers: None. User authorized the three behaviors 2026-09-29; contracts above supply the preserved semantics.

## Surface and Interface Changes

- Interfaces: `POST /api/devices/pairings/{code}/approve` reuses the most recent unrevoked unexpired device with the same normalized name instead of inserting a duplicate row; response shapes unchanged. Web removes ClaimReview mutation forms, Revise proposal entry, and Add-memory structured proposal editor entry, replacing them with an autonomous-learning notice and read-only history. Ask defaults to Search evidence tab with clarified empty states.
- Storage: N/A: no schema or migration change; dedupe reuses existing device rows with token rotation and expiry reset.
- Ownership: server pairing lives in `crates/server/src/devices.rs`; review UI lives in `web/src/ClaimReviewDialog.tsx` and `web/src/ClaimsPanel.tsx`; Ask/Search lives in `web/src/features/ask/AskPage.tsx` and `web/src/RecallPanel.tsx`.

## Data and Authority

- Inputs: pairing name 1-120 chars, browser approval identity, Brain grants, existing device rows, claim/review history reads, recall/answer responses.
- Authority: device bearer still acts as the user with current Brain grants; execution profiles keep separate evaluation. Server review APIs remain canonical and unchanged; UI removal grants no new authority. Retrieval eligibility and scope rules are unchanged.
- Blind spots: rendered configuration remains `configured_only` until a live call succeeds; historical duplicate device rows remain until individually revoked.

## States and Edge Cases

- Loading: N/A: issuance and rendering remain synchronous besides the existing browser approval wait.
- Empty: a user with no Brain grants still sees explicit no-access; empty Ask/Search shows explicit next steps instead of silent blank.
- Error: invalid/expired/revoked tokens fail explicitly with re-auth guidance; secrets never appear in plugin files, settings, or proof output.
- Blocked: N/A beyond the recorded decisions.
- No-access: Brain-wide search denied unless permitted; in-Brain scope remains default; readers remain read-only.
- Duplicate or replay: re-issuing for the same normalized name updates one credential record and rotates its token; repeating acknowledgement stays idempotent; legacy duplicates reuse the most recent match.
- Stale data: revocation still denies new calls per existing semantics; already-issued credential handling follows the explicit mechanism.
- Reconciliation divergence: N/A: no new mutable server state beyond the reused credential record.

## Integrations and Runtime Inputs

- Providers: installed Codex, Claude Code, OpenCode v2 shapes unchanged; OpenAI server key remains operator-only.
- Environment: variable names only; never values. Existing `RECOLLECT_URL` and `RECOLLECT_DEVICE_PROFILE` patterns apply.
- Secrets: user token lives in host secret handling or OS store; never in plugin files, generated settings, URLs, or proof output.
- Failure handling: transport retry/timeout behavior reuses the existing runtime contract.

## Tests and Acceptance

- Automated: server device dedupe unit/integration proof plus existing device tests; web typecheck; `./scripts/validate.sh`; `git diff --check`.
- Manual: N/A beyond the documented commands; live-host verification is recorded acceptance, not a manual claim.
- Acceptance: second pairing with the same name reuses one device row and rotates the token; review mutation UI is absent while history and learning log remain; Ask defaults to Search with explicit empty-state guidance; revoke still denies new calls.

## Closeout

- Planned: combined successor slice delivery with proof for all three behaviors.
- Shipped: device approve reuses one row per account and normalized name with token rotation and expiry reset; review/proposal/overwrite mutation UI removed, read-only history plus autonomous-learning notice kept, server APIs unchanged; Ask defaults to Search evidence tab with explicit no-evidence next steps, retrieval semantics unchanged.
- Not shipped: OAuth, published marketplace, capture hooks, deployment; legacy duplicate rows remain until individually revoked.
- New blockers: None.
- Docs updated: owning epic slice map and dependencies, active/archive indexes, this pack, device-pairing runbook, successor-cleanup evidence mapping.
- Validation: `cargo clippy --locked -p recollect-server --all-targets` clean; `cargo test --locked -p recollect-server --test operations_offline` 3 passed; live ignored pairing test passed; web `npm run typecheck` passed; live dedupe proof reused=True with 1 live row then revoked; live web chunks contain new copy and zero review-mutation strings; `./scripts/validate.sh` 32/32; `git diff --check` clean.
- Version: N/A: no release policy.
- Commit: Uncommitted.
