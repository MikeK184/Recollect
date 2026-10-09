# Evidence, Collections and Workspaces

Status: complete

## Purpose

Make Brain knowledge reproducible and attributable: organize sources in
collections/areas, bind tasks to workspaces and environments, publish exact
repository snapshots, and capture permitted session evidence durably.

## Governing Sources

- [Desktop contract](../../contracts/desktop-experience.md)
- [Desktop ADR](../../adr/0014-desktop-experience-and-answers.md)
- [Collections contract](../../contracts/evidence-collections.md)
- [Workspace scope contract](../../contracts/evidence-workspace-scope.md)
- [Committed publication decision](../../adr/0004-committed-repository-publication.md)
- [Repository publication contract](../../contracts/evidence-repository-publication.md)
- [Automatic session capture](../../contracts/evidence-session-capture.md)
- [Vision: Brain organization](../../foundation/vision.md#brain-and-scope)
- [Vision: repositories and retained knowledge](../../foundation/vision.md#repository-identity-and-retained-knowledge)
- [Stack: extraction and capture](../../foundation/techstack.md#extraction-capture-and-models)
- [Engineering principles: evidence](../../foundation/engineering-principles.md#preserve-evidence-before-deriving-belief)

## Dependencies and Boundaries

Use platform authorization, device identities and durable commands. Own source
versions/support locations, collection membership, overlapping area/environment
associations, repository UUIDs, workspace/task scope and immutable extraction
artifacts. Environment manifests distinguish committed, desired and observed
revisions. Contributor identity remains separate from resource ownership.

Collections provide dataset-like organization/import/update/removal under Brain
grants. Removing membership does not erase a source used elsewhere. Collection
and source browsing include provenance, missing-source and processing states.
Raw source capture remains policy-controlled; keep ordinary checkouts local.

Select Enola's artifact interface for the first repository adapter and prove
representative Rust/TypeScript/Terraform/GitOps coverage. Preserve unsupported
languages, unresolved links and extraction limits instead of inventing runtime
relationships. Neo4j projection/linking belongs to graph intelligence.

The first three slices provide evidence inputs for memory lifecycle work.
Automatic session capture waits for that epic's retention/erasure behavior;
this slice-level handoff avoids making both whole epics wait for each other.
Codex/Claude adapters sanitize before local persistence, retain event/scope/host
identity and expose incomplete or ambiguous capture. MCP execution observations
later enter this same pipeline through the coordinator.

## Decisions Before Implementation

- Specify source/version/collection identities, capture policy and removal
  semantics using the shared mutation/audit path.
- Specify workspace discovery, task/subagent scope and exact manifest/publication
  contracts; decide supported fixture coverage without promising universal parsers.
- Specify host event compatibility, inbox/retry protocol and the retention handoff
  before enabling automatic capture. Exact endpoint names remain pack work.

## Slice Map

The [automated memory improvement plan](../../research/automated-memory-improvement-plan-2026-10-07.md)
proposes session-derived handovers with original capture attribution and native
minimal supporting excerpts under both existing content/retention permissions.
Memory lifecycle owns synthesis; this epic owns exact source/excerpt lineage,
coverage and private-workspace metadata boundaries. The accepted
[digest contract](../../contracts/memory-automatic-session-digests.md) and archived
[memory-owned pack](../execution/archive/memory-automatic-session-digests.md)
record local delivery, intent-ledger recovery, exact scope/class preservation,
expiry/erasure and older-backup proof. No customer checkout is modified.

Small-fix exception, 2026-09-14: the user authorized SWEG as real test input.
Expose the existing bounded Git observation helper to an opt-in native integration
probe. This adds validation tooling under the shipped workspace contract, with
no new wire behavior, schema or Brain-selection rule. The probe uses an explicit
checkout list, labels coverage partial and writes test artifacts only in Recollect;
it does not place a selector in the customer workspace.

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `evidence-typescript-admission` | shipped | contract-backed | small-fix: distinguish fenced TypeScript type annotations from literal credential assignments | Preserve unchanged history admission while retaining configured-secret, token, URL and literal-value exclusions |
| `evidence-collections` | shipped | contract-backed | pack | Delivered immutable text/reference sources, durable artifacts, shared collection/area/environment views, provenance, processing and browser proof |
| `evidence-workspace-scope` | shipped | contract-backed | pack | Delivered nearest/nested discovery, private checkout catalogue, shared repository identities and independent immutable task/subagent operation scope with CLI/browser proof |
| `evidence-repository-publication` | shipped | adr-backed, contract-backed | pack | Delivered exact committed extraction, resumable native publication, immutable evidence, contributor history, environment manifests and browser/real SWEG proof |
| `evidence-session-capture` | shipped | contract-backed | pack | Delivered actual Codex/Claude hooks, native setup/launch and durable delivery, original-scope learning, browser coverage/source controls and local/central retention with replay proof |
| `evidence-desktop-workflows` | shipped | adr-backed, contract-backed | pack | Sources, repositories/manifests and agent/session/private scope views with literal title search before pagination |

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `evidence-collections` | `platform-durable-work` |
| `evidence-workspace-scope` | `evidence-collections`, `platform-device-pairing` |
| `evidence-repository-publication` | `evidence-workspace-scope` |
| `evidence-session-capture` | `evidence-workspace-scope`, `memory-retention-and-erasure` |
| `evidence-desktop-workflows` | `platform-desktop-shell` |

## Completion Criteria

- Collections organize sources without duplicated authority or accidental
  cross-Brain access; removing an association preserves independently used content.
- Nested workspaces, directory changes and concurrent subagents preserve their
  correct bindings. Scope changes expose a handoff for fresh retrieval.
- Different local paths and two repository revisions produce attributable,
  reproducible published artifacts; dirty trees are never mislabeled as HEAD.
- Production and development select their own snapshots; source absence and
  unresolved links remain visible, including when a contributor is offline.
- Capture survives interruption/retry without false event attribution or secret
  persistence. The 30-day default, Brain override and supporting-excerpt policy
  work locally and centrally; erasure fences queued publication.

## Desktop implementation evidence, 2026-09-26

The [dated implementation mapping](../../mappings/desktop-experience-implementation-2026-09-26.md)
records implemented routes/assets and the verified final local image separately
from remaining current-code domain and integrated acceptance. The
[desktop guide](../../runbooks/desktop-experience.md) documents the current function
locations. Product/proof slices stay in progress until their required checks pass;
prior shipped domain records remain historical evidence rather than redesign proof.

## Final desktop and tooling acceptance — 2026-10-01

The owning desktop/tooling slices are shipped with [current deployed and host evidence](../../mappings/desktop-final-acceptance-2026-10-01.md). Earlier pending checks above describe the September 26 snapshot; their remaining acceptance is now complete. No release, commit or push was performed.

The October 8 TypeScript admission correction passes its credential-regression
fixture and actual unchanged LongMemEval history import. Direct runtime proof
checks all 2,402 retained conversation sources, including the previously rejected 49-session
case, byte-for-byte against frozen inputs. See the
[dated benchmark report](../../mappings/openrouter-benchmark-proof-2026-10-08.md).
No credential value or source text was sanitized to obtain this result.
