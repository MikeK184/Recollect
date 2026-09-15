# Committed repository publication local proof

Observed: 2026-09-14
Confidence: verified

## Sources and Method

Implemented the accepted [publication contract](../contracts/evidence-repository-publication.md)
and [ADR 0004](../adr/0004-committed-repository-publication.md), using the
[official-interface experiment](enola-extraction-proof-2026-09-14.md).
The previous eight core platform scenarios remain passing; the new integrated
publication scenario uses an independently created disposable PostgreSQL database.
Local runtime and UI proof are separate from external deployment, which is excluded.

## Observations

- Native Git/Enola fixture: two commits, a dirty worktree, export-ignore, replacement
  refs, symlink/gitlink inventory, binary/oversized/credential exclusions, unsupported
  paths, missing promisor objects, disabled text policy and explicit retained files.
  Repeated extraction preserved stable artifact values. Bundles retained immutable
  operation/input identity, enforced exclusive creation and removed only owned
  successful input. Failed extraction cleaned its own transient tree. Latest native
  fixture passed in 2.17 seconds under `.cache/publication-test-341c449a-4266-423c-bbbb-e7895c4dd369/`.
- Shared API/DB: concurrent same-key uploads replayed one result; a second authenticated
  contributor reused the snapshot without losing attribution. Conflicting stable
  artifacts, wrong scope/account/device, forbidden capture, malformed locations/spans,
  configured credential material, foreign Brain reads and revoked access were denied.
  A fresh request identity could not relabel an existing operation's contributor
  provenance; that case returned 409 in the final focused API proof.
- Worker: expired leases could not publish; the replacement lease completed. Duplicate
  upstream IDs remained distinct records by ordinal, and reprocessing preserved their
  UUIDs. Revoked-device queued work cancelled; a missing artifact failed explicitly,
  and restoring it let the same valid lease complete.
- Manifests: independent environments retained different exact revisions; committed,
  desired and recorded-observed meanings stayed distinct. Missing observation metadata
  and stale editors failed; prior history remained inspectable. Ordinary environment
  deletion returned a conflict when history existed. Direct mutation of immutable
  snapshots was denied, and RLS hid snapshots after grant removal.
  An explicit commit without a retained snapshot remained a valid, visibly
  unavailable manifest selection.
- Real native/browser workflow: an HTTP fixture interrupted the first upload after
  preparation, the working copy changed again, and resume published the original
  committed bytes. Browser inspection showed ready facts, permitted text, unavailable
  unretained text, coverage and dirty contributor provenance. Concurrent manifest
  editing produced a visible stale-editor error and preserved revision history.
- Desktop/mobile views were inspected. A narrow repository-label layout was corrected
  so mobile repository names use a full row above actions. Capture-policy feedback
  reflects pending and confirmed writes. Browser tests use isolated Chrome and
  disposable service databases; traces/video are disabled.

Validation: nine core API/database scenarios passed; the native extraction fixture,
existing native discovery and origin-normalization tests passed. All six ordinary
browser workflows passed in one `./scripts/test-ui.sh` run, including publication;
the optional OIDC browser test was skipped because that run did not enable Dex.
The earlier dedicated OIDC proof remains historical, not a new claim for this run.
All 75 OpenAPI operation IDs are unique; generated client, frontend build, rustfmt,
Clippy with warnings denied, governance lint and 32 checker tests passed. Fresh
native workers drained each persisted browser-test queue.

Initial fixture failures were resolved: a test used an incorrect revocation route;
controlled switch timing needed an eventual checked assertion; a text assertion
initially included hidden tab panels. These failures did not establish acceptance.
The successful runs above exercised the corrected scenarios. One owned failed
integration database was removed only after verifying its explicit ownership comment.

## SWEG Runtime Proof

After synthetic isolation/negative proof, used the user's authorized SWEG test Brain
and one existing registered checkout, read-only:
`/Users/mike/devops/customer/SWEG/infrastructure/terraform-modules/terraform-vsphere-vm`.
No customer selector or source file was written. A dedicated native device paired,
published under an explicit task/repository/environment scope, then unpaired and
removed its OS-store credential. The task was closed after processing.

- Brain: `5c054930-d266-4c18-a42b-942729f942aa` (SWEG — test).
- Repository: `ee3bdb6c-5d84-4ba5-868c-f504788ee164`.
- Exact commit: `7bc1e85812a1740b942c2a1ddb4e0264459bc987`.
- Snapshot: `260c6505-f1b8-40ad-bb2a-c7db5cd48a1f`.
- Result: 99 ready facts, 37 inventory files, 36 materialized inputs, 16 files
  producing facts, zero retained raw file texts, zero parse errors and four
  unresolved relations. One input was excluded before extraction.
- Manifest: `49b3f13d-d658-4831-a4da-f5dbeee0c19d`, a committed selection in the
  existing Local test environment. It makes no desired/deployed assertion.
- The five recorded checkout metadata paths retained their inode, size, modification
  and change times. No SWEG selector exists. The broader earlier metadata smoke
  remains separate: 80 checkouts/79 origins observed, only this one repository
  published by this check.

Proof state, preservation observations and screenshots are in
`.cache/sweg-publication-proof/`; browser fixture screenshots are in `.cache/ui/`.
The normal local application was restarted with migration 007; readiness returned
true and owner sign-in/actual publication succeeded at `http://127.0.0.1:8787`.
The existing test Brain and other user state were preserved.

## Limits and Handoff

Static facts and heuristic insights are evidence, not accepted claims or verified
runtime relationships. This adapter has demonstrated Rust/TypeScript/HCL coverage;
Kubernetes YAML and unsupported formats remain explicit gaps. PostgreSQL/artifacts
are canonical; no Neo4j projection, semantic recall or model extraction is claimed.
The selected Luna/embedding-large provider pair remains in the separate
[provider preflight](openai-provider-preflight-2026-09-14.md) and future gateway slice.
Memory claims/time is the next dependent product slice. Erasure must cover raw
retained repository files, command receipts, facts/artifacts, manifests and local
prepared bundles; ordinary policy disablement does not erase historical evidence.
