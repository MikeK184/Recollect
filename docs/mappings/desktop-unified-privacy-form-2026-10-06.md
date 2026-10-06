# Unified Privacy form and current recovery-field audit

Observed: 2026-10-06
Confidence: verified

## Sources and Method

- [Accepted latest Privacy correction](../contracts/desktop-experience.md#unified-privacy-form-follow-up--2026-10-06),
  [retention](../contracts/memory-retention-and-erasure.md) and
  [recovery adapter](../contracts/operations-recovery-drills.md).
- Current `PrivacyOverview.tsx`, `PrivacyForm.tsx`, `privacyDraft.ts` and
  `privacy-form.css`; existing capture/retention/storage command implementations.
- Four pure `privacy-draft.spec.ts` checks passed without a browser or database
  fixture: partial save/retry, all revision/storage drift cases, unchanged/invalid
  no-write cases, and accepted 16385-byte capture preservation during unrelated
  retention edits. Capture exclusions and original draft inputs remain intact.
- Production build passed type, design and Vite checks; local Compose rebuild
  log `.cache/unified-privacy-stack-build.log`. API/worker/PostgreSQL/Neo4j healthy,
  readiness returned `{"ready":true}`. CodeGraph synchronized.
- Repository validation passed 32 governance tests and whitespace checks.
- Existing capture/retention/management browser regression sources were updated
  for the shared form and discovery parsed all four cases successfully. Those
  fixture-writing browser cases were not run against the real Brain.
- Parent CUA used the user's local Brain at the actual 1384 × 1473 viewport.
  Independent agent audited recovery and source; final screenshots reviewed
  separately from the pure mutation tests.

## Observations

The Privacy tab contains one named form and one shared action group. All previous
raw retention, capture kinds/limit/exclusions, durable retention/support flags,
document/repository storage and activity/backup fields remain. Titles are simple
separators within one surface. There are no per-section Edit buttons, card borders,
disclosures or bottom AI transmission button. Activity/backup fields have their
labels and day values, with no explanatory paragraphs.

CUA entered the shared Edit mode, changed raw retention 30→31 days, capture
32→64 KiB, document storage off and repository file text on, then clicked Cancel.
All original values returned; no Save or policy command was executed for proof.
Read-only SQL before/after confirmed capture revision
`4493b7bf-dfca-4a25-8fc9-7495e0b3560b`, retention revision
`3f6ac20f-c3e6-4ebb-b040-40366d3bc3b4`, 32768-byte capture, 365-day audit,
7-day backup, document content allowed and repository text disabled unchanged.

The source reviewer found an initial overly narrow whole-KiB validation rule;
it was corrected to preserve every canonical accepted integer-byte limit. The
16385-byte fixture verifies unrelated edits remain possible. Partial-failure text
was also corrected to call remaining work a draft rather than claim an uncertain
request did not commit. Initial live pixels exposed oversized Switch hit areas
and grey checked read switches; final CSS constrains hit areas and retains sage
checked state without horizontal scrolling.
Final read/edit pixels include visible Questions and Replies labels next to
their paired switches. Independent reviewer confirmed the remaining clarity
issue resolved and no visual blockers. Read-view document width 1369 px fits
the 1384 px viewport. The browser was left in its saved read view.

### Activity and backup behavior

Current source audit, not a new backup operation:

- Migration 010 maps `audit_days` and `recollect_expire_audit` cleans eligible
  mutation details; worker maintenance invokes the privacy-journal cleanup.
  Provider, answer and handover detail paths also enforce the audit deadline.
  Minimal operational anchors and rows still referenced by jobs may remain;
  Activity detail is not a promise to erase every activity identity wholesale.
- `scripts/recovery_installation.py` inventories the shortest included Brain
  backup window. `scripts/recovery.py` stamps archive expiry, prunes owned
  archives against captured/current shorter windows and rejects expired restore.
  The adapter is implemented; the former UI's "forthcoming" sentence was stale.
- Available recovery tooling and a policy value do not establish a configured
  schedule, existing backup or successful recent run. No backup creation, pruning,
  restore or scheduling was performed in this UI correction.

## Translation and Limits

One UI Save sends only changed domains through four existing commands. Capture
and retention keep separate observed revisions and stable input idempotency keys.
Document/repository booleans retain fresh-read observed-value guards, without a
new server CAS or cross-domain transaction. Each successful response updates its
baseline/cache; explicit retry skips acknowledged work. Retention is applied last.
If a later command fails, the form retains drafts and names acknowledged domains;
Cancel discards remaining edits and does not undo successful commands.

The shared form disables editing on read errors, archive/authority loss or stale
state; pending controls cannot change an in-flight request. Changed retention
retains the accepted immediate-expiry warning. Stored arrays and untouched fields
are preserved, including empty/optional durable values and odd-byte limits.

Version N/A, work local/uncommitted. No push, external release, real policy save or
new backend/schema/dependency. Original seven-point evidence remains historical;
the latest correction supersedes its separate-card Privacy layout.

## Follow-up

Final read/edit screenshots:
[single form](../../output/browser-review-2026-10-06/privacy-unified.jpg),
[shared draft](../../output/browser-review-2026-10-06/privacy-unified-edit.jpg).
Reconciled pack records final independent pixel acceptance and repository checks.
