# Unified Privacy form

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: One Privacy form with title separators and shared editing actions.
- Non-goals: New policy endpoints, cross-domain transactions or backup scheduling.
- Delivery shape: Local frontend correction and current recovery source audit.

## Governing Sources

- [Desktop correction](../../../contracts/desktop-experience.md)
- [Retention/erasure](../../../contracts/memory-retention-and-erasure.md)
- [Recovery adapter](../../../contracts/operations-recovery-drills.md)
- [Capture policy](../../../contracts/evidence-session-capture.md)
- [Visual epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Unified fields, title separators, one Edit/Save/Cancel; remove footer
  AI link and Activity/backup explanation text; preserve all existing controls.
- Out of scope: Schema/backend changes, backup operation or real policy saves.
- Blockers: None; user supplied presentation authority and existing commands apply.

## Surface and Interface Changes

- Interfaces: Existing capture, retention, evidence and repository policy commands.
- Storage: N/A; no migration or changed policy defaults.
- Ownership: Privacy page presentation and draft/save orchestration only.

## Data and Authority

- Inputs: Four current policy reads and full-preserving drafts.
- Authority: Existing admin/archive, revisions, idempotency and storage drift checks.
- Blind spots: Storage booleans lack server CAS; preserve frontend observed-value
  drift guards without claiming atomicity. Recovery tooling is not schedule proof.

## States and Edge Cases

- Loading: Existing policy loading state before fields are usable.
- Empty: Optional retention blanks mean until erased; empty exclusions hidden in read.
- Error: Keep unfinished drafts, identify acknowledged domains and permit bounded retry.
- Blocked: Stale reads require Cancel/reload; no silent baseline replacement.
- No-access: Read-only fields; editing clears on archive or authority loss.
- Duplicate or replay: Capture/retention keep stable input keys; retry skips successful
  domains. Storage retains current canonical boolean commands.
- Stale data: Fresh pre-save read and existing capture/retention server revisions.
- Reconciliation divergence: One Save is not atomic; Cancel never rolls back completed
  writes. Retention is applied last and its shortening warning remains visible.

## Integrations and Runtime Inputs

- Providers: Existing bundled React/Mantine and API client; no new dependency.
- Environment: Existing local Compose stack, no new variables.
- Secrets: No credential entry; existing capture exclusion rules preserved.
- Failure handling: No automatic write retry; explicit retry uses acknowledged baseline.

## Tests and Acceptance

- Automated: Meaningful pure partial-save/changed-domain/stale-policy checks;
  production build/type/design, governance and whitespace.
- Manual: CUA one-form read/edit, several domains edited then Cancel; all policies
  unchanged. Independent source and actual screenshot review.
- Acceptance: One surface and one action group, no per-section Edit/card or footer
  AI link, all required fields and saved custom rules preserved; no Activity/backup
  explanations or false backup execution claim.

## Closeout

- Planned: Unified form, concise recovery fields and truthful source audit.
- Shipped: One named form, simple title separators and shared Edit/Save/Cancel;
  every policy field retained, no footer AI link or Activity/backup explanations.
  Per-domain acknowledgments preserve unfinished drafts and bounded explicit retry.
- Not shipped: New atomic API or backup schedule; unrelated native-host acceptance.
- New blockers: None.
- Docs updated: Desktop/managed amendments, this archived pack, epic/index,
  [dated evidence](../../../mappings/desktop-unified-privacy-form-2026-10-06.md),
  mappings/active/archive indexes and handoff.
- Validation: Four pure draft/save checks, production type/design/build checks,
  32 governance tests, whitespace and CodeGraph synchronization pass. Rebuilt
  local stack is healthy and ready. CUA edited four domains then Cancelled;
  SQL confirmed original policies and revisions. Independent final source and
  labeled read/edit pixel review accepted the form without visual blockers.
  Source audit confirms implemented activity-detail expiry and recovery archive
  windows; no fresh backup, schedule inspection or policy write used for proof.
- Version: N/A; no release requested.
- Commit: Uncommitted.
