# Cross-repository interface evidence — 2026-09-15

## Method and boundary

Confidence: verified for source/package inspections below; proposed transfer is
inferred until implemented and tested. The user-authorized Atlas reviewer read
pinned Enola/Cognee sources and Atlas comparison pointers while the root agent
closed graph slice 16. No reference checkout was edited, customer source read or
model request made. The [contract](../contracts/graph-cross-repository-views.md)
owns behavior; this mapping records evidence rather than shipment.

## Existing components and limitations

- Enola v0.4.19 [HCL extraction](https://github.com/enola-labs/enola/blob/v0.4.19/internal/extractors/hclextractor/hcl.go)
  records remote module sources on module symbols, deliberately without a remote
  edge. The reviewer found regex matching over block bodies, allowing source-like
  comment/heredoc text to confuse the hint. Recollect preserves the source fact
  but requires parsed literal evidence before a validated remote edge.
  These implementation files were read from pinned GitHub URLs, not the partial
  local Enola cache. A follow-up found its golden module location starts on the
  preceding blank line; qualification must check the exact intervening whitespace,
  not assume equal line numbers. The target is one `kind=module`,
  `props.language=hcl` record whose `name` and `file` both identify the directory
  (`.` at the root). Duplicate labels across files and Terraform override/JSON
  files prevent a unique effective source witness.
- Enola's [cross-repository signals](https://github.com/enola-labs/enola/blob/v0.4.19/internal/linkers/crossrepo/crossrepo.go)
  organize candidates deterministically. Its import signal matches repository
  labels/path segments; HTTP confidence means a unique route match. Those checks
  do not resolve exact repository origins and commits. Symmetric shared-code
  evidence is correctly distinct from a directional dependency.
- Cognee's local `tasks/code_graph/extract_code_graph.py` and corresponding test
  reuse Go module-path normalization. Global/suffix-name and ancestor fallbacks
  are unsuitable for exact cross-repository identity. Recollect reuses its own
  `canonical_origin`/URL parser, immutable facts and native Neo4j traversal.
- Current [Terraform module configuration](https://developer.hashicorp.com/terraform/language/modules/configuration)
  permits constant source expressions; an unsupported expression is not invalid
  Terraform. Git refs can be names or commit IDs. Without recorded external
  resolution, only a full commit matching the manifest supplies exact revision
  evidence. The [module reference](https://developer.hashicorp.com/terraform/language/block/module)
  describes subdirectories/query arguments and the depth/commit-ref limitation.

## Parser interface selection

Context7 did not resolve `hcl-rs`; it returned `/hashicorp/hcl`, whose syntax and
range documentation was consulted. Published `hcl-edit` documentation mixed
0.9.7 and cached 0.9.5 pages, and two exact API-page opens failed. The 0.9.7 crate
source was downloaded into the owned cache and inspected. It provides a parser,
but no bounded progress-cancellation interface was found in that source.

The selected reusable parser is [Tree-sitter](https://github.com/tree-sitter/tree-sitter)
with the [HCL grammar](https://github.com/tree-sitter-grammars/tree-sitter-hcl).
Registry searches returned `tree-sitter 0.27.0` and `tree-sitter-hcl 1.1.0`.
The grammar crate's Rust `LANGUAGE` binding and generated node/grammar definitions
were inspected from its published archive. Context7 `/tree-sitter/tree-sitter`
documents native progress cancellation and the Rust `ParseOptions` callback.
The implementation must prove the pinned pair loads and distinguish literal
source syntax from expressions and recovered/error trees. No custom HCL scanner
or graph traversal implementation is required.

Downloaded research inputs live under `.cache/cross-graph-proof/`; they are not
mandatory product runtimes.

## Implementation evidence so far

The pinned parser/grammar pair compiled and two focused parser tests passed,
including comment/heredoc decoys, Enola whitespace locations, expressions,
duplicate directory labels, overrides/JSON and native cancellation. Five protocol
tests passed, including exact Git destination normalization and rejected guesses.
The existing real Enola committed-publication fixture was extended and passed in
2.49 seconds: the precise witness selected the real source, preserved the original
blank-line location, retained no Terraform file text and left the dirty checkout
unchanged. Credential-bearing module URLs were excluded before materialization.

The first combined PostgreSQL/Neo4j fixture passed in 1.72 seconds. A three-edge
native path crossed two distinct repositories with the same basename. Another
environment's identical snapshot set reused both repository generations and the
cross-link generation; the exact manifest remained in its response. An old
unverified hint stayed unresolved, and narrower/mismatched selections failed.
No model requests were made. Evidence: `parser.log`, `protocol.log`,
`native-publication.log` and `combined-path.log` in the same owned cache.

These initial results preceded the expanded proof below; normal-runtime and
complete regression closeout remain open.

## Follow-up review and executed proof

The user-authorized reviewer found five issues in the initial implementation:
repeated prefix scans after parsing, missing linker identity in reuse, frozen
read-retention deadlines, queue starvation at low capacity and synthetic path
directions that differed from Enola. Current code uses indexed offsets/declarations,
inspection time/count bounds, exact linker lookup, current deadlines for every
required snapshot and a persisted rotating discovery cursor. A native browser
fixture now verifies Enola's actual `symbol -> directory` direction in `both`
mode; the UI shows recorded direction and marks reverse traversal.

Real PostgreSQL/Neo4j fixtures prove alias and exact production/development
revision selection, ambiguous/missing targets, missing inputs, revoked native
operations/jobs, automatic linker replacement, shared generation reuse and
one-slot discovery fairness. A rejected short route yields a longer eligible
route while the old edge is still physically present; rebuild preserves that
exclusion. Erasing one snapshot removes incident cross edges while preserving
the other repository graph. Policy extension keeps retained input readable after
its old attempt deadline; a delayed native path refuses publication when a
required but hidden input expires.

The shared raw-fact recall predicate now honors manifest config paths before
ranking/graph admission. Positive, prefix-excluded and literal `%` selectors
are tested through recall and graph APIs. Combined unions above 5,000 candidates
or 64 MiB of descriptor inputs fail explicitly while narrower views succeed.

The first fairness fixture omitted repository applicability on its synthetic
claim. Correcting that fixture produced a pass in 1.46 seconds; its failed
UUID-owned database was retained. The first browser fixture used the reserved
build-directory name `target`, correctly excluded by discovery. Renaming the
owned fixture checkout `callee` fixed the test without changing discovery.

The native and ordinary graph browser workflows passed together in 17.5 seconds,
including the recorded/reverse direction labels. The native flow publishes two
real local Git repositories through the paired companion and Enola, verifies a
three-hop path and source witness, preserves the dirty checkout, and makes zero
model calls. Evidence lives in `.cache/cross-graph-proof/ui.log`,
`native-{desktop,mobile}.png` and its UUID-owned browser fixture `proof.json`.
The complete platform regression passed: 79 tests, zero failures and one unrelated
live OIDC test filtered, in 221.27 seconds. Thirteen ordinary workspace tests passed,
including native committed extraction in 2.52 seconds. Clippy passed with warnings
denied after two style corrections. Governance passed all 32 tests. Logs are
`platform.log`, `workspace.log`, `clippy.log` and `governance.log` in the same cache.

Normal migration 019 applied after a private 611,118-byte PostgreSQL backup.
The restarted local app listened at `http://127.0.0.1:8787` at 18:00:55 UTC.
Actual API/Chrome verification at 18:01:02 preserved all seven Brains and SWEG's
exact snapshot/manifest and 99-node/183-edge graph, and verified the new combined
selector on desktop/mobile. The normal recall preservation proof at 18:01:45
also retained strict proposal exclusion, autonomous revision history and session
capture. Model request counts were unchanged: no new model requests and no
customer source-file reads. Evidence: `migration019.log`, `runtime-start.log`,
`runtime.json`, `runtime-{desktop,mobile}.png`, `recall/state.json` and `recall.log`.

The [combined slice](../roadmap/execution/archive/graph-cross-repository-views.md)
is shipped locally. Native combined paths were proved with synthetic publications;
no new SWEG publication was made to retrofit witnesses onto old facts. Analytics,
graph exploration and graph retrieval fusion remain required successors.
