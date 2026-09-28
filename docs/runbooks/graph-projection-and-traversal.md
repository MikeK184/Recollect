# Explore canonical graph evidence

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and Prerequisites

Use the locally delivered repository and knowledge projections with an authorized
Brain account. Run `./scripts/dev.sh` and open `http://127.0.0.1:8787`. The API and
worker need the same PostgreSQL database, retained artifacts, erasure journal and
`NEO4J_URL`, `NEO4J_USERNAME`, `NEO4J_PASSWORD` environment configuration.
Neo4j Community supplies native traversal; no model request builds this graph.

Repository graphs require a successfully processed immutable publication.
Knowledge graphs use current claims and their recorded supports/contributions.
The worker discovers those inputs automatically under a current Brain writer.
A Brain with no claims or repository publications can have no graph generations.

## Procedure

1. Open the Brain and select **Graph → Filters**. Choose **Repository structure**
   or **Knowledge and supporting evidence**, then select scope and evidence mode.
2. For structure, select one repository and its exact **Graph snapshot**. An
   environment selection requires its exact **Graph manifest**; the snapshot
   must be an entry of that manifest. Snapshot and manifest selectors have paging.
   Development publication does not replace an older production input.
3. Choose optional areas, fact time and relation filters. Knowledge views also
   support collection selection. **Apply graph filters** returns canonical entities,
   qualified relationship counts, generation identity and coverage. Read partial
   reasons before interpreting an empty view or a path.
4. Use an entity's **Use as start** and **Use as end** buttons, choose direction
   and a maximum of one to eight hops, then **Find shortest eligible path**.
   **Inspect evidence** opens the same retained evidence used by recall. Selecting
   the same entity at both ends explicitly requests its zero-hop path.
5. Writers can request **Graph status → Rebuild graph**. The previous ready generation remains
   qualified while the new one builds. Inspect generation status and the Brain's
   Processing panel for progress, attempts, cancellation and retry. Page refresh
   does not repeat rebuild, view or path requests.

Static repository relationships establish committed structure. Knowledge edges
record `supported_by` citations and `contributed_to` handover inputs. Their
direction and evidence class are displayed; neither makes an unreviewed claim
accepted or establishes deployed behavior. Strict modes retain qualified claims
and do not promote their raw evidence to accepted knowledge.

## Verification

The [executed validation](../mappings/graph-validation-2026-09-15.md) records real
PostgreSQL/Neo4j failure, recovery, removal, scope and browser checks. The normal
SWEG test Brain produced 99 eligible entities and 183 relationships for its
retained snapshot and exact manifest on 2026-09-15. Its partial coverage remained
visible. All seven normal Brains and model request counts were preserved.

Repeatable synthetic checks are:

```sh
./scripts/test-platform.sh
./scripts/test-ui.sh tests/graph.spec.ts
./scripts/validate.sh
```

The platform script uses UUID-owned databases and actual loopback Neo4j calls.
Successful tests clean only their own graph identities; failed fixtures remain
for diagnosis. These commands do not prove future analytics or deployment.

## Failure and Recovery

- **No ready projection:** verify publication/source processing and the native
  worker. Inspect failed work before requesting rebuild or retry. Missing exact
  input never falls back to a different snapshot.
- **Partial:** inspect unresolved/ambiguous extraction targets, unavailable or
  withheld evidence, unsupported relationships and rebuild status. The service
  cannot infer missing links from similar names. A path is limited to the
  currently eligible graph and chosen hop bound.
- **Scope too large:** narrow the selection. Reads admit at most 5,000 candidates
  and 20,000 edges; generation import admits 100,000 nodes, 250,000 edges and
  64 MiB of descriptors. Overflow is an explicit error, not a truncated success.
- **Graph unavailable or physical mismatch:** check the installation health view,
  `./scripts/docker.sh compose ps` and `.cache/runtime/worker.log`. Restore the
  configured service and retry/rebuild through the application. Backend errors
  remain different from a successful no-path result. Do not edit Neo4j properties
  to repair canonical evidence or delete unrelated labels.
- **Cancelled, expired or obsolete input:** retry rechecks current authority and
  retention. A superseded generation cannot be retried; rebuild the current exact
  input. Cleanup fences old generations and preserves shared entities.
- **Erasure pending:** canonical removal applies immediately; graph cleanup must
  complete before physical erasure reports completion. Recover Neo4j and let the
  worker replay the durable journal. Keep the journal separately from backups.
  At startup, pending journal graph keys require Neo4j for the privacy barrier;
  startup refuses to enable reads if replay cannot finish. During a running
  graph outage, ordinary recall uses its independent canonical read boundary.

For delivered cross-repository linking, use the
[exact combined views procedure](graph-cross-repository-views.md). Queued analytics
and a full graph canvas remain subsequent slices. This procedure covers canonical
projection, entity/evidence browsing and bounded native paths.
