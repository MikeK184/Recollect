# Brain identity and focused workspace navigation

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Keep Brains a chooser, support uploaded Brain icons, and consolidate account credential management under Agents.
- Non-goals: A new dashboard, TV completion, or expanding private task/payload visibility.
- Delivery shape: Local Rust/web implementation, local deployment and browser proof; separate generated Agents comparison.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md), amended for the user's October 3 browser feedback.
- [Device pairing](../../../contracts/platform-device-pairing.md).
- [Brain deletion](../../../contracts/platform-brain-deletion.md).
- [Owning epic](../../epics/desktop-visual-experience.md).

## Scope

- In scope: Remove global pipeline; open Brain cards directly; PNG/SVG/ICO upload, replacement and removal in Brain creation/editing; authenticated icon rendering; Agents access drawer and Devices compatibility routing; approved grouped Agents list with account-only global scope and Brain-scoped cross-account attribution.
- Out of scope: Changing processing attribution, permission grants, native pairing protocol, or credential lifetimes.
- Blockers: None. User selected A and clarified own agents globally, all observed contributors within an accessible Brain.

## Surface and Interface Changes

- Interfaces: GET/PUT/DELETE `/api/brains/{id}/icon`; PUT accepts bounded PNG bytes, returns Brain metadata. Brain DTO adds nullable icon revision. `/devices` redirects to `/agents?access=true`; opaque pairing code survives in the Agents drawer and sign-in return.
- Interfaces also scope `/api/agents` to the caller for every account; per-Brain roster shares only observed identity/host/use metadata and marks own credentials as revocable. Foreign credential timestamps and private payloads remain private.
- Storage: Migration 032 adds bounded PNG bytes and revision on the canonical Brain row. No originals or image bytes in receipts/audit. Migration 033 exposes a narrowly authorized Brain/device last-use projection without widening private MCP payload RLS. Existing whole-Brain deletion and restore replay remove the row atomically.
- Ownership: Browser normalizes PNG/SVG/ICO to a maximum 256px PNG using image-context rendering, never inline SVG. Server independently decodes only PNG with strict dimension limits, normalizes and re-encodes. Brain admin/browser authority matches metadata editing, with existing CSRF. Reads require Brain visibility and use no-store/nosniff.

## Data and Authority

- Inputs: Actual authorized Brain metadata, own-account devices, opaque pairing code; uploaded artwork only.
- Authority: PostgreSQL Brain row and existing device/pairing handlers. Existing per-Brain Activity remains the processing home.
- Blind spots: Pairing is not online state or evidence of Brain use. Generated Agents proposal contains labeled illustrative content.

## States and Edge Cases

- Loading: Upload preparation/save disables repeated submissions; images have neutral fallback.
- Empty: Generic icon; no fake image or activity. Own unused credentials remain reachable.
- Error: Malformed/oversized images fail explicitly. Partial Brain-create success is retained for icon retry without creating a duplicate.
- Blocked: Archived/deleting Brain follows existing write fences. Image preparation rejects external SVG resources and excessive SVG element counts.
- No-access: Authenticated reads, admin mutations, existing RLS/fences; no public icon URL or persistent browser image cache. Failed roster does not prevent own access controls/pairing.
- Duplicate or replay: PUT replaces bytes atomically; DELETE is harmless when absent. No image-bearing idempotency receipts. Metadata create retains its existing receipt.
- Stale data: Icon revision changes the image URL; query invalidation refreshes chooser/shell after mutation. Credential polling/failure handling preserved.
- Reconciliation divergence: No new artifact store or cleanup queue; icon lives and dies with Brain row.

## Integrations and Runtime Inputs

- Providers: No external upload or image service. Existing managed-memory creation disclosure remains.
- Environment: Existing local Compose, test database and browser credentials only.
- Secrets: No environment values, bearer credentials or private pairing codes in evidence.
- Failure handling: Conversion and API errors stay visible; icon failure cannot reissue Brain creation; existing pairing expiry/decline/revoke handling preserved.

## Tests and Acceptance

- Automated: Real database icon handlers including role isolation, CSRF, malformed/oversized decode, replacement/removal and deletion; browser chooser, upload, conversion failure/retry, access drawer, pairing redirect/history tests; typecheck/build, clippy and validate.sh.
- Manual: Local deployed chooser without activity, preserved per-Brain Activity, icon preview, Agents access drawer and generated comparison.
- Acceptance: Brain selection is direct; uploaded PNG/SVG/ICO displays as a static normalized icon; Devices has no independent normal page; pairing and own-account revocation still work. Reviewer checks implementation.

## Closeout

- Planned: Authorized chooser/icon/navigation changes and approved personal grouped Agents list.
- Shipped: Direct Brain chooser, uploaded normalized icons, personal Agents grouped by used Brain, shared-Brain contributor metadata and Agents access drawer with pairing redirects. Rebuilt and verified on the existing local stack; [dated evidence](../../../mappings/desktop-brain-identity-2026-10-03.md).
- Not shipped: TV completion and overall visual epic closeout remain separate; no external release.
- New blockers: None.
- Docs updated: Desktop/pairing contracts, epic/index, archived pack/index, mappings/index and handoff.
- Validation: 3 real database/HTTP tests, 2 decoder tests, 9 browser cases, build/typecheck/design, all-target clippy, independent review, CodeGraph and 32 governance checks passed. Local browser proof has no page errors and unchanged inventory.
- Version: N/A; no release policy.
- Commit: Uncommitted.
