# Explore exact combined repository graphs

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and Prerequisites

Use the [canonical graph runtime](graph-projection-and-traversal.md) with two or
more processed exact repository snapshots in a Brain. Neo4j performs traversal;
the companion uses Enola and Tree-sitter/HCL for parsed source evidence. This
operation makes no model request and does not resolve remote Git refs.

The worker discovers complete manifest sets and projects cross links automatically.
Old publications without parsed witnesses remain queryable but cannot establish
remote links from source hints alone.

## Procedure

1. Publish the committed repositories through the paired companion. Use a literal
   Git module source with a full commit and optional subdirectory. The target
   repository must be registered in the Brain and its snapshot must match that
   commit. Branch/tag/registry addresses stay unresolved.
2. Create an environment manifest with one exact snapshot per repository. Empty
   configuration paths select the whole snapshot; otherwise select literal files
   or directories. Descendants match at a `/` boundary, without glob expansion.
3. Open **Graph → Filters**, choose **Combined repositories**, and select
   **Graph environment** and **Graph manifest**. **Graph repositories** can narrow
   the view. Load after the required inputs finish processing.
4. Inspect **Exact repository inputs** and **Unresolved repository links**. At most
   100 eligible source issues appear with an explicit total. Ambiguous targets,
   unsupported sources and mismatched commits do not become edges. Identical
   snapshot sets can share generations across environment manifests.
5. Open **Find path**, choose endpoints and **Find shortest eligible path**. Enola's HCL declarations
   point from symbols to containing directories; **both** can traverse them in
   reverse. The path displays recorded direction and marks reverse steps. A
   `terraform_module` edge means an exact declared dependency, without claiming
   initialization, deployment or runtime behavior.
6. **Inspect evidence** shows the canonical fact and parsed source witness.
   **Rebuild graph** queues work for the exact set. Polling does not repeat a
   mutation. Inspect the existing Processing panel for cancellation and retry.

## Verification

The [validation mapping](../mappings/cross-repository-interfaces-2026-09-15.md)
records executed proof and remaining closeout. Repeat the synthetic native
workflow and regressions with:

```sh
./scripts/test-ui.sh tests/graph-combined.spec.ts
./scripts/test-platform.sh
./scripts/validate.sh
```

The browser fixture publishes committed bytes from two owned local Git repositories,
preserves its dirty working copy and verifies a three-hop path without model calls.
It does not scan customer repositories.

## Failure and Recovery

- Missing input/projection: finish publication/materialization and run the worker.
  Missing manifest entries never substitute a newer snapshot.
- Unregistered alias: attach an origin only when it identifies the same repository.
  The link epoch changes and current links rebuild automatically. Conflicting
  origin ownership cannot silently merge repositories.
- Large view: narrow repositories or configuration paths. The complete view
  admits 5,000 candidates and 20,000 edges; descriptor loading also has a 64 MiB
  aggregate budget, narrowed by repository selection.
- Rejected/erased/expired input: canonical eligibility applies before traversal.
  Rebuilding cannot revive a rejected route. Every required manifest snapshot
  must remain available even when a narrower display omits its nodes.
- Backend outage/incomplete import: preserve durable work and the privacy journal,
  recover Neo4j, then retry through the application. Shared endpoints survive
  removal of another generation. Do not edit Neo4j to repair canonical evidence.

[Bounded analytics](graph-analytics.md) is available for exact eligible inputs.
[Desktop graph exploration](graph-exploration.md) is also available. Retrieval
fusion remains the next slice.
