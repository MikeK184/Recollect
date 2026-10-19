# 0002: Local CodeGraph Navigation

Status: superseded

## Decision

Use CodeGraph as a local development CLI for Recollect. Pin version 1.6.0
under `.codex/tools/codegraph/`, with a repository-owned launcher and npm
lockfile. Store the disposable index in Recollect's ignored `.codegraph/`.
Explicitly include the separate, ignored `cognee/` Git checkout through the
root `codegraph.json`; preserve its files and Git state.

Use CLI commands directly from this session and document them in root agent
instructions. Keep installation, cache, temporary files, and index writes
inside Recollect. Disable telemetry, automatic update checks, and fallback
downloads. Refresh explicitly with `sync`; no background watcher or additional
MCP registration is part of this setup.

## Why

On 2026-09-13 the user approved setting up CodeGraph after inspection found a
working 1.6.0 CLI linked to another repository and no Recollect/Cognee index.
A local pinned copy removes that unrelated repository dependency. Symbol and
caller navigation can help locate extension points in Cognee's existing
permission, dataset, and session code.

## Consequences

The navigation contract it referenced was removed (see the runbook history).
This supplemented [ADR 0001](0001-repository-governance.md)
for developer tooling only. The graph describes the local working tree and
may be incomplete or stale. It is not Recollect's product extractor, committed
snapshot publication, authorization evidence, or a replacement for source/tests.

## Supersession

Superseded on 2026-10-19 by the [development tooling runbook](../runbooks/development-tooling.md),
which records what is installed and how to verify it. This ADR added an optional
developer CLI without replacing bootstrap authority or the product extraction
baseline. The original text and its contract remain at commit
[`f1b6bb3`](https://github.com/MikeK184/Recollect/tree/f1b6bb3040dfe65b7fac90359af782cec1f7df31/docs).
