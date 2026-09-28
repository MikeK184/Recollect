# Analyze exact eligible graph inputs

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and Prerequisites

Run PageRank centrality, Leiden communities or connected components through the
existing local Neo4j/GDS service. The API and worker must be running, and the
selected repository, combined manifest or knowledge generation must be ready.
Start the local application with `./scripts/dev.sh` and open
`http://127.0.0.1:8787`. Credentials remain in the ignored `.env`.

These reports describe recorded topology or evidence relationships. They do not
accept claims, establish operational impact or call an LLM. Normal autonomous
memory maintenance continues under the Brain's standing policy.

## Procedure

1. Open **Graph → Filters** in the Brain. Select its graph kind, exact
   repository snapshot or environment manifest, evidence mode and scope. Wait
   for current projection processing to finish, including automatic rebuilds
   after a correction.
2. Open **Insights** and choose at least one **Graph relation** explicitly. Structural relations and
   knowledge provenance form separate analytical inputs. Combined repository
   reports may include validated `terraform_module` links.
3. Select **Analysis** and **Analysis direction**. PageRank supports outgoing,
   incoming and both; communities and connected components use both. Every
   recorded edge has unit weight, including parallel edges and self loops.
   Isolated eligible vertices are retained.
4. Choose **Queue analysis**. Writers can queue and cancel; readers can open
   eligible reports. The shared heavy worker processes one job at a time. Watch
   progress and cleanup state, then **Open analytical report**.
5. Expand **Exact analytical inputs and parameters** to inspect the actual GDS
   version, recipe, generation, exact inputs and coverage. **Inspect analytical
   evidence** opens the retained canonical evidence for a result. Group numbers
   belong only to that report.
6. **Refresh report** reads current state without queuing another calculation.
   If inputs change, old numerical rows disappear. After current graph processing
   finishes, queue a fresh report. A newer ready generation can invalidate a
   report even when the displayed eligible vertices look unchanged.

Reports admit at most 10,000 canonical candidates, 50,000 selected relationships
and a combined 64 MiB native memory estimate. The read page contains at most 100
results and the metadata page at most 20 reports. Oversized inputs are refused;
the application does not publish a truncated aggregate. Fixed PageRank iterations
are not a convergence guarantee. Partial extraction remains explicitly partial.

## Verification

The [contract](../contracts/graph-analytics.md) defines the complete behavior and
the [evidence mapping](../mappings/graph-analytics-interfaces-2026-09-15.md)
records executed results and limitations. With repository-owned services running:

```sh
set -a
source .env
set +a
cargo test -p recollect-server --test platform graph_analytics_ -- --ignored --test-threads=1
./scripts/test-ui.sh tests/graph-analytics.spec.ts
./scripts/validate.sh
```

These checks use isolated databases and synthetic native graphs. They cover
complete input invalidation, ordinary-reader concurrency, delayed native creation,
interrupted attempts, replaced leases and retained-journal recovery from an
actual older PostgreSQL copy. The termination fixture uses a bounded synthetic
stress recipe to observe cancellation; it does not change product parameters.

## Failure and Recovery

- Empty or unavailable input: finish processing and choose a ready selection
  with retained evidence. Strict accepted mode excludes proposed claims.
- Stale or removed report: queue a new report after current graph processing.
  Old scores cannot be salvaged by hiding the removed contributor.
- Resource refusal: narrow scope or relationships. Native estimates are an
  admission check, not a guarantee against all memory pressure.
- Cancellation or cleanup pending: cancellation is requested immediately;
  Neo4j can take time to retire its transaction. Automatic maintenance retries
  exact scratch cleanup. New allocation and privacy completion wait for verified
  absence. Do not interpret a cancelled job as proof of physical removal.
- Missing or conflicting ownership: restore the matching private journal.
  Keep `.data/erasure-journal`, including its `analytics-<installation>` directory,
  together with canonical database and artifact backups. Do not fabricate an
  installation marker or remove an unexpired attempt record to bypass recovery.
- Older database restore: keep the newer retained journal, stop API/worker
  serving and run `recollect-server privacy-reconcile` with the operator database
  configuration loaded, following the [privacy runbook](retention-and-erasure.md).
  Reconciliation uses journal ownership even when the database predates an
  analytical attempt. Preserve uncertain state for retry; do not delete native
  graphs by prefix or touch another installation's catalog entries.

Each attempt has a 120-second deadline and separately bounded compute/control
queries. Closed ownership remains retained through the creation deadline so
delayed requests cannot resurrect untracked scratch state. Reports are capped at
100 per Brain with 20 pending; terminal reports are pruned after seven days or
when capacity is needed. Active work and unresolved scratch are not evicted.
