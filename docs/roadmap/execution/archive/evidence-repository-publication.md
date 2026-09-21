# Committed repository publication and revision manifests

Status: shipped
Owning epic: `docs/roadmap/epics/evidence-and-workspaces.md`
Work type: product

## Summary

- Goal: Browse attributable committed code snapshots and select immutable environment revisions while contributors are offline.
- Non-goals: Graph projection, semantic/model processing, automatic capture and runtime deployment verification.
- Delivery shape: Local Rust companion/service, PostgreSQL migration, React UI, tests and runbook; uncommitted.

## Governing Sources

[ADR 0004](../../../adr/0004-committed-repository-publication.md),
[publication contract](../../../contracts/evidence-repository-publication.md),
[workspace scope](../../../contracts/evidence-workspace-scope.md),
[durable work](../../../contracts/platform-durable-work.md),
[collections](../../../contracts/evidence-collections.md) and the
[owning epic](../../epics/evidence-and-workspaces.md).

## Scope

- In scope: Exact Git object extraction, isolated Enola execution, sanitized resumable bundles, immutable artifacts, contributor deduplication, scoped durable processing, repository browsing and environment manifest histories.
- Out of scope: Neo4j/linking, model inference, universal language parsing and source erasure, which have named successor slices.
- Blockers: None. The native adapter experiment resolved the interface and explicitly unsupported Kubernetes YAML coverage.

## Surface and Interface Changes

- Interfaces: Brain-scoped endpoints, limits and states in the publication contract; native `repository publish BRAIN REPOSITORY TASK CHECKOUT [--revision REF] [--retain-file PATH]`, `repository resume BRAIN BUNDLE`, and snapshot/manifest inspection. Browser repository detail, capture policy, manifest editor/history.
- Storage: Migration 007 adds Brain policy, immutable snapshots/files/facts/contributions, artifact references and manifest revisions under RLS. UUID identities and normalized JSON equality; no product hashing or strict format gate.
- Ownership: Protocol DTOs/validation shared by companion/service; companion owns local Git, Enola and staging; service owns admission, artifacts, worker materialization and manifests; React consumes the API.

## Data and Authority

- Inputs: Locally available exact Git objects, sanitized Enola artifacts, explicitly selected retained text, authenticated operation scope and explicit environment selections.
- Authority: Registered Brain repository origins, current grants/device and immutable operation binding at capture; canonical PostgreSQL snapshots and artifact IDs after admission.
- Blind spots: Contributor assertions do not prove remote authenticity. Static HCL does not evaluate Terraform. YAML has inventory/optional text but no verified semantic facts. Observed deployment is an attributable recorded observation.

## States and Edge Cases

- Loading: Paginated lists and durable processing state remain visible; incomplete work is never ready.
- Empty: Empty inventory/fact lists and missing snapshots have explicit UI states.
- Error: Bounded Git/Enola errors, rejected payloads, unavailable artifacts and failed jobs expose safe messages; upload failure preserves its sanitized bundle.
- Blocked: Missing executable, absent Git object, forbidden capture, closed task or scope mismatch stop publication without fallback.
- No-access: Current Brain/device checks and RLS apply to reads, mutation, replay and worker execution; checkout paths remain private.
- Duplicate or replay: Equivalent snapshots reuse canonical evidence while retaining distinct contributors; same idempotency body replays, conflicting bodies/artifacts return 409.
- Stale data: Manifest base revision conflicts and immutable historical selections remain inspectable; later HEAD/task changes cannot relabel capture.
- Reconciliation divergence: Missing artifacts remain unavailable. Fenced processing can retry against retained inputs without replacing canonical snapshot/fact identities.

## Integrations and Runtime Inputs

- Providers: Repo-owned Enola installation from its official release, invoked through the verified artifact adapter; Git object plumbing only. No network source fetch or model calls.
- Environment: Existing `RECOLLECT_URL`, `RECOLLECT_DEVICE_PROFILE`; optional `RECOLLECT_ENOLA_BIN`, `RECOLLECT_PUBLICATION_DIR` with repo-owned defaults.
- Secrets: Never persist credentials or pass provider keys/HOME to Enola; path/content exclusions precede staging. Backend rejects configured secrets/private keys before persistence.
- Failure handling: Contract bounds, minimal extractor environment, explicit retries and preserved original operation/idempotency for resume. Cleanup touches only exclusively created owned files.

## Tests and Acceptance

- Automated: Compound native Git/Enola fixture for two commits, dirty worktree, archive attributes, exclusions and failures; integrated API/DB/RLS proof for policy, deduplication, contributors, conflicts, fencing, artifact absence and manifest history; browser/native publication and manifest workflow including stale edits/mobile. Run focused tests, formatting/clippy/frontend build and `./scripts/validate.sh`.
- Manual: Inspect actual extractor artifacts and browser views; bounded SWEG publication after synthetic isolation/exclusion proof, preserving customer state.
- Acceptance: All publication-contract acceptance scenarios must pass before archiving. Keep runtime integration and implementation claims separate.

## Closeout

- Planned: Native committed publication/resume, shared immutable evidence and contributor history, durable facts, environment manifests and browser workflow.
- Shipped: Exact native Git/Enola extraction, resumable sanitized bundles, immutable shared snapshots/artifacts/facts, contributor provenance, scoped fenced processing, capture policy, environment manifest history and desktop/mobile inspection. The authorized SWEG test Brain contains one processed committed snapshot with 99 facts and no retained raw file text.
- Not shipped: N/A within this slice. Graph/semantic/model/capture/erasure behavior remains in its named successor slices; Kubernetes YAML semantic extraction is an explicit adapter coverage gap.
- New blockers: None.
- Docs updated: ADR 0004, publication and collections contracts, Enola experiment and [delivery proof](../../../mappings/repository-publication-proof-2026-09-14.md), [runbook](../../../runbooks/repository-publication.md), local development guidance, epic and execution indexes.
- Validation: Nine core API/database scenarios; native extraction, discovery and origin tests; six ordinary browser workflows (optional OIDC skipped); actual read-only SWEG extraction/publication and metadata preservation; 75 unique OpenAPI operations, frontend build, rustfmt, Clippy, governance lint and 32 checker tests. Final focused API proof passed after adding immutable same-operation provenance checks. Local runtime restarted with migration 007; external deployment is outside the goal.
- Version: N/A; no release policy or release requested.
- Commit: Uncommitted.
