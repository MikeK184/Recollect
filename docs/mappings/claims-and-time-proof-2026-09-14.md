# Claims and temporal history dependency and local proof

Observed: 2026-09-14
Confidence: verified

## Sources and Method

Implemented [ADR 0005](../adr/0005-canonical-claims-and-time.md) and the
[claims contract](../contracts/memory-claims-and-time.md) using the existing Rust,
SQLx, Axum, PostgreSQL 17 and React/Mantine dependency set. Context7 resolved
`/websites/postgresql_17` and returned official
[timestamp functions](https://www.postgresql.org/docs/17/functions-datetime.html)
and [RLS behavior](https://www.postgresql.org/docs/17/ddl-rowsecurity.html).
`clock_timestamp()` advances during a transaction; transaction timestamps do not.
Knowledge times are assigned after the shared Brain writer lock. Migration 008
also changes new source/manifest evidence timestamps to wall-clock assignment after
that lock, avoiding transaction-start ordering when writers wait. No new dependency,
hash or strict product version gate was added.

CodeGraph status/sync and exact-symbol inspection located the existing operation
and Brain lock helpers; source reads established authority. One unsupported
`node --json` option failed, then the normal `node` command succeeded. Context7
lookups succeeded. No external source or OpenAI call was needed for claim authoring.

## Observations

- Two new API/database scenarios passed against independent disposable databases.
  The complete core suite passed 11 scenarios in 7.41 seconds; the optional OIDC
  scenario remained filtered and is not newly claimed by this run.
- Concurrent identical commands produced one result. Concurrent edits admitted
  one revision and rejected the stale one. Canonical revisions, audit and durable
  refresh work retained equal counts; direct application-role UPDATE on historical
  claim rows failed. Reconstructed application state read the retained records.
- Late evidence changed the assertion for an earlier fact interval while an earlier
  knowledge-time query still returned the previous assertion. Knowledge intervals
  used the next revision's time. Half-open endpoints, unknown bounds and minute
  observation buckets had positive/negative controls.
- Proposed operational assessments did not become accepted context. An accepted
  eligibility fixture required actual reviewer/policy attribution; missing authority,
  changed evidence and unavailable support blocked strict eligibility. Unknown input
  review fields and configured credential material were rejected before persistence.
- Exact document versions retained their original text after later versions arrived.
  Current freshness changed while the prior knowledge view stayed current for its
  time. A missing file and same-length invalid UTF-8 were visible limitations;
  restoring the fixture restored readable evidence. Reference-only support remained
  explicitly qualified. Source spans were validated against actual retained lines.
- Production and development selected separate immutable manifests. Updating an
  unrelated repository/notes left Production current; changing Development's supporting
  repository qualified only that claim. Foreign support IDs and mismatched fact,
  repository/environment and device operation scopes were denied. Revocation and
  archived Brain checks applied to reads, new writes and replay with positive controls.
- The new browser workflow passed in 8.3 seconds. It authored a supported claim,
  inspected original text, retried an interrupted save, rejected a competing stale
  editor and displayed prior knowledge without offering edits to that old revision.
  Document changes updated current freshness. Time filters and strict exclusion
  worked through the visible controls. Desktop/mobile screenshots were inspected;
  the narrow viewport had no horizontal overflow. A restarted native worker drained
  the persisted test queue.

Initial test failures were corrected: a grant-removal assertion expected 204 instead
of the existing 200 result; required-field label matching included the visual asterisk;
native datetime controls normalized zero seconds. Successful tests used the existing
API response contract and accessible field names/valid browser values.

Generated OpenAPI contains 81 unique operations. Frontend build, rustfmt, Clippy with
warnings denied and governance lint/32 checker tests passed. Six ordinary workflows
passed in the full browser regression before workspace pairing exposed the issue
below. After its repair, the real device/workspace workflows and deterministic code
test passed together (three tests, 19.8 seconds). All seven ordinary workflows now
have passing evidence; optional OIDC was skipped. The final focused memory API
scenarios passed in 1.47 seconds after the readability and origin checks.

The pairing regression exposed opaque public codes being interpreted as JSON numbers
and then removed by string-only route validation. Local dependency execution showed
`12345678` and `1234E567` losing their string identity; leading-zero `01234567` was a
positive control. Context7 and the installed
[TanStack search parser](https://github.com/TanStack/router/blob/main/packages/router-core/src/searchParams.ts)
confirmed JSON parsing and parse/stringify normalization. Custom handling now preserves
only the pairing code's exact opaque string while retaining default behavior for
other parameters. A deterministic browser wire fixture proves all three codes;
separate actual native calls prove approval, OS-store persistence, revocation and
workspace operation history. The owning platform epic records the small-fix exception.

## Runtime and Handoff

The normal local service was restarted with migration 008 and the final UI built
into its served directory. Readiness returned true. An authenticated browser/API
proof at `2026-09-14T12:00:39Z` reused the retained SWEG publication without reading
customer files or calling OpenAI. The existing test Brain and other user data remain.

- Brain: `5c054930-d266-4c18-a42b-942729f942aa` (SWEG — test).
- Claim: `5571b985-652e-44b1-ab77-857c9c0bb514`, revision
  `292ca80a-a5a5-4ef2-82d4-1d67332c6b50`.
- Exact repository support: fact `019c7bdb-d68f-43d6-aaa2-361f49e2a6c6` from
  snapshot `260c6505-f1b8-40ad-bb2a-c7db5cd48a1f`, commit
  `7bc1e85812a1740b942c2a1ddb4e0264459bc987`.
- Result: proposed, browser-authored, current effective freshness, unknown fact
  validity, declared assessment and no strict acceptance. The browser inspected
  both the original structural fact and the exact applicability manifest. Desktop
  and mobile proof/state are in `.cache/sweg-claims-proof/`.

One failed integration database was removed after checking its explicit test ownership
comment, along with only its owned fixture artifacts. No external deployment, commit
or release is authorized.
Human review/correction follows this slice; no claim acceptance, model extraction,
embedding or general semantic search is implied by the manual workflow.
