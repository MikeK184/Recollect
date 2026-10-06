# Brain pages: UI review and proposed direction

Status: implemented locally; original proposal retained

Date: 2026-10-03. Requested scope: inspect Ask, Memory, Sources, Graph and
Repositories, including their secondary views; consider a live Activity/dashboard
as the Brain landing page. The user explicitly requested a second UI reviewer.
The user subsequently approved implementation. The [archived delivery pack](../roadmap/execution/archive/desktop-brain-pages.md) records the accepted result, live proof and remaining scope. The proposal and original audit below are retained as historical design evidence.

## Evidence boundary

The primary agent inspected the locally deployed owner session at
`http://127.0.0.1:8787`, using authorized reads and cancelled forms. The secondary
UI agent independently reviewed the current source, governing contracts and
proposal. The secondary agent had no independent live browser session.

The inspected SWEG — test Brain returned 90 memory candidates, 31 sources and
79 registered repositories. Memory is paged at 20 records; Sources at 50.
The Graph overview displayed 111 eligible entities and 86 relationships.
Its independently materialized generation recorded 113 nodes and 88 edges;
these are different scopes, not interchangeable counters. The pipeline returned
the latest bounded 30 entries. All are dated observations, not capacity proof.

The prototype uses captured, authorized example data and makes no network calls,
model requests, writes or invented live activity. It is visibly labelled as a
snapshot. Captured content is exported proposal material, not a current
authorization/expiry guarantee for a running product. Live product views retain
their current authority, retention and validity checks.

Local audit artifacts are in `.cache/brain-pages-audit/`. The editable proposal
is `.cache/brain-ui-proposal-20261003.html`, with renders and focused interaction
proof in `.cache/brain-ui-proposal-shots/`. These are ignored local artifacts.
The initial proposal review did not change application source or services. Implementation and subsequent deployed validation are recorded in the delivery pack.

## Views inspected

| Surface | Browser inspection and source review |
| --- | --- |
| Ask | Question welcome/composer, scope drawer, Search evidence, actual lexical result and advanced search; answer/citation/failure lifecycle reviewed in source, without starting an answering provider request. |
| Memory | All/Claims/Decisions/Procedures/Handovers, filters, Add memory, lineage inspector, exact history/actions and conflict review. Empty kind and generated-handover states were inspected; generation/correction/erasure were not executed. |
| Sources | List, filters, Manage views, import methods/text-file controls/reference-only form, content/derived-memory lineage, exact version/history/actions. No import, reprocess, update or removal was saved. |
| Graph | Canvas, current-view list, entity selection, Filters, Find path, Insights, Graph status and eligible-entity pages. Bounded read-only graph exploration supplied the real neighbourhood example. No rebuild or analytical job was started. |
| Repositories | Registered repositories, private checkout view, identity inspector, snapshots, native publication instructions, Environments and exact manifest history. A populated snapshot's Files/Facts/Coverage/Contributors/Insights/Receipt were inspected. Publication/edit/erase were not executed. |
| Activity | Overview, Pipeline, Processing, Tool calls, Model usage, Data removal and Timeline. Existing authorizations and current blocker distinctions were reviewed. |

## Recommended structure

Open a Brain on **Dashboard**. Make the current real processing pipeline the
dominant content, with recent inputs and exact recorded outcomes. Keep Ask one
click away. Operational Activity sections remain secondary, deep-linkable
detail surfaces rather than seven equal homepage tabs. The prototype illustrates
Dashboard and Activity details separately; they may share the existing Activity
route owner rather than duplicating queries or evidence handling.

Keep the four Knowledge destinations useful and reachable. Memory answers
“what was learned”; Sources answers “where did it come from”; Graph explains
recorded relationships; Repositories fixes knowledge to committed revisions.
They are inspection tools rather than a mandatory daily inbox. Dashboard and
Ask are the normal daily path. Do not make users scroll through their complete
capture history to understand the Brain.

Preserve the original cream/ink/sage palette, flat surfaces, readable density,
consistent icons and shared inspector. Do not add decorative 3D or a new
rendering stack. The current Motion, React Flow and Cytoscape integrations are
appropriate for the respective existing surfaces; this proposal introduces no
new package or external interface assumption.

## Page proposals

### Dashboard and motion

- Active recorded work first; otherwise explicitly show last recorded activity.
  Several active inputs may use compact parallel lanes, each with actual person,
  reported host/agent, input, stage and timestamp. Do not infer online presence
  from an enabled credential or merge distinct input runs.
- A focused pipeline explains one exact source version/job/learning run. Its
  recorded outcomes retain exact run/source IDs; available claim-ID links explicitly open current memory, because the pipeline does not return output revision IDs. A run may produce zero memories.
  Proposed and conflicting counts may overlap: the captured example is
  `0 accepted · 3 proposed (3 conflicting)`, not six memories.
- Display only known contributor information. Unknown subagent/host remains
  unknown. Within a Brain, authorized shared contributions may span users;
  private task context and foreign credential details remain excluded.
- Animate only newly observed state changes, processing whose live state is
  actually recorded, and selection/navigation transitions. Completed history
  and static knowledge edges stay still. Failed/expired/forbidden reads clear
  payloads and stop motion. Pause hidden-page motion and respect reduced motion.
- No “today”, total-learning, live-user, throughput or completeness claims
  derived from the bounded latest-30 window. Graph generation remains independent
  of a particular source run. Exceptions mean current blockers, not any old error.

### Ask and Search evidence

- Replace duplicate introductions with a compact Question / Search evidence
  switch and one clear composer. Empty title: **Ask this Brain**.
- Keep `Temporary · Each question stands alone` visible. Earlier turns are not
  sent as model context. Put the longer read-only/no-memory-write explanation
  behind an information disclosure.
- Keep applied scope and uncertainties visible. Move advanced operational
  options into the existing drawer; do not change retrieval defaults or policy.
- Answers should lead with readable supported text; citation selection opens
  the exact snippet, source version/location and qualifications in the shared
  inspector. Loading means awaiting a validated result, not token streaming.
- Retain Matches, Disagreements, Sources and Copy context in Search. Preserve
  exact/text search when answering is unavailable. No question/answer text in URLs.

### Memory

- Show subject/property **and actual value**. Three current React rows share the
  same subject/property, so the existing list hides their meaningful difference.
- Use compact rows: readable assertion; kind, contributor and recorded date;
  current exceptions visible. Keep review, freshness and operational evidence
  independently readable in inspection rather than treating acceptance as truth
  or deployment proof.
- Keep Claims/Decisions/Procedures/Handovers as filters within the same library.
  Generated-handover attempts/history become secondary to handover records.
- Stable selection plus one inspector for support, qualifications, history and
  correction actions. Do not silently merge records, hide disputes or promote
  extracted tool output to accepted facts through presentation.

### Sources

- Dense rows: title, type, contributor, processing, version count and date.
  Repeated `opencode tool_result` titles need attribution/time/version context.
  Use only authorized identity information, not guessed agent/device names.
- One inspector owns content, linked memories, versions and actions. Avoid
  one source drawer opening a competing viewer for the same selection.
- Linked memories show their own proposed/conflicting qualifications. A cited source does not prove automatic derivation. Shared
  source ancestry does not make them independent corroboration.
- Keep title-only search honestly labelled. Full evidence search remains Ask's
  Search mode. Reference-only, unavailable bytes, processing and failure remain
  distinct. Organize/import/version controls stay contextual.

### Graph

- The canvas is the primary view. Remove the competing Canvas/List presentation
  switch and duplicate list entry points; keep one optional **Entities** panel
  within the canvas chrome for textual selection and paths.
- Clearly separate **Displayed graph** from **All eligible entities**. Today's
  canvas list filters the displayed nodes; the other drawer pages eligible
  entities. Neither is a whole-Brain server search merely because it has an input.
- Use explicit scope and readable, focused neighbourhoods. The prototype shows
  a real source-centred one-hop read: four entities and three supported-by edges.
  Canonical edge direction is assertion → source. Keep overview reachable.
- The selected read reports partial coverage. Preserve a persistent indicator
  and disclose its actual reasons: source text not fully indexed, fragment
  truncated and projection inputs outside the current view.
- Current limits are 500 displayed nodes / 2,000 edges. Oversized reads should
  guide narrower repository/area/scope or entity exploration, not render an
  unlabelled prefix or invented clusters. Do not animate static edges as work.

### Repositories and snapshot workspace

- Flatten the nested Repositories/Published repositories navigation. Keep
  Repositories / Environments at the top, with private Your checkouts secondary.
- Lead with readable repository identity; keep the full normalized canonical
  origin available so similar names remain distinguishable. Move UUID and Add
  origin out of every repeated row.
- Use an exact snapshot workspace for Files/Facts/Coverage/Contributors.
  Insights and Receipt are secondary details. Facts lead with structured names,
  relationships and file/line location; raw JSON remains expandable.
- Show contributor names through an authorized scoped identity projection;
  never substitute device IDs or presumed person identity. Preserve contributor
  history and committed-bytes versus dirty-working-copy distinctions.
- Keep committed, desired configuration and observed deployment separate.
  No retained file text remains explicit; publication instructions describe
  actual native behavior and do not imply that the browser reads local files.

## Copy reduction

Cut repeated page subtitles, instructional footers, duplicate section headings
and repeated UUID disclosures. Specific examples: `More filters` → `Scope` on
Ask, `Browse eligible entity pages` → `Entities`, and `Manage views` → `Organize`.
Keep consequential constraints near the affected action: temporary independent
questions, partial coverage, missing bytes, revision meaning, effects,
uncertainty, access and destructive-action previews.

## Scale: presentation versus backend work

Existing Memory/Sources pagination and filtering before pagination remain the
baseline. Contributor filters, richer sorts or time-range filters require
supported server queries. Never filter one fetched page and label it all results.
The prototype explicitly says when it searches a captured page.

The repository catalogue currently loads every repository. A large catalogue
needs an explicit searchable, paginated repository endpoint and bounded request
behavior. CSS virtualization alone does not fix that query. Snapshot lists,
files, facts and contributors already have separate pagination.

Preserve the record someone is reading when new activity arrives. Offer a quiet
new-records indicator rather than moving the selected list under the pointer.
Avoid per-row lookups and loading the whole Brain into a browser cache. Filters,
record selection, Back/forward and exact deep links must remain consistent.

## Implementation and validation boundary

Before product changes, reconcile the proposed dashboard default with
[ADR 0017](../adr/0017-desktop-knowledge-and-ask-experience.md),
[desktop experience](../contracts/desktop-experience.md) and
[tiered knowledge](../contracts/desktop-knowledge-surface.md), which currently
specify Ask as the default. Update the owning visual epic and create a
decision-complete pack for the authorized slice. Preserve
[temporary answers](../contracts/retrieval-answers.md) and
[bounded graph exploration](../contracts/graph-exploration.md).

Proposal checks covered page switching, exact source links, preserved searches,
kind/search intersection and layout overflow. The second reviewer identified
and checked source/version mismatches, edge direction, partial coverage,
overlapping counts and missing investigation controls. Rendered views were
visually inspected. These checks prove the prototype, not a product implementation.

Product acceptance should include 1,000+ records across pages, duplicate long
titles, multiple contributors, bounded active pipelines, empty kinds, oversized
graphs, missing bytes, failed/expired/no-access states, Brain switching and
desktop layouts. Prove no synthetic activity or provider calls on navigation;
run focused backend/browser proof, typecheck, required Rust checks for affected
code, `./scripts/validate.sh` and `git diff --check`.

Ambient TV acceptance, visual epic closeout and the deferred claude-mem research
backlog are separate from this proposal and remain unchanged.
