# Retention, controlled erasure and replay evidence

Observed: 2026-09-14
Confidence: verified for delivered local consumers; successor integrations remain separate

## Method and sources

Implemented the accepted [contract](../contracts/memory-retention-and-erasure.md) on
the existing Rust/Axum, PostgreSQL 17/SQLx 0.8.6 and React/Mantine stack. Context7's
PostgreSQL lookup returned primary documentation for
[policy combination](https://www.postgresql.org/docs/17/sql-createpolicy.html),
[row security](https://www.postgresql.org/docs/17/ddl-rowsecurity.html) and
[transaction locks](https://www.postgresql.org/docs/17/functions-admin.html).
Canonical operations share the Brain row lock. Private helpers accept authorized
targets or committed requests; only the operator role can replay supplied journal
closures. No hashes, strict format gate or new external service was introduced.
Server integration tests reuse the local native companion crate.

## Core and native proof

The full integration suite passed **19 core scenarios in 15.93 seconds**, excluding
optional live OIDC. Five compound retention scenarios passed together in 4.07 seconds:

- Deadlines deny raw reads, new support, excerpts and processing before file cleanup.
  A permitted exact-line excerpt and unrelated document remain readable. Physical
  expiry removes bytes; extending policy does not restore them. Claim expiry leaves
  independently retained excerpts. Reader/device/foreign controls and receipt fences pass.
- Source erasure follows excerpt/support dependencies, clears affected revisions,
  co-supported content, rules and review payloads, and preserves independent revisions.
  Current erased intervals do not select older assertions. Stale preview/lease fail.
  An injected audit failure rolls back requests, redaction and receipts; retry succeeds.
- Collection erasure reports shared sources and removes them throughout the Brain,
  preserving unrelated sources. Whole-claim/manifest erasure covers every revision
  without deleting independent inputs. Archived cleanup and foreign-role RLS pass.
- Missing journal and malformed artifact storage report errors/pending work. Cleanup
  continues after the initiating admin loses access. An actual PostgreSQL copy made
  before erasure fails the startup barrier until replay clears restored source/claim
  payloads and files. A restored file cannot survive behind a completed DB request.
  Missing acknowledged journal entries fail closed.
- Real publication, manifest/claim dependence and snapshot erasure preserve a second
  snapshot. New adapter/publication IDs cannot bypass a commit fence. A real local
  HTTP server/native client removes the exact managed bundle and records acknowledgement.
  Other Brain/device bundles remain; cached expiry works after the server stops.

The existing native exact-Git extraction/resumable-publication test passed in 2.39
seconds. Clippy with warnings denied and generated OpenAPI passed; there are 93 unique
API operations. Tests exposed a PL/pgSQL variable ambiguity and reserved argument name,
both corrected. The native fixture initially drained capture instead of the publication
heavy lane; it was corrected before asserting actual fact support. Failed test databases
were removed only after their ownership comments were checked.

## Browser and runtime boundary

The combined retention/claims/review browser run passed all three workflows in
**20.5 seconds**, then a restarted native worker drained the persisted queue.
Retention proof changed policy, retained exact lines, previewed erasure, interrupted
its response after commit, retried after background refresh and confirmed one request
with completed journal/file cleanup. Desktop/mobile screenshots were inspected;
no horizontal overflow or uncaught browser errors occurred. A duplicate button
attribute found by TypeScript was corrected before the passing run.

The final added browser check passed in 7.0 seconds: a dependent erased claim
appears as a content-free history marker with inspection and whole-claim erasure
available. Its API assertion passed separately in 0.90 seconds; final Clippy passed.

The normal local runtime applied migration 010 at `2026-09-14T14:08:02.088494Z`
and resumed serving at `14:08:08.782106Z` with bundle `index-DoJ4y138.js`.
At `14:08:39.149Z` an authenticated browser verified readiness, the retention UI
and unchanged SWEG claim/evidence/review history. Both existing Brains remain open;
the SWEG snapshot still has 99 facts, 37 inventory files and zero retained raw files.
Its claim retains its original proposed/browser-authored revision and no review
decisions. The journal is initialized, there are zero erasure requests, raw defaults
are 30 days, and repository/claim defaults have no automatic expiry.

No customer source was erased or edited. The selected Luna/embedding-3-large pair
is unchanged; retention makes no model calls and does not implement the model gateway.
