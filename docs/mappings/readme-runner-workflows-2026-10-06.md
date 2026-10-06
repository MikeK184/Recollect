# README agent, environment and private-runner workflows

Observed: 2026-10-06
Confidence: verified

The user subsequently rejected the SVG artwork. The
[concept-imagery follow-up](readme-concept-imagery-2026-10-06.md) supersedes its
README presentation; the observations and original artifacts below remain
historical evidence.

## Sources and Method

The user requested the newest changes committed and a GitHub README with more
visuals for environments, runners and plugin/direct MCP. The accepted
[catalogue contract](../contracts/mcp-catalogue-and-profiles.md),
[private-runner contract](../contracts/mcp-vault-and-private-runners.md),
[plugin guide](../../plugins/recollect/README.md), current UI labels and epic/pack
status govern the descriptions. This is a bounded documentation follow-up in
the repository-governance epic; it introduces no product behavior or interface.

## Observations

The README now includes three editable SVG diagrams in `docs/assets/`:
agent/plugin/direct-MCP entry and separate memory/execution policy;
connection/group/environment scope and independent grants; and a private
runner's outbound request/result flow. Examples contain generic product labels,
not installation identities. OpenCode, current Connect agent wording and local
desktop acceptance replace stale introductory/status text. Active native-host
direct-token acceptance and the blocked LongMemEval pack remain explicit.

CUA rendered a local GitHub-style Markdown preview at a 1100px desktop viewport.
All eight images loaded and the page had no horizontal overflow. Each final
diagram was also rendered and captured at its native 1100px width, with complete,
readable text. SVG assets have title/description, no scripts/foreign objects or
external references. PNG captures are retained in
`output/readme-workflows-2026-10-06/`; the README embeds the SVG originals.

Focused checks passed: 39 README local links/anchors, four valid described SVGs,
eight permission-projection tests, two private-binding/legacy-summary Rust tests,
web design/type/build, Rust formatting, Python compilation, shell syntax and
governance lint with 32 tests. The runner UI test's outdated explanatory-footer
assertion now checks the actual empty relationship state and setup action.
Pending source/docs and JSON proof files were scanned without credential findings.

## Translation and Limits

The prior UI/Ubuntu proof records retain their original observations. A fresh
isolated MCP browser run could not start because the Docker daemon was stopped;
this task does not assert renewed live runner connectivity. No provider calls,
installation changes, external publication or deployment occurred. The temporary
preview tab and viewport override were removed. Version N/A; the user-requested
local commit is identified by the containing Git history. No push is included.
