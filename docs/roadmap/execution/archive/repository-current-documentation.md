# Current documentation and obsolete material cleanup

Status: shipped
Owning epic: `docs/roadmap/epics/repository-governance.md`
Work type: governance

## Summary

- Goal: Keep current operating guidance, accepted authority and actionable open work easy to find; remove superseded reports and design artifacts.
- Non-goals: Implement product features, change product decisions, rerun paid benchmarks or publish/deploy the application.
- Delivery shape: Documentation cleanup, reconciled links and indexes, current code checks and offline/site validation.

## Governing Sources

- [Repository governance](../../../contracts/repository-governance.md).
- [ADR 0001](../../../adr/0001-repository-governance.md).
- [Documentation lifecycle](../../../README.md).
- The user's 2026-10-18 instruction to remove old, unused or corrected documentation and report remaining features.

## Scope

- In scope: Superseded studies, mockups and artwork; current navigation, runbooks, roadmap and feature status; exact Git links for removed evidence still cited by delivery records.
- Out of scope: Accepted product behavior, source/test fixtures, reference checkouts, runtime data and product implementation.
- Blockers: None; obsolete documentation removal is explicitly authorized.

## Surface and Interface Changes

- Interfaces: Existing documentation entry points and links; no API change.
- Storage: Remove only identified tracked documentation/assets; preserve historical evidence in commit f2673d71c4b63b536d41b3143eb39c207955b201.
- Ownership: Repository governance; product owners retain their unfinished slices.

## Data and Authority

- Inputs: Current Git state, accepted contracts, source/configuration, delivery records and dated benchmark receipts.
- Authority: Current explicit cleanup request and existing governance; dated reports cannot redefine product behavior.
- Blind spots: Documentation verification does not rerun runtime acceptance or prove current external connectivity.

## States and Edge Cases

- Loading: N/A: offline documentation changes have no loading state.
- Empty: Retain required indexes/templates and useful current entry points.
- Error: Broken links or inconsistent lifecycle fail validation and are repaired.
- Blocked: Preserve unfinished product acceptance and explicit missing decisions.
- No-access: N/A: no runtime permission changes; keep credentials out of docs.
- Duplicate or replay: One current roadmap; removed historical material stays reachable by exact Git revision.
- Stale data: Mark benchmark/runtime observations with their actual dates; replace obsolete present-tense claims.
- Reconciliation divergence: Keep implementation, validation, publication and deployed state distinct.

## Integrations and Runtime Inputs

- Providers: No paid model or judge calls; repository-local Rust Analyzer/Graft may verify code.
- Environment: Bash/Python governance checks and the existing local website build.
- Secrets: No credentials or private payloads are copied into documentation.
- Failure handling: Inspect every deleted-target reference; retain accepted authority and required delivery records.

## Tests and Acceptance

- Automated: Governance and checker tests, local Markdown links/anchors/assets, historical Git target existence, public site build/check and git diff --check.
- Manual: Review active versus proposed features against current source; inspect the removal set and current entry points.
- Acceptance: Current docs lead to implemented behavior and explicit remaining work, superseded artifacts are removed, and no missing link or false shipment claim is introduced.

## Closeout

- Planned: Remove obsolete documentation and reconcile current guidance and remaining features.
- Shipped: Removed 109 obsolete reports, studies, design artifacts and assets (about 17.5 MiB); consolidated current guidance and the remaining authorized deliverables. Accepted ADRs/contracts, shipped packs, frozen evaluation inputs and unresolved failures remain available. Necessary references to removed evidence now point to exact files at the pre-cleanup Git revision.
- Not shipped: Product implementation, new benchmark scores, commit/push and deployment.
- New blockers: None identified for the documentation cleanup.
- Docs updated: Documentation and foundation entry points, governance authority, all epic summaries, execution indexes, current runbooks, retained evidence/proposals and the repository/plugin entry points. This pack is archived. Separate graph implementation resumed concurrently during closeout; its new ADR, contract, pack and code changes are preserved and are outside this cleanup's validation claim. Current product acceptance remains with the active packs.
- Validation: Governance lint and 35 governance tests, support baseline and 10 support tests, the public site build/check (30 HTML routes), repository Markdown/asset/anchor checks, exact historical Git target checks, deletion-manifest and frozen-input verification, and git diff --check passed. Source checks included successful Rust Analyzer and Graft queries; no new runtime or paid model evaluation was run.
- Version: N/A: documentation only.
- Commit: Uncommitted.
