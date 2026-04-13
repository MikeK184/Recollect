# Investigate recalled memory on desktop

## Prerequisites

Run `./scripts/dev.sh`, open `http://127.0.0.1:8787`, sign in and select a Brain.
Start with [canonical recall](exact-lexical-recall.md); optional
[semantic](semantic-recall.md) and [graph recall](graph-recall.md) use the same
selection and lifecycle. Investigation does not require manual review of normal
learning or maintenance. Current UI work targets desktop.

## Search and compare

Enter a query or exact identity in **Recall memory**. Select an eligibility mode,
collection, scope, optional exact manifest and time filters. **Maximum results**
accepts one to twenty; the context budget includes full provenance and graph
witnesses. Press **Recall** explicitly. The selection card records the actual
Brain, named scope, manifest and server knowledge cutoff used for this response.

Use the three views without running another query:

- **Matches** shows the returned records, separate trust/lifecycle states,
  applicability, qualifications, channels and exact source locations. A policy
  acceptance is labeled separately from an authenticated account's review.
- **Disagreements** compares canonical conflict links from this result set.
  A related claim outside the bounded response has an explicit inspection link;
  opening it does not add it to context or claim it matched the query. Ten links
  are displayed per page. An empty view does not establish global consistency.
- **Sources** groups exact evidence references cited by the returned records,
  preserving used spans and repository commits. Shared citations do not establish
  independent corroboration. Missing bytes remain explicitly unavailable.

**Copy attributed context** copies the original bounded response regardless of
the active view. It does not copy extra inspected claims or sources. Clipboard
failure is displayed. External copies cannot be revoked by the browser; discard
them when scope or canonical authority changes.

## Inspect history, evidence and optional review

**Inspect claim and history** starts at the query's frozen knowledge cutoff and
fact-time selection. A claim revised after recall therefore opens the version
that the query saw. Choose **Latest knowledge** explicitly to inspect newer
information. Prior revisions, unknown validity, observation precision, review,
freshness and operational assessment remain distinct.

Procedure details include conditions, ordered steps and actual outcomes;
handovers retain contributor inspection. No execution is authorized by opening
or recalling them. **Review and corrections** uses the existing optional memory
workflow. Readers and archived Brains cannot mutate, and historical revisions
require an explicit return to current knowledge. A successful correction clears
the investigation and sibling views; submit a new query to see its effect.

**Inspect evidence** and **Inspect exact source** load the precise retained source
version or repository fact. They do not substitute newer bytes. A retained claim
may outlive its raw support; the evidence dialog explains missing/expired content.
Transient read failures hide the previous body and offer **Refresh evidence**.

## Follow recorded relationships

On a current graph-compatible result, choose **Explore relationships**. This
opens the existing native explorer around its exact revision key, even if it is
outside the graph's first entity page. The dialog preserves the parent scope,
collection, fact-time mode, exact manifest and returned repository snapshot.
Plain exact/text results can open this view without a query embedding.

The graph is a fresh current read, labeled separately from the recall cutoff.
Historical selections explain why graph exploration is unavailable. Missing or
withheld centers, incomplete projections and scope limits display errors rather
than switching to a broader graph.

Expand **Exploration settings** to change direction/hops or request an eligible
overview. Select nodes/edges to inspect evidence and choose path endpoints.
**Shortest path controls** uses the same native path handler; successful paths
can be displayed on the canvas. Connectivity explains recorded relationships,
not runtime impact or truth. Existing graph bounds and readiness procedures are
documented in [desktop graph exploration](graph-exploration.md).

## Clearing and recovery

Filter/Brain changes discard the prior response. Local mutations, observed
permission changes, advertised deadlines and remote eligibility changes clear
all investigation views and nested dialogs. The server metadata poll runs every
three seconds while results are displayed; remote changes are observed on that
bounded schedule, not through a push guarantee. No invalidation silently repeats
a paid semantic query.

If **Investigation cleared** appears, inspect the current selection and submit a
fresh query when appropriate. No-match, insufficient eligible evidence and partial
coverage have separate messages. A high similarity or a graph path does not
establish enough evidence to answer an unsupported question.

## Verification

With repository-owned services running:

```sh
./scripts/test-ui.sh tests/investigation.spec.ts tests/recall.spec.ts tests/recall-graph.spec.ts tests/graph.spec.ts tests/review.spec.ts
set -a
source .env
set +a
cargo test -p recollect-server --test platform retention -- --ignored --test-threads=1
./scripts/validate.sh
```

The fixtures are synthetic and require no actual model charge. The
[dated evidence](../mappings/retrieval-investigation-2026-09-22.md) records real
database/native/browser calls, controlled browser error/deadline cases and the
normal retained-runtime check. Changes remain local and uncommitted.
