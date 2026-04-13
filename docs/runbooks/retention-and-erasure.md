# Retention, erasure and deletion replay

Status: active

## Browser workflow

Start `./scripts/dev.sh` and open `http://127.0.0.1:8787`. Each Brain has a
**Retention and erasure** panel. Readers inspect policy/status; admins edit retention
and erase content, including in archived Brains. Devices cannot initiate admin erasure.

New sources select document, sanitized raw-session or sanitized tool-output retention.
The class stays with the source. Raw classes default to 30 days; documents, excerpts,
repository snapshots and claims default to until erased. Durations apply to existing
content from capture time. Shortening can expire it immediately; extending cannot restore
removed bytes. Retention and permission to send content to a model are separate policies.

Open retained source evidence and **Retain supporting excerpt** to select exact lines,
up to 8 KiB. The server copies those lines with provenance and independent retention.
Ordinary raw expiry preserves a permitted excerpt; source erasure also removes its copies.

Open a source, claim, snapshot or manifest and choose **Erase**. Collection erasure is
under **Manage views**. Inspect counts, shared-source effects and retained independent
revisions, then **Confirm erasure**. Changed inputs require a fresh preview. Retry an
interrupted response with unchanged input. Content becomes unavailable before file
cleanup finishes. The panel reports journal/file progress, errors and central completion;
**Retry cleanup** resumes the committed request.

Erasing a current claim never promotes an older one. **Include historical states**,
without repository/environment filters, shows removed markers. Open one to inspect
remaining independent history or erase the whole claim. An expired sole support qualifies
a retained claim. Erasure also clears affected review payloads/rules and recent receipt
payloads; invalidated command keys remain fenced for their original 24-hour window.
Source-supported claim erasure also fences its supporting source versions from
model transmission. The preview counts these inputs; independently retained source
evidence stays manually inspectable. The opaque source IDs travel with the erasure
journal to protect older backups where the claim did not yet exist.

## Storage, maintenance and restore

`RECOLLECT_ERASURE_JOURNAL` defaults to `.data/erasure-journal`. Migration explicitly
initializes the installation directory. Keep it with the live installation and separately
from rotating PostgreSQL/artifact backups. Entries contain identities, times and commit
fences, without source bodies, paths or review reasons. Never replace an initialized
journal with an empty one to make an old database start.

The worker runs bounded expiry/cleanup passes. A deterministic operator pass is:

```sh
set -a
source .env
set +a
target/debug/recollect-server privacy-once
```

Database redaction, receipt invalidation and job cancellation commit together. Journal
export and file removal are retryable; storage failures remain visible. An accepted
request continues after the initiating account loses access. Cleanup removes only recorded
Brain/UUID artifacts and managed prepared bundles, preserving customer checkouts.

Serve/worker startup checks the retained journal against canonical installation state.
An older database, missing acknowledged entry or unavailable journal cannot serve content.
Restore the latest retained journal, then replay using the operator database role:

```sh
set -a
source .env
set +a
target/debug/recollect-server privacy-reconcile
```

Migration also reconciles before local startup. Serve/worker can reconcile when
`DATABASE_ADMIN_URL` is explicitly available; otherwise run the operator command first.
Normal requests use the non-owner role. Startup also removes restored artifact copies
whose database requests were already complete.

The backup window defaults to seven days. The full backup/rotation adapter belongs to
operational readiness. Current proof covers an actual older PostgreSQL copy and restored
artifact bytes; it does not claim removal from external copies.

## Companion copies

Publication/resume synchronizes policy and commit fences before upload. Cleanup is
limited to the selected endpoint, paired device and Brain under the managed publication
directory. Other Brain/device bundles remain intact. With the usual paired profile:

```sh
target/debug/recollect-agent repository cleanup BRAIN_UUID
target/debug/recollect-agent repository cleanup BRAIN_UUID --offline
```

Connected cleanup saves minimal policy, removes affected bundles and acknowledges the
observed deletion position. Offline cleanup applies only cached policy/fences and reports
no new acknowledgement. Output includes synchronization time, position and counts.
Unreadable managed identity metadata prevents acknowledgement; unrelated files are not
guessed to be disposable.

Central completion differs from device acknowledgement. Offline/revoked devices may
still hold copies. The [model gateway and learning worker](provider-learning.md)
enforce current retention and in-flight erasure. Later capture-inbox, vector,
graph and backup adapters must
enforce the [retention contract](../contracts/memory-retention-and-erasure.md) and prove
their own cleanup. See the [dated proof](../mappings/retention-and-erasure-proof-2026-09-14.md)
and run `./scripts/test-platform.sh` plus the retention/claims/review browser tests.
