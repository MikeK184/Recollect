# Governance Validation

## Purpose and Prerequisites

Run Bash and Python 3.11+ from any working directory using the script's absolute
path, or run this from the repository root:

```sh
./scripts/validate.sh
```

The default run checks document shape only: governed files, links and metadata.
It does not compile or test any Rust or web code, so a passing run says nothing
about whether the application builds. It uses only the Python standard library,
runs checker tests in disposable temporary directories, and does not install
packages, call a network service, traverse Cognee, or select an application stack.

To also check code, add `--code`:

```sh
./scripts/validate.sh --code
```

After the document checks it runs `cargo test --workspace --lib` and
`npm --prefix web run typecheck`, so it needs the Rust toolchain and installed web
dependencies. It runs no integration, platform or browser tests.

## Checks

- Required documents, templates, metadata, sections, and local Markdown links.
- Accepted authority links; pending foundations reported and rejected as
  governing sources; in-progress and shipped slices need accepted authority.
- Unique epic/slice/pack identities, exact owning-epic relationship, statuses,
  and active-pack plus epic index agreement.
- Slice Execution values: `pack` (an active pack file must exist; never for a
  shipped slice), `inline: <reason>` (no pack) and `archived` (shipped only, no
  pack). Active packs need the slim sections (Summary, Governing Sources, Scope,
  Decisions, Acceptance) and may add more; in-progress packs have them filled.
- Every mapping has an `Observed:` line; a mapping over 5,000 words prints a
  non-failing warning.
- Presence of the Codex configuration and role files; their contents and the
  MCP stanzas are not compared (see the development tooling runbook).
- Parent-repo ignore rule for the separate Cognee checkout.

The checker accepts the documented simple Markdown tables and inline links.
Use one row per record without pipe characters inside table cells. Metadata
uses one `Key: value` line above the first second-level heading. It checks local
file targets, not heading anchors or remote URL availability. It does not judge
semantic completeness, factual accuracy, or implementation correctness.

## Expected Result and Recovery

Success prints `Governance lint passed`, reports any pending foundations, and
runs the fixture tests successfully. Failure prints a file and actionable
reason, returning nonzero. Correct the governing record and all affected
indexes together, then rerun. Do not weaken the checker to hide unfinished work.

For only the current documentation/configuration check:

```sh
./docs/tools/lint_agent_governance.sh
```

The [GitHub Pages workflow](../../.github/workflows/pages.yml) runs this command
alongside the public site build/check. For guide changes, also run
`npm --prefix site run build` and `npm --prefix site run check` locally. Verify
Context7 separately using the
[Codex setup procedure](../../.codex/README.md).
