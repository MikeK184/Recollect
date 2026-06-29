# Exact combined repository graphs

Status: accepted

## Source

[ADR 0007](../adr/0007-canonical-graph-projections.md),
[projection/traversal](graph-projection-and-traversal.md),
[publication](evidence-repository-publication.md),
[workspace scope](evidence-workspace-scope.md) and the
[graph epic](../roadmap/epics/graph-intelligence.md). The active full-product goal
authorizes these routine decisions within the accepted foundations. The
[reference review](../mappings/cross-repository-interfaces-2026-09-15.md) records
why exact source evidence is required and which existing libraries are reused.

## Contract

### Exact combined inputs

Extend graph selection with `kind=combined`. Require an immutable manifest
revision in its matching environment, containing two to 100 distinct repositories
with exact materialized snapshot entries. Missing or expired input returns an
explicit unavailable-input error; no latest-snapshot substitution or successful
prefix is allowed. Repository and area selection can narrow that manifest at
read time, under the existing browser/paired-operation authority boundary.
Knowledge and single-repository graph behavior remains governed by its contract.

Reuse the existing repository generations. A combined generation stores only
cross-repository edges, their canonical endpoint memberships and unresolved
evidence. Identify its input by a sorted exact snapshot UUID array, linker label
and the Brain's origin-binding epoch. Identical sets in different environment
manifests reuse that generation; base repository nodes/edges are not copied for
each combined view. Record the exact base generations used in every response.
Origin additions advance the link epoch so automatic maintenance can resolve a
previously unknown alias; old descriptors cannot assert a new identity binding.

### Supported linker and retained evidence

The first supported directional link is `terraform_module` in family
`cross_repository`. It means a literal module declaration identifies an exact
Git repository, full commit and module directory. It does not establish successful
Terraform initialization, deployment, runtime calls or compatibility.

Reuse the native Tree-sitter engine and HCL grammar in the companion to inspect
already-permitted committed `.tf`/`.hcl` text. Parse full files, rejecting error
recovery trees for linking. Bound each parse to one second with the engine's
progress cancellation callback, within the existing whole-publication deadline
and byte limits. Do not evaluate expressions, run Terraform, fetch repositories
or admit more than 100,000 module declarations per publication. Do not
transmit source text for this operation. Pin compatible library versions and
record parser/grammar labels in publication settings and derived evidence.

Preserve Enola facts. Attach bounded normalized module-source evidence only to
a unique fact matching the parsed file, module address and declaration location.
Enola may place its start line on preceding blank lines: accept that location
only when the intervening committed bytes are exclusively whitespace, preserving
the original fact location and the precise parser span separately. Duplicate
declarations in the same module directory, ambiguous fact matches,
parse errors/timeouts and unsupported expressions remain unresolved. Parse
comments/heredocs as syntax; source-like text inside them cannot become a module
source. Override files and unexamined Terraform JSON in an affected directory
withhold its witnesses until their merging semantics are supported. Directory
facts used as targets must uniquely match the exact parsed HCL
module directory and retained materialized inventory. Old snapshots without
verified source evidence remain queryable but cannot gain a validated remote link
from Enola's regex-derived `module_source` hint alone. A new publication with
different parser settings has its own normal snapshot identity; no strict wire
version handshake or new hash is introduced.

Support literal `git::https://`, `git::ssh://` and GitHub/Bitbucket repository
shorthand with optional `//subdirectory` and exactly one `ref` query argument
containing a full existing Git object ID. Use the shared URL/origin normalizer
and registered origin aliases, not repository basenames. Reject duplicate query
parameters, credentials, fragments, ambiguous paths and unsupported query options
(including `depth` with a commit ref). Do not reinterpret percent-encoded path
separators or traversal. Literal decoding uses the parser's string boundary and
the existing JSON decoder for its supported escape subset; other escape forms
remain unsupported. Branches, tags, missing/abbreviated refs, registry addresses,
source expressions and other transports retain specific unresolved reasons.
Local directory sources remain owned by ordinary intra-repository extraction.

The normalized origin must identify one repository in the exact selected set;
its source ref must equal that entry's full commit. Require exactly one eligible
target module fact for the normalized directory in that snapshot. No match,
multiple matches, unsupported target extraction and mismatched revision produce
unresolved evidence, not an edge. Preserve the source fact UUID, both endpoint
snapshot UUIDs, linker label and source span through canonical descriptors and
response input/evidence references. Exact tuple joining is Recollect's small
adapter; parsing and path algorithms come from existing libraries/Neo4j.

### Work, publication and lifecycle

Use the existing `graph.project` heavy lane, heartbeat, retry, cancellation,
staged import, physical descriptor verification and final retention checks.
Keep the existing per-generation byte/node/edge bounds; inspect at most 100,000
combined fact inputs before linking. Overflow fails explicitly. The worker
discovers complete manifest input sets automatically, with at most ten queued
graph inputs per Brain/pass and the existing total backpressure. Preserve fair
progress between repository, knowledge and combined discovery.
Rotate their starting priority across passes so one remaining queue slot cannot
permanently favor repository work. The per-kind maxima still apply.

Freeze the exact snapshot set and origin epoch for every attempt. Lost authority,
expired inputs or changed origin bindings cannot publish stale work. A transient
failure retries the same frozen descriptors. Cleanup uses existing generation
and entity fences, retains shared endpoints and remains bounded/recoverable.
Retiring a repository snapshot invalidates combined inputs that require it;
privacy erasure removes incident cross edges with the same canonical identity
journal. Rebuilding cannot reintroduce rejected or erased evidence.

### Qualification, paths and UI

Hydrate and qualify every selected base node through canonical recall gates,
including the exact manifest config paths, scope, current rejection and retention.
Apply configuration paths through the shared raw-fact recall predicate before
admission: empty means the whole snapshot, otherwise literal path equality or
descendants of a selected directory. Unlocated facts and unselected parents stay
outside the view. This also repairs ordinary recall's raw-fact path selection.
Only edges whose source evidence and both endpoints remain eligible enter path
selection. Verify each relationship against its owning generation. Query the
union of those exact generation IDs with Neo4j's native shortest-path pre-filter;
never fetch a path and only afterward filter its intermediates. Recheck authority
and deadlines before publication. Existing read admission, hop, timeout and
concurrency bounds apply to the complete union, not separately to each repository.
Read deadlines use current policy for every required manifest snapshot, including
inputs hidden by narrowing; frozen projection-attempt deadlines do not determine
current read retention after a policy extension.
Before loading base descriptors, enforce a 64 MiB total serialized input budget
using persisted descriptor sizes; large unions cannot multiply the per-generation
memory envelope. Exceeding it requires a narrower repository selection.

Return the manifest, exact base/link generations, eligible counts, and qualified
unresolved reasons. Display at most 100 unresolved records with an explicit total;
withheld source evidence cannot leak through diagnostic details. Empty/no-path,
missing projection, damaged store and partial extraction remain distinct.
Removing a relationship can reveal a longer eligible path across repositories.

The graph panel adds **Combined repositories**, manifest selection, input
readiness, linking coverage and evidence-backed path display. Optional repository
selection narrows the chosen manifest. Rebuild uses its exact manifest inputs;
polling never repeats mutations. A link's meaning and source evidence are visible,
and all identity/scope/error states work on desktop and mobile. The full canvas
and queued analytical reports remain owned by the following slices.

## Acceptance

Prove native committed parsing and retained evidence with literal pinned links,
subdirectories, comments/heredoc decoys, source expressions, duplicate declarations,
wrong refs/query parameters and unchanged dirty checkout state. Through real
PostgreSQL and Neo4j, prove an eligible path crossing repository boundaries and
its exact manifest; different production/development inputs, aliases, duplicate
basenames/targets, missing input and config-path scope must not invent paths.
Pair removal/rejection/erasure and unauthorized cases with surviving positive
controls, including a longer eligible alternative and rebuilding. Prove shared
input reuse, queued recovery/revocation, origin-epoch invalidation, limits and
browser states. Run focused regressions and `./scripts/validate.sh` before archive.

## Explicit Deferrals

Other dependency protocols/linkers, registry or branch/tag resolution, Terraform
constant-expression evaluation, historical graph queries and model-inferred
associations. These unsupported cases remain visible. GDS analytics (18), full
exploration (19) and retrieval fusion (20) remain required successor slices.
