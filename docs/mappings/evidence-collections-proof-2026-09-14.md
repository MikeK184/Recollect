# Collections dependency and local proof

Observed: 2026-09-14
Confidence: verified

## Sources and Method

The [collections contract](../contracts/evidence-collections.md) governs this
slice. Context7 queries against `/tokio-rs/axum` and `/transact-rs/sqlx` checked
route body limits, transactions and UUID-array binding. Primary references:

- [Axum 0.8.9 body limits](https://docs.rs/axum/0.8.9/axum/extract/struct.DefaultBodyLimit.html)
- [Tokio filesystem API, observed 1.53.1](https://docs.rs/tokio/latest/tokio/fs/struct.File.html)
- [PostgreSQL 17 row security](https://www.postgresql.org/docs/17/ddl-rowsecurity.html)

The version-specific Tokio page returned an internal lookup error during final
review; its official latest page identified 1.53.1 and supplied the interface.
Exclusive file creation, flush and synchronization were checked against that
API. PostgreSQL policies require command-specific access; writer row locks use
an access-checked helper while Brain metadata updates remain admin-only.
Existing compatible dependencies sufficed; no new parser or model dependency
was needed. The tested database is PostgreSQL 17.10, not a claim about the newest
available server patch.

Used real API handlers, non-owner PostgreSQL RLS, filesystem artifacts, fenced
workers and isolated Playwright Chrome. Tests use disposable databases and
artifact directories; the normal development database is preserved. Inspected
desktop source content, collection views and the complete mobile layout.

## Observations

| Behavior | Evidence |
| --- | --- |
| Authority | Writers import and organize; readers retain read access after downgrade but cannot mutate or replay writes. Foreign Brain/group/version IDs are denied with authorized positive controls. Archived Brains reject mutations. |
| History | A Unicode upload larger than the former request limit is read exactly before processing. Appending preserves old content; stale base versions fail. Processing an older version cannot move the current pointer. |
| Organization | Intersecting collection/area/environment filters share one source. Removing an association or a collection preserves history and other views. |
| Processing | Real capture workers produce bounded UTF-8 chunks whose byte ranges reconstruct the original. Reprocessing preserves span IDs. Revoked submitting devices cannot publish. |
| Recovery | A missing registered artifact is shown missing with no fabricated content/spans. Restoring it and reprocessing recovers. Unavailable storage retries; failed imports leave metadata, audit and jobs unchanged. |
| Capture policy | Admin policy blocks future retained imports while existing permitted content remains readable. Reference-only imports contain no document text. Configured credential input is rejected before artifact or receipt storage. |
| Browser | File import, request-error retry, immutable editing/history, overlapping views, collection removal and reference-only capture pass. Uploaded script markup stays inert text. Tested mobile viewport has no horizontal overflow. |

Seven core Rust integration scenarios passed, plus the dedicated real signed
OIDC scenario. Clippy with warnings denied, rustfmt, generated API validation
(51 unique operations) and the TypeScript/Vite build passed. The final browser
run passed five workflows across four separately created databases: device
pairing (9.2 seconds), evidence (13.9), platform (4.9), and two team/OIDC workflows
(11.3). A fresh native worker drained each database's persisted queue.

Initial proof exposed and fixed the writer row-lock policy issue, a browser-test
artifact path that depended on its working directory, capture-policy UI timing,
and tests sharing an assumed-empty database. Each browser file now has its own
database. Governance lint and all 32 checker tests passed. Normal local startup
applied the migration; actual owner login, Brain catalogue and readiness calls
succeeded at `http://127.0.0.1:8787`.

## Translation and Limits

This is local macOS/Docker/API/browser evidence. No external deployment, remote
document retrieval, PDF parsing, semantic extraction, search or graph delivery
is claimed. UUIDs and byte lengths do not establish cryptographic integrity.
Credential detection is bounded, not a universal text-secret detector.
Filesystem/database crash leftovers require later controlled reconciliation;
no unreferenced file is automatically served or deleted. Existing command
receipts may retain permitted input for 24 hours and belong in later erasure.

## Follow-up

Continue with workspace/task scope, then exact repository publication. Memory
retention/erasure and operational recovery own the named cleanup boundaries.
